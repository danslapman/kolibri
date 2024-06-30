use actix_web::{App, HttpServer, web};
use simple_logger::SimpleLogger;

pub mod api;
pub mod model;
pub mod predicate_dsl;
pub mod utils;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    SimpleLogger::new()
        .env()
        .with_level(log::LevelFilter::Debug)
        .with_local_timestamps()
        .init()
        .unwrap();

    HttpServer::new(move || {
        App::new()
            .service(api::exec_get)
            .service(api::exec_post)
    })
        .bind(("0.0.0.0", 8080))?
        .run()
        .await
}
