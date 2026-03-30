//! CLI for alarm-server

use chrono::NaiveDateTime;
use clap::{Parser, Subcommand};
use reqwest::blocking::Client;
use serde_json::Value;
use std::process::ExitCode;
use std::env;
use log::LevelFilter::Info;

#[derive(Parser)]
#[command(name = "alarm-cli", version, author, about = "CLI for alarm-server")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    /// Server base URL, overrides env var ALARM_SERVER_URL
    #[arg(long, global = true)]
    server: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new alarm
    Create {
        #[command(subcommand)]
        kind: CreateKind,
    },
    /// List alarms
    List {
        #[arg(long, value_parser = validate_status)]
        status: Option<String>,
    },
    /// Get alarm detail
    Get { id: String },
}

#[derive(Subcommand)]
enum CreateKind {
    /// One-time alarm
    Once {
        #[arg(long)]
        name: Option<String>,
        #[arg(long, required = true)]
        once_at: String,
        #[arg(long, required = true)]
        callback_url: String,
        #[arg(long)]
        callback_body: Option<String>,
    },
    /// Cron alarm
    Cron {
        #[arg(long)]
        name: Option<String>,
        #[arg(long, required = true)]
        cron: String,
        #[arg(long, required = true)]
        callback_url: String,
        #[arg(long)]
        callback_body: Option<String>,
    },
}

fn validate_status(s: &str) -> Result<String, String> {
    match s {
        "active" | "completed" => Ok(s.to_string()),
        _ => Err("status must be 'active' or 'completed'".to_string()),
    }
}

fn get_server_url(cli_server: Option<String>) -> String {
    cli_server.unwrap_or_else(|| {
        env::var("ALARM_SERVER_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_string())
    })
}

fn validate_once_at(s: &str) -> Result<(), String> {
    // Expect format YYYY-MM-DDTHH:MM:SS
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
    let _ = custom_utils::logger::logger_feature("alarm-cli"
                                                 , "info,alarm-server=debug,alarm-client=debug,timer-util=debug", Info, false).build();

    let cli = Cli::parse();
    let server_url = get_server_url(cli.server);
    let client = Client::new();

    let result = match cli.command {
        Commands::Create { kind } => match kind {
            CreateKind::Once {
                name,
                once_at,
                callback_url,
                callback_body,
            } => {
                // Validate once-at format
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
            CreateKind::Cron {
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
        },
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
                match r.text() {
                    Ok(text) => eprintln!("Response body: {}", text),
                    Err(_) => {}
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
