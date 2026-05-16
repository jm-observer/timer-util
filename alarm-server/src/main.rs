mod config;
mod db;
mod models;
mod handlers;
mod scheduler;
mod dashboard;
mod callback;
mod error;

use actix_web::{web, App, HttpServer};
use crate::config::Config;
use crate::db::Database;
use crate::scheduler::{Scheduler, SchedulerCommand};
use std::sync::Arc;
use reqwest::Client;
use std::time::Duration;
use log::LevelFilter::Info;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let _ = custom_utils::logger::logger_feature("alarm-server"
     , "info,alarm-server=debug,alarm-client=debug,timer-util=debug", Info, false).build();
    log::info!("alarm-server starting...");

    // Load configuration
    let config = Config::from_env();

    // Initialize database
    let db = Database::new(&config.db_path).expect("Failed to open database");
    db.initialize().expect("Failed to initialize database schema");

    // Recover expired one-time alarms
    recover_expired_alarms(&db);

    // Create scheduler channel
    let (tx, rx) = tokio::sync::mpsc::channel::<SchedulerCommand>(256);

    // Shared HTTP client for callbacks
    let http_client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to create HTTP client");

    // Start scheduler
    let scheduler = Scheduler::new(rx, Arc::new(db.clone()), http_client);
    tokio::spawn(scheduler.run());

    // Set up Actix app data
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
        if alarm.alarm_type == "once" {
            if let Some(at_str) = alarm.once_at {
                if let Ok(at) = chrono::NaiveDateTime::parse_from_str(&at_str, "%Y-%m-%dT%H:%M:%S") {
                    if at <= now {
                        if let Err(e) = db.update_status(&alarm.id, "completed") {
                            log::error!("Failed to mark expired alarm {} as completed: {}", alarm.id, e);
                        }
                    }
                }
            }
        }
    }
}
