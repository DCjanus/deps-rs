use actix_web::{HttpResponse, Responder};

use crate::model::{Identity, Status};

#[get("/repo/{site}/{owner}/{repo}/status.svg")]
pub async fn svg(_input: actix_web::web::Path<Identity>) -> impl Responder {
    let status = Status::Known {
        total: 1024,
        outdated: 42,
    };

    HttpResponse::Ok()
        .content_type("image/svg+xml;charset=utf-8")
        .body(status.to_svg())
}
