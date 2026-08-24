use crate::config::SharedConfig;
use crate::enums::input::{InputFormat, OfficeInputFormat, VideoInputFormat, VipsInputFormat};
use crate::params::RequestParams;
use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub struct Source {
    pub format: InputFormat,
    pub path: PathBuf,
    pub width: Option<u16>,
    pub height: Option<u16>,
    pub shrink: f64,
}

impl Source {
    pub fn new(config: &SharedConfig, path: &str, params: &RequestParams) -> Result<Self> {
        let path = format!("{}/{path}", config.data.dir);
        let source_path = Self::get_path(&path, &config.data.dir);

        let source = match source_path {
            Some(path) => Self {
                format: Self::get_format(&path),
                path,
                width: None,
                height: None,
                shrink: 1.0,
            },
            None => {
                tracing::info!("File not found: {}", path);

                match params.fallback {
                    Some(ref fallback) if fallback != &path => {
                        tracing::debug!("Trying fallback: {}", fallback);

                        Self::new(config, &fallback, params)
                    }
                    _ => Err(anyhow!("File not found: {}", path)),
                }?
            }
        };

        Ok(source)
    }

    fn get_path(path: &str, data_dir: &str) -> Option<PathBuf> {
        let canonical_data_dir = Path::new(data_dir).canonicalize().ok()?;
        let canonical_path = Path::new(path).canonicalize().ok()?;

        if !canonical_path.starts_with(&canonical_data_dir) {
            tracing::warn!(
                "Path traversal attempt blocked: \"{}\" is outside of data directory \"{}\"",
                canonical_path.display(),
                canonical_data_dir.display()
            );

            return None;
        }

        if !canonical_path.is_file() {
            tracing::warn!("File not found: {path}");
            return None;
        }

        Some(path.into())
    }

    pub(crate) fn get_format(path: &PathBuf) -> InputFormat {
        let extension = match path.extension() {
            Some(ext) => match ext.to_str() {
                Some(ext) => ext.to_lowercase(),
                None => return InputFormat::Unsupported,
            },
            None => return InputFormat::Unsupported,
        };

        let extension = match path.ends_with(".svg.gz") {
            true => "svg".into(),
            false => extension,
        };

        let format = match extension.to_lowercase().as_str() {
            "jpg" | "jpeg" | "jpe" | "jif" | "jfif" | "jfi" => {
                InputFormat::Vips(VipsInputFormat::Jpeg)
            }
            "jp2" | "j2k" | "j2c" | "jpc" | "jpt" => InputFormat::Vips(VipsInputFormat::Jp2k),
            "png" => InputFormat::Vips(VipsInputFormat::Png),
            "tiff" | "tif" => InputFormat::Vips(VipsInputFormat::Tiff),
            "ico" => InputFormat::Vips(VipsInputFormat::Ico),
            "gif" => InputFormat::Vips(VipsInputFormat::Gif),
            "webp" => InputFormat::Vips(VipsInputFormat::Webp),
            "heif" | "heic" | "avif" => InputFormat::Vips(VipsInputFormat::Heif),
            "jxl" => InputFormat::Vips(VipsInputFormat::Jxl),
            "pdf" => InputFormat::Vips(VipsInputFormat::Pdf),
            "svg" | "svgz" => InputFormat::Vips(VipsInputFormat::Svg),
            // "ai" => InputFormat::Vips(VipsInputFormat::Ai),
            "eps" => InputFormat::Vips(VipsInputFormat::Eps),
            // "cdr" => InputFormat::Vips(VipsInputFormat::Cdr),
            "psd" => InputFormat::Vips(VipsInputFormat::Psd),
            "bmp" => InputFormat::Vips(VipsInputFormat::Bmp),
            "raw" | "rw2" | "raf" | "pef" | "orf" | "nrw" | "nef" | "dng" | "cr2" | "cr3"
            | "crw" | "arw" => InputFormat::Vips(VipsInputFormat::Raw),
            "doc" | "docx" | "odt" | "docm" | "dotx" | "dotm" => {
                InputFormat::Office(OfficeInputFormat::Doc)
            }
            "ppt" | "pptx" | "odp" | "pptm" | "potx" | "potm" => {
                InputFormat::Office(OfficeInputFormat::Ppt)
            }
            "xls" | "xlsx" | "ods" | "xlsm" | "xltx" | "xltm" | "csv" => {
                InputFormat::Office(OfficeInputFormat::Xls)
            }
            "mp4" => InputFormat::Video(VideoInputFormat::Mp4),
            "webm" => InputFormat::Video(VideoInputFormat::Webm),
            "mkv" => InputFormat::Video(VideoInputFormat::Mkv),
            "avi" => InputFormat::Video(VideoInputFormat::Avi),
            "av1" => InputFormat::Video(VideoInputFormat::Av1),
            "3gp" => InputFormat::Video(VideoInputFormat::_3gp),
            "m4v" => InputFormat::Video(VideoInputFormat::M4v),
            "flv" => InputFormat::Video(VideoInputFormat::Flv),
            "mov" => InputFormat::Video(VideoInputFormat::Mov),
            "mpeg" => InputFormat::Video(VideoInputFormat::Mpeg),
            "mts" => InputFormat::Video(VideoInputFormat::Mts),
            "hevc" => InputFormat::Video(VideoInputFormat::Hevc),
            _ => InputFormat::Unsupported,
        };

        format
    }
}
