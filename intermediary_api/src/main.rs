mod models;
mod utils;
use std::sync::Arc;
use std::time::Duration;

use dashmap::*;
use models::pessoa::Pessoa;
use rinha::rinha_server::{Rinha, RinhaServer};
use rinha::{
    CreatePessoaReply, CreatePessoaRequest, PessoaByIdRequest,
    PessoaReply, PessoaSearchReply, PessoaSearchRequest,
};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tonic::{transport::Server, Request, Response, Status};
use tower_http::trace::TraceLayer;
use utils::env::EnvironmentValues;

pub mod rinha {
    tonic::include_proto!("rinha");
}

#[derive(Debug)]
pub struct MyRinha {
    pub pessoa_by_apelido_exists_set: DashSet<String>,
    pub pessoa_by_id_map: DashMap<String, String>,
    pub pessoa_search_map: DashMap<String, String>,
    pub person_queue: Arc<deadqueue::unlimited::Queue<Pessoa>>,
    pub db: PgPool,
}

impl MyRinha {
    pub async fn from(env_values: &EnvironmentValues) -> Result<Self, Box<dyn std::error::Error>> {
        let db = PgPoolOptions::new()
            .max_connections(256)
            .min_connections(32)
            .connect(&env_values.database_url)
            .await?;
        Ok(Self {
            db,
            person_queue: Arc::new(deadqueue::unlimited::Queue::new()),
            pessoa_by_apelido_exists_set: Default::default(),
            pessoa_by_id_map: Default::default(),
            pessoa_search_map: Default::default(),
        })
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
            self.person_queue.push(pessoa);
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
}

const BATCH_INSERT_INTERVAL_SECS: u64 = 2;

pub async fn batch_insert_task(queue: Arc<deadqueue::unlimited::Queue<Pessoa>>, db: PgPool) {
    let mut pessoas_to_insert = Vec::with_capacity(256);
    loop {
        tokio::time::sleep(Duration::from_secs(BATCH_INSERT_INTERVAL_SECS)).await;
        while queue.len() > 0 && pessoas_to_insert.len() < 256 {
            let input = queue.pop().await;
            pessoas_to_insert.push(input);
        }
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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
    .with_max_level(tracing::Level::from_str(&env_values.rust_log)?)
    .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .init();
    let addr = "[::]:50051".parse()?;
    let env_values = EnvironmentValues::init();
    let rinha_svc = MyRinha::from(&env_values).await?;
    tracing::info!(message = "Starting server.", %addr);
    tokio::spawn(batch_insert_task(rinha_svc.person_queue.clone(), rinha_svc.db.clone()));
    Server::builder()
        .layer(TraceLayer::new_for_grpc())
        .add_service(RinhaServer::new(rinha_svc))
        .serve(addr)
        .await?;
    Ok(())
}
