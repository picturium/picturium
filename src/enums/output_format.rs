use serde::Deserialize;
use strum::EnumString;

#[derive(Debug, Clone, Default, PartialEq, Deserialize, EnumString)]
#[serde(rename_all = "lowercase")]
#[strum(ascii_case_insensitive)]
pub enum OutputFormat {
    #[default]
    Auto,
    #[serde(alias = "jpg")]
    Jpeg,
    Png,
    Gif,
    Webp,
    Avif,
    Jxl,
    Pdf,
    Svg,
}

pub fn get_output_mime(format: &OutputFormat) -> &'static str {
    match format {
        OutputFormat::Jpeg => "image/jpeg",
        OutputFormat::Png => "image/png",
        OutputFormat::Gif => "image/gif",
        OutputFormat::Webp => "image/webp",
        OutputFormat::Avif => "image/avif",
        OutputFormat::Jxl => "image/jxl",
        OutputFormat::Pdf => "application/pdf",
        OutputFormat::Svg => "image/svg+xml",
        OutputFormat::Auto => unreachable!(),
    }
}

/// Returns the canonical file extension for an output format.
///
/// `Auto` resolves to `unreachable!()` exactly like [`get_output_mime`], because the
/// pipeline always resolves `Auto` to a concrete format before reaching the response
/// builder. Used to derive a `Content-Disposition` filename for processed outputs.
pub fn get_output_extension(format: &OutputFormat) -> &'static str {
    match format {
        OutputFormat::Jpeg => "jpg",
        OutputFormat::Png => "png",
        OutputFormat::Gif => "gif",
        OutputFormat::Webp => "webp",
        OutputFormat::Avif => "avif",
        OutputFormat::Jxl => "jxl",
        OutputFormat::Pdf => "pdf",
        OutputFormat::Svg => "svg",
        OutputFormat::Auto => unreachable!(),
    }
}