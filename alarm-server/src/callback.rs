use crate::db::Database;
use crate::models::AlarmRecord;
use chrono::Utc;
use custom_utils::trace::{self, SpanLink, SpanStatus, TraceContext};
use log::{error, info, warn};
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

/// Send the HTTP POST request for the alarm callback.
/// 返回 (HTTP status, 响应体文本)；响应体读不到时为空串。
async fn send_request(
    client: &Client,
    alarm: &AlarmRecord,
    fire_ctx: Option<&TraceContext>,
) -> Result<(u16, String), reqwest::Error> {
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
    let status = resp.status().as_u16();
    let text = resp.text().await.unwrap_or_default();
    Ok((status, text))
}

/// 4KB 截断，避免 trace 节点被巨型响应撑爆。
fn truncate_body(s: String, limit: usize) -> (String, bool) {
    if s.len() <= limit {
        (s, false)
    } else {
        // 按字符边界切，防止 UTF-8 中间裁。
        let mut end = limit;
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        (s[..end].to_string(), true)
    }
}

/// 从 alarm 的 callback_body.metadata.traceparent 解析出本次触发的 trace 上下文。
///
/// **两种类型都用 `root_with_id` + `SpanLink`**——不再用 `continued`：
/// - alarm_fire 在 alarm 设置 N 秒/分钟/小时**之后**才发生，跟 setup 时的 tool_call
///   没时间重叠。若 parent_span_id=setup_span 会导致 UI 上"父节点 1ms 结束、子节点
///   N 秒后出现"的时间倒挂；
/// - 改成 alarm_fire 自身是该 trace 的另一个 root（同 trace_id 但 parent=None），
///   通过 SpanLink 引回 setup 时的 span（trace-hub UI 画虚线箭头），既保留因果
///   关联，又避免时间线错乱。
/// - 同 trace_id 还能让 setup 端的 trace 在搜索时把 alarm_fire 也带出来。
/// 无 traceparent 时返回 None（不记录，避免孤立 trace）。
fn fire_trace_ctx(alarm: &AlarmRecord) -> Option<(TraceContext, Option<SpanLink>)> {
    let body = alarm.callback_body.as_ref()?;
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    let tp = v.get("metadata")?.get("traceparent")?.as_str()?;
    let remote = TraceContext::from_traceparent(tp)?;
    let link = SpanLink {
        trace_id: remote.trace_id.clone(),
        span_id: remote.span_id.clone(),
    };
    // 用 root_with_id 复用同 trace_id；span_id 是新生成的（跟 setup 的 span 不冲突）；
    // parent_span_id=None 让 alarm_fire 当 trace 顶层节点，UI 不嵌套到 setup 之下。
    let ctx = TraceContext::root_with_id(remote.trace_id);
    Some((ctx, Some(link)))
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

    // 本次触发的 trace 上下文（root + SpanLink，跨异步引回 setup span，避免时间倒挂）。
    let traced = fire_trace_ctx(alarm);
    let fire_ctx = traced.as_ref().map(|(c, _)| c.clone());
    // SpanScope：两阶段 emit。第一阶段 anchor 即时 emit「闹钟正在触发」+ 回调入参，
    // 即使后续重试 20 次/几十分钟没结果，trace-hub 也立刻能看到。重试结束后 emit_end
    // 用结果（http_status / 响应 body / 错误）覆盖。
    let fire_scope = traced.as_ref().map(|(ctx, link)| {
        let schedule = alarm
            .cron_expr
            .clone()
            .or_else(|| alarm.once_at.clone())
            .unwrap_or_default();
        let scope = trace::SpanScope::new(ctx.clone(), "alarm_fire")
            .with_flow_name("闹钟触发")
            .with_summary(serde_json::json!({
                "alarm_id": alarm.id,
                "name": alarm.name,
                "alarm_type": alarm.alarm_type,
                "schedule": schedule,
                "callback_url": alarm.callback_url,
            }))
            .with_request_body(
                alarm
                    .callback_body
                    .clone()
                    .unwrap_or_else(|| "null".to_string()),
            );
        scope.emit_start();
        // SpanLink 留到 emit_end 时附带（emit_start 只占位，UI 在重试阶段就看到锚点）。
        (scope, link.clone())
    });
    let mut outcome_ok = false;
    let mut cancelled = false;
    // 给 span 记录最后一次回调的状态/响应/错误。
    let mut last_status: Option<u16> = None;
    let mut last_response_body: Option<String> = None;
    let mut last_response_truncated = false;
    let mut last_error: Option<String> = None;

    loop {
        let now_str = || {
            Utc::now()
                .naive_utc()
                .format("%Y-%m-%dT%H:%M:%S")
                .to_string()
        };
        match send_request(client, alarm, fire_ctx.as_ref()).await {
            Ok((status_code, body_text)) => {
                let (body_capped, truncated) = truncate_body(body_text, 4096);
                last_status = Some(status_code);
                last_response_body = Some(body_capped);
                last_response_truncated = truncated;
                last_error = None;
                if (200..300).contains(&status_code) {
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
                            http_status: Some(status_code),
                            error_message: None,
                            attempt,
                            triggered_at: triggered_at.clone(),
                            completed_at: now_str(),
                        },
                    );
                    outcome_ok = true;
                    break;
                } else {
                    warn!(
                        "Alarm '{}' callback got status {} (attempt {})",
                        alarm_id, status_code, attempt
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
                            http_status: Some(status_code),
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
            }
            Err(e) => {
                error!(
                    "Alarm '{}' callback error (attempt {}): {}",
                    alarm_id, attempt, e
                );
                last_status = None;
                last_error = Some(e.to_string());
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

    // 记录「闹钟触发」span 终态（取消重试不计入：anchor 在 trace 里仍存，方便排查）。
    if !cancelled && let Some((scope, link)) = fire_scope {
        let status = if outcome_ok {
            SpanStatus::Ok
        } else {
            let why = last_error
                .clone()
                .or_else(|| last_status.map(|c| format!("HTTP {}", c)))
                .unwrap_or_else(|| "callback failed".to_string());
            SpanStatus::Error(why)
        };
        scope.emit_end_full(
            last_response_body,
            status,
            Some(serde_json::json!({
                "attempts": attempt,
                "http_status": last_status,
                "triggered_at": triggered_at,
                "error": last_error,
            })),
            last_response_truncated,
            link.into_iter().collect(),
        );
    }
}
