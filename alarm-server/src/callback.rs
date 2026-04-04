use crate::models::AlarmRecord;
use reqwest::Client;
use std::time::Duration;
use std::sync::Arc;
use chrono::Utc;
use crate::db::Database;
use log::{info, warn, error};
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

/// Send the HTTP POST request for the alarm callback.
async fn send_request(client: &Client, alarm: &AlarmRecord) -> Result<reqwest::Response, reqwest::Error> {
    let mut req = client.post(&alarm.callback_url);
    req = req.header("Content-Type", "application/json");
    req = req.header("X-Alarm-Id", &alarm.id);
    req = req.header("X-Alarm-Name", &alarm.name);
    // Use provided callback_body JSON string, or null if None
    let body = alarm.callback_body.clone().unwrap_or_else(|| "null".to_string());
    let resp = req.body(body).send().await?;
    Ok(resp)
}

/// Fire the callback with exponential backoff retry logic.
/// `cancel` token can be used to abort the retry loop (e.g., when a cron alarm is rescheduled).
pub async fn fire_callback(client: &Client, alarm: &AlarmRecord, cancel: CancellationToken, db: Arc<Database>) {
    // Record the trigger timestamp
    let triggered_at = Utc::now().naive_utc().format("%Y-%m-%dT%H:%M:%S").to_string();
    let mut attempt: i32 = 1;
    let alarm_name = alarm.name.clone();
    let alarm_id = alarm.id.clone();
    let callback_url = alarm.callback_url.clone();
    // Loop start
    let mut retry_interval = Duration::from_secs(5);
    let max_interval = Duration::from_secs(600); // 10 minutes

    loop {
        // Determine status and log before sending? We'll send then log based on result.
        match send_request(client, alarm).await {
            Ok(resp) if resp.status().is_success() => {
                info!("Alarm '{}' callback succeeded", alarm.id);
                // Insert success log
                let log = crate::models::NotificationLog {
                    id: 0, // placeholder, autoincrement
                    alarm_id: alarm_id.clone(),
                    alarm_name: alarm_name.clone(),
                    callback_url: callback_url.clone(),
                    status: "success".to_string(),
                    http_status: Some(resp.status().as_u16()), // need i32? Actually field is Option<u16>
                    error_message: None,
                    attempt,
                    triggered_at: triggered_at.clone(),
                    completed_at: Utc::now().naive_utc().format("%Y-%m-%dT%H:%M:%S").to_string(),
                };
                let _ = db.insert_notification_log(&log);
                break;
            }
            Ok(resp) => {
                warn!("Alarm '{}' callback got status {}", alarm.id, resp.status());
                // Insert retrying log with status "retrying" and http_status
                let log = crate::models::NotificationLog {
                    id: 0,
                    alarm_id: alarm_id.clone(),
                    alarm_name: alarm_name.clone(),
                    callback_url: callback_url.clone(),
                    status: "retrying".to_string(),
                    http_status: Some(resp.status().as_u16()),
                    error_message: None,
                    attempt,
                    triggered_at: triggered_at.clone(),
                    completed_at: Utc::now().naive_utc().format("%Y-%m-%dT%H:%M:%S").to_string(),
                };
                let _ = db.insert_notification_log(&log);
            }
            Err(e) => {
                error!("Alarm '{}' callback failed: {}", alarm.id, e);
                // Insert retrying log with error_message
                let log = crate::models::NotificationLog {
                    id: 0,
                    alarm_id: alarm_id.clone(),
                    alarm_name: alarm_name.clone(),
                    callback_url: callback_url.clone(),
                    status: "retrying".to_string(),
                    http_status: None,
                    error_message: Some(e.to_string()),
                    attempt,
                    triggered_at: triggered_at.clone(),
                    completed_at: Utc::now().naive_utc().format("%Y-%m-%dT%H:%M:%S").to_string(),
                };
                let _ = db.insert_notification_log(&log);
            }
        }
        // Wait for retry interval or cancellation
        tokio::select! {
            _ = sleep(retry_interval) => {}
            _ = cancel.cancelled() => {
                info!("Alarm '{}' callback retry cancelled", alarm.id);
                // Insert cancelled log
                let log = crate::models::NotificationLog {
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
                };
                let _ = db.insert_notification_log(&log);
                break;
            }
        }
        // Increment attempt and exponential backoff
        attempt += 1;
        retry_interval = (retry_interval * 2).min(max_interval);
    }
}

