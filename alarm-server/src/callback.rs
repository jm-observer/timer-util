use crate::db::Database;
use crate::models::AlarmRecord;
use chrono::Utc;
use custom_utils::trace::{self, SpanLink, SpanRecord, SpanStatus, TraceContext};
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
    fire_ctx: Option<&TraceContext>,
) -> Result<reqwest::Response, reqwest::Error> {
    let mut req = client.post(&alarm.callback_url);
    req = req.header("Content-Type", "application/json");
    req = req.header("X-Alarm-Id", &alarm.id);
    req = req.header("X-Alarm-Name", &alarm.name);
    // 把本次触发的 trace 上下文透传给回调方（zero gateway 据此续接同一条 trace）。
    if let Some(ctx) = fire_ctx {
        req = req.header("traceparent", ctx.to_traceparent());
    }
    // Use provided callback_body JSON string, or null if None
    let body = alarm
        .callback_body
        .clone()
        .unwrap_or_else(|| "null".to_string());
    let resp = req.body(body).send().await?;
    Ok(resp)
}

/// 从 alarm 的 callback_body.metadata.traceparent 解析出本次触发的 trace 上下文。
///
/// once：续用同 trace_id（`continued`），无 link；
/// cron：每次触发新起 trace_id + `SpanLink` 回指设置时的 span（避免一棵树无限膨胀）。
/// 无 traceparent 时返回 None（不记录，避免孤立 trace）。
fn fire_trace_ctx(alarm: &AlarmRecord) -> Option<(TraceContext, Option<SpanLink>)> {
    let body = alarm.callback_body.as_ref()?;
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    let tp = v.get("metadata")?.get("traceparent")?.as_str()?;
    let remote = TraceContext::from_traceparent(tp)?;
    if alarm.alarm_type == "cron" {
        let link = SpanLink {
            trace_id: remote.trace_id.clone(),
            span_id: remote.span_id.clone(),
        };
        Some((TraceContext::root(), Some(link)))
    } else {
        Some((
            TraceContext::continued(remote.trace_id, remote.span_id),
            None,
        ))
    }
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

    // 本次触发的 trace 上下文（once 续接 / cron link）；无 traceparent 则不记录。
    let fire_start = trace::now_ms();
    let traced = fire_trace_ctx(alarm);
    let fire_ctx = traced.as_ref().map(|(c, _)| c.clone());
    let mut outcome_ok = false;
    let mut cancelled = false;

    loop {
        let now_str = || {
            Utc::now()
                .naive_utc()
                .format("%Y-%m-%dT%H:%M:%S")
                .to_string()
        };
        match send_request(client, alarm, fire_ctx.as_ref()).await {
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
                outcome_ok = true;
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
                cancelled = true;
                break;
            }
        }

        attempt += 1;
        retry_interval = (retry_interval * 2).min(max_interval);
    }

    // 记录「闹钟触发」span（取消重试不计入；无 traceparent 不记录，避免孤立 trace）。
    if !cancelled && let Some((ctx, link)) = traced {
        let status = if outcome_ok {
            SpanStatus::Ok
        } else {
            SpanStatus::Error("callback failed".to_string())
        };
        trace::record_span(SpanRecord {
            trace_id: ctx.trace_id,
            span_id: ctx.span_id,
            parent_span_id: ctx.parent_span_id,
            service: String::new(),
            kind: "alarm_fire".to_string(),
            flow_name: Some("闹钟触发".to_string()),
            start_ms: fire_start,
            end_ms: trace::now_ms(),
            status,
            summary: serde_json::json!({ "alarm_id": alarm_id, "name": alarm_name, "attempts": attempt }),
            detail: serde_json::json!({ "callback_url": callback_url }),
            request_body: None,
            response_body: None,
            body_truncated: false,
            links: link.into_iter().collect(),
        });
    }
}
