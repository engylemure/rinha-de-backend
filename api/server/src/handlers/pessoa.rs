#[cfg(not(feature = "without_cache"))]
pub use with_cache::*;

#[cfg(feature = "without_cache")]
pub use without_cache::*;

#[cfg(not(feature = "without_cache"))]
mod with_cache {
    use crate::{
        models::pessoa::PessoaInput,
        rinha::{CountPessoaRequest, PessoaByIdRequest, PessoaSearchRequest},
        utils::app_state::AppState,
    };
    use actix_web::{http::StatusCode, web, HttpResponse, Responder};
    use serde::Deserialize;
    #[actix_web::post("/pessoas")]
    pub async fn create(
        input: web::Json<PessoaInput>,
        app_state: web::Data<AppState>,
    ) -> impl Responder {
        match app_state
            .rinha_client
            .clone()
            .create_pessoa(tonic::Request::new(input.into_inner().into()))
            .await
            .map(tonic::Response::into_inner)
            .ok()
        {
            Some(res) => {
                let mut response =
                    HttpResponse::build(StatusCode::from_u16(res.status as u16).unwrap());
                if let Some(id) = res.id {
                    let location = format!("/pessoas/{}", id);
                    response.append_header(("Location", location)).finish()
                } else {
                    response.finish()
                }
            }
            None => HttpResponse::InternalServerError().finish(),
        }
    }

    #[actix_web::get("/pessoas/{id}")]
    pub async fn get(id: web::Path<String>, app_state: web::Data<AppState>) -> impl Responder {
        match app_state
            .rinha_client
            .clone()
            .pessoa_by_id(tonic::Request::new(PessoaByIdRequest {
                id: id.into_inner(),
            }))
            .await
        {
            Ok(pessoa) => match pessoa.into_inner().json {
                Some(json) => HttpResponse::Ok()
                    .append_header(actix_web::http::header::ContentType::json())
                    .body(json),
                None => HttpResponse::NotFound().finish(),
            },
            Err(_) => HttpResponse::InternalServerError().finish(),
        }
    }

    #[derive(Deserialize)]
    pub struct SearchInput {
        t: String,
    }

    #[actix_web::get("/pessoas")]
    pub async fn all(
        input: web::Query<SearchInput>,
        app_state: web::Data<AppState>,
    ) -> impl Responder {
        match app_state
            .rinha_client
            .clone()
            .pessoa_search(tonic::Request::new(PessoaSearchRequest {
                term: input.into_inner().t,
            }))
            .await
            .ok()
            .map(|res| res.into_inner().json)
            .flatten()
        {
            Some(pessoas) => HttpResponse::Ok()
                .append_header(actix_web::http::header::ContentType::json())
                .body(pessoas),
            None => HttpResponse::InternalServerError().finish(),
        }
    }

    #[actix_web::get("/contagem-pessoas")]
    pub async fn count(app_state: web::Data<AppState>) -> impl Responder {
        match app_state
            .rinha_client
            .clone()
            .count_pessoa(tonic::Request::new(CountPessoaRequest {}))
            .await
            .ok()
            .map(|res| res.into_inner().amount)
        {
            Some(amount) => HttpResponse::Ok().json(amount),
            _ => HttpResponse::InternalServerError().finish(),
        }
    }

    pub fn config(cfg: &mut web::ServiceConfig) {
        cfg.service(create).service(get).service(all).service(count);
    }
}

#[cfg(feature = "without_cache")]
mod without_cache {
    use crate::{
        models::pessoa::{Pessoa, PessoaInput},
        utils::app_state::AppState,
    };
    use actix_web::{web, HttpResponse, Responder};
    use serde::Deserialize;
    use sqlx::Executor;
    #[actix_web::post("/pessoas")]
    pub async fn create(
        input: web::Json<PessoaInput>,
        app_state: web::Data<AppState>,
    ) -> impl Responder {
        match Pessoa::from(input.into_inner()) {
            Some(pessoa) => {
                let id = pessoa.id.clone();


                let insert_query = app_state.pool.get().await.query(

                );
                // let query = sqlx::query::<sqlx::Postgres>(
                //     "INSERT INTO pessoas (id, nome, apelido, nascimento, stack) values ($1, $2, $3, $4, $5)"
                // ).bind(pessoa.id)
                // .bind(pessoa.nome)
                // .bind(pessoa.apelido)
                // .bind(pessoa.nascimento)
                // .bind(pessoa.stack.unwrap_or_default());
            

                // if app_state.db.execute(query).await.is_ok() {
                //     let location = format!("/pessoas/{}", id);
                //     HttpResponse::Created()
                //         .append_header(("Location", location))
                //         .finish()
                // } else {
                //     HttpResponse::UnprocessableEntity().finish()
                // }
            }
            None => HttpResponse::BadRequest().finish(),
        }
    }

    #[actix_web::get("/pessoas/{id}")]
    pub async fn get(id: web::Path<String>, app_state: web::Data<AppState>) -> impl Responder {
        match sqlx::query_as::<_, Pessoa>(
            "
    SELECT * FROM pessoas where id = $1;
",
        )
        .persistent(true)
        .bind(id.into_inner())
        .fetch_one(&app_state.db)
        .await
        {
            Ok(pessoa) => HttpResponse::Ok().json(pessoa),
            Err(sqlx::Error::RowNotFound) => HttpResponse::NotFound().finish(),
            Err(_) => HttpResponse::InternalServerError().finish(),
        }
    }

    #[derive(Deserialize)]
    pub struct SearchInput {
        t: String,
    }

    #[actix_web::get("/pessoas")]
    pub async fn all(
        input: web::Query<SearchInput>,
        app_state: web::Data<AppState>,
    ) -> impl Responder {
        let term_param = format!("%{}%", input.t.to_lowercase());
        match sqlx::query_as::<sqlx::Postgres, Pessoa>(
            "
            SELECT id, apelido, nome, nascimento, stack FROM pessoas p where p.busca_trgm LIKE $1 LIMIT 50;
        ",
        )
        .bind(&term_param)
        .persistent(true)
        .fetch_all(&app_state.db).await {
            Ok(pessoas) => HttpResponse::Ok().json(pessoas),
            Err(_) => HttpResponse::InternalServerError().finish()
        }
    }

    #[actix_web::get("/contagem-pessoas")]
    pub async fn count(app_state: web::Data<AppState>) -> impl Responder {
        match sqlx::query_as::<_, (i64,)>("SELECT COUNT(id) FROM pessoas;")
            .fetch_one(&app_state.db)
            .await
        {
            Ok(amount) => HttpResponse::Ok().json(amount.0),
            _ => HttpResponse::InternalServerError().finish(),
        }
    }

    pub fn config(cfg: &mut web::ServiceConfig) {
        cfg.service(create).service(get).service(all).service(count);
    }
}
