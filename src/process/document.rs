use anyhow::{Result, anyhow};
use axum::body::Body;
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use lopdf::Document;
use std::collections::BTreeSet;
use std::path::Path;

use crate::enums::download::Download;
use crate::enums::input::{InputFormat, VipsInputFormat};
use crate::enums::output_format::{OutputFormat, get_output_mime, get_output_extension};
use crate::process::download::apply_disposition;
use crate::process::outline;
use crate::process::pipeline;
use crate::process::pipeline::request::PipelineRequest;
use crate::process::raw;
use crate::services::http_cache::{self, Validators};

/// Serve PDF and SVG files
pub(super) async fn serve(headers: &HeaderMap, request: &PipelineRequest<'_>, validators: Option<&Validators>, cache_control: &str) -> Result<Response> {
    match request.output_format {
        OutputFormat::Svg => serve_svg(headers, request, cache_control).await,
        OutputFormat::Pdf => serve_pdf(headers, request, validators, cache_control).await,
        ref format => Err(anyhow!("{format:?} is not a document format")),
    }
}

async fn serve_svg(headers: &HeaderMap, request: &PipelineRequest<'_>, cache_control: &str) -> Result<Response> {
    if !matches!(request.source.format, InputFormat::Vips(VipsInputFormat::Svg)) {
        return Ok(unsupported(&OutputFormat::Svg, request));
    }

    let name = request
        .source
        .path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("image.svg");

    raw::serve(headers, &request.source.path, &request.parameters.download, name, cache_control).await
}

async fn serve_pdf(headers: &HeaderMap, request: &PipelineRequest<'_>, validators: Option<&Validators>, cache_control: &str) -> Result<Response> {
    let path = match request.source.format {
        InputFormat::Vips(VipsInputFormat::Pdf) | InputFormat::Office(_) => {
            pipeline::resolve_source_path(request).await?
        }
        InputFormat::Vips(VipsInputFormat::Svg) => pipeline::svg::process(request).await?,
        _ => return Ok(unsupported(&OutputFormat::Pdf, request)),
    };

    let path = Path::new(&path);
    let name = get_pdf_name(request);

    let Some(pages) = request.parameters.pages.as_deref() else {
        return raw::serve(headers, path, &request.parameters.download, &name, cache_control).await;
    };

    match tokio::task::block_in_place(|| get_page_subset(path, pages))? {
        Subset::Whole => raw::serve(headers, path, &request.parameters.download, &name, cache_control).await,
        Subset::Pages(pdf) => Ok(respond(pdf, &request.parameters.download, &name, validators, cache_control)),
        Subset::OutOfRange => Ok(out_of_range(pages)),
    }
}

enum Subset {
    Whole,
    Pages(Vec<u8>),
    OutOfRange,
}

fn get_page_subset(path: &Path, pages: &[u32]) -> Result<Subset> {
    let mut document = Document::load(path)?;
    let numbered = document.get_pages();
    let existing: BTreeSet<u32> = numbered.keys().copied().collect();

    let keep: BTreeSet<u32> = pages
        .iter()
        .copied()
        .filter(|page| existing.contains(page))
        .collect();

    if keep.is_empty() {
        return Ok(Subset::OutOfRange);
    }

    let discard: Vec<u32> = existing.difference(&keep).copied().collect();

    if discard.is_empty() {
        return Ok(Subset::Whole);
    }

    let removed = discard.iter().filter_map(|page| numbered.get(page).copied()).collect();
    outline::prune(&mut document, &removed);

    document.delete_pages(&discard);
    document.prune_objects();

    let mut buffer = Vec::new();
    document.save_to(&mut buffer)?;

    Ok(Subset::Pages(buffer))
}

fn get_pdf_name(request: &PipelineRequest<'_>) -> String {
    let stem = request
        .source
        .path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("document");

    format!("{stem}.pdf")
}

fn respond(pdf: Vec<u8>, download: &Download, name: &str, validators: Option<&Validators>, cache_control: &str) -> Response {
    let builder = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", get_output_mime(&OutputFormat::Pdf));
    
    let builder = match validators {
        Some(validators) => validators.apply(builder, cache_control, None),
        None => http_cache::apply(builder, None, None, cache_control, None),
    };

    apply_disposition(builder, download, name)
        .body(Body::from(pdf))
        .unwrap()
}

fn unsupported(format: &OutputFormat, request: &PipelineRequest<'_>) -> Response {
    text(
        StatusCode::UNSUPPORTED_MEDIA_TYPE,
        format!(
            "Cannot produce {:?} from {:?}",
            get_output_extension(format), request.source.format.to_string()
        ),
    )
}

fn out_of_range(pages: &[u32]) -> Response {
    text(
        StatusCode::BAD_REQUEST,
        format!("Requested pages {pages:?} are outside the document"),
    )
}

fn text(status: StatusCode, message: String) -> Response {
    Response::builder()
        .status(status)
        .header("Content-Type", "text/plain; charset=utf-8")
        .header(axum::http::header::CACHE_CONTROL, http_cache::NO_STORE)
        .body(Body::from(message))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{Object, ObjectId, Stream, dictionary};

    /// Build a minimal `pages`-page PDF on disk, one outline entry per page.
    fn write_pdf(pages: usize) -> tempfile::NamedTempFile {
        let mut document = Document::with_version("1.5");
        let pages_id = document.new_object_id();
        let outlines_id = document.new_object_id();

        let page_ids: Vec<ObjectId> = (0..pages)
            .map(|_| {
                let contents = document.add_object(Stream::new(dictionary! {}, Vec::new()));
                document.add_object(dictionary! {
                    "Type" => "Page",
                    "Parent" => pages_id,
                    "Contents" => contents,
                    "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
                })
            })
            .collect();

        document
            .objects
            .insert(pages_id, Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Count" => pages as i64,
                "Kids" => page_ids.iter().copied().map(Object::Reference).collect::<Vec<_>>(),
            }));

        let entry_ids: Vec<ObjectId> = page_ids
            .iter()
            .enumerate()
            .map(|(index, page)| {
                document.add_object(dictionary! {
                    "Title" => Object::string_literal(format!("page {}", index + 1)),
                    "Parent" => outlines_id,
                    "Dest" => vec![Object::Reference(*page), "Fit".into()],
                })
            })
            .collect();

        for (index, id) in entry_ids.iter().enumerate() {
            let entry = document.get_dictionary_mut(*id).unwrap();

            if index > 0 {
                entry.set("Prev", Object::Reference(entry_ids[index - 1]));
            }

            if let Some(next) = entry_ids.get(index + 1) {
                entry.set("Next", Object::Reference(*next));
            }
        }

        document
            .objects
            .insert(outlines_id, Object::Dictionary(dictionary! {
                "Type" => "Outlines",
                "First" => Object::Reference(entry_ids[0]),
                "Last" => Object::Reference(entry_ids[pages - 1]),
                "Count" => pages as i64,
            }));

        let catalog = document.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
            "Outlines" => Object::Reference(outlines_id),
        });
        document.trailer.set("Root", catalog);

        let file = tempfile::NamedTempFile::new().unwrap();
        document.save_to(&mut std::fs::File::create(file.path()).unwrap()).unwrap();
        file
    }

    /// Outline entry titles of a saved PDF, in reading order.
    fn outline_titles(pdf: &[u8]) -> Vec<String> {
        let document = Document::load_mem(pdf).unwrap();

        let mut current = document
            .catalog()
            .and_then(|catalog| catalog.get(b"Outlines"))
            .and_then(Object::as_reference)
            .and_then(|id| document.get_dictionary(id))
            .and_then(|outlines| outlines.get(b"First"))
            .and_then(Object::as_reference)
            .ok();

        let mut titles = Vec::new();

        while let Some(id) = current {
            let entry = document.get_dictionary(id).unwrap();
            let title = entry.get(b"Title").and_then(Object::as_str).unwrap();

            titles.push(String::from_utf8_lossy(title).into_owned());
            current = entry.get(b"Next").and_then(Object::as_reference).ok();
        }

        titles
    }

    fn page_count(pdf: &[u8]) -> usize {
        Document::load_mem(pdf).unwrap().get_pages().len()
    }

    #[test]
    fn keeps_only_the_requested_pages() {
        let file = write_pdf(3);

        let Subset::Pages(pdf) = get_page_subset(file.path(), &[1, 3]).unwrap() else {
            panic!("expected a rewritten subset");
        };

        assert_eq!(page_count(&pdf), 2);
        assert_eq!(outline_titles(&pdf), vec!["page 1", "page 3"]);
    }

    #[test]
    fn requesting_every_page_streams_the_original() {
        let file = write_pdf(3);
        assert!(matches!(
            get_page_subset(file.path(), &[1, 2, 3]).unwrap(),
            Subset::Whole
        ));
    }

    #[test]
    fn ignores_pages_past_the_end_of_the_document() {
        let file = write_pdf(2);

        let Subset::Pages(pdf) = get_page_subset(file.path(), &[2, 9]).unwrap() else {
            panic!("expected a rewritten subset");
        };

        assert_eq!(page_count(&pdf), 1);
    }

    #[test]
    fn reports_a_selection_with_no_existing_pages() {
        let file = write_pdf(2);
        assert!(matches!(
            get_page_subset(file.path(), &[7, 8]).unwrap(),
            Subset::OutOfRange
        ));
    }


}
