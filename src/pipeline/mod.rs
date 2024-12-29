use std::path::PathBuf;
use log::debug;
use picturium_libvips::Vips;
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
mod icc;

pub type PipelineResult<T> = Result<T, PipelineError>;

#[derive(Debug)]
pub struct PipelineError(pub String);

pub enum PipelineOutput {
    Image(PathBuf),
    OutputFormat(OutputFormat)
}

pub async fn run(url_parameters: &UrlParameters<'_>, output_format: OutputFormat) -> PipelineResult<PipelineOutput> {
    let mut image = thumbnail::run(url_parameters.path, url_parameters).await?;

    if output_format == OutputFormat::Pdf {
        return Ok(PipelineOutput::Image(cache::get_document_path_from_url_parameters(url_parameters).into()));
    }

    if is_svg(url_parameters.path) {
        image = rasterize::run(image, url_parameters).await?;
    }

    debug!("Performing autorotate");
    image = rotate::autorotate(image).await?;

    let valid_output_format = validate_output_format(&image, url_parameters, &output_format)?;

    if valid_output_format != output_format {
        return Ok(PipelineOutput::OutputFormat(valid_output_format));
    }

    let output_format = valid_output_format;

    // if url_parameters.crop.is_some() {
    //     crop::run(&image, &url_parameters, &output_format).await?;
    // }

    if url_parameters.width.is_some() || url_parameters.height.is_some() {
        image = resize::run(image, url_parameters).await?;
    }

    if url_parameters.rotate != Rotate::No {
        debug!("Rotating image");
        image = rotate::run(image, url_parameters).await?;
    }

    if supports_transparency(url_parameters.path) {
        debug!("Applying background");
        image = background::run(image, url_parameters).await?;
    }

    debug!("Performing ICC transform");
    image = icc::transform(image).await?;

    let result = match finalize::run(image, url_parameters, &output_format).await {
        Ok(result) => Ok(PipelineOutput::Image(result)),
        Err(e) => Err(e)
    };

    Vips::thread_shutdown();
    result
}