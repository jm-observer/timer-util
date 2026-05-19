//! CLI for alarm-server

use chrono::NaiveDateTime;
use clap::{Parser, Subcommand};
use log::LevelFilter::Info;
use reqwest::blocking::Client;
use serde::Deserialize;
use serde_json::Value;
use std::process::ExitCode;

const APP_NAME: &str = "alarm-server";
const CONFIG_FILENAME: &str = "config.toml";

#[derive(Debug, Deserialize, Default)]
struct TomlConfig {
    pub port: Option<u16>,
}

#[derive(Parser)]
#[command(
    name = "alarm-cli",
    version,
    author,
    about = "CLI tool for managing alarms on alarm-server. Supports creating one-time or cron-based recurring alarms with HTTP callback notifications."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    /// Workspace directory where config.toml and database are stored (default: ~/.config/alarm-server)
    #[arg(long, short = 'w', global = true)]
    workspace: Option<String>,
    /// Server base URL. If not set, reads port from workspace config.toml and uses http://127.0.0.1:{port}
    #[arg(long, global = true, default_value = "http://127.0.0.1:8080")]
    server: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a one-time alarm that fires once at a specific datetime then marks itself completed.
    /// Example: alarm-cli once --once-at 2026-06-01T08:00:00 --callback-url http://example.com/notify
    Once {
        /// Human-readable alarm name for identification in logs and dashboard
        #[arg(long)]
        name: Option<String>,
        /// Trigger datetime in format YYYY-MM-DDTHH:MM:SS (local time, must be in the future)
        #[arg(long, required = true)]
        once_at: String,
        /// HTTP URL to POST when the alarm fires. Server sends callback_body as JSON payload
        #[arg(long, required = true)]
        callback_url: String,
        /// Optional JSON object sent as POST body to callback_url, e.g. '{"msg":"hello"}'
        #[arg(long)]
        callback_body: Option<String>,
    },
    /// Create a recurring cron alarm that fires on a schedule and repeats indefinitely.
    /// Uses 5-field (min hour dom dow) or 6-field (sec min hour dom dow) cron syntax; month must be *.
    /// Example: alarm-cli cron --cron "0 30 8 * * 1-5" --callback-url http://example.com/notify
    Cron {
        /// Human-readable alarm name for identification in logs and dashboard
        #[arg(long)]
        name: Option<String>,
        /// Cron expression: "sec min hour day-of-month day-of-week" (6-field) or "min hour dom dow" (5-field). Month field is not supported.
        /// Supports lists (1,3,5), ranges (9-17), steps (*/15).
        #[arg(long, required = true)]
        cron: String,
        /// HTTP URL to POST when the alarm fires. Server sends callback_body as JSON payload
        #[arg(long, required = true)]
        callback_url: String,
        /// Optional JSON object sent as POST body to callback_url, e.g. '{"msg":"hello"}'
        #[arg(long)]
        callback_body: Option<String>,
    },
    /// List all alarms, optionally filtered by status. Returns JSON array with alarm details including next fire time.
    List {
        /// Filter by alarm status: "active" (pending/recurring) or "completed" (finished one-time alarms)
        #[arg(long, value_parser = validate_status)]
        status: Option<String>,
    },
    /// Get detailed information about a specific alarm by its UUID
    Get {
        /// Alarm UUID (e.g. 550e8400-e29b-41d4-a716-446655440000)
        id: String,
    },
    /// Cancel an alarm by its UUID. The alarm is unscheduled and its record is removed.
    Cancel {
        /// Alarm UUID (e.g. 550e8400-e29b-41d4-a716-446655440000)
        id: String,
    },
    // /// Update alarm-cli and alarm-server binaries to the latest GitHub release.
    // /// Downloads from https://github.com/jm-observer/timer-util/releases
    // Update {
    //     /// Force update even if already on the latest version
    //     #[arg(long)]
    //     force: bool,
    // },
    // /// Install alarm-server as a systemd service (Linux only, requires root).
    // /// Copies binaries to /usr/local/bin, creates a systemd unit, and enables the service.
    // Install {
    //     /// Workspace directory for the service (default: /etc/alarm-server)
    //     #[arg(long, default_value = "/etc/alarm-server")]
    //     workspace: String,
    //     /// System user to run the service as (created if missing)
    //     #[arg(long, default_value = "alarm-server")]
    //     user: String,
    //     /// Just print the generated systemd unit file without installing
    //     #[arg(long)]
    //     dry_run: bool,
    // },
}

fn validate_status(s: &str) -> Result<String, String> {
    match s {
        "active" | "completed" => Ok(s.to_string()),
        _ => Err("status must be 'active' or 'completed'".to_string()),
    }
}

fn get_server_url(cli_server: &str, arg_workspace: &Option<String>) -> String {
    if cli_server != "http://127.0.0.1:8080" {
        return cli_server.to_string();
    }
    if let Ok(workspace) = custom_utils::args::workspace(arg_workspace, APP_NAME) {
        let config_path = workspace.join(CONFIG_FILENAME);
        if let Ok(content) = std::fs::read_to_string(&config_path)
            && let Ok(cfg) = toml::from_str::<TomlConfig>(&content)
            && let Some(port) = cfg.port
        {
            return format!("http://127.0.0.1:{}", port);
        }
    }
    cli_server.to_string()
}

fn validate_once_at(s: &str) -> Result<(), String> {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
        .map(|_| ())
        .map_err(|_| {
            format!(
                "'{}' is not a valid datetime (expected YYYY-MM-DDTHH:MM:SS)",
                s
            )
        })
}

fn parse_callback_body(raw: Option<String>) -> Result<Option<Value>, String> {
    raw.map(|body| {
        serde_json::from_str::<Value>(&body)
            .map_err(|e| format!("invalid --callback-body JSON: {}", e))
    })
    .transpose()
}

fn main() -> ExitCode {
    let _ = custom_utils::logger::logger_feature(
        "alarm-cli",
        "info,alarm-server=debug,alarm-client=debug,timer-util=debug",
        Info,
        false,
    )
    .build();

    let cli = Cli::parse();
    let server_url = get_server_url(&cli.server, &cli.workspace);
    let client = Client::new();

    let result = match cli.command {
        Commands::Once {
            name,
            once_at,
            callback_url,
            callback_body,
        } => {
            if let Err(e) = validate_once_at(&once_at) {
                eprintln!("Invalid --once-at: {}", e);
                return ExitCode::FAILURE;
            }
            let callback_body = match parse_callback_body(callback_body) {
                Ok(body) => body,
                Err(e) => {
                    eprintln!("{}", e);
                    return ExitCode::FAILURE;
                }
            };
            let mut map = serde_json::Map::new();
            map.insert("alarm_type".to_string(), Value::String("once".to_string()));
            if let Some(n) = name {
                map.insert("name".to_string(), Value::String(n));
            }
            map.insert("once_at".to_string(), Value::String(once_at));
            map.insert("callback_url".to_string(), Value::String(callback_url));
            if let Some(b) = callback_body {
                map.insert("callback_body".to_string(), b);
            }
            let resp = client
                .post(format!("{}/api/alarms", server_url))
                .json(&map)
                .send();
            handle_response(resp)
        }
        Commands::Cron {
            name,
            cron,
            callback_url,
            callback_body,
        } => {
            let callback_body = match parse_callback_body(callback_body) {
                Ok(body) => body,
                Err(e) => {
                    eprintln!("{}", e);
                    return ExitCode::FAILURE;
                }
            };
            let mut map = serde_json::Map::new();
            map.insert("alarm_type".to_string(), Value::String("cron".to_string()));
            if let Some(n) = name {
                map.insert("name".to_string(), Value::String(n));
            }
            map.insert("cron_expr".to_string(), Value::String(cron));
            map.insert("callback_url".to_string(), Value::String(callback_url));
            if let Some(b) = callback_body {
                map.insert("callback_body".to_string(), b);
            }
            let resp = client
                .post(format!("{}/api/alarms", server_url))
                .json(&map)
                .send();
            handle_response(resp)
        }
        Commands::List { status } => {
            let url = if let Some(st) = status {
                format!("{}/api/alarms?status={}", server_url, st)
            } else {
                format!("{}/api/alarms", server_url)
            };
            let resp = client.get(url).send();
            handle_response(resp)
        }
        Commands::Get { id } => {
            let resp = client
                .get(format!("{}/api/alarms/{}", server_url, id))
                .send();
            handle_response(resp)
        }
        Commands::Cancel { id } => {
            let resp = client
                .delete(format!("{}/api/alarms/{}", server_url, id))
                .send();
            handle_delete_response(resp, &id)
        }
    };

    if result.is_ok() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn handle_response(resp: Result<reqwest::blocking::Response, reqwest::Error>) -> Result<(), ()> {
    match resp {
        Ok(r) => {
            if r.status().is_success() {
                match r.json::<Value>() {
                    Ok(json) => {
                        println!("{}", serde_json::to_string_pretty(&json).unwrap());
                        Ok(())
                    }
                    Err(e) => {
                        eprintln!("Failed to parse JSON response: {}", e);
                        Err(())
                    }
                }
            } else {
                eprintln!("Error: HTTP {}", r.status());
                if let Ok(text) = r.text() {
                    eprintln!("Response body: {}", text);
                }
                Err(())
            }
        }
        Err(e) => {
            eprintln!("Failed to send request: {}", e);
            Err(())
        }
    }
}

fn handle_delete_response(
    resp: Result<reqwest::blocking::Response, reqwest::Error>,
    id: &str,
) -> Result<(), ()> {
    match resp {
        Ok(r) => {
            if r.status().is_success() {
                println!("Alarm {} cancelled", id);
                Ok(())
            } else {
                eprintln!("Error: HTTP {}", r.status());
                if let Ok(text) = r.text() {
                    eprintln!("Response body: {}", text);
                }
                Err(())
            }
        }
        Err(e) => {
            eprintln!("Failed to send request: {}", e);
            Err(())
        }
    }
}
