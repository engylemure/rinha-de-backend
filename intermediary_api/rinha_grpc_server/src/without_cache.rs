use sqlx::{postgres::PgPoolOptions, PgPool};
use tonic::{transport::Server, Request, Response, Status};
use tonic_tracing_opentelemetry::middleware::server;
use tower_http::trace::TraceLayer;

use crate::{
    models::pessoa::Pessoa,
    rinha::{
        self,
        rinha_server::{Rinha, RinhaServer},
        CountPessoaReply, CountPessoaRequest, CreatePessoaReply, CreatePessoaRequest,
        PessoaByIdRequest, PessoaReply, PessoaSearchReply, PessoaSearchRequest,
    },
    utils::{
        env::{EnvironmentValues, LoggerOutput},
        telemetry,
    },
};
use std::{sync::Arc, time::Duration};

pub struct MyRinha {
    pub db: PgPool,
}

impl MyRinha {
    pub async fn from(env_values: &EnvironmentValues) -> Result<Self, Box<dyn std::error::Error>> {
        let db = PgPoolOptions::new()
            .max_connections(env_values.db_pool_max_size)
            .connect(&env_values.database_url)
            .await?;
        Ok(Self { db })
    }
}

#[tonic::async_trait]
impl Rinha for MyRinha {
    async fn pessoa_by_id(
        &self,
        request: Request<PessoaByIdRequest>,
    ) -> Result<Response<PessoaReply>, Status> {
        let json = sqlx::query_as::<_, Pessoa>(
            "
    SELECT * FROM pessoas where id = $1;
",
        )
        .persistent(true)
        .bind(request.get_ref().id.as_str())
        .fetch_one(&self.db)
        .await
        .map(|pessoa| serde_json::to_string(&pessoa).ok())
        .ok()
        .flatten();
        Ok(Response::new(PessoaReply { json }))
    }

    async fn pessoa_search(
        &self,
        request: Request<PessoaSearchRequest>,
    ) -> Result<Response<PessoaSearchReply>, Status> {
        let term = request.into_inner().term;
        let term_param = format!("%{}%", term.to_lowercase());
        let search_res = sqlx::query_as::<sqlx::Postgres, Pessoa>(
            "
            SELECT id, apelido, nome, nascimento, stack FROM pessoas p where p.busca_trgm LIKE $1 LIMIT 50;
        ",
        )
        .bind(&term_param)
        .persistent(true)
        .fetch_all(&self.db).await.ok().map(|res| serde_json::to_string(&res).ok()).flatten();
        Ok(Response::new(PessoaSearchReply { json: search_res }))
    }

    async fn create_pessoa(
        &self,
        request: Request<CreatePessoaRequest>,
    ) -> Result<Response<CreatePessoaReply>, Status> {
        let request = request.into_inner();
        if let Some(pessoa) = Pessoa::from(request) {
            let id = pessoa.id.clone();
            let query = sqlx::query::<sqlx::Postgres>(
                    "INSERT INTO pessoas (id, nome, apelido, nascimento, stack) values ($1, $2, $3, $4, $5)"
                ).bind(pessoa.id)
                .bind(pessoa.nome)
                .bind(pessoa.apelido)
                .bind(pessoa.nascimento)
                .bind(pessoa.stack.unwrap_or_default());
            if query.execute(&self.db).await.is_ok() {
                Ok(Response::new(CreatePessoaReply {
                    id: Some(id),
                    status: 201,
                }))
            } else {
                Ok(Response::new(CreatePessoaReply {
                    id: None,
                    status: 422,
                }))
            }
        } else {
            Ok(Response::new(CreatePessoaReply {
                id: None,
                status: 400,
            }))
        }
    }

    async fn count_pessoa(
        &self,
        _: Request<CountPessoaRequest>,
    ) -> Result<Response<CountPessoaReply>, Status> {
        let amount = sqlx::query_as::<_, (i64,)>("SELECT COUNT(id) FROM pessoas;")
            .fetch_one(&self.db)
            .await;
        match amount {
            Ok(amount) => Ok(Response::new(CountPessoaReply {
                amount: amount.0 as u64,
            })),
            _ => Err(Status::unavailable("Internal server error")),
        }
    }
}

pub async fn server() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::]:50051".parse()?;
    let env_values = Arc::new(EnvironmentValues::init());
    match env_values.logger {
        Some(LoggerOutput::Otel) => telemetry::init_otel(),
        Some(LoggerOutput::Stdout) => telemetry::init(),
        _ => (),
    }
    let (mut health_reporter, health_service) = tonic_health::server::health_reporter();
    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(rinha::FILE_DESCRIPTOR_SET)
        .build()?;
    let rinha_svc = MyRinha::from(&env_values).await?;
    let db_pool = rinha_svc.db.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            if db_pool.acquire().await.is_ok() {
                health_reporter.set_serving::<RinhaServer<MyRinha>>().await;
            } else {
                health_reporter
                    .set_not_serving::<RinhaServer<MyRinha>>()
                    .await;
            }
        }
    });
    tracing::info!(message = "Starting server without cache.", %addr);
    match env_values.logger {
        Some(LoggerOutput::Otel) => {
            Server::builder()
                .layer(server::OtelGrpcLayer::default())
                .add_service(RinhaServer::new(rinha_svc))
                .add_service(health_service)
                .add_service(reflection_service)
                .serve(addr)
                .await?
        }
        Some(LoggerOutput::Stdout) => {
            Server::builder()
                .layer(TraceLayer::new_for_grpc())
                .add_service(RinhaServer::new(rinha_svc))
                .add_service(health_service)
                .add_service(reflection_service)
                .serve(addr)
                .await?
        }
        None => {
            Server::builder()
                .add_service(health_service)
                .add_service(reflection_service)
                .add_service(RinhaServer::new(rinha_svc))
                .serve(addr)
                .await?
        }
    }
    // Ensure all spans have been shipped.
    if let Some(LoggerOutput::Otel) = env_values.logger {
        opentelemetry::global::shutdown_tracer_provider();
    }
    Ok(())
}
