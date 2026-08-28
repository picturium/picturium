use crate::enums::dpi::Dpi;
use crate::process::pipeline::request::PipelineRequest;
use crate::process::pipeline::vips::background::resolve_background;
use crate::services::size::calculate_load_size;
use anyhow::{Result, anyhow};
use picturium_libvips::{FromPdfOptions, VipsAccess, VipsImage};

pub fn load(request: &PipelineRequest, source_path: &str) -> Result<VipsImage> {
    VipsImage::new_from_pdf(source_path, Some(get_pdf_options(request, source_path)?))
        .map_err(|e| anyhow!(e))
}

fn get_pdf_options(request: &PipelineRequest, source_path: &str) -> Result<FromPdfOptions> {
    let (dpi, scale) = resolve_sizing(request, source_path)?;
    let empty_vec = vec![];

    let min_page = request
        .parameters
        .pages
        .as_ref()
        .unwrap_or(&empty_vec)
        .iter()
        .min()
        .unwrap_or(&1);

    let max_page = request
        .parameters
        .pages
        .as_ref()
        .unwrap_or(&empty_vec)
        .iter()
        .max()
        .unwrap_or(&1);

    let page_count = max_page - min_page + 1;

    Ok(FromPdfOptions {
        page: (*min_page as i32) - 1,
        page_count: page_count as i32,
        dpi,
        scale,
        background: resolve_background(request.parameters.background).to_vec(),
        access: match page_count > 1 {
            true => VipsAccess::Random,
            false => VipsAccess::Sequential,
        },
        revalidate: true,
        ..Default::default()
    })
}

fn resolve_sizing(request: &PipelineRequest, source_path: &str) -> Result<(f64, f64)> {
    let dpi = match request.parameters.dpi {
        Dpi::Auto => request.state.config.pdf.load_dpi as f64,
        Dpi::Value(value) => value as f64,
    };

    Ok((dpi, resolve_scale(request, dpi, source_path)?))
}

fn resolve_scale(request: &PipelineRequest, dpi: f64, source_path: &str) -> Result<f64> {
    let image = VipsImage::new_from_pdf(
        source_path,
        Some(FromPdfOptions {
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
