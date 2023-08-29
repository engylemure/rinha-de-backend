mod handlers;
mod models;
mod utils;

use crate::handlers::pessoa;
use crate::utils::app_state::AppState;
use crate::utils::env::EnvironmentValues;
use crate::utils::telemetry;
use actix_cors::Cors;
use actix_web::{web, App, HttpServer};
use std::net::SocketAddr;
use tracing_actix_web::TracingLogger;





#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let env_values = EnvironmentValues::init();
    if env_values.with_otel {
        telemetry::init_otel();
    } else {
        telemetry::init(&env_values)?;
    }
    let app_state = AppState::from(&env_values).await?;
    let socket: SocketAddr = format!("[::]:{}", env_values.server_port).parse()?;
    tracing::info!("Starting App Server at: {}", socket);
    tokio::spawn(pessoa::batch_insert_task(app_state.clone()));
    let app_state = web::Data::new(app_state);
    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .wrap(Cors::permissive())
            .wrap(TracingLogger::default())
            .configure(pessoa::config)
    })
    .bind(&socket)?
    .run()
    .await?;
    // Ensure all spans have been shipped to Jaeger.
    if env_values.with_otel {
        opentelemetry::global::shutdown_tracer_provider();
    }
    Ok(())
}
