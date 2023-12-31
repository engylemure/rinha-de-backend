use dotenv::dotenv;
use std::{env, str::FromStr};

pub struct EnvironmentValues {
    pub redis_url: String,
    pub database_url: String,
    pub server_port: u16,
    pub rust_env: String,
    pub logger: Option<LoggerOutput>,
    pub rinha_url: String,
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
            rinha_url: std::env::var("RINHA_URL")
                .ok()
                .unwrap_or(String::from("http://[::]:50051")),
        }
    }
}
