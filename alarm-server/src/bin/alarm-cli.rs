//! CLI for alarm-server

use chrono::NaiveDateTime;
use clap::{Parser, Subcommand};
use log::LevelFilter::Info;
use reqwest::blocking::Client;
use serde::Deserialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const APP_NAME: &str = "alarm-server";
const CONFIG_FILENAME: &str = "config.toml";
const GITHUB_REPO: &str = "jm-observer/timer-util";

#[derive(Debug, Deserialize, Default)]
struct TomlConfig {
    pub port: Option<u16>,
}

#[derive(Parser)]
#[command(name = "alarm-cli", version, author, about = "CLI tool for managing alarms on alarm-server. Supports creating one-time or cron-based recurring alarms with HTTP callback notifications.")]
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
    /// Update alarm-cli and alarm-server binaries to the latest GitHub release.
    /// Downloads from https://github.com/jm-observer/timer-util/releases
    Update {
        /// Force update even if already on the latest version
        #[arg(long)]
        force: bool,
    },
    /// Install alarm-server as a systemd service (Linux only, requires root).
    /// Copies binaries to /usr/local/bin, creates a systemd unit, and enables the service.
    Install {
        /// Workspace directory for the service (default: /etc/alarm-server)
        #[arg(long, default_value = "/etc/alarm-server")]
        workspace: String,
        /// System user to run the service as (created if missing)
        #[arg(long, default_value = "alarm-server")]
        user: String,
        /// Just print the generated systemd unit file without installing
        #[arg(long)]
        dry_run: bool,
    },
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
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            if let Ok(cfg) = toml::from_str::<TomlConfig>(&content) {
                if let Some(port) = cfg.port {
                    return format!("http://127.0.0.1:{}", port);
                }
            }
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
        Commands::Update { force } => self_update(force),
        Commands::Install {
            workspace,
            user,
            dry_run,
        } => install_systemd(&workspace, &user, dry_run),
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

fn current_target() -> &'static str {
    if cfg!(target_os = "windows") && cfg!(target_arch = "x86_64") {
        "x86_64-pc-windows-msvc"
    } else if cfg!(target_os = "linux") && cfg!(target_arch = "aarch64") {
        "aarch64-unknown-linux-gnu"
    } else if cfg!(target_os = "linux") && cfg!(target_arch = "x86_64") {
        "x86_64-unknown-linux-gnu"
    } else {
        "unknown"
    }
}

fn self_update(force: bool) -> Result<(), ()> {
    let current_version = env!("CARGO_PKG_VERSION");
    let target = current_target();
    if target == "unknown" {
        eprintln!("Unsupported platform for self-update");
        return Err(());
    }

    let client = Client::builder()
        .user_agent("alarm-cli")
        .build()
        .map_err(|e| eprintln!("Failed to create HTTP client: {}", e))?;

    println!("Checking for updates...");
    let resp = client
        .get(format!(
            "https://api.github.com/repos/{}/releases/latest",
            GITHUB_REPO
        ))
        .send()
        .map_err(|e| eprintln!("Failed to check for updates: {}", e))?;

    if !resp.status().is_success() {
        eprintln!("GitHub API error: HTTP {}", resp.status());
        return Err(());
    }

    let release: Value = resp
        .json()
        .map_err(|e| eprintln!("Failed to parse release info: {}", e))?;

    let tag = release["tag_name"]
        .as_str()
        .ok_or_else(|| eprintln!("No tag_name in release"))?;
    let latest_version = tag.strip_prefix('v').unwrap_or(tag);

    if latest_version == current_version && !force {
        println!("Already up to date (v{})", current_version);
        return Ok(());
    }

    println!(
        "Current: v{}, Latest: v{}",
        current_version, latest_version
    );

    let assets = release["assets"]
        .as_array()
        .ok_or_else(|| eprintln!("No assets in release"))?;

    let current_exe =
        std::env::current_exe().map_err(|e| eprintln!("Cannot determine current exe: {}", e))?;
    let install_dir = current_exe.parent().unwrap();

    let ext = if cfg!(windows) { ".exe" } else { "" };

    for bin_name in &["alarm-cli", "alarm-server"] {
        let asset_name = format!("{}-{}{}", bin_name, target, ext);
        let asset = assets
            .iter()
            .find(|a| a["name"].as_str() == Some(&asset_name));

        match asset {
            Some(asset) => {
                let download_url = asset["browser_download_url"]
                    .as_str()
                    .ok_or_else(|| eprintln!("No download URL for {}", asset_name))?;

                println!("Downloading {}...", asset_name);
                let data = client
                    .get(download_url)
                    .send()
                    .and_then(|r| r.bytes())
                    .map_err(|e| eprintln!("Failed to download {}: {}", asset_name, e))?;

                let dest = install_dir.join(format!("{}{}", bin_name, ext));
                replace_binary(&dest, &data)
                    .map_err(|e| eprintln!("Failed to install {}: {}", bin_name, e))?;
                println!("Updated: {}", dest.display());
            }
            None => {
                println!("Asset {} not found in release, skipping", asset_name);
            }
        }
    }

    println!("Update complete!");
    Ok(())
}

fn replace_binary(dest: &Path, data: &[u8]) -> std::io::Result<()> {
    if cfg!(windows) {
        let backup = dest.with_extension("old.exe");
        if dest.exists() {
            let _ = std::fs::remove_file(&backup);
            std::fs::rename(dest, &backup)?;
        }
        std::fs::write(dest, data)?;
    } else {
        let tmp = dest.with_extension("tmp");
        std::fs::write(&tmp, data)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))?;
        }
        std::fs::rename(&tmp, dest)?;
    }
    Ok(())
}

fn generate_systemd_unit(workspace: &str, user: &str) -> String {
    format!(
        r#"[Unit]
Description=Alarm Server - Recurring alarm scheduler with HTTP callbacks
After=network.target

[Service]
Type=simple
User={user}
Group={user}
ExecStart=/usr/local/bin/alarm-server -w {workspace}
Restart=on-failure
RestartSec=5
WorkingDirectory={workspace}

[Install]
WantedBy=multi-user.target
"#,
        user = user,
        workspace = workspace,
    )
}

fn install_systemd(workspace: &str, user: &str, dry_run: bool) -> Result<(), ()> {
    if cfg!(not(target_os = "linux")) {
        eprintln!("The install command is only supported on Linux");
        return Err(());
    }

    let unit_content = generate_systemd_unit(workspace, user);

    if dry_run {
        println!("{}", unit_content);
        return Ok(());
    }

    // Check root
    #[cfg(unix)]
    {
        if unsafe { libc::geteuid() } != 0 {
            eprintln!("Error: install requires root privileges. Run with sudo.");
            return Err(());
        }
    }

    let current_exe =
        std::env::current_exe().map_err(|e| eprintln!("Cannot determine current exe: {}", e))?;
    let src_dir = current_exe.parent().unwrap();

    // Copy binaries
    let bins = ["alarm-server", "alarm-cli"];
    for bin in &bins {
        let src = src_dir.join(bin);
        let dest = PathBuf::from(format!("/usr/local/bin/{}", bin));
        if src.exists() {
            std::fs::copy(&src, &dest)
                .map_err(|e| eprintln!("Failed to copy {} to /usr/local/bin: {}", bin, e))?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755));
            }
            println!("Installed: {}", dest.display());
        } else if *bin == "alarm-server" {
            eprintln!(
                "Error: {} not found in {}",
                bin,
                src_dir.display()
            );
            return Err(());
        }
    }

    // Create user if not exists
    let user_check = std::process::Command::new("id")
        .arg(user)
        .output();
    if let Ok(output) = user_check {
        if !output.status.success() {
            println!("Creating system user: {}", user);
            let status = std::process::Command::new("useradd")
                .args(["--system", "--no-create-home", "--shell", "/usr/sbin/nologin", user])
                .status()
                .map_err(|e| eprintln!("Failed to create user: {}", e))?;
            if !status.success() {
                eprintln!("Failed to create system user '{}'", user);
                return Err(());
            }
        }
    }

    // Create workspace directory
    std::fs::create_dir_all(workspace)
        .map_err(|e| eprintln!("Failed to create workspace {}: {}", workspace, e))?;

    let _ = std::process::Command::new("chown")
        .args([&format!("{}:{}", user, user), workspace])
        .status();

    // Write systemd unit
    let unit_path = "/etc/systemd/system/alarm-server.service";
    std::fs::write(unit_path, &unit_content)
        .map_err(|e| eprintln!("Failed to write {}: {}", unit_path, e))?;
    println!("Installed: {}", unit_path);

    // Reload systemd and enable
    let _ = std::process::Command::new("systemctl")
        .args(["daemon-reload"])
        .status();
    let _ = std::process::Command::new("systemctl")
        .args(["enable", "alarm-server.service"])
        .status();

    println!("\nInstallation complete!");
    println!("  Start:   sudo systemctl start alarm-server");
    println!("  Status:  sudo systemctl status alarm-server");
    println!("  Logs:    sudo journalctl -u alarm-server -f");
    Ok(())
}
