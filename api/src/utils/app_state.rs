use std::sync::Arc;

use sqlx::{postgres::PgPoolOptions, PgPool};

use crate::models::pessoa::Pessoa;

use super::env::EnvironmentValues;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub redis: r2d2::Pool<redis::Client>,
    pub person_queue: Arc<deadqueue::unlimited::Queue<Pessoa>>,
}

impl AppState {
    pub async fn from(env_values: &EnvironmentValues) -> Result<Self, Box<dyn std::error::Error>> {
        let db = PgPoolOptions::new()
            .max_connections(16_384)
            .connect(&env_values.database_url)
            .await?;
        let redis =
            r2d2::Pool::builder().build(redis::Client::open(env_values.redis_url.clone())?)?;
        Ok(Self {
            db,
            redis,
            person_queue: Arc::new(deadqueue::unlimited::Queue::new()),
        })
    }
}
