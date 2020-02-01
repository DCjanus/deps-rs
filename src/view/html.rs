use super::data::DepData;
use actix_web::{http::header::ContentType, HttpResponse};
use askama::Template;

#[derive(Template, Debug)]
#[template(path = "server_error.html")]
struct ServerErrorTemplate<'a> {
    message: &'a str,
}

pub fn server_error_response(e: impl AsRef<str>) -> HttpResponse {
    let template = ServerErrorTemplate {
        message: e.as_ref(),
    };

    HttpResponse::InternalServerError()
        .set(ContentType::html())
        .body(template.render().unwrap())
}

pub fn render_template(template: impl Template) -> HttpResponse {
    HttpResponse::Ok()
        .set(ContentType::html())
        .body(template.render().unwrap())
}

#[derive(Debug, Template)]
#[template(path = "dependencies_table.html")]
pub struct DependenciesTableTemplate {
    pub deps: Vec<DepData>,
    pub count_total: usize,
    pub count_insecure: usize,
    pub count_outdated: usize,
}

impl DependenciesTableTemplate {
    pub fn new(deps: Vec<DepData>) -> Self {
        let count_outdated = deps.iter().filter(|x| x.outdated).count();
        let count_insecure = deps.iter().filter(|x| x.insecure).count();
        let count_total = deps.len();
        Self {
            deps,
            count_total,
            count_outdated,
            count_insecure,
        }
    }
}

#[derive(Template, Debug)]
#[template(path = "crate_section.html", escape = "none")]
pub struct CrateSectionTemplate {
    pub name: String,
    pub dependencies: DependenciesTableTemplate,
    pub build_dependencies: DependenciesTableTemplate,
    pub dev_dependencies: DependenciesTableTemplate,
}
