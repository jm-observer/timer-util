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
use custom_utils::updater::{CliAction, DeployCommand, LinuxService};
use log::LevelFilter::Info;
use reqwest::Client;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

const APP_NAME: &str = "alarm-server";
const REPO_OWNER: &str = "jm-observer";
const REPO_NAME: &str = "timer-util";
const ABOUT: &str = "Alarm server: recurring/one-time alarm scheduler with HTTP callbacks, SQLite persistence and a dashboard.";

/// The host owns its top-level CLI; the unified deploy stack is embedded as a
/// single pass-through variant. `LinuxService` never reads argv for us beyond
/// `parse_deploy`, and never writes stdout — text outcomes come back for us to
/// print alongside our own usage.
enum AppCmd {
    Serve,
    Deploy(DeployCommand),
}

fn service() -> LinuxService {
    LinuxService::new(APP_NAME, REPO_OWNER, REPO_NAME, env!("CARGO_PKG_VERSION"))
        .description("Alarm Server - recurring/one-time alarm scheduler")
        .extra_bins(["alarm-cli"])
        .watchdog_sec(30)
}

/// Our own usage block; spliced ahead of the library's deploy usage on `--help`.
fn own_usage() -> String {
    format!(
        "{ABOUT}\n\n\
         Usage:\n  \
         {APP_NAME} [serve] [-w|--workspace <path>]   run the server (default; workspace default ~/.config/{APP_NAME})\n\n\
         Use `alarm-cli` to create/list/cancel alarms against a running server."
    )
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    let _ = custom_utils::logger::logger_feature(
        "alarm-server",
        "info,alarm-server=debug,alarm-client=debug,timer-util=debug",
        Info,
        false,
    )
    .build();

    let svc = service();

    let cmd = match svc.parse_deploy() {
        Some(c) => AppCmd::Deploy(c),
        None => AppCmd::Serve,
    };

    match cmd {
        AppCmd::Deploy(c) => {
            match svc.dispatch(c).await? {
                // Library did no I/O: we print, splicing our own usage in.
                CliAction::Version(v) => println!("{APP_NAME} {v}"),
                CliAction::Help(deploy_usage) => {
                    println!("{}\n\n{}", own_usage(), deploy_usage)
                }
                CliAction::DryRun(unit) => print!("{unit}"),
                // install / update already ran and logged.
                CliAction::Handled => {}
                // dispatch of a deploy command never returns Run.
                CliAction::Run { .. } => unreachable!(),
            }
            Ok(())
        }
        AppCmd::Serve => {
            // Honor `-w/--workspace` for the run path while keeping the unified
            // `~/.config/<app>` default; `svc.workspace()` stays the single
            // source of truth (matches `args::workspace` / the unit's
            // WorkingDirectory).
            let svc = match custom_utils::args::arg_value("--workspace", "-w") {
                Some(w) => svc.workspace_arg(w),
                None => svc,
            };
            let _wd = svc.spawn_watchdog();
            run_server(svc.workspace()?).await
        }
    }
}

/// 启用全生命周期追踪（仅当设置 `TRACE_HUB_ENDPOINT` 时）；未设则零影响。
fn init_trace() {
    if let Ok(endpoint) = std::env::var("TRACE_HUB_ENDPOINT") {
        custom_utils::trace::init(custom_utils::trace::TraceConfig::new(
            endpoint,
            "alarm-server",
        ));
        log::info!("trace enabled → trace-hub");
    }
}

async fn run_server(workspace: PathBuf) -> anyhow::Result<()> {
    log::info!("alarm-server starting...");
    init_trace();

    let (config, db_path) = Config::load(&workspace).expect("Failed to load configuration");
    log::info!("database: {}", db_path.display());

    let db = Database::new(db_path.to_str().unwrap()).expect("Failed to open database");
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
    .await?;
    Ok(())
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
