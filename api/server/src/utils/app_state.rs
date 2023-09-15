#[cfg(not(feature = "without_cache"))]
pub use with_cache::*;
#[cfg(not(feature = "without_cache"))]
mod with_cache {
    use std::time::Duration;

    use crate::rinha::rinha_client::RinhaClient;
    use crate::utils::env::EnvironmentValues;
    use tonic::transport::Channel;
    use tonic_tracing_opentelemetry::middleware::client::OtelGrpcService;
    use tower::ServiceBuilder;

    #[derive(Clone)]
    pub struct AppState {
        pub rinha_client: RinhaClient<OtelGrpcService<tonic::transport::Channel>>,
    }

    impl AppState {
        pub async fn from(
            env_values: &EnvironmentValues,
        ) -> Result<Self, Box<dyn std::error::Error>> {
            let channel = {
                // Seconds to wait for the channel to be available
                let mut wait = 1;
                loop {
                    match Channel::from_shared(env_values.rinha_url.clone())?
                        .connect()
                        .await
                    {
                        Ok(channel) => break channel,
                        err => {
                            if wait > 10 {
                                tracing::error!(
                                    "Rinha GRPC server not available at {}",
                                    env_values.rinha_url
                                );
                                err?;
                            }
                            tracing::warn!(
                                "Rinha GRPC server not available at {} we will wait for {}",
                                env_values.rinha_url,
                                chrono::Duration::seconds(wait)
                            );
                            tokio::time::sleep(Duration::from_secs(wait as u64)).await;
                            wait *= 2;
                        }
                    }
                }
            };
            let channel = ServiceBuilder::new()
                .layer(tonic_tracing_opentelemetry::middleware::client::OtelGrpcLayer::default())
                .service(channel);
            Ok(Self {
                rinha_client: RinhaClient::new(channel),
            })
        }
    }
}

#[cfg(feature = "without_cache")]
pub use without_cache::*;
#[cfg(feature = "without_cache")]
mod without_cache {
    use sqlx::postgres::PgPoolOptions;
    use sqlx::PgPool;

    use crate::utils::env::EnvironmentValues;

    #[derive(Clone)]
    pub struct AppState {
        pub db: PgPool,
        pub pool: deadpool_postgres::Pool,
    }

    impl AppState {
        pub async fn from(
            env_values: &EnvironmentValues,
        ) -> Result<Self, Box<dyn std::error::Error>> {
            let db = PgPoolOptions::new()
                .max_connections(env_values.db_pool_max_size)
                .connect(&env_values.database_url)
                .await?;
            let pool = deadpool_postgres::Config {
                host: Some(env_values.db_host.clone()),
                dbname: Some(env_values.db_name.clone()),
                port: Some(5432),
                password: Some(env_values.db_password.clone()),
                user: Some(env_values.db_user.clone()),
                pool: Some(deadpool_postgres::PoolConfig {
                    max_size: env_values.db_pool_max_size as usize,
                    ..Default::default()
                }),
                ..Default::default()
            }.create_pool(Some(deadpool_postgres::Runtime::Tokio1), tokio_postgres::NoTls)?;
            Ok(Self { db, pool })
        }
    }
}
