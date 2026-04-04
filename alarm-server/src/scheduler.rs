use crate::models::AlarmRecord;
use crate::db::Database;
use crate::callback::fire_callback;
use chrono::{NaiveDateTime, Local};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::Receiver;
use tokio::task::JoinHandle;
use reqwest::Client;
use log::error;
// SchedulerCommand is defined in this module

#[derive(Debug)]
#[allow(dead_code)]
pub enum SchedulerCommand {
    Reload,
    Shutdown,
}

pub struct Scheduler {
    rx: Receiver<SchedulerCommand>,
    db: Arc<Database>,
    http_client: Client,
    active_callbacks: HashMap<String, JoinHandle<()>>,
}

impl Scheduler {
    pub fn new(rx: Receiver<SchedulerCommand>, db: Arc<Database>, http_client: Client) -> Self {
        Self {
            rx,
            db,
            http_client,
            active_callbacks: HashMap::new(),
        }
    }

    fn compute_next_fire(alarm: &AlarmRecord, now: NaiveDateTime) -> Option<NaiveDateTime> {
        match alarm.alarm_type.as_str() {
            "cron" => {
                let expr = alarm.cron_expr.as_ref()?;
                let conf = timer_util::TimerConf::from_cron(expr).ok()?;
                Some(conf.next_with_time(now))
            }
            "once" => {
                let at_str = alarm.once_at.as_ref()?;
                let at = NaiveDateTime::parse_from_str(at_str, "%Y-%m-%dT%H:%M:%S").ok()?;
                if at > now { Some(at) } else { None }
            }
            _ => None,
        }
    }

    fn trigger_alarm(&mut self, alarm: &AlarmRecord) {
        // Cancel previous callback for cron alarms
        if alarm.alarm_type == "cron" {
            if let Some(prev) = self.active_callbacks.remove(&alarm.id) {
                prev.abort();
            }
        }
        let client = self.http_client.clone();
        let alarm_clone = alarm.clone();
        let db = self.db.clone();
        let is_once = alarm.alarm_type == "once";
        // Create a cancellation token
        let cancel_token = tokio_util::sync::CancellationToken::new();
        // For cron alarms, store handle and token? We'll store handle only; cancel will be via abort of handle.
        let handle = tokio::spawn(async move {
            fire_callback(&client, &alarm_clone, cancel_token, db.clone()).await;
            if is_once {
                if let Err(e) = db.update_status(&alarm_clone.id, "completed") {
                    error!("Failed to update status for alarm {}: {}", alarm_clone.id, e);
                }
            }
        });
        if !is_once {
            self.active_callbacks.insert(alarm.id.clone(), handle);
        }
    }

    pub async fn run(mut self) {
        loop {
            // Load active alarms
            let alarms_res = self.db.list_alarms(Some("active"));
            let alarms = match alarms_res {
                Ok(a) => a,
                Err(e) => {
                    error!("Failed to list alarms: {}", e);
                    vec![]
                }
            };
            let now = Local::now().naive_local();
            // Compute next fire times
            let mut fire_list: Vec<(AlarmRecord, NaiveDateTime)> = Vec::new();
            for alarm in &alarms {
                if let Some(next) = Self::compute_next_fire(alarm, now) {
                    fire_list.push((alarm.clone(), next));
                }
            }
            // Determine nearest time
            let nearest_opt = fire_list.iter().map(|(_, t)| *t).min();
            let sleep_duration = match nearest_opt {
                Some(t) if t > now => {
                    let dur = (t - now).to_std().unwrap_or_else(|_| std::time::Duration::from_secs(3600));
                    dur.min(std::time::Duration::from_secs(3600))
                }
                Some(_) => std::time::Duration::from_secs(0),
                None => std::time::Duration::from_secs(3600),
            };
            tokio::select! {
                _ = tokio::time::sleep(sleep_duration) => {
                    let now2 = Local::now().naive_local();
                    for (alarm, fire_at) in &fire_list {
                        if *fire_at <= now2 {
                            self.trigger_alarm(alarm);
                        }
                    }
                }
                cmd_opt = self.rx.recv() => {
                    match cmd_opt {
                        Some(SchedulerCommand::Reload) => continue,
                        Some(SchedulerCommand::Shutdown) | None => {
                            // Cancel any active callbacks
                            for (_, handle) in self.active_callbacks.drain() {
                                handle.abort();
                            }
                            break;
                        }
                    }
                }
            }
        }
    }
}
