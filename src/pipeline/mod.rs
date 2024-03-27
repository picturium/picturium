mod thumbnail;
mod rotate;
mod resize;
mod crop;
mod finalize;

use std::path::PathBuf;
use libvips::VipsImage;
use crate::parameters::{Rotate, UrlParameters};
use crate::services::formats::OutputFormat;

pub type PipelineResult<T> = Result<T, PipelineError>;

#[derive(Debug)]
pub struct PipelineError(pub String);

pub async fn run(url_parameters: UrlParameters<'_>, output_format: OutputFormat) -> PipelineResult<PathBuf> {

    let working_file: PathBuf = url_parameters.path.into();

    // if url_parameters.thumbnail.is_some() {
    //     thumbnail::run(&mut working_file, &url_parameters, &output_format).await?;
    // }
    
    let mut image = match VipsImage::new_from_file(&working_file.to_string_lossy()) {
        Ok(image) => image,
        Err(error) => return Err(PipelineError(format!("Failed to open image: {}", error)))
    };

    if url_parameters.rotate != Rotate::No {
        image = rotate::run(&image, &url_parameters).await?;
    }

    // if url_parameters.crop.is_some() {
    //     crop::run(&image, &url_parameters, &output_format).await?;
    // }

    if url_parameters.width.is_some() || url_parameters.height.is_some() {
        image = resize::run(&image, &url_parameters).await?;
    }

    finalize::run(&image, &url_parameters, &output_format).await

}