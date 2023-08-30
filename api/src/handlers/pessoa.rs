use crate::{
    models::pessoa::PessoaInput,
    rinha::{PessoaByIdRequest, PessoaSearchRequest},
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
pub async fn all(input: web::Query<SearchInput>, app_state: web::Data<AppState>) -> impl Responder {
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
    let amount = sqlx::query_as::<_, (i64,)>("SELECT COUNT(id) FROM pessoas;")
        .fetch_one(&app_state.db)
        .await;
    match amount {
        Ok(amount) => HttpResponse::Ok().json(amount.0),
        _ => HttpResponse::InternalServerError().finish(),
    }
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(create).service(get).service(all).service(count);
}
