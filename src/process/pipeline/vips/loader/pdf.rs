use crate::enums::dpi::Dpi;
use crate::params::background::Background;
use crate::process::pipeline::RenderLimitExceeded;
use crate::process::pipeline::request::PipelineRequest;
use crate::process::pipeline::vips::background::resolve_background;
use crate::process::pipeline::vips::pages;
use crate::services::size::calculate_load_size;
use anyhow::{Result, anyhow};
use lopdf::{Dictionary, Document, LoadOptions, Object, ObjectId};
use picturium_libvips::{
    EmbedOptions, FromPdfOptions, VipsAccess, VipsAnimations, VipsCrop, VipsExtend, VipsImage,
    VipsOperations, arrayjoin,
};
use std::collections::HashSet;

pub fn load(request: &mut PipelineRequest, source_path: &str) -> Result<VipsImage> {
    let (dpi, scale, page_pixels) = resolve_sizing(request, source_path)?;

    // The page is rendered straight to the target size, so the source dimensions
    // are recovered from the render scale instead of a shrink-on-load factor.
    request.source.shrink = 1.0 / scale;

    let pages = request.parameters.pages.as_deref().unwrap_or(&[1]);
    let runs = page_runs(pages);
    let random_access = runs.iter().map(|(_, count)| count).sum::<i32>() > 1;

    let max_pixels = request.state.config.pdf.max_image_pixels as f64 * 1_000_000.0;

    if page_pixels > max_pixels {
        return Err(RenderLimitExceeded(format!(
            "PDF page render of {:.1} Mpx exceeds pdf.max_image_pixels ({} Mpx)",
            page_pixels / 1_000_000.0,
            request.state.config.pdf.max_image_pixels
        )).into());
    }

    let image_pixels = largest_embedded_image(source_path, pages)?;

    if image_pixels > max_pixels {
        return Err(RenderLimitExceeded(format!(
            "PDF embedded image of {:.1} Mpx exceeds pdf.max_image_pixels ({} Mpx)",
            image_pixels / 1_000_000.0,
            request.state.config.pdf.max_image_pixels
        )).into());
    }

    let background = pdf_background(
        request.parameters.background,
        &request.state.config.pdf.background,
    );

    let mut loaded = Vec::with_capacity(runs.len());

    for (page, page_count) in runs {
        loaded.push(
            VipsImage::new_from_pdf(
                source_path,
                Some(FromPdfOptions {
                    page,
                    page_count,
                    dpi,
                    scale,
                    background: background.to_vec(),
                    access: match random_access {
                        true => VipsAccess::Random,
                        false => VipsAccess::Sequential,
                    },
                    revalidate: true,
                    ..Default::default()
                }),
            )
            .map_err(|e| anyhow!(e))?,
        );
    }

    match loaded.len() {
        1 => Ok(loaded.pop().unwrap()),
        _ => join_runs(loaded, &background),
    }
}

fn largest_embedded_image(source_path: &str, pages: &[u32]) -> Result<f64> {
    let document = Document::load_with_options(source_path, LoadOptions::with_filter(strip_stream_content))?;
    let page_ids = document.get_pages();
    let mut seen = HashSet::new();
    let mut pending: Vec<&Dictionary> = Vec::new();
    let mut largest = 0.0;

    for page in pages {
        let Some(&page_id) = page_ids.get(&(*page).max(1)) else { continue };
        let (inline, referenced) = document.get_page_resources(page_id)?;
        pending.extend(inline);
        pending.extend(referenced.into_iter().filter_map(|id| document.get_dictionary(id).ok()));
    }

    while let Some(resources) = pending.pop() {
        let Ok(xobjects) = resources.get_deref(b"XObject", &document).and_then(Object::as_dict) else { continue };

        for (_, xobject) in xobjects.iter() {
            let Ok(id) = xobject.as_reference() else { continue };

            if !seen.insert(id) {
                continue;
            }

            let Ok(dict) = document.get_dictionary(id) else { continue };
            let dimension = |key: &[u8]| dict.get_deref(key, &document).and_then(Object::as_i64).unwrap_or(0).max(0) as f64;

            match dict.get(b"Subtype").and_then(Object::as_name) {
                Ok(b"Image") => largest = f64::max(largest, dimension(b"Width") * dimension(b"Height")),
                Ok(b"Form") => pending.extend(dict.get_deref(b"Resources", &document).and_then(Object::as_dict)),
                _ => {}
            }
        }
    }

    Ok(largest)
}

/// Keeps only stream dictionaries, so the parsed document doesn't hold a second copy of the file.
fn strip_stream_content(id: ObjectId, object: &mut Object) -> Option<(ObjectId, Object)> {
    // Object streams are unpacked after this filter runs, so their content must stay.
    if let Object::Stream(stream) = object && !stream.dict.has_type(b"ObjStm") {
        *object = Object::Dictionary(std::mem::take(&mut stream.dict));
    }

    Some((id, object.clone()))
}

/// Groups the requested 1-based pages into contiguous `(first 0-based page, count)` runs.
fn page_runs(pages: &[u32]) -> Vec<(i32, i32)> {
    let mut pages = pages.to_vec();
    pages.sort_unstable();
    pages.dedup();

    let mut runs: Vec<(i32, i32)> = Vec::new();

    for page in pages {
        let page = page.max(1) as i32 - 1;

        match runs.last_mut() {
            Some((first, count)) if *first + *count == page => *count += 1,
            _ => runs.push((page, 1)),
        }
    }

    if runs.is_empty() {
        runs.push((0, 1));
    }

    runs
}

fn join_runs(runs: Vec<VipsImage>, background: &[f64]) -> Result<VipsImage> {
    let mut frames = Vec::new();

    for run in runs {
        let (width, height) = (run.get_width(), pages::page_height(&run));

        for index in 0..pages::page_count(&run) {
            frames.push(run.clone().extract_area(0, index * height, width, height)?);
        }
    }

    // Separate loads can differ in page size; pad them like pdfload pads a single run.
    let width = frames.iter().map(VipsImage::get_width).max().unwrap_or(1);
    let height = frames.iter().map(VipsImage::get_height).max().unwrap_or(1);

    let frames = frames
        .into_iter()
        .map(|frame| match (frame.get_width(), frame.get_height()) == (width, height) {
            true => Ok(frame),
            false => frame.embed(0, 0, width, height, Some(EmbedOptions {
                extend: VipsExtend::Background,
                background,
            })),
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(arrayjoin(frames, 1)?.set_page_height(height)?)
}

fn pdf_background(requested: Option<Background>, configured: &str) -> [f64; 4] {
    let configured = configured
        .parse()
        .expect("pdf.background must be validated before processing requests");

    resolve_background(requested.or(Some(configured)))
}

/// Returns `(dpi, scale, rendered pixels of the first page)`.
fn resolve_sizing(request: &PipelineRequest, source_path: &str) -> Result<(f64, f64, f64)> {
    let dpi = match request.parameters.dpi {
        Dpi::Auto => request.state.config.pdf.load_dpi as f64,
        Dpi::Value(value) => value as f64,
    };

    let (scale, page_pixels) = resolve_scale(request, dpi, source_path)?;
    Ok((dpi, scale, page_pixels))
}

fn resolve_scale(request: &PipelineRequest, dpi: f64, source_path: &str) -> Result<(f64, f64)> {
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

    let (width, height) = (image.get_width() as f64, image.get_height() as f64);

    let scaling = vec![
        process_width as f64 / width,
        process_height as f64 / height,
    ];

    let scale = scaling.into_iter().reduce(f64::max).unwrap_or(1.0);
    let page_pixels = width * height * scale * scale;

    Ok((scale, page_pixels))
}

#[cfg(test)]
mod tests {
    use super::{page_runs, pdf_background};

    #[test]
    fn requested_pages_load_as_contiguous_runs() {
        assert_eq!(page_runs(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]), vec![(0, 10)]);
        assert_eq!(page_runs(&[1, 900]), vec![(0, 1), (899, 1)]);
        assert_eq!(page_runs(&[40, 41, 42, 1, 2, 3, 4, 5, 3]), vec![(0, 5), (39, 3)]);
        assert_eq!(page_runs(&[]), vec![(0, 1)]);
    }

    #[test]
    fn request_background_overrides_the_configured_default() {
        assert_eq!(pdf_background(None, "white"), [255.0; 4]);
        assert_eq!(
            pdf_background(Some("transparent".parse().unwrap()), "white"),
            [0.0; 4]
        );
    }
}
