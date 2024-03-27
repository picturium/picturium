use std::collections::HashMap;

use actix_files::NamedFile;
use actix_web::{get, HttpRequest, HttpResponse, Responder};
use actix_web::web::{Path, Query};
use log::error;

use crate::parameters::{RawUrlParameters, UrlParameters};
use crate::pipeline;
use crate::cache;

pub mod formats;
pub mod vips;

#[get("{path:.*}")]
pub async fn serve(req: HttpRequest, path: Path<String>, parameters: Query<HashMap<String, String>>, raw_url_parameters: Query<RawUrlParameters>) -> impl Responder {

    let path = path.into_inner();

    if let Err(e) = raw_url_parameters.verify_token(&path, &parameters) {
        return HttpResponse::Forbidden().body(e);
    }

    let url_parameters = UrlParameters::new(&path, raw_url_parameters.into_inner());

    // Check if the input format is supported
    if formats::check_supported_input_formats(url_parameters.path).is_err() {
        return HttpResponse::BadRequest().body("Unsupported input format");
    }

    // Check if original image exists
    if !url_parameters.path.exists() {
        return HttpResponse::NotFound().into();
    }

    let output_format = formats::determine_output_format(req.headers().get("Accept"));
    let cache_path = cache::get_path_from_url_parameters(&url_parameters, &output_format);

    if cache::is_cached(&cache_path, &url_parameters) {
        return NamedFile::open(cache_path).unwrap().into_response(&req);
    }

    // Serve original image
    if url_parameters.original {
        return match NamedFile::open(url_parameters.path) {
            Ok(named_file) => NamedFile::into_response(named_file.prefer_utf8(true), &req),
            Err(_) => HttpResponse::BadRequest().into()
        };
    }
    
    let output = match pipeline::run(url_parameters, output_format).await {
        Ok(output) => output,
        Err(e) => {
            error!("Failed to process image: {}", e.0);
            return HttpResponse::InternalServerError().into();
        }
    };

    match NamedFile::open(output) {
        Ok(named_file) => NamedFile::into_response(named_file.prefer_utf8(true), &req),
        Err(_) => HttpResponse::InternalServerError().into()
    }

}