use std::path::PathBuf;

use log::debug;

use crate::cache;
use crate::parameters::{Rotate, UrlParameters};
use crate::services::formats::{is_svg, OutputFormat, supports_transparency, validate_output_format};

mod thumbnail;
mod rotate;
mod resize;
mod crop;
mod finalize;
mod rasterize;
mod background;
mod mpv_thumb;

pub type PipelineResult<T> = Result<T, PipelineError>;

#[derive(Debug)]
pub struct PipelineError(pub String);

pub async fn run(url_parameters: &UrlParameters<'_>, output_format: OutputFormat) -> PipelineResult<PathBuf> {

    debug!("Running pipeline for {}", url_parameters.path.to_string_lossy());

    let mut image = thumbnail::run(url_parameters.path, url_parameters).await?;

    if output_format == OutputFormat::Pdf {
        return Ok(cache::get_document_path_from_url_parameters(url_parameters).into());
    }
    
    if is_svg(url_parameters.path) {
        image = rasterize::run(image, url_parameters).await?;
    }
    
    image = rotate::autorotate(image).await?;

    // if url_parameters.crop.is_some() {
    //     crop::run(&image, &url_parameters, &output_format).await?;
    // }

    let output_format = validate_output_format(&image, url_parameters, &output_format)?;
    
    if url_parameters.width.is_some() || url_parameters.height.is_some() {
        image = resize::run(image, url_parameters).await?;
    }

    debug!("Before rotate");

    if url_parameters.rotate != Rotate::No {
        image = rotate::run(image, url_parameters).await?;
    }

    debug!("Checking if background is required");

    if supports_transparency(url_parameters.path) && output_format != OutputFormat::Jpg {
        image = background::run(image, url_parameters).await?;
    }

    finalize::run(image, url_parameters, &output_format).await

}