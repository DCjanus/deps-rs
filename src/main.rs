#![allow(dead_code)] // TODO: remove this once whole project is done.

#[macro_use]
extern crate actix_web;
#[macro_use]
extern crate anyhow;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde;

use crate::utils::AnyResult;

mod analyze;
mod command;
mod fetch;
mod logger;
mod model;
mod parser;
mod utils;
mod version;
mod view;

#[actix_rt::main]
async fn main() -> AnyResult {
    init()?;

    actix_web::HttpServer::new(|| {
        actix_web::App::new()
            .wrap(actix_web::middleware::Logger::default())
            .service(crate::view::status::svg)
    })
    .bind("127.0.0.1:8000")?
    .run()
    .await?;

    Ok(())
}

fn init() -> AnyResult {
    crate::logger::init_logger()?;
    lazy_static::initialize(&crate::command::COMMAND);
    lazy_static::initialize(&crate::fetch::GLOBAL_CLIENT);
    crate::version::init()?;

    Ok(())
}
