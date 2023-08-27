use std::{collections::HashSet, time::Duration};

use crate::{
    models::pessoa::{Pessoa, PessoaInput},
    utils::app_state::AppState,
};
use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;
use sqlx::{Error as SqlxError, Executor, FromRow};
use uuid::Uuid;

#[actix_web::post("/pessoas")]
pub async fn create(
    input: web::Json<PessoaInput>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let pessoa = if let Some(pessoa) = Option::<Pessoa>::from(input.into_inner()) {
        if let Ok(mut conn) = app_state.redis.get() {
            let key = format!("/pessoas/apelido/{}", pessoa.apelido);
            if redis::cmd("GET")
                .arg(key.as_str())
                .query::<Option<bool>>(&mut conn)
                .ok()
                .flatten()
                .is_some()
            {
                return HttpResponse::UnprocessableEntity().finish();
            } else {
                let pessoa_str = serde_json::to_string(&pessoa).unwrap();
                let _ = redis::cmd("MSET")
                    .arg(key.as_str())
                    .arg(true)
                    .arg(format!("/pessoas/id/{}", pessoa.id))
                    .arg(pessoa_str)
                    .execute(&mut conn);
            }
        } else {
            return HttpResponse::InternalServerError().finish();
        }
        pessoa
    } else {
        return HttpResponse::BadRequest().finish();
    };
    let location = format!("/pessoas/{}", pessoa.id);
    (app_state.as_ref().person_queue.as_ref()).push(pessoa);
    HttpResponse::Created()
        .append_header(("Location", location))
        .finish()
}

#[actix_web::get("/pessoas/{id}")]
pub async fn get(id: web::Path<Uuid>, app_state: web::Data<AppState>) -> impl Responder {
    let id = id.into_inner();
    let person = if let Ok(mut conn) = app_state.redis.get() {
        if let Some(person) = redis::cmd("GET")
            .arg(format!("/pessoas/id/{}", id))
            .query::<String>(&mut conn)
            .map(|val| serde_json::from_str::<Pessoa>(&val).ok())
            .ok()
            .flatten()
        {
            return HttpResponse::Ok().json(person);
        } else {
            sqlx::query_as::<_, Pessoa>(
                "
        SELECT * FROM pessoas where id = $1;
    ",
            )
            .bind(&id.to_string())
            .fetch_one(&app_state.db)
            .await
        }
    } else {
        return HttpResponse::InternalServerError().finish();
    };
    match person {
        Ok(person) => HttpResponse::Ok().json(person),
        Err(SqlxError::RowNotFound) => HttpResponse::NotFound().finish(),
        _ => HttpResponse::InternalServerError().finish(),
    }
}

#[derive(Deserialize)]
pub struct SearchInput {
    t: String,
}

#[actix_web::get("/pessoas")]
pub async fn all(input: web::Query<SearchInput>, app_state: web::Data<AppState>) -> impl Responder {
    let term = format!("%{}%", input.t);
    let query = sqlx::query(
        "
        SELECT * FROM pessoas p where p.busca_trgm LIKE $1 LIMIT 50;
    ",
    )
    .bind(&term);
    let persons = app_state.db.fetch_all(query).await;
    match persons {
        Ok(persons) => HttpResponse::Ok().json(
            persons
                .into_iter()
                .filter_map(|p| Pessoa::from_row(&p).ok())
                .collect::<Vec<_>>(),
        ),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[actix_web::get("/contagem-pessoas")]
pub async fn count(app_state: web::Data<AppState>) -> impl Responder {
    let amount = sqlx::query_as::<_, (i64,)>("SELECT COUNT(id) FROM pessoas;")
        .fetch_one(&app_state.db)
        .await;
    match amount {
        Ok(amount) => HttpResponse::Ok().json(amount.0),
        _ => HttpResponse::InternalServerError().finish(),
    }
}

const BATCH_INSERT_INTERVAL_SECS: u64 = 3;

pub async fn batch_insert_task(app_state: AppState) {
    let mut apelidos = HashSet::<String>::new();
    loop {
        tokio::time::sleep(Duration::from_secs(BATCH_INSERT_INTERVAL_SECS)).await;
        let mut pessoas_to_insert = Vec::with_capacity(128);
        while app_state.person_queue.len() > 0 {
            let input = app_state.person_queue.pop().await;
            if apelidos.contains(&input.apelido) {
                continue;
            } else {
                apelidos.insert(input.apelido.clone());
            }
            pessoas_to_insert.push(input);
        }
        if pessoas_to_insert.len() > 0 {
            let mut query = sqlx::QueryBuilder::<sqlx::Postgres>::new(
                "INSERT INTO pessoas (id, nome, apelido, nascimento, stack) ",
            );
            query.push_values(pessoas_to_insert, |mut b, pessoa| {
                b.push_bind(pessoa.id)
                    .push_bind(pessoa.nome)
                    .push_bind(pessoa.apelido)
                    .push_bind(pessoa.nascimento)
                    .push_bind(pessoa.stack.map(|stacks| stacks.join(" ")));
            });
            query.push(" ON CONFLICT DO NOTHING;");
            if let Ok(mut tx) = app_state.db.begin().await {
                let _ = if query.build().execute(&mut *tx).await.is_ok() {
                    tx.commit().await
                } else {
                    tx.rollback().await
                };
            }
        }
    }
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(create).service(get).service(all).service(count);
}
