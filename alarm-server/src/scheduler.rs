use crate::callback::fire_callback;
use crate::db::Database;
use crate::models::AlarmRecord;
use chrono::{Local, NaiveDateTime};
use log::error;
use reqwest::Client;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::Receiver;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

#[derive(Debug)]
pub enum SchedulerCommand {
    Reload,
    #[allow(dead_code)]
    Shutdown,
}

pub struct Scheduler {
    rx: Receiver<SchedulerCommand>,
    db: Arc<Database>,
    http_client: Client,
    active_callbacks: HashMap<String, (JoinHandle<()>, CancellationToken)>,
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

    fn cancel_callback(&mut self, alarm_id: &str) {
        if let Some((handle, token)) = self.active_callbacks.remove(alarm_id) {
            token.cancel();
            handle.abort();
        }
    }

    fn trigger_alarm(&mut self, alarm: &AlarmRecord) {
        // Cancel any in-flight callback for this alarm
        self.cancel_callback(&alarm.id);

        let client = self.http_client.clone();
        let alarm_clone = alarm.clone();
        let db = self.db.clone();
        let is_once = alarm.alarm_type == "once";
        let cancel_token = CancellationToken::new();
        let cancel_child = cancel_token.child_token();
        let handle = tokio::spawn(async move {
            fire_callback(&client, &alarm_clone, cancel_child, db.clone()).await;
            if is_once {
                if let Err(e) = db.update_status(&alarm_clone.id, "completed") {
                    error!(
                        "Failed to update status for alarm {}: {}",
                        alarm_clone.id, e
                    );
                }
            }
        });
        self.active_callbacks
            .insert(alarm.id.clone(), (handle, cancel_token));
    }

    pub async fn run(mut self) {
        loop {
            let alarms = match self.db.list_alarms(Some("active")) {
                Ok(a) => a,
                Err(e) => {
                    error!("Failed to list alarms: {}", e);
                    vec![]
                }
            };

            // Cancel callbacks for alarms that are no longer active
            let active_ids: std::collections::HashSet<&str> =
                alarms.iter().map(|a| a.id.as_str()).collect();
            let removed: Vec<String> = self
                .active_callbacks
                .keys()
                .filter(|id| !active_ids.contains(id.as_str()))
                .cloned()
                .collect();
            for id in removed {
                self.cancel_callback(&id);
            }

            // Clean up finished handles
            self.active_callbacks
                .retain(|_, (handle, _)| !handle.is_finished());

            let now = Local::now().naive_local();
            let mut fire_list: Vec<(String, NaiveDateTime)> = Vec::new();
            for alarm in &alarms {
                if let Some(next) = Self::compute_next_fire(alarm, now) {
                    fire_list.push((alarm.id.clone(), next));
                }
            }

            let nearest_opt = fire_list.iter().map(|(_, t)| *t).min();
            let sleep_duration = match nearest_opt {
                Some(t) if t > now => {
                    let dur = (t - now)
                        .to_std()
                        .unwrap_or_else(|_| std::time::Duration::from_secs(3600));
                    dur.min(std::time::Duration::from_secs(3600))
                }
                Some(_) => std::time::Duration::from_secs(0),
                None => std::time::Duration::from_secs(3600),
            };

            tokio::select! {
                _ = tokio::time::sleep(sleep_duration) => {
                    let now2 = Local::now().naive_local();
                    for (alarm_id, fire_at) in &fire_list {
                        if *fire_at <= now2 {
                            // Re-verify the alarm is still active before triggering
                            match self.db.get_alarm(alarm_id) {
                                Ok(Some(current)) if current.status == "active" => {
                                    self.trigger_alarm(&current);
                                }
                                Ok(_) => {}
                                Err(e) => error!("Failed to verify alarm {} before trigger: {}", alarm_id, e),
                            }
                        }
                    }
                }
                cmd_opt = self.rx.recv() => {
                    match cmd_opt {
                        Some(SchedulerCommand::Reload) => continue,
                        Some(SchedulerCommand::Shutdown) | None => {
                            for id in self.active_callbacks.keys().cloned().collect::<Vec<_>>() {
                                self.cancel_callback(&id);
                            }
                            break;
                        }
                    }
                }
            }
        }
    }
}
