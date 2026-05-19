use serde::Deserialize;
use std::path::{Path, PathBuf};

const DEFAULT_PORT: u16 = 8080;
const DB_FILENAME: &str = "alarms.db";
const CONFIG_FILENAME: &str = "config.toml";

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

        let port = toml_cfg.port.unwrap_or(DEFAULT_PORT);
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
