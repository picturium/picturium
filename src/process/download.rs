use axum::http::response::Builder as ResponseBuilder;

use crate::enums::download::Download;

pub(super) fn apply_disposition(
    builder: ResponseBuilder,
    download: &Download,
    fallback_name: &str,
) -> ResponseBuilder {
    match download {
        Download::No => builder,
        Download::Auto => attachment(builder, fallback_name),
        Download::Filename(name) => attachment(builder, name),
    }
}

fn attachment(builder: ResponseBuilder, name: &str) -> ResponseBuilder {
    builder.header(
        "Content-Disposition",
        format!("attachment; filename=\"{}\"", ascii_filename(name)),
    )
}

fn ascii_filename(name: &str) -> String {
    name.chars()
        .map(|character| {
            if character.is_ascii() && !matches!(character, '"' | '\\') {
                character
            } else {
                '_'
            }
        })
        .collect()
}
