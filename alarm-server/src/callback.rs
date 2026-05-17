use crate::db::Database;
use crate::models::AlarmRecord;
use chrono::Utc;
use log::{error, info, warn};
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

/// Send the HTTP POST request for the alarm callback.
async fn send_request(
    client: &Client,
    alarm: &AlarmRecord,
) -> Result<reqwest::Response, reqwest::Error> {
    let mut req = client.post(&alarm.callback_url);
    req = req.header("Content-Type", "application/json");
    req = req.header("X-Alarm-Id", &alarm.id);
    req = req.header("X-Alarm-Name", &alarm.name);
    // Use provided callback_body JSON string, or null if None
    let body = alarm
        .callback_body
        .clone()
        .unwrap_or_else(|| "null".to_string());
    let resp = req.body(body).send().await?;
    Ok(resp)
}

const MAX_ATTEMPTS: i32 = 20;

fn log_result(db: &Database, log: &crate::models::NotificationLog) {
    if let Err(e) = db.insert_notification_log(log) {
        error!("Failed to insert notification log: {}", e);
    }
}

/// Fire the callback with exponential backoff retry logic.
/// `cancel` token can be used to abort the retry loop (e.g., when a cron alarm is rescheduled).
pub async fn fire_callback(
    client: &Client,
    alarm: &AlarmRecord,
    cancel: CancellationToken,
    db: Arc<Database>,
) {
    let triggered_at = Utc::now()
        .naive_utc()
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string();
    let mut attempt: i32 = 1;
    let alarm_name = alarm.name.clone();
    let alarm_id = alarm.id.clone();
    let callback_url = alarm.callback_url.clone();
    let mut retry_interval = Duration::from_secs(5);
    let max_interval = Duration::from_secs(600);

    loop {
        let now_str = || {
            Utc::now()
                .naive_utc()
                .format("%Y-%m-%dT%H:%M:%S")
                .to_string()
        };
        match send_request(client, alarm).await {
            Ok(resp) if resp.status().is_success() => {
                info!(
                    "Alarm '{}' callback succeeded on attempt {}",
                    alarm_id, attempt
                );
                log_result(
                    &db,
                    &crate::models::NotificationLog {
                        id: 0,
                        alarm_id: alarm_id.clone(),
                        alarm_name: alarm_name.clone(),
                        callback_url: callback_url.clone(),
                        status: "success".to_string(),
                        http_status: Some(resp.status().as_u16()),
                        error_message: None,
                        attempt,
                        triggered_at: triggered_at.clone(),
                        completed_at: now_str(),
                    },
                );
                break;
            }
            Ok(resp) => {
                warn!(
                    "Alarm '{}' callback got status {} (attempt {})",
                    alarm_id,
                    resp.status(),
                    attempt
                );
                let is_last = attempt >= MAX_ATTEMPTS;
                log_result(
                    &db,
                    &crate::models::NotificationLog {
                        id: 0,
                        alarm_id: alarm_id.clone(),
                        alarm_name: alarm_name.clone(),
                        callback_url: callback_url.clone(),
                        status: if is_last {
                            "failed".to_string()
                        } else {
                            "retrying".to_string()
                        },
                        http_status: Some(resp.status().as_u16()),
                        error_message: None,
                        attempt,
                        triggered_at: triggered_at.clone(),
                        completed_at: now_str(),
                    },
                );
                if is_last {
                    error!(
                        "Alarm '{}' callback gave up after {} attempts",
                        alarm_id, MAX_ATTEMPTS
                    );
                    break;
                }
            }
            Err(e) => {
                error!(
                    "Alarm '{}' callback error (attempt {}): {}",
                    alarm_id, attempt, e
                );
                let is_last = attempt >= MAX_ATTEMPTS;
                log_result(
                    &db,
                    &crate::models::NotificationLog {
                        id: 0,
                        alarm_id: alarm_id.clone(),
                        alarm_name: alarm_name.clone(),
                        callback_url: callback_url.clone(),
                        status: if is_last {
                            "failed".to_string()
                        } else {
                            "retrying".to_string()
                        },
                        http_status: None,
                        error_message: Some(e.to_string()),
                        attempt,
                        triggered_at: triggered_at.clone(),
                        completed_at: now_str(),
                    },
                );
                if is_last {
                    error!(
                        "Alarm '{}' callback gave up after {} attempts",
                        alarm_id, MAX_ATTEMPTS
                    );
                    break;
                }
            }
        }

        tokio::select! {
            _ = sleep(retry_interval) => {}
            _ = cancel.cancelled() => {
                info!("Alarm '{}' callback retry cancelled", alarm_id);
                log_result(&db, &crate::models::NotificationLog {
                    id: 0,
                    alarm_id: alarm_id.clone(),
                    alarm_name: alarm_name.clone(),
                    callback_url: callback_url.clone(),
                    status: "cancelled".to_string(),
                    http_status: None,
                    error_message: None,
                    attempt,
                    triggered_at: triggered_at.clone(),
                    completed_at: Utc::now().naive_utc().format("%Y-%m-%dT%H:%M:%S").to_string(),
                });
                break;
            }
        }

        attempt += 1;
        retry_interval = (retry_interval * 2).min(max_interval);
    }
}
