use actix_web::{web, HttpResponse};
use crate::models::{CreateAlarmRequest, AlarmResponse, AlarmListResponse, ListQuery};
use crate::dashboard::{dashboard_page, dashboard_stats, list_notifications};
use crate::db::Database;
use crate::error::AppError;
use crate::scheduler::SchedulerCommand;
use uuid::Uuid;
use chrono::{Local, NaiveDateTime};


pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/api/health").route(web::get().to(health))
    )
    .service(
        web::resource("/api/alarms").route(web::post().to(create_alarm)).route(web::get().to(list_alarms))
    )
    .service(
        web::resource("/api/alarms/{id}")
            .route(web::get().to(get_alarm))
            .route(web::delete().to(delete_alarm))
    );
    cfg.service(
        web::resource("/").route(web::get().to(dashboard_page))
    )
    .service(
        web::resource("/api/dashboard/stats").route(web::get().to(dashboard_stats))
    )
    .service(
        web::resource("/api/notifications").route(web::get().to(list_notifications))
    );
}

pub async fn health(db: web::Data<Database>) -> Result<HttpResponse, AppError> {
    let count = db.count_by_status("active").map_err(AppError::from)?;
    Ok(HttpResponse::Ok().json(serde_json::json!({"status": "ok", "active_alarms": count})))
}

pub async fn create_alarm(
    body: web::Json<CreateAlarmRequest>,
    db: web::Data<Database>,
    tx: web::Data<tokio::sync::mpsc::Sender<SchedulerCommand>>,
) -> Result<HttpResponse, AppError> {
    // Validate alarm_type
    let alarm_type = body.alarm_type.as_str();
    if alarm_type != "cron" && alarm_type != "once" {
        return Err(AppError::Validation("alarm_type must be 'cron' or 'once'".into()));
    }
    // Validate fields according to type
    if alarm_type == "cron" {
        let expr = body.cron_expr.as_ref().ok_or_else(|| AppError::Validation("cron_expr required for cron alarm".into()))?;
        // Validate cron expression using TimerConf
        timer_util::TimerConf::from_cron(expr).map_err(|e| AppError::Validation(e.to_string()))?;
    } else {
        // once
        let once_str = body.once_at.as_ref().ok_or_else(|| AppError::Validation("once_at required for once alarm".into()))?;
        let at = NaiveDateTime::parse_from_str(once_str, "%Y-%m-%dT%H:%M:%S")
            .map_err(|_| AppError::Validation("invalid once_at datetime format".into()))?;
        if at <= Local::now().naive_local() {
            return Err(AppError::Validation("once_at must be a future time".into()));
        }
    }
    // callback_url must not be empty
    if body.callback_url.trim().is_empty() {
        return Err(AppError::Validation("callback_url cannot be empty".into()));
    }
    // Build AlarmRecord
    let id = Uuid::new_v4().to_string();
    let name = body.name.clone().unwrap_or_default();
    let alarm = crate::models::AlarmRecord {
        id: id.clone(),
        name,
        alarm_type: alarm_type.to_string(),
        cron_expr: body.cron_expr.clone(),
        once_at: body.once_at.clone(),
        callback_url: body.callback_url.clone(),
        callback_body: body.callback_body.as_ref().map(|v| serde_json::to_string(v).unwrap()),
        status: "active".to_string(),
        created_at: "".to_string(), // will be set by DB
        updated_at: "".to_string(),
    };
    // Insert into DB using blocking thread
    let db_clone = db.clone();
    let alarm_insert = alarm.clone();
    let _ = web::block(move || db_clone.insert_alarm(&alarm_insert)).await.map_err(|e| AppError::Internal(e.to_string()))?;
    // Notify scheduler to reload
    let _ = tx.send(SchedulerCommand::Reload).await.map_err(|_| AppError::Internal("Failed to notify scheduler".into()))?;
    // Compute next fire time
    let next = alarm.next_fire_at().map_err(|e| AppError::Internal(e.to_string()))?;
    let resp = AlarmResponse {
        id: alarm.id,
        name: alarm.name,
        alarm_type: alarm.alarm_type,
        cron_expr: alarm.cron_expr,
        once_at: alarm.once_at,
        callback_url: alarm.callback_url,
        callback_body: alarm.callback_body,
        status: alarm.status,
        created_at: alarm.created_at,
        updated_at: alarm.updated_at,
        next_fire_at: next,
    };
    Ok(HttpResponse::Created().json(resp))
}

pub async fn list_alarms(
    query: web::Query<ListQuery>,
    db: web::Data<Database>,
) -> Result<HttpResponse, AppError> {
    let status_opt = query.status.as_deref();
    let alarms = db.list_alarms(status_opt).map_err(AppError::from)?;
    let mut resp_alarms = Vec::new();
    for a in alarms {
        let next = a.next_fire_at().map_err(|e| AppError::Internal(e.to_string()))?;
        let resp = AlarmResponse {
            id: a.id,
            name: a.name,
            alarm_type: a.alarm_type,
            cron_expr: a.cron_expr,
            once_at: a.once_at,
            callback_url: a.callback_url,
            callback_body: a.callback_body,
            status: a.status,
            created_at: a.created_at,
            updated_at: a.updated_at,
            next_fire_at: next,
        };
        resp_alarms.push(resp);
    }
    let total = resp_alarms.len();
    let list_resp = AlarmListResponse {
        alarms: resp_alarms,
        total,
    };
    Ok(HttpResponse::Ok().json(list_resp))
}

pub async fn get_alarm(
    path: web::Path<String>,
    db: web::Data<Database>,
) -> Result<HttpResponse, AppError> {
    let id = path.into_inner();
    let alarm = db.get_alarm(&id).map_err(AppError::from)?
        .ok_or_else(|| AppError::NotFound(format!("Alarm {} not found", id)))?;
    let next = alarm.next_fire_at().map_err(|e| AppError::Internal(e.to_string()))?;
    let resp = AlarmResponse {
        id: alarm.id,
        name: alarm.name,
        alarm_type: alarm.alarm_type,
        cron_expr: alarm.cron_expr,
        once_at: alarm.once_at,
        callback_url: alarm.callback_url,
        callback_body: alarm.callback_body,
        status: alarm.status,
        created_at: alarm.created_at,
        updated_at: alarm.updated_at,
        next_fire_at: next,
    };
    Ok(HttpResponse::Ok().json(resp))
}

pub async fn delete_alarm(
    path: web::Path<String>,
    db: web::Data<Database>,
    tx: web::Data<tokio::sync::mpsc::Sender<SchedulerCommand>>,
) -> Result<HttpResponse, AppError> {
    let id = path.into_inner();
    let existed = db.delete_alarm(&id).map_err(AppError::from)?;
    if !existed {
        return Err(AppError::NotFound(format!("Alarm {} not found", id)));
    }
    // Notify scheduler
    let _ = tx.send(SchedulerCommand::Reload).await.map_err(|_| AppError::Internal("Failed to notify scheduler".into()))?;
    Ok(HttpResponse::NoContent().finish())
}
