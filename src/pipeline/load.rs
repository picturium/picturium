use std::env;
use crate::parameters::UrlParameters;
use crate::pipeline::{rasterize, PipelineError, PipelineResult};
use crate::services::formats::{is_svg, is_thumbnail_format};
use log::debug;
use picturium_libvips::{FromSvgOptions, VipsImage};
use std::path::Path;

pub(crate) async fn run(working_file: &Path, url_parameters: &UrlParameters<'_>) -> PipelineResult<Option<VipsImage>> {

    if is_thumbnail_format(url_parameters.path) {
        return Ok(None);
    }

    let filename = working_file.to_string_lossy();

    if is_svg(url_parameters.path) {

        let dpi = url_parameters.load.dpi.unwrap_or(env::var("SVG_DPI").unwrap_or("72".into()).parse().unwrap_or(72.0));
        let unlimited = env::var("SVG_UNLIMITED").unwrap_or("true".to_string()) == "true";
        
        let image = match VipsImage::new_from_svg(&filename, FromSvgOptions {
            dpi,
            unlimited,
            ..FromSvgOptions::default()
        }.into()) {
            Ok(image) => image,
            Err(error) => return Err(PipelineError(format!("Failed to load SVG: {}", error)))
        };

        debug!("Loaded SVG: {filename}");
        return Ok(Some(rasterize::run(image, url_parameters).await?));

    }

    match VipsImage::new_from_file(&filename, None) {
        Ok(image) => Ok(Some(image)),
        Err(error) => Err(PipelineError(format!("Failed to open image: {}", error)))
    }

}