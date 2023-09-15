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
    #[actix_web::post("/pessoas")]
    pub async fn create(
        input: web::Json<PessoaInput>,
        app_state: web::Data<AppState>,
    ) -> impl Responder {
        match Option::<Pessoa>::from(input.into_inner())
            .map(|pessoa| (pessoa.insert_query(), pessoa))
        {
            Some((Some(query), pessoa)) => {
                let id = pessoa.id.clone();

                if let Ok(conn) = app_state.pool.get().await {
                    if conn.query_one(&query, &[]).await.is_ok() {
                        let location = format!("/pessoas/{}", id);
                        return HttpResponse::Created()
                            .append_header(("Location", location))
                            .finish();
                    }
                }
                HttpResponse::UnprocessableEntity().finish()
            }
            _ => HttpResponse::BadRequest().finish(),
        }
    }

    #[actix_web::get("/pessoas/{id}")]
    pub async fn get(id: web::Path<String>, app_state: web::Data<AppState>) -> impl Responder {
        let id = id.into_inner();
        if let Ok(conn) = app_state.pool.get().await {
            return match conn
                .query_one(
                    "
            SELECT * FROM pessoas where id = $1;
        ",
                    &[&id],
                )
                .await
                .map(Pessoa::from)
            {
                Ok(pessoa) => HttpResponse::Ok().json(pessoa),
                Err(_) => HttpResponse::NotFound().finish(),
            };
        } else {
            HttpResponse::InternalServerError().finish()
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
        if let Ok(conn) = app_state.pool.get().await {
            if let Ok(stmt) = conn
                .prepare_cached(
                    "
            SELECT ID, APELIDO, NOME, NASCIMENTO, STACK
            FROM PESSOAS P
            WHERE P.BUSCA_TRGM LIKE $1
            LIMIT 50;
        ",
                )
                .await
            {
                if let Ok(pessoas) = conn.query(&stmt, &[&term_param]).await {
                    return HttpResponse::Ok()
                        .json(pessoas.into_iter().map(Pessoa::from).collect::<Vec<_>>());
                }
            }
        }
        HttpResponse::InternalServerError().finish()
    }

    #[actix_web::get("/contagem-pessoas")]
    pub async fn count(app_state: web::Data<AppState>) -> impl Responder {
        if let Ok(conn) = app_state.pool.get().await {
            if let Ok(data) = conn.query_one("SELECT COUNT(id) FROM pessoas;", &[]).await {
                return HttpResponse::Ok().json(data.get::<usize, i64>(0));
            }
        }
        HttpResponse::InternalServerError().finish()
    }

    pub fn config(cfg: &mut web::ServiceConfig) {
        cfg.service(create).service(get).service(all).service(count);
    }
}
