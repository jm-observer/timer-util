use std::env;

pub struct Config {
    pub port: u16,
    pub db_path: String,
}

impl Config {
    /// Load configuration from environment variables, providing defaults.
    /// `ALARM_SERVER_PORT` defaults to 8080.
    /// `ALARM_DB_PATH` defaults to "./alarms.db".
    pub fn from_env() -> Self {
        let port = env::var("ALARM_SERVER_PORT")
            .ok()
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(8080);
        let db_path = env::var("ALARM_DB_PATH").unwrap_or_else(|_| "./alarms.db".to_string());
        Self { port, db_path }
    }
}
