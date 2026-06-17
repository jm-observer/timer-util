use serde::Deserialize;
use std::path::{Path, PathBuf};

const DEFAULT_PORT: u16 = 8080;
const DB_FILENAME: &str = "alarms.db";
const CONFIG_FILENAME: &str = "config.toml";
const PORT_ENV: &str = "ALARM_SERVER_PORT";

#[derive(Debug, Deserialize, Default)]
struct TomlConfig {
    pub port: Option<u16>,
}

pub struct Config {
    pub port: u16,
}

impl Config {
    /// Loads configuration from `<workspace>/config.toml` and returns the
    /// parsed config together with the database path. The database always
    /// lives in the same folder as the config file. The workspace is the
    /// single source of truth resolved by `LinuxService::workspace()`.
    pub fn load(workspace: &Path) -> anyhow::Result<(Self, PathBuf)> {
        std::fs::create_dir_all(workspace)?;

        let config_path = workspace.join(CONFIG_FILENAME);
        let toml_cfg = Self::load_toml(&config_path);

        // 端口解析优先级：ALARM_SERVER_PORT env > config.toml > DEFAULT_PORT。
        // env 缺省或解析失败（非数字 / 越界 u16）→ 回退 toml/默认，并 warn。
        let port = match std::env::var(PORT_ENV) {
            Ok(raw) => match raw.trim().parse::<u16>() {
                Ok(p) => p,
                Err(e) => {
                    log::warn!(
                        "Invalid {}={:?}: {}; falling back to config/default",
                        PORT_ENV,
                        raw,
                        e
                    );
                    toml_cfg.port.unwrap_or(DEFAULT_PORT)
                }
            },
            Err(_) => toml_cfg.port.unwrap_or(DEFAULT_PORT),
        };
        let db_path = workspace.join(DB_FILENAME);

        Ok((Self { port }, db_path))
    }

    fn load_toml(path: &Path) -> TomlConfig {
        match std::fs::read_to_string(path) {
            Ok(content) => toml::from_str(&content).unwrap_or_else(|e| {
                log::warn!("Failed to parse {}: {}", path.display(), e);
                TomlConfig::default()
            }),
            Err(_) => TomlConfig::default(),
        }
    }
}
