use std::str::FromStr;
use axum::http::HeaderMap;
use crate::enums::input::{InputFormat, VipsInputFormat};
use crate::enums::output_format::OutputFormat;
use crate::params::parsed::Parameters;
use crate::state::AppState;
use crate::process::source::Source;

const DEFAULT_OUTPUT_FORMAT: OutputFormat = OutputFormat::Webp;

pub fn resolve_output_format(headers: &HeaderMap, state: &AppState, parameters: &Parameters) -> OutputFormat {
    if parameters.format != OutputFormat::Auto {
        return parameters.format.clone();
    }

    let accept = headers.get("Accept").and_then(|v| v.to_str().ok());

    match accept {
        Some(accept) => state.config.output.format_priority.iter().find_map(|extension| {
            let mime = format!("image/{}", extension);

            if !accept.contains(&mime) {
                return None;
            }

            if let Ok(output_format) = OutputFormat::from_str(&extension) {
                return Some(output_format);
            }
            
            None
        }).unwrap_or(DEFAULT_OUTPUT_FORMAT),
        None => DEFAULT_OUTPUT_FORMAT,
    }
}

pub fn resolve_intermediate_format(source: &Source) -> Option<VipsInputFormat> {
    match source.format {
        InputFormat::Video(_) => Some(VipsInputFormat::Png),
        InputFormat::Office(_) | InputFormat::Vector(_) => Some(VipsInputFormat::Pdf),
        _ => None,
    }
}

pub fn resolve_input_format(source: &Source, intermediate_format: Option<VipsInputFormat>) -> VipsInputFormat {
    match source.format {
        InputFormat::Vips(format) => format,
        _ => intermediate_format.unwrap_or(VipsInputFormat::Jpeg),
    }
}