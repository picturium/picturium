mod gif;
mod jpeg;
mod pdf;
mod svg;
mod tiff;
mod webp;

use crate::enums::dpi::Dpi;
use crate::enums::input::{InputFormat, VipsInputFormat};
use crate::process::pipeline::request::PipelineRequest;
use crate::process::source::Source;
use crate::services::size::calculate_processing_size;
use anyhow::{Result, anyhow};
use picturium_libvips::{FromFileOptions, FromSvgOptions, VipsAccess, VipsImage};
use std::path::PathBuf;

/// Load a file known to be vips-compatible from `source_path`.
/// The format is re-derived from the path extension so that temporary files
/// produced by conversion pipelines (video / office) are handled correctly.
pub fn load_file(request: &mut PipelineRequest, source_path: &str) -> Result<VipsImage> {
    let format = match Source::get_format(&PathBuf::from(source_path)) {
        InputFormat::Vips(format) => format,
        _ => {
            return Err(anyhow!(
                "Unsupported vips input format for source path: {}",
                source_path
            ));
        }
    };

    let image = load_vips_file(request, source_path, format)?;
    println!("Loaded image: {:?}", image);

    request.source.width = Some(image.get_width() as u16);
    request.source.height = Some(image.get_height() as u16);

    Ok(image)
}

fn load_vips_file(
    request: &PipelineRequest,
    source_path: &str,
    format: VipsInputFormat,
) -> Result<VipsImage> {
    match format {
        VipsInputFormat::Svg => svg::load(request, source_path),
        VipsInputFormat::Jpeg => jpeg::load(request, source_path),
        VipsInputFormat::Gif => gif::load(request, source_path),
        VipsInputFormat::Tiff => tiff::load(request, source_path),
        VipsInputFormat::Webp => webp::load(request, source_path),
        VipsInputFormat::Pdf => pdf::load(request, source_path),
        _ => default_load(source_path, None),
    }
}

fn default_load(source_path: &str, parameters: Option<Vec<(&str, &str)>>) -> Result<VipsImage> {
    let mut parameters = parameters.unwrap_or_else(|| vec![]);
    parameters.push(("revalidate", "true"));

    VipsImage::new_from_file(
        &(source_path.to_owned() + &generate_params(parameters)),
        Some(FromFileOptions {
            access: VipsAccess::Sequential,
            ..Default::default()
        }),
    )
    .map_err(|e| anyhow!(e))
}

fn generate_params(params: Vec<(&str, &str)>) -> String {
    if params.is_empty() {
        return String::new();
    }

    let param_strings = params
        .into_iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>();
    let result = param_strings.join(",");

    format!("[{result}]")
}

fn get_shrink_factor_float(request: &PipelineRequest, source_path: &str) -> Result<f64> {
    let image = default_load(source_path, None)?;

    let (width, height) = calculate_processing_size(request, &image);
    let (width, height) = (width as f64, height as f64);

    let original_width = image.get_width() as f64;
    let original_height = image.get_height() as f64;

    let shrink_factor = (original_width / width).min(original_height / height);

    Ok(shrink_factor)
}

fn get_shrink_factor_precise(request: &PipelineRequest, source_path: &str) -> Result<String> {
    Ok(get_shrink_factor_float(request, source_path)?.to_string())
}

fn get_shrink_factor(request: &PipelineRequest, source_path: &str) -> Result<Option<&'static str>> {
    get_shrink_factor_float(request, source_path).map(|factor| match factor {
        2.0..4.0 => Some("2"),
        4.0..8.0 => Some("4"),
        8.0.. => Some("8"),
        _ => None,
    })
}
