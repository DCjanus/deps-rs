use crate::model::{Identity, Status};
use actix_web::{HttpResponse, Responder};

#[get("/repo/{site}/{owner}/{repo}/status.svg")]
pub async fn svg(_input: actix_web::web::Path<Identity>) -> impl Responder {
    let status = Status {
        total: 1024,
        outdated: 42,
    };

    HttpResponse::Ok()
        .content_type("image/svg+xml;charset=utf-8")
        .body(status.to_svg())
}
