use dotenv::dotenv;
use std::{env, str::FromStr};

pub struct EnvironmentValues {
    pub redis_url: String,
    pub server_port: u16,
    pub rust_env: String,
    pub logger: Option<LoggerOutput>,
    pub rinha_url: String,
    #[cfg(feature = "without_cache")]
    pub db_pool_max_size: u32,
    #[cfg(feature = "without_cache")]
    pub database_url: String,
    #[cfg(feature = "without_cache")]
    pub db_host: String,
    #[cfg(feature = "without_cache")]
    pub db_password: String,
    #[cfg(feature = "without_cache")]
    pub db_name: String,
    #[cfg(feature = "without_cache")]
    pub db_user: String,
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
            #[cfg(feature = "without_cache")]
            database_url: env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            #[cfg(feature = "without_cache")]
            db_pool_max_size: std::env::var("DATABASE_POOL_MAX_SIZE")
                .map(|s| s.parse().ok())
                .ok()
                .flatten()
                .unwrap_or(256),
            #[cfg(feature = "without_cache")]
            db_host: std::env::var("DB_HOST").expect("DB_HOST must be set"),
            #[cfg(feature = "without_cache")]
            db_user: std::env::var("DB_USER").expect("DB_USER must be set"),
            #[cfg(feature = "without_cache")]
            db_password: std::env::var("DB_PASSWORD").expect("DB_PASSWORD must be set"),
            #[cfg(feature = "without_cache")]
            db_name: std::env::var("DB_NAME").expect("DB_NAME must be set"),
        }
    }
}
