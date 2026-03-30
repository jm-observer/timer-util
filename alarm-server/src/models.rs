use crate::error::AppError;
use chrono::{Local, NaiveDateTime};
use serde::{Deserialize, Serialize};
use timer_util::TimerConf;

#[derive(Debug, Deserialize)]
pub struct CreateAlarmRequest {
    pub name: Option<String>,
    pub alarm_type: String, // "cron" or "once"
    pub cron_expr: Option<String>,
    pub once_at: Option<String>, // ISO 8601 naive datetime
    pub callback_url: String,
    pub callback_body: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlarmRecord {
    pub id: String,
    pub name: String,
    pub alarm_type: String,
    pub cron_expr: Option<String>,
    pub once_at: Option<String>,
    pub callback_url: String,
    // store JSON string representation for DB simplicity
    pub callback_body: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Clone)]
pub struct AlarmResponse {
    pub id: String,
    pub name: String,
    pub alarm_type: String,
    pub cron_expr: Option<String>,
    pub once_at: Option<String>,
    pub callback_url: String,
    pub callback_body: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub next_fire_at: Option<String>,
}

#[derive(Serialize)]
pub struct AlarmListResponse {
    pub alarms: Vec<AlarmResponse>,
    pub total: usize,
}

#[derive(Deserialize)]
pub struct ListQuery {
    pub status: Option<String>,
}

impl AlarmRecord {
    /// Compute the next fire time as ISO8601 string, based on current time.
    pub fn next_fire_at(&self) -> Result<Option<String>, AppError> {
        let now = Local::now().naive_local();
        match self.alarm_type.as_str() {
            "cron" => {
                let expr = self.cron_expr.as_ref().ok_or_else(|| {
                    AppError::Validation("cron_expr missing for cron alarm".into())
                })?;
                let conf =
                    TimerConf::from_cron(expr).map_err(|e| AppError::Validation(e.to_string()))?;
                let next = conf.next_with_time(now);
                Ok(Some(next.format("%Y-%m-%dT%H:%M:%S").to_string()))
            }
            "once" => {
                if let Some(at_str) = &self.once_at {
                    let at = NaiveDateTime::parse_from_str(at_str, "%Y-%m-%dT%H:%M:%S")
                        .map_err(|_| AppError::Validation("invalid once_at format".into()))?;
                    if at > now && self.status == "active" {
                        Ok(Some(at_str.clone()))
                    } else {
                        Ok(None)
                    }
                } else {
                    Err(AppError::Validation(
                        "once_at missing for once alarm".into(),
                    ))
                }
            }
            _ => Ok(None),
        }
    }
}
