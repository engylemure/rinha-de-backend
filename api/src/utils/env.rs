use dotenv::dotenv;
use std::{env, str::FromStr};

pub struct EnvironmentValues {
    pub redis_url: String,
    pub database_url: String,
    pub server_port: u16,
    pub rust_env: String,
    pub rust_log: String,
    pub logger: Option<LoggerOutput>,
    pub db_pool_max_size: u32,
    pub redis_pool_max_size: usize,
    pub max_batch_insert_size: usize,
    pub batch_insert_interval_secs: u64,
}

pub enum LoggerOutput {
    Otel,
    Stdout,
}

impl FromStr for LoggerOutput {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "otel" => Ok(Self::Otel),
            "stdout" => Ok(Self::Stdout),
            _ => Err(()),
        }
    }
}

impl EnvironmentValues {
    pub fn init() -> Self {
        dotenv().ok();
        Self {
            database_url: env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            redis_url: env::var("REDIS_URL").expect("REDIS_URL must be set"),
            server_port: env::var("SERVER_PORT")
                .unwrap_or_else(|_| String::from("80"))
                .parse()
                .expect("SERVER_PORT must be a number"),
            rust_env: env::var("RUST_ENV").unwrap_or_else(|_| "dev".into()),
            rust_log: std::env::var("RUST_LOG").unwrap_or_else(|_| "debug".to_owned()),
            logger: std::env::var("LOGGER_OUTPUT")
                .ok()
                .map(|s| s.parse().ok())
                .flatten(),
            db_pool_max_size: std::env::var("DATABASE_POOL_MAX_SIZE")
                .map(|s| s.parse().ok())
                .ok()
                .flatten()
                .unwrap_or(256),
            redis_pool_max_size: std::env::var("REDIS_POOL_MAX_SIZE")
                .map(|s| s.parse().ok())
                .ok()
                .flatten()
                .unwrap_or(1024),
            max_batch_insert_size: std::env::var("MAX_BATCH_INSERT_SIZE")
                .map(|s| s.parse().ok())
                .ok()
                .flatten()
                .unwrap_or(256),
            batch_insert_interval_secs: std::env::var("BATCH_INSERT_INTERVAL_SECS")
                .map(|s| s.parse().ok())
                .ok()
                .flatten()
                .unwrap_or(1),
        }
    }
}
