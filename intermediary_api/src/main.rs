mod models;
mod utils;
use std::sync::Arc;
use std::time::Duration;

use dashmap::*;
use models::pessoa::Pessoa;
use rinha::rinha_server::{Rinha, RinhaServer};
use rinha::{
    CountPessoaReply, CountPessoaRequest, CreatePessoaReply, CreatePessoaRequest,
    PessoaByIdRequest, PessoaReply, PessoaSearchReply, PessoaSearchRequest,
};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tokio::select;
use tokio::sync::mpsc::{self, UnboundedReceiver};
use tonic::{transport::Server, Request, Response, Status};
use tonic_tracing_opentelemetry::middleware::server;
use tower_http::trace::TraceLayer;
use utils::env::{EnvironmentValues, LoggerOutput};

use crate::utils::telemetry;

pub mod rinha {
    tonic::include_proto!("rinha");
    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("rinha_descriptor");
}

#[derive(Debug)]
pub struct MyRinha {
    pub pessoa_by_apelido_exists_set: DashSet<String>,
    pub pessoa_by_id_map: DashMap<String, String>,
    pub pessoa_search_map: DashMap<String, String>,
    pub pessoa_sender: mpsc::UnboundedSender<Pessoa>,
    pub pessoa_create_count: std::sync::atomic::AtomicU64,
    pub db: PgPool,
}

impl MyRinha {
    pub async fn from(
        env_values: &EnvironmentValues,
    ) -> Result<(Self, UnboundedReceiver<Pessoa>), Box<dyn std::error::Error>> {
        let db = PgPoolOptions::new()
            .max_connections(env_values.db_pool_max_size)
            .connect(&env_values.database_url)
            .await?;
        let (pessoa_sender, pessoa_receiver) = mpsc::unbounded_channel();
        Ok((
            Self {
                db,
                pessoa_sender,
                pessoa_by_apelido_exists_set: Default::default(),
                pessoa_by_id_map: Default::default(),
                pessoa_search_map: Default::default(),
                pessoa_create_count: Default::default(),
            },
            pessoa_receiver,
        ))
    }
}

#[tonic::async_trait]
impl Rinha for MyRinha {
    async fn pessoa_by_id(
        &self,
        request: Request<PessoaByIdRequest>,
    ) -> Result<Response<PessoaReply>, Status> {
        if let Some(json) = self
            .pessoa_by_id_map
            .get(&request.get_ref().id)
            .map(|json| json.clone())
        {
            Ok(Response::new(PessoaReply { json: Some(json) }))
        } else {
            let json = sqlx::query_as::<_, Pessoa>(
                "
        SELECT * FROM pessoas where id = $1;
    ",
            )
            .bind(request.get_ref().id.as_str())
            .fetch_one(&self.db)
            .await
            .map(|pessoa| serde_json::to_string(&pessoa).ok())
            .ok()
            .flatten();
            if let Some(json) = json.as_ref() {
                self.pessoa_by_id_map
                    .insert(request.get_ref().id.clone(), json.clone());
            }
            Ok(Response::new(PessoaReply { json }))
        }
    }

    async fn pessoa_search(
        &self,
        request: Request<PessoaSearchRequest>,
    ) -> Result<Response<PessoaSearchReply>, Status> {
        let term = request.into_inner().term;
        if let Some(json) = self.pessoa_search_map.get(&term) {
            return Ok(Response::new(PessoaSearchReply {
                json: Some(json.clone()),
            }));
        }
        let term_param = format!("%{}%", term);
        let search_res = sqlx::query_as::<sqlx::Postgres, Pessoa>(
            "
            SELECT id, apelido, nome, nascimento, stack FROM pessoas p where p.busca_trgm LIKE $1 LIMIT 50;
        ",
        )
        .bind(&term_param)
        .persistent(true)
        .fetch_all(&self.db).await.ok().map(|res| serde_json::to_string(&res).ok()).flatten();
        if let Some(search_res) = search_res.as_ref() {
            self.pessoa_search_map.insert(term, search_res.clone());
        }
        Ok(Response::new(PessoaSearchReply { json: search_res }))
    }

    async fn create_pessoa(
        &self,
        request: Request<CreatePessoaRequest>,
    ) -> Result<Response<CreatePessoaReply>, Status> {
        let request = request.into_inner();
        if self.pessoa_by_apelido_exists_set.contains(&request.apelido) {
            return Ok(Response::new(CreatePessoaReply {
                id: None,
                status: 422,
            }));
        }
        if let Some(pessoa) = Pessoa::from(request) {
            let id = pessoa.id.clone();
            self.pessoa_by_id_map
                .insert(id.clone(), serde_json::to_string(&pessoa).unwrap());
            self.pessoa_by_apelido_exists_set.insert(id.clone());
            let _ = self.pessoa_sender.send(pessoa);
            self.pessoa_create_count
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(Response::new(CreatePessoaReply {
                id: Some(id),
                status: 201,
            }))
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

    async fn count_success_create_pessoa(
        &self,
        _: Request<CountPessoaRequest>,
    ) -> Result<Response<CountPessoaReply>, Status> {
        Ok(Response::new(CountPessoaReply {
            amount: self
                .pessoa_create_count
                .load(std::sync::atomic::Ordering::Acquire),
        }))
    }
}

async fn batch_insert(pessoas_to_insert: &mut Vec<Pessoa>, db: &PgPool) {
    if pessoas_to_insert.len() > 0 {
        let mut query = sqlx::QueryBuilder::<sqlx::Postgres>::new(
            "INSERT INTO pessoas (id, nome, apelido, nascimento, stack) ",
        );
        query.push_values(pessoas_to_insert.drain(..), |mut b, pessoa| {
            b.push_bind(pessoa.id)
                .push_bind(pessoa.nome)
                .push_bind(pessoa.apelido)
                .push_bind(pessoa.nascimento)
                .push_bind(pessoa.stack.map(|stacks| stacks.join(" ")));
        });
        query.push(" ON CONFLICT DO NOTHING;");
        if let Ok(mut tx) = db.begin().await {
            let _ = if query.build().execute(&mut *tx).await.is_ok() {
                tx.commit().await
            } else {
                tx.rollback().await
            };
        }
    }
}

enum PessoaOrTimeout {
    ReceiverClosed,
    Timeout,
    Pessoa(Pessoa),
}

pub async fn batch_insert_task(
    mut pessoa_receiver: UnboundedReceiver<Pessoa>,
    db: PgPool,
    env_values: Arc<EnvironmentValues>,
) {
    let mut pessoas_to_insert = Vec::with_capacity(env_values.batch_max_insert_size);
    loop {
        let pessoa_fut = pessoa_receiver.recv();
        let sleep_fut = tokio::time::sleep(Duration::from_secs(
            env_values.batch_max_wait_on_insert_channel,
        ));
        match select! {
            pessoa = pessoa_fut => pessoa.map(PessoaOrTimeout::Pessoa).unwrap_or(PessoaOrTimeout::ReceiverClosed),
            _ = sleep_fut => PessoaOrTimeout::Timeout,
        } {
            PessoaOrTimeout::Pessoa(pessoa) => {
                pessoas_to_insert.push(pessoa);
                if pessoas_to_insert.len() == env_values.batch_max_insert_size {
                    batch_insert(&mut pessoas_to_insert, &db).await
                }
            }
            PessoaOrTimeout::Timeout => batch_insert(&mut pessoas_to_insert, &db).await,
            PessoaOrTimeout::ReceiverClosed => break,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    let (rinha_svc, pessoa_receiver) = MyRinha::from(&env_values).await?;
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
    tracing::info!(message = "Starting server.", %addr);
    tokio::spawn(batch_insert_task(
        pessoa_receiver,
        rinha_svc.db.clone(),
        env_values.clone(),
    ));
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
