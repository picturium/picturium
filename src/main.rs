use std::env;
use std::fs::File;

use actix_cors::Cors;
use actix_web::{App, HttpServer};
use actix_web::middleware::Logger;
use dotenv::dotenv;
use log::{error, LevelFilter};
use picturium_libvips::{Cache, Vips};
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
    
    let _scheduler_handle = services::scheduler::schedule();

    let vips_concurrency = env::var("VIPS_CONCURRENCY").unwrap_or("0".into()).parse::<i32>().unwrap_or(0);
    let mut workers = env::var("WORKERS").unwrap_or("0".into()).parse::<usize>().unwrap_or(0);

    if workers <= 0 {
        workers = num_cpus::get();
    }

    let app = match Vips::new("picturium") {
        Ok(vips) => vips,
        Err(e) => {
            error!("Failed to initialize libvips: {e}");
            std::process::exit(1);
        },
    };

    app.concurrency(vips_concurrency);
    app.cache(Cache::disabled());
    app.check_leaks();

    HttpServer::new(|| {

        let mut cors = Cors::default()
            .max_age(86400);
        
        let domains = env::var("CORS").unwrap();
        let domains = domains.split(',').skip_while(|x| x.is_empty());
        
        if domains.clone().count() == 0 {
            cors = cors.allow_any_origin();
        } else {
            for domain in domains {
                cors = cors.allowed_origin(domain);
            }
        }

        cors = cors.allow_any_method().allow_any_header();

        App::new()
            .wrap(Logger::new("%t \"%r\" %s %b \"%{Referer}i\" \"%{User-Agent}i\""))
            .wrap(cors)
            .configure(routes)

    })
        .bind((env::var("HOST").unwrap(), env::var("PORT").unwrap().parse().unwrap()))?
        .workers(workers)
        .run()
        .await

}