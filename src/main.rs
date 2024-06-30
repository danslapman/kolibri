extern crate clap;

use crate::api::exec::ExecHandler;
use crate::api::resolver::StubResolver;
use crate::model::persistent::{HttpStub, State};
use actix_web::{App, HttpServer};
use actix_web::web::Data;
use clap::Parser;
use simple_logger::SimpleLogger;
use std::fs::File;
use std::io::{BufReader, Read};
use std::sync::Mutex;

pub mod api;
pub mod error;
pub mod model;
pub mod predicate_dsl;
pub mod utils;

#[derive(Parser, Debug)]
#[clap(
    author = "Daniel Slapman <danslapman@gmail.com>",
    version = "0.1",
    about = "Standalone mocking server"
)]
struct Args {
    #[clap(help = "File containing mock configurations")]
    mocks: String
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();

    let mock_file = File::open(args.mocks)?;
    let mut mock_file_contents = "".to_string();
    BufReader::new(mock_file).read_to_string(&mut mock_file_contents)?;

    let mocks = serde_json::from_str::<Vec<HttpStub>>(&mock_file_contents)?;

    let states: Vec<State> = Vec::new();

    let stub_resolver = StubResolver::new(mocks, Mutex::new(states));

    let exec_handler = Data::new(ExecHandler::new(stub_resolver));

    SimpleLogger::new()
        .env()
        .with_level(log::LevelFilter::Debug)
        .with_local_timestamps()
        .init()
        .unwrap();

    HttpServer::new(move || {
        App::new()
            .app_data(exec_handler.clone())
            .service(api::exec_get)
            .service(api::exec_post)
    })
        .bind(("0.0.0.0", 8080))?
        .run()
        .await
}
