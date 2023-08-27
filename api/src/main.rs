mod utils;
mod handlers;
mod models;

use actix_cors::Cors;
use actix_web::{web, App, HttpServer};
use std::net::SocketAddr;
use std::str::FromStr;
use tracing_actix_web::TracingLogger;
use tracing_subscriber::fmt::format::FmtSpan;

use crate::handlers::pessoa;
use crate::utils::app_state::AppState;
use crate::utils::env::EnvironmentValues;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let env_values = EnvironmentValues::init();
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::from_str(&env_values.rust_log)?)
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .init();
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
    Ok(())
}
