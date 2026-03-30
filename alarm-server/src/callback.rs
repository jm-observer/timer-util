use crate::models::AlarmRecord;
use reqwest::Client;
use std::time::Duration;
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
pub async fn fire_callback(client: &Client, alarm: &AlarmRecord, cancel: CancellationToken) {
    let mut retry_interval = Duration::from_secs(5);
    let max_interval = Duration::from_secs(600); // 10 minutes

    loop {
        match send_request(client, alarm).await {
            Ok(resp) if resp.status().is_success() => {
                info!("Alarm '{}' callback succeeded", alarm.id);
                break;
            }
            Ok(resp) => {
                warn!("Alarm '{}' callback got status {}", alarm.id, resp.status());
            }
            Err(e) => {
                error!("Alarm '{}' callback failed: {}", alarm.id, e);
            }
        }
        // Wait for retry interval or cancellation
        tokio::select! {
            _ = sleep(retry_interval) => {}
            _ = cancel.cancelled() => {
                info!("Alarm '{}' callback retry cancelled", alarm.id);
                break;
            }
        }
        // Exponential backoff, capping at max_interval
        retry_interval = (retry_interval * 2).min(max_interval);
    }
}
