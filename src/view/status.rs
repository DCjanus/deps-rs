use actix_web::{HttpResponse, Responder};

use crate::model::{CrateIdentity, RepoIdentity, Status};

#[get("/repo/{site}/{owner}/{repo}/status.svg")]
pub async fn repo_svg(input: actix_web::web::Path<RepoIdentity>) -> impl Responder {
    let status = match crate::analyze::analyze_repo(input.as_ref()).await {
        Ok(x) => x.into_iter().map(|x| x.status()).sum::<Status>(),
        Err(e) => {
            error!("{:?}", e);
            Status::Unknown
        }
    };
    HttpResponse::Ok()
        .content_type("image/svg+xml;charset=utf-8")
        .body(status.to_svg())
}

#[get("/crate/{name}/{version}/status.svg")]
pub async fn crate_svg(input: actix_web::web::Path<CrateIdentity>) -> impl Responder {
    let status = match crate::analyze::analyze_crate(&input.name, input.version.clone()) {
        None => Status::Unknown,
        Some(x) => x.status(),
    };
    HttpResponse::Ok()
        .content_type("image/svg+xml;charset=utf-8")
        .body(status.to_svg())
}
