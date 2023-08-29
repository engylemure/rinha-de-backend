use dotenv::dotenv;
use std::env;

pub struct EnvironmentValues {
    pub redis_url: String,
    pub database_url: String,
    pub server_port: u16,
    pub rust_env: String,
    pub rust_log: String,
    pub with_otel: bool,
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
            with_otel: std::env::var("WITH_OTEL")
                .unwrap_or_else(|_| String::from("false"))
                .parse()
                .expect("WITH_TELEMETRY should be 'true' or 'false'"),
        }
    }
}
