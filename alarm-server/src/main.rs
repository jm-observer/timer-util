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
use clap::{Parser, Subcommand};
use custom_utils::updater::{ServiceConfig, UpdateConfig, UpdateOutcome};
use log::LevelFilter::Info;
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;

const APP_NAME: &str = "alarm-server";
const REPO_OWNER: &str = "jm-observer";
const REPO_NAME: &str = "timer-util";

#[derive(Parser)]
#[command(
    name = "alarm-server",
    version,
    author,
    about = "Alarm server: recurring/one-time alarm scheduler with HTTP callbacks, SQLite persistence and a dashboard."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    /// Workspace directory where config.toml and the database are stored
    /// (default: ~/.config/alarm-server).
    #[arg(long, short = 'w', global = true)]
    workspace: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the alarm server (this is also the default when no subcommand is given).
    Serve,
    /// Install alarm-server as a systemd service (Linux only, requires root).
    /// Copies binaries to /usr/local/bin, creates the service user and unit file.
    Install {
        /// System user to run the service as (created if missing).
        #[arg(long, default_value = APP_NAME)]
        user: String,
        /// Print the generated systemd unit file instead of installing.
        #[arg(long)]
        dry_run: bool,
    },
    /// Update alarm-server and alarm-cli to the latest GitHub release.
    Update {
        /// Force update even if already on the latest version.
        #[arg(long)]
        force: bool,
    },
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let _ = custom_utils::logger::logger_feature(
        "alarm-server",
        "info,alarm-server=debug,alarm-client=debug,timer-util=debug",
        Info,
        false,
    )
    .build();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Install { user, dry_run }) => run_install(&cli.workspace, &user, dry_run),
        Some(Commands::Update { force }) => run_update(force).await,
        Some(Commands::Serve) | None => run_server(&cli.workspace).await,
    }
}

fn run_install(arg_workspace: &Option<String>, user: &str, dry_run: bool) -> std::io::Result<()> {
    let mut svc = ServiceConfig::new(APP_NAME)
        .description("Alarm Server - recurring/one-time alarm scheduler")
        .binaries(["alarm-server", "alarm-cli"])
        .exec_args("serve -w {workspace}")
        .user(user);
    if let Some(ws) = arg_workspace {
        svc = svc.workspace(ws.clone());
    }

    if dry_run {
        print!("{}", svc.generate_unit());
        return Ok(());
    }

    match svc.install() {
        Ok(()) => {
            log::info!("systemd service '{}' installed", APP_NAME);
            Ok(())
        }
        Err(e) => {
            log::error!("Install failed: {:#}", e);
            std::process::exit(1);
        }
    }
}

async fn run_update(force: bool) -> std::io::Result<()> {
    let result = UpdateConfig::new(REPO_OWNER, REPO_NAME, env!("CARGO_PKG_VERSION"))
        .bin_name("alarm-server")
        .extra_bins(["alarm-cli"])
        .force(force)
        .execute()
        .await;

    match result {
        Ok(UpdateOutcome::UpToDate { current, latest }) => {
            log::info!(
                "Already up to date (current {}, latest {})",
                current,
                latest
            );
            Ok(())
        }
        Ok(UpdateOutcome::Updated { from, to, bins }) => {
            log::info!("Updated {} from {} to {}", bins.join(", "), from, to);
            Ok(())
        }
        Err(e) => {
            log::error!("Update failed: {:#}", e);
            std::process::exit(1);
        }
    }
}

async fn run_server(arg_workspace: &Option<String>) -> std::io::Result<()> {
    log::info!("alarm-server starting...");

    let (config, db_path) = Config::load(arg_workspace).expect("Failed to load configuration");
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
