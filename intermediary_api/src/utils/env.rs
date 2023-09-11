use dotenv::dotenv;
use std::{env, str::FromStr};

pub struct EnvironmentValues {
    pub redis_url: String,
    pub database_url: String,
    pub server_port: u16,
    pub rust_env: String,
    pub logger: Option<LoggerOutput>,
    pub db_pool_max_size: u32,
    pub batch_max_insert_size: usize,
    pub batch_max_wait_on_insert_channel: u64,
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
            logger: std::env::var("LOGGER_OUTPUT")
                .ok()
                .map(|s| s.parse().ok())
                .flatten(),
            db_pool_max_size: std::env::var("DATABASE_POOL_MAX_SIZE")
                .map(|s| s.parse().ok())
                .ok()
                .flatten()
                .unwrap_or(256),
            batch_max_insert_size: std::env::var("BATCH_MAX_INSERT_SIZE")
                .map(|s| s.parse().ok())
                .ok()
                .flatten()
                .unwrap_or(256),
            batch_max_wait_on_insert_channel: std::env::var("BATCH_MAX_WAIT_ON_INSERT_CHANNEL")
                .map(|s| s.parse().ok())
                .ok()
                .flatten()
                .unwrap_or(1),
        }
    }
}
