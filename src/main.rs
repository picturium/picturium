use std::env;
use std::fs::File;

use actix_cors::Cors;
use actix_web::{App, HttpServer};
use actix_web::middleware::Logger;
use dotenv::dotenv;
use libvips::VipsApp;
use log::LevelFilter;
use simplelog::{ColorChoice, CombinedLogger, Config, TerminalMode, TermLogger, WriteLogger};

use crate::routes::routes;

mod routes;
mod services;
mod parameters;
mod crypto;
mod pipeline;
mod cache;

#[actix_web::main]
async fn main() -> std::io::Result<()> {

    dotenv().ok();

    let log_level = match env::var("LOG").unwrap().as_str() {
        "warn" => LevelFilter::Warn,
        "error" => LevelFilter::Error,
        "trace" => LevelFilter::Trace,
        "debug" => LevelFilter::Debug,
        "info" => LevelFilter::Info,
        _ => LevelFilter::Off,
    };

    CombinedLogger::init(
        vec![
            TermLogger::new(log_level, Config::default(), TerminalMode::Mixed, ColorChoice::Auto),
            WriteLogger::new(log_level, Config::default(), File::create("picturium.log").unwrap()),
        ]
    ).unwrap();

    let available_threads = num_cpus::get();
    let app = VipsApp::new("libvips instance", false).expect("Cannot initialize libvips instance");
    app.concurrency_set(available_threads as i32 / 2);

    HttpServer::new(|| {

        let mut cors = Cors::default()
            .max_age(86400);

        for domain in env::var("CORS").unwrap().split(',').skip_while(|x| x.is_empty()) {
            cors = cors.allowed_origin(domain);
        }

        App::new()
            .wrap(Logger::new("%t \"%r\" %s %b \"%{Referer}i\" \"%{User-Agent}i\""))
            .wrap(cors)
            .configure(routes)

    })
        .bind((env::var("HOST").unwrap(), env::var("PORT").unwrap().parse().unwrap()))?
        .run()
        .await

}