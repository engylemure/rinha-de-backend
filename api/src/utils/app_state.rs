use sqlx::{postgres::PgPoolOptions, PgPool};
use crate::rinha::rinha_client::RinhaClient;
use super::env::EnvironmentValues;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub rinha_client: RinhaClient<tonic::transport::Channel>
}

impl AppState {
    pub async fn from(env_values: &EnvironmentValues) -> Result<Self, Box<dyn std::error::Error>> {
        let db = PgPoolOptions::new()
            .max_connections(env_values.db_pool_max_size)
            .min_connections(32)
            .connect(&env_values.database_url)
            .await?;
        Ok(Self {
            db,
            rinha_client: RinhaClient::connect(env_values.rinha_url.clone()).await?,
        })
    }
}
