use std::{collections::HashSet, time::Duration};

use crate::{
    models::pessoa::{Pessoa, PessoaInput},
    utils::app_state::AppState,
};
use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;
use sqlx::Error as SqlxError;

#[actix_web::post("/pessoas")]
pub async fn create(
    input: web::Json<PessoaInput>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let pessoa = if let Some(pessoa) = Pessoa::from(input.into_inner()) {
        if let Ok(mut conn) = app_state.redis.get().await {
            let apelido_key = format!("/pessoas/apelido/{}", pessoa.apelido);
            if let Ok(Some(_)) = redis::cmd("GET")
                .arg(apelido_key.as_str())
                .query_async::<_, Option<()>>(&mut conn)
                .await
            {
                return HttpResponse::UnprocessableEntity().finish();
            } else if let Ok(pessoa_str) = serde_json::to_string(&pessoa) {
                let id_key = format!("/pessoas/id/{}", pessoa.id);
                let _ = redis::cmd("MSET")
                    .arg(&[apelido_key.as_str(), "true", &id_key, pessoa_str.as_str()])
                    .query_async::<_, ()>(&mut conn)
                    .await;
            }
            pessoa
        } else {
            return HttpResponse::InternalServerError().finish();
        }
    } else {
        return HttpResponse::BadRequest().finish();
    };
    let location = format!("/pessoas/{}", pessoa.id);
    app_state.person_queue.push(pessoa);
    HttpResponse::Created()
        .append_header(("Location", location))
        .finish()
}

#[actix_web::get("/pessoas/{id}")]
pub async fn get(id: web::Path<String>, app_state: web::Data<AppState>) -> impl Responder {
    let person = if let Ok(mut conn) = app_state.redis.get().await {
        if let Ok(person) = redis::cmd("GET")
            .arg(format!("/pessoas/id/{}", id))
            .query_async::<_, String>(&mut conn)
            .await
        {
            return HttpResponse::Ok()
                .append_header(actix_web::http::header::ContentType::json())
                .body(person);
        } else {
            sqlx::query_as::<_, Pessoa>(
                "
        SELECT * FROM pessoas where id = $1;
    ",
            )
            .bind(id.as_str())
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

async fn _get_cached_search(term: &str, app_state: &AppState) -> Option<String> {
    let mut conn = app_state.redis.get().await.ok()?;
    redis::cmd("GET")
        .arg(format!("/pessoas/search/{}", term))
        .query_async::<_, String>(&mut conn)
        .await
        .ok()
}

async fn set_cached_search(term: String, pessoas: String, app_state: web::Data<AppState>) {
    if let Ok(mut conn) = app_state.redis.get().await {
        let _ = redis::cmd("SET")
            .arg(format!("/pessoas/search/{}", term))
            .arg(pessoas)
            .arg("EX")
            .arg(15)
            .query_async::<_, ()>(&mut conn)
            .await;
    }
}

#[actix_web::get("/pessoas")]
pub async fn all(input: web::Query<SearchInput>, app_state: web::Data<AppState>) -> impl Responder {
    let term = format!("%{}%", input.t);
    {
        if let Ok(mut conn) = app_state.redis.get().await {
            if let Ok(cached) = redis::cmd("GET")
                .arg(format!("/pessoas/search/{}", term))
                .query_async::<_, String>(&mut conn)
                .await
            {
                return HttpResponse::Ok()
                    .append_header(actix_web::http::header::ContentType::json())
                    .body(cached);
            }
        }
    }
    match sqlx::query_as::<sqlx::Postgres, Pessoa>(
        "
        SELECT id, apelido, nome, nascimento, stack FROM pessoas p where p.busca_trgm LIKE $1 LIMIT 50;
    ",
    )
    .bind(&term)
    .persistent(true)
    .fetch_all(&app_state.db).await {
        Ok(pessoas) => {
            if let Ok(pessoas) = serde_json::to_string(&pessoas) {
                tokio::spawn(set_cached_search(
                    input.into_inner().t,
                    pessoas.clone(),
                    app_state.clone(),
                ));
                HttpResponse::Ok()
                    .append_header(actix_web::http::header::ContentType::json())
                    .body(pessoas)
            } else {
                HttpResponse::InternalServerError().finish()
            }
        }
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

const BATCH_INSERT_INTERVAL_SECS: u64 = 2;

pub async fn batch_insert_task(app_state: AppState) {
    let mut apelidos = HashSet::<String>::new();
    let mut pessoas_to_insert = Vec::with_capacity(512);
    loop {
        tokio::time::sleep(Duration::from_secs(BATCH_INSERT_INTERVAL_SECS)).await;
        while app_state.person_queue.len() > 0 && pessoas_to_insert.len() < 512 {
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
            query.push_values(pessoas_to_insert.drain(..), |mut b, pessoa| {
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
