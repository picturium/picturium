mod jpeg;
mod pdf;
mod svg;
mod tiff;
mod webp;

use crate::enums::dpi::Dpi;
use crate::enums::input::{InputFormat, VipsInputFormat};
use crate::process::pipeline::request::PipelineRequest;
use crate::process::source::Source;
use crate::services::size::calculate_load_size;
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
    super::update_source_dimensions(request, &image);

    Ok(image)
}

fn load_vips_file(
    request: &mut PipelineRequest,
    source_path: &str,
    format: VipsInputFormat,
) -> Result<VipsImage> {
    match format {
        VipsInputFormat::Svg => svg::load(request, source_path),
        VipsInputFormat::Jpeg => jpeg::load(request, source_path),
        VipsInputFormat::Tiff => tiff::load(request, source_path),
        VipsInputFormat::Webp => webp::load(request, source_path),
        VipsInputFormat::Pdf => pdf::load(request, source_path),
        VipsInputFormat::Gif | VipsInputFormat::Heif => default_load(source_path, Some(animation_params(request, source_path)?)),
        _ => default_load(source_path, None),
    }
}

pub(super) fn animation_params(request: &PipelineRequest, source_path: &str) -> Result<Vec<(&'static str, String)>> {
    let pages = source_page_count(source_path)?;
    let page = start_page(request).clamp(0, pages - 1);

    let frames = frame_budget(
        request.parameters.animate.requested_frames(),
        request.state.config.output.max_animation_frames,
        pages - page,
    );

    let mut params = vec![("n", frames.to_string())];

    if page > 0 {
        params.push(("page", page.to_string()));
    }

    Ok(params)
}

fn start_page(request: &PipelineRequest) -> i32 {
    if matches!(request.source.format, InputFormat::Video(_)) {
        return 0;
    }

    request
        .parameters
        .pages
        .as_ref()
        .and_then(|pages| pages.first())
        .map_or(0, |page| page.saturating_sub(1) as i32)
}

fn source_page_count(source_path: &str) -> Result<i32> {
    Ok(default_load(source_path, Some(vec![("n", "1".into())]))?
        .get_page_count()
        .max(1))
}

fn frame_budget(requested: i32, cap: i32, available: i32) -> i32 {
    let available = available.max(1);

    let wanted = match (requested, cap) {
        (_, cap) if cap < 1 => requested,
        (requested, cap) if requested < 1 => cap,
        (requested, cap) => requested.min(cap),
    };

    match wanted < 1 {
        true => available,
        false => wanted.min(available),
    }
}

fn default_load(source_path: &str, parameters: Option<Vec<(&str, String)>>) -> Result<VipsImage> {
    let mut parameters = parameters.unwrap_or_else(|| vec![]);
    
    let access = match parameters.iter().any(|(key, value)| *key == "n" && value != "1") {
        true => VipsAccess::Random,
        false => VipsAccess::Sequential,
    };

    parameters.push(("revalidate", "true".into()));

    VipsImage::new_from_file(
        &(source_path.to_owned() + &generate_params(parameters)),
        Some(FromFileOptions {
            access,
            ..Default::default()
        }),
    )
    .map_err(|e| anyhow!(e))
}

fn generate_params(params: Vec<(&str, String)>) -> String {
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

#[cfg(test)]
mod tests {
    use super::frame_budget;

    #[test]
    fn a_frame_cap_bounds_a_request_for_every_frame() {
        assert_eq!(frame_budget(-1, 300, 1000), 300);
        assert_eq!(frame_budget(500, 300, 1000), 300);
        assert_eq!(frame_budget(12, 300, 1000), 12);
        assert_eq!(frame_budget(1, 300, 1000), 1);
    }

    #[test]
    fn a_cap_of_zero_only_leaves_the_file_as_the_bound() {
        assert_eq!(frame_budget(-1, 0, 1000), 1000);
        assert_eq!(frame_budget(500, 0, 1000), 500);
    }

    #[test]
    fn nothing_ever_reaches_past_the_end_of_the_file() {
        // libvips errors on a page window past the end rather than truncating.
        assert_eq!(frame_budget(-1, 300, 131), 131);
        assert_eq!(frame_budget(500, 300, 131), 131);
        assert_eq!(frame_budget(12, 300, 4), 4);
        assert_eq!(frame_budget(-1, 0, 4), 4);
    }
}

fn get_shrink_factor_float(request: &PipelineRequest, source_path: &str) -> Result<f64> {
    let image = default_load(source_path, None)?;

    let (width, height) = calculate_load_size(request, &image);
    let (width, height) = (width as f64, height as f64);

    let original_width = image.get_width() as f64;
    let original_height = image.get_height() as f64;

    let shrink_factor = (original_width / width).min(original_height / height).max(1.0);

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
