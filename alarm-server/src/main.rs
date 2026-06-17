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

/// 安装时写进 unit `Environment=ALARM_SERVER_PORT=<port>` 的默认端口；
/// 可被 install 的 `--port` 覆盖。serve 路径靠该 env（env>config>8080）决定监听端口。
const DEFAULT_PORT: u16 = 8080;
const PORT_ENV: &str = "ALARM_SERVER_PORT";

/// The host owns its top-level CLI; the unified deploy stack is embedded as a
/// single pass-through variant. `LinuxService` never reads argv for us beyond
/// `parse_deploy`, and never writes stdout — text outcomes come back for us to
/// print alongside our own usage.
enum AppCmd {
    Serve,
    Deploy(DeployCommand),
}

/// 构造 systemd 安装描述。`port` 经 `.env()` 写进生成 unit 的
/// `Environment=ALARM_SERVER_PORT=<port>`，install 时由 `--port`（default 8080）决定，
/// 后续 G10 部署脚本可经 `--port` 注入 g10-services.json 配置的端口。
fn service(port: u16) -> LinuxService {
    LinuxService::new(APP_NAME, REPO_OWNER, REPO_NAME, env!("CARGO_PKG_VERSION"))
        .description("Alarm Server - recurring/one-time alarm scheduler")
        .extra_bins(["alarm-cli"])
        .env(PORT_ENV, port.to_string())
        .watchdog_sec(30)
}

/// install 端口取值：argv 里的 `--port <n>`（如 `--port 9000`）→ 解析失败/缺省回退
/// `DEFAULT_PORT`。G10 部署脚本经此参数把 g10-services.json 端口注入生成的 unit。
fn install_port() -> u16 {
    custom_utils::args::arg_value("--port", "-p")
        .and_then(|raw| raw.trim().parse::<u16>().ok())
        .unwrap_or(DEFAULT_PORT)
}

/// Our own usage block; spliced ahead of the library's deploy usage on `--help`.
fn own_usage() -> String {
    format!(
        "{ABOUT}\n\n\
         Usage:\n  \
         {APP_NAME} [serve] [-w|--workspace <path>]   run the server (default; workspace default ~/.config/{APP_NAME})\n  \
         {APP_NAME} install [--port <n>] [...]        install systemd unit (writes Environment=ALARM_SERVER_PORT, default {DEFAULT_PORT})\n\n\
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

    // 端口仅在 install 写 unit 时有意义（serve 路径靠 env/config 解析），
    // 但 service() 始终带上 `.env(ALARM_SERVER_PORT)`：dry-run/install 时端口才落进 unit。
    let svc = service(install_port());

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
