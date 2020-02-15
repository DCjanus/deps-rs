use actix_web::{HttpResponse, Responder};
use askama::Template;

use crate::{
    analyze::AnalyzedCrate,
    model::{CrateIdentity, RepoIdentity, Status},
    view::html::{render_template, server_error_response},
};

use super::html::{CrateSectionTemplate, DependenciesTableTemplate};

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
        .body(status.to_badge().to_svg())
}

#[derive(Template, Debug)]
#[template(path = "repo_status.html", escape = "none")]
struct RepoHtmlTemplate<'a> {
    hero_class: &'static str,
    ident: &'a RepoIdentity,
    status: &'a Status,
    crates: Vec<CrateSectionTemplate>,
}

#[get("/repo/{site}/{owner}/{repo}")]
pub async fn repo_html(input: actix_web::web::Path<RepoIdentity>) -> HttpResponse {
    let analyze_result = match crate::analyze::analyze_repo(input.as_ref()).await {
        Ok(x) => x,
        Err(e) => {
            error!("{:?}", e);
            return server_error_response("failed to analyze given repo");
        }
    };

    let status = analyze_result.iter().map(|x| x.status()).sum::<Status>();
    let hero_class = match status {
        Status::Unknown => unreachable!(),
        Status::Insecure => "is-danger",
        Status::Normal { outdated, .. } => {
            if outdated > 0 {
                "is-warning"
            } else {
                "is-success"
            }
        }
    };

    let crates: Vec<CrateSectionTemplate> = analyze_result
        .into_iter()
        .map(|x: AnalyzedCrate| CrateSectionTemplate {
            name: x.name,
            dependencies: DependenciesTableTemplate::new(
                x.dependencies.into_iter().map(|d| d.into()).collect(),
            ),
            build_dependencies: DependenciesTableTemplate::new(
                x.build_dependencies.into_iter().map(|d| d.into()).collect(),
            ),
            dev_dependencies: DependenciesTableTemplate::new(
                x.dev_dependencies.into_iter().map(|d| d.into()).collect(),
            ),
        })
        .collect();

    render_template(RepoHtmlTemplate {
        hero_class,
        ident: &input,
        status: &status,
        crates,
    })
}

#[get("/crate/{name}/{version}/status.svg")]
pub async fn crate_svg(input: actix_web::web::Path<CrateIdentity>) -> impl Responder {
    let status = match crate::analyze::analyze_crate(&input.name, input.version.clone()) {
        None => Status::Unknown,
        Some(x) => x.status(),
    };
    HttpResponse::Ok()
        .content_type("image/svg+xml;charset=utf-8")
        .body(status.to_badge().to_svg())
}

#[derive(Debug, Template)]
#[template(path = "crate_status.html", escape = "none")]
struct CrateHtmlTemplate<'a> {
    hero_class: &'static str,
    ident: &'a CrateIdentity,
    status: Status,
    the_crate: CrateSectionTemplate,
}

#[get("/crate/{name}/{version}")]
pub fn crate_html(input: actix_web::web::Path<CrateIdentity>) -> HttpResponse {
    let analyze_result = match crate::analyze::analyze_crate(&input.name, input.version.clone()) {
        Some(x) => x,
        None => {
            error!("failed to analyze crate: {} {}", input.name, input.version);
            return server_error_response("failed to analyze given crate");
        }
    };

    let status = analyze_result.status();
    let hero_class = match status {
        Status::Unknown => unreachable!(),
        Status::Insecure => "is-danger",
        Status::Normal { outdated, .. } => {
            if outdated > 0 {
                "is-warning"
            } else {
                "is-success"
            }
        }
    };

    let the_crate = CrateSectionTemplate {
        name: analyze_result.name,
        dependencies: DependenciesTableTemplate::new(
            analyze_result
                .dependencies
                .into_iter()
                .map(|d| d.into())
                .collect(),
        ),
        build_dependencies: DependenciesTableTemplate::new(
            analyze_result
                .build_dependencies
                .into_iter()
                .map(|d| d.into())
                .collect(),
        ),
        dev_dependencies: DependenciesTableTemplate::new(
            analyze_result
                .dev_dependencies
                .into_iter()
                .map(|d| d.into())
                .collect(),
        ),
    };

    render_template(CrateHtmlTemplate {
        hero_class,
        ident: &input,
        status,
        the_crate,
    })
}
