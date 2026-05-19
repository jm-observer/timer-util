mod callback;
mod config;
mod dashboard;
mod db;
mod error;
mod handlers;
mod models;
mod scheduler;

use crate::config::Config;
use crate::db::Database;
use crate::scheduler::{Scheduler, SchedulerCommand};
use actix_web::{App, HttpServer, web};
use log::LevelFilter::Info;
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let log_dir = custom_utils::args::get_user_home()
        .expect("Failed to resolve home directory")
        .join(".config")
        .join("alarm-server");
    let _ = std::fs::create_dir_all(&log_dir);
    let _ = custom_utils::logger::logger_feature_with_path(
        "alarm-server",
        "info,alarm-server=debug,alarm-client=debug,timer-util=debug",
        Info,
        log_dir.clone(),
        false,
        log_dir,
    )
    .build();
    log::info!("alarm-server starting...");

    let arg_workspace = custom_utils::args::arg_value("--workspace", "-w");

    let config = Config::load(&arg_workspace).expect("Failed to load configuration");
    log::info!("workspace: {}", config.workspace.display());

    let db = Database::new(config.db_path.to_str().unwrap()).expect("Failed to open database");
    db.initialize()
        .expect("Failed to initialize database schema");

    recover_expired_alarms(&db);

    let (tx, rx) = tokio::sync::mpsc::channel::<SchedulerCommand>(256);

    let http_client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to create HTTP client");

    let scheduler = Scheduler::new(rx, Arc::new(db.clone()), http_client);
    tokio::spawn(scheduler.run());

    let db_data = web::Data::new(db);
    let tx_data = web::Data::new(tx);

    log::info!("alarm-server listening on 0.0.0.0:{}", config.port);
    HttpServer::new(move || {
        App::new()
            .app_data(db_data.clone())
            .app_data(tx_data.clone())
            .configure(handlers::init_routes)
    })
    .bind(format!("0.0.0.0:{}", config.port))?
    .run()
    .await
}

fn recover_expired_alarms(db: &Database) {
    let active = match db.list_alarms(Some("active")) {
        Ok(a) => a,
        Err(e) => {
            log::error!("Failed to load active alarms during recovery: {}", e);
            return;
        }
    };
    let now = chrono::Local::now().naive_local();
    for alarm in active {
        if alarm.alarm_type == "once"
            && let Some(at_str) = alarm.once_at
            && let Ok(at) = chrono::NaiveDateTime::parse_from_str(&at_str, "%Y-%m-%dT%H:%M:%S")
            && at <= now
            && let Err(e) = db.update_status(&alarm.id, "completed")
        {
            log::error!(
                "Failed to mark expired alarm {} as completed: {}",
                alarm.id,
                e
            );
        }
    }
}
