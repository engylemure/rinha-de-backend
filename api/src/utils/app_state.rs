use std::{sync::Arc, time::Duration};

use sqlx::{postgres::PgPoolOptions, PgPool};

use crate::models::pessoa::Pessoa;

use super::env::EnvironmentValues;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub redis: deadpool_redis::Pool,
    pub person_queue: Arc<deadqueue::unlimited::Queue<Pessoa>>,
}

impl AppState {
    pub async fn from(env_values: &EnvironmentValues) -> Result<Self, Box<dyn std::error::Error>> {
        let db = PgPoolOptions::new()
            .max_connections(32768)
            .min_connections(64)
            .connect(&env_values.database_url)
            .await?;
        let redis = deadpool_redis::Config {
            url: Some(env_values.redis_url.clone()),
            pool: Some(deadpool_redis::PoolConfig {
                max_size: 32768,
                timeouts: deadpool_redis::Timeouts {
                    wait: Some(Duration::from_secs(60)),
                    create: Some(Duration::from_secs(60)),
                    recycle: Some(Duration::from_secs(60)),
                },
            }),
            connection: None
        }.create_pool(Some(deadpool_redis::Runtime::Tokio1))?;
        Ok(Self {
            db,
            redis,
            person_queue: Arc::new(deadqueue::unlimited::Queue::new()),
        })
    }
}
