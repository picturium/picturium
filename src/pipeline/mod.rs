use std::path::PathBuf;
use libvips::ops::icc_transform;

use crate::parameters::{Rotate, UrlParameters};
use crate::services::formats::{is_svg, OutputFormat};

mod thumbnail;
mod rotate;
mod resize;
mod crop;
mod finalize;
mod rasterize;

pub type PipelineResult<T> = Result<T, PipelineError>;

#[derive(Debug)]
pub struct PipelineError(pub String);

pub async fn run(url_parameters: &UrlParameters<'_>, output_format: OutputFormat) -> PipelineResult<PathBuf> {

    let mut image = thumbnail::run(url_parameters.path, url_parameters).await?;
    
    if is_svg(url_parameters.path) {
        image = rasterize::run(&image, url_parameters).await?;
    }
    
    image = rotate::autorotate(&image).await?;

    // if url_parameters.crop.is_some() {
    //     crop::run(&image, &url_parameters, &output_format).await?;
    // }

    if url_parameters.width.is_some() || url_parameters.height.is_some() {
        image = resize::run(&image, url_parameters).await?;
    }

    if url_parameters.rotate != Rotate::No {
        image = rotate::run(&image, url_parameters).await?;
    }

    finalize::run(&image, url_parameters, &output_format).await

}