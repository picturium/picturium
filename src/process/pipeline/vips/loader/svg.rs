use crate::enums::dpi::Dpi;
use crate::process::pipeline::request::PipelineRequest;
use crate::services::size::calculate_load_size;
use anyhow::{Result, anyhow};
use picturium_libvips::{FromSvgOptions, VipsAccess, VipsFailOn, VipsImage};

pub fn load(request: &PipelineRequest, source_path: &str) -> Result<VipsImage> {
    VipsImage::new_from_svg(source_path, Some(get_svg_options(request, source_path)?))
        .map_err(|e| anyhow!(e))
}

fn get_svg_options(request: &PipelineRequest, source_path: &str) -> Result<FromSvgOptions> {
    let (dpi, scale) = resolve_sizing(request, source_path)?;

    Ok(FromSvgOptions {
        dpi,
        scale,
        unlimited: request.state.config.svg.unlimited,
        stylesheet: request
            .parameters
            .style
            .as_ref()
            .unwrap_or(&"".to_string())
            .to_string(),
        high_bitdepth: false,
        memory: false,
        access: VipsAccess::Sequential,
        fail_on: VipsFailOn::Error,
        revalidate: true,
    })
}

fn resolve_sizing(request: &PipelineRequest, source_path: &str) -> Result<(f64, f64)> {
    let dpi = match request.parameters.dpi {
        Dpi::Auto => request.state.config.svg.load_dpi as f64,
        Dpi::Value(value) => value as f64,
    };

    Ok((dpi, resolve_scale(request, dpi, source_path)?))
}

fn resolve_scale(request: &PipelineRequest, dpi: f64, source_path: &str) -> Result<f64> {
    let image = VipsImage::new_from_svg(
        source_path,
        Some(FromSvgOptions {
            dpi,
            revalidate: true,
            ..Default::default()
        }),
    )
    .map_err(|e| anyhow!(e))?;

    let (process_width, process_height) = calculate_load_size(request, &image);

    let (width, height) = (image.get_width() as u16, image.get_height() as u16);

    let scaling = vec![
        process_width as f64 / width as f64,
        process_height as f64 / height as f64,
    ];

    Ok(scaling.into_iter().reduce(f64::max).unwrap_or(1.0))
}
