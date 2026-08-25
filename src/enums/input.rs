use std::path::Path;
use std::fmt;
use strum::{Display, EnumString};

#[derive(Debug, Clone, Copy, Default)]
pub enum InputFormat {
    Vips(VipsInputFormat),
    Office(OfficeInputFormat),
    Video(VideoInputFormat),
    #[default]
    Unsupported,
}

impl fmt::Display for InputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Vips(format) => format.fmt(f),
            Self::Office(format) => format.fmt(f),
            Self::Video(format) => format.fmt(f),
            Self::Unsupported => f.write_str("unsupported"),
        }
    }
}

#[derive(Debug, Clone, Copy, EnumString, Display, PartialEq)]
#[strum(serialize_all = "lowercase")]
pub enum VipsInputFormat {
    #[strum(serialize = "jpeg")]
    Jpeg,
    Jp2k,
    Png,
    Tiff,
    Ico,
    Gif,
    Webp,
    Heif,
    Jxl,
    Pdf,
    Svg,
    // Ai,
    Eps,
    // Cdr,
    Psd,
    Bmp,
    Raw,
}

#[derive(Debug, Clone, Copy, Display)]
#[strum(serialize_all = "lowercase")]
pub enum OfficeInputFormat {
    Doc,
    Ppt,
    Xls,
}

#[derive(Debug, Clone, Copy, Display)]
#[strum(serialize_all = "lowercase")]
pub enum VideoInputFormat {
    Mp4,
    Webm,
    Mkv,
    Avi,
    Av1,
    #[strum(serialize = "3gp")]
    _3gp,
    M4v,
    Flv,
    Mov,
    Mpeg,
    Mts,
    Hevc,
}

pub fn is_gzipped_svg(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            let name = name.to_lowercase();
            name.ends_with(".svgz") || name.ends_with(".svg.gz")
        })
}

/// Resolves the MIME type of a source file from its extension.
///
/// This mirrors the extension table in [`crate::process::source::Source::get_format`]
/// (including the `.svg.gz` special case) so the MIME always matches the format
/// detection. Used when serving a file untouched, where we never construct an
/// `OutputFormat` and `get_output_mime` does not apply.
pub fn get_input_mime(path: &Path) -> &'static str {
    if is_gzipped_svg(path) {
        return "image/svg+xml";
    }

    let extension = match path.extension().and_then(|ext| ext.to_str()) {
        Some(ext) => ext.to_lowercase(),
        None => return "application/octet-stream",
    };

    match extension.as_str() {
        // Raster + vector image formats
        "jpg" | "jpeg" | "jpe" | "jif" | "jfif" | "jfi" => "image/jpeg",
        "jp2" | "j2k" | "j2c" | "jpc" | "jpt" => "image/jp2",
        "png" => "image/png",
        "tiff" | "tif" => "image/tiff",
        "ico" => "image/x-icon",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "heif" | "heic" => "image/heif",
        "avif" => "image/avif",
        "jxl" => "image/jxl",
        "pdf" => "application/pdf",
        "svg" | "svgz" => "image/svg+xml",
        "eps" => "application/postscript",
        "psd" => "image/vnd.adobe.photoshop",
        "bmp" => "image/bmp",
        "raw" | "rw2" | "raf" | "pef" | "orf" | "nrw" | "nef" | "dng" | "cr2" | "cr3" | "crw" | "arw" => "image/raw",
        // Office documents
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "docm" => "application/vnd.ms-word.document.macroEnabled.12",
        "dotx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.template",
        "dotm" => "application/vnd.ms-word.template.macroEnabled.12",
        "odt" => "application/vnd.oasis.opendocument.text",
        "ppt" => "application/vnd.ms-powerpoint",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "pptm" => "application/vnd.ms-powerpoint.presentation.macroEnabled.12",
        "potx" => "application/vnd.openxmlformats-officedocument.presentationml.template",
        "potm" => "application/vnd.ms-powerpoint.template.macroEnabled.12",
        "odp" => "application/vnd.oasis.opendocument.presentation",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "xlsm" => "application/vnd.ms-excel.sheet.macroEnabled.12",
        "xltx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.template",
        "xltm" => "application/vnd.ms-excel.template.macroEnabled.12",
        "ods" => "application/vnd.oasis.opendocument.spreadsheet",
        "csv" => "text/csv",
        // Video
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mkv" => "video/x-matroska",
        "avi" => "video/x-msvideo",
        "av1" => "video/av1",
        "3gp" => "video/3gpp",
        "m4v" => "video/x-m4v",
        "flv" => "video/x-flv",
        "mov" => "video/quicktime",
        "mpeg" => "video/mpeg",
        "mts" => "video/mp2t",
        "hevc" => "video/mp4",
        // Formats we do not process but may serve
        "txt" | "md" => "text/plain; charset=utf-8",
        "json" => "application/json",
        "xml" => "application/xml",
        "zip" => "application/zip",
        "gz" => "application/gzip",
        "mp3" => "audio/mpeg",
        "m4a" => "audio/mp4",
        "wav" => "audio/wav",
        "ogg" | "oga" => "audio/ogg",
        "flac" => "audio/flac",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_both_spellings_of_gzipped_svg() {
        assert!(is_gzipped_svg(Path::new("logo.svgz")));
        assert!(is_gzipped_svg(Path::new("/data/logo.svg.gz")));
        assert!(is_gzipped_svg(Path::new("LOGO.SVG.GZ")));
        assert!(!is_gzipped_svg(Path::new("logo.svg")));
        assert!(!is_gzipped_svg(Path::new("archive.gz")));
    }

    #[test]
    fn gzipped_svg_keeps_the_svg_media_type() {
        assert_eq!(get_input_mime(Path::new("logo.svg.gz")), "image/svg+xml");
        assert_eq!(get_input_mime(Path::new("logo.svgz")), "image/svg+xml");
        assert_eq!(get_input_mime(Path::new("archive.gz")), "application/gzip");
    }
}
