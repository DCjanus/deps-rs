#[macro_use]
extern crate actix_web;
#[macro_use]
extern crate serde;

mod logger;
mod model;
mod view;

#[actix_rt::main]
async fn main() -> anyhow::Result<()> {
    crate::logger::init_logger()?;

    actix_web::HttpServer::new(|| {
        actix_web::App::new()
            .wrap(actix_web::middleware::Logger::default())
            .service(crate::view::status::svg)
    })
    .bind("127.0.0.1:8080")?
    .start()
    .await?;

    Ok(())
}
