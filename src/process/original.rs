use axum::{
    body::Body,
    http::{HeaderMap, StatusCode, header},
    response::Response,
};
use tokio::io::{AsyncReadExt, AsyncSeekExt, SeekFrom};

use crate::{
    enums::input::get_input_mime,
    params::parsed::Parameters,
    process::{download::apply_disposition, source::Source},
};

pub(super) async fn serve(
    headers: &HeaderMap,
    source: &Source,
    parameters: &Parameters,
) -> anyhow::Result<Response> {
    let metadata = tokio::fs::metadata(&source.path).await?;
    let size = metadata.len();
    let modified = metadata.modified().ok();
    let mime = get_input_mime(&source.path);
    let etag = modified.map(|mtime| {
        let seconds = mtime
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        format!("\"{seconds}-{size}\"")
    });
    let last_modified = modified.map(httpdate::fmt_http_date);

    if is_not_modified(headers, etag.as_deref(), modified) {
        return Ok(not_modified_response(
            etag.as_deref(),
            last_modified.as_deref(),
            mime,
        ));
    }

    let range = match resolve_range(headers, size, etag.as_deref(), modified) {
        Ok(range) => range,
        Err(RangeError::Unsatisfiable) => {
            return Ok(range_not_satisfiable_response(
                size,
                etag.as_deref(),
                last_modified.as_deref(),
                mime,
            ));
        }
    };

    let mut file = tokio::fs::File::open(&source.path).await?;
    let (status, content_length, content_range) = match range {
        Some((start, end)) => {
            file.seek(SeekFrom::Start(start)).await?;
            let length = end - start + 1;
            (
                StatusCode::PARTIAL_CONTENT,
                length,
                Some(format!("bytes {start}-{end}/{size}")),
            )
        }
        None => (StatusCode::OK, size, None),
    };

    let body = Body::from_stream(tokio_util::io::ReaderStream::new(file.take(content_length)));
    let mut builder = Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, mime)
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CONTENT_LENGTH, content_length.to_string());
    if let Some(content_range) = content_range {
        builder = builder.header(header::CONTENT_RANGE, content_range);
    }
    if let Some(etag) = &etag {
        builder = builder.header(header::ETAG, etag);
    }
    if let Some(last_modified) = last_modified.as_deref() {
        builder = builder.header(header::LAST_MODIFIED, last_modified);
    }

    let builder = match source.path.file_name().and_then(|name| name.to_str()) {
        Some(name) => apply_disposition(builder, &parameters.download, name),
        None => builder,
    };

    Ok(builder.body(body)?)
}

fn is_not_modified(
    headers: &HeaderMap,
    etag: Option<&str>,
    modified: Option<std::time::SystemTime>,
) -> bool {
    if let Some(if_none_match) = headers
        .get(header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
    {
        return etag.is_some_and(|etag| {
            if_none_match.trim() == "*"
                || if_none_match.split(',').any(|value| value.trim() == etag)
        });
    }

    let Some(modified) = modified else {
        return false;
    };
    let Some(if_modified_since) = headers
        .get(header::IF_MODIFIED_SINCE)
        .and_then(|value| value.to_str().ok())
    else {
        return false;
    };

    httpdate::parse_http_date(if_modified_since).is_ok_and(|since| modified <= since)
}

fn resolve_range(
    headers: &HeaderMap,
    size: u64,
    etag: Option<&str>,
    modified: Option<std::time::SystemTime>,
) -> Result<Option<(u64, u64)>, RangeError> {
    let Some(range_header) = headers
        .get(header::RANGE)
        .and_then(|value| value.to_str().ok())
    else {
        return Ok(None);
    };

    if headers
        .get(header::IF_RANGE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|if_range| !if_range_validates(if_range, etag, modified))
    {
        return Ok(None);
    }

    let range = http_range_header::parse_range_header(range_header)
        .and_then(|range| range.validate(size))
        .map_err(|_| RangeError::Unsatisfiable)?;

    match range.as_slice() {
        [range] => Ok(Some((*range.start(), *range.end()))),
        [] | [_, ..] => Err(RangeError::Unsatisfiable),
    }
}

fn if_range_validates(
    value: &str,
    etag: Option<&str>,
    modified: Option<std::time::SystemTime>,
) -> bool {
    if let Ok(date) = httpdate::parse_http_date(value) {
        return modified.is_some_and(|modified| modified <= date);
    }

    etag.is_some_and(|etag| etag == value)
}

fn not_modified_response(etag: Option<&str>, last_modified: Option<&str>, mime: &str) -> Response {
    let mut builder = Response::builder()
        .status(StatusCode::NOT_MODIFIED)
        .header(header::CONTENT_TYPE, mime)
        .header(header::ACCEPT_RANGES, "bytes");
    if let Some(etag) = etag {
        builder = builder.header(header::ETAG, etag);
    }
    if let Some(last_modified) = last_modified {
        builder = builder.header(header::LAST_MODIFIED, last_modified);
    }
    builder.body(Body::empty()).unwrap()
}

fn range_not_satisfiable_response(
    size: u64,
    etag: Option<&str>,
    last_modified: Option<&str>,
    mime: &str,
) -> Response {
    let mut builder = Response::builder()
        .status(StatusCode::RANGE_NOT_SATISFIABLE)
        .header(header::CONTENT_TYPE, mime)
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CONTENT_RANGE, format!("bytes */{size}"));
    if let Some(etag) = etag {
        builder = builder.header(header::ETAG, etag);
    }
    if let Some(last_modified) = last_modified {
        builder = builder.header(header::LAST_MODIFIED, last_modified);
    }
    builder.body(Body::empty()).unwrap()
}

enum RangeError {
    Unsatisfiable,
}
