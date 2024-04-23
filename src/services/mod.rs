use std::collections::HashMap;
use std::{env, thread};

use actix_files::NamedFile;
use actix_web::{get, HttpRequest, HttpResponse, Responder};
use actix_web::http::header;
use actix_web::http::header::{ContentDisposition, DispositionParam, DispositionType};
use actix_web::web::{Path, Query};
use log::{debug, error};

use crate::parameters::{RawUrlParameters, UrlParameters};
use crate::pipeline;
use crate::cache;

pub mod formats;
pub mod vips;
pub mod scheduler;

#[get("{path:.*}")]
pub async fn serve(req: HttpRequest, path: Path<String>, parameters: Query<HashMap<String, String>>, raw_url_parameters: Query<RawUrlParameters>) -> impl Responder {

    let path = path.into_inner();

    if let Err(e) = raw_url_parameters.verify_token(&path, &parameters) {
        return HttpResponse::Forbidden().body(e);
    }

    let url_parameters = UrlParameters::new(&path, raw_url_parameters.into_inner());

    // Check if original file exists
    if !url_parameters.path.exists() {
        return HttpResponse::NotFound().into();
    }

    // Serve original image or file
    if url_parameters.original || formats::check_supported_input_formats(url_parameters.path).is_err() {
        return match NamedFile::open(url_parameters.path) {
            Ok(named_file) => {
                let mut response = NamedFile::into_response(named_file.prefer_utf8(true), &req);
                response.headers_mut().insert(header::CACHE_CONTROL, header::HeaderValue::from_static("public, max-age=604800, must-revalidate"));
                response
            },
            Err(_) => HttpResponse::BadRequest().into()
        };
    }

    let output_format = formats::determine_output_format(req.headers().get("Accept"));
    let cache_path = cache::get_path_from_url_parameters(&url_parameters, &output_format);

    let cache_enable = env::var("CACHE_ENABLE").unwrap_or("true".to_string()) == "true";

    // Return from cache
    if cache_enable && cache::is_cached(&cache_path, &url_parameters) {
        debug!("Using cache @{cache_path}");

        let mut response = NamedFile::open(cache_path).unwrap().set_content_disposition(
            ContentDisposition {
                disposition: DispositionType::Inline,
                parameters: vec![DispositionParam::Filename(url_parameters.path.file_name().unwrap().to_string_lossy().into())]
            }
        ).into_response(&req);

        response.headers_mut().insert(header::CACHE_CONTROL, header::HeaderValue::from_static("public, max-age=604800, must-revalidate"));
        return response;
    }

    debug!("Running pipeline for @{cache_path}");

    // Process image
    let output = match pipeline::run(&url_parameters, output_format).await {
        Ok(output) => output,
        Err(e) => {
            error!("Failed to process image: {}", e.0);
            return HttpResponse::InternalServerError().into();
        }
    };

    match NamedFile::open(output) {
        Ok(named_file) => {

            let named_file = named_file.set_content_disposition(
                ContentDisposition {
                    disposition: DispositionType::Inline,
                    parameters: vec![DispositionParam::Filename(url_parameters.path.file_name().unwrap().to_string_lossy().into())]
                }
            );

            let path = url_parameters.path.to_owned();
            thread::spawn(move || cache::index(cache_path, path));
            
            let mut response = NamedFile::into_response(named_file.prefer_utf8(true), &req);
            response.headers_mut().insert(header::CACHE_CONTROL, header::HeaderValue::from_static("public, max-age=604800, must-revalidate"));
            
            response

        },
        Err(_) => HttpResponse::InternalServerError().into()
    }

}