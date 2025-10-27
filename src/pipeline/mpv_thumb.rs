use std::collections::hash_map::DefaultHasher;
use std::env;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::process::Command;

use libvips::VipsImage;
use log::debug;

use crate::parameters::UrlParameters;
use crate::pipeline::{PipelineError, PipelineResult};
use crate::pipeline::resize::get_dimensions;

#[derive(Debug, Clone, Copy, PartialEq)]
enum VideoBackend {
    FFmpeg,
    Mpv,
}

#[derive(Debug, Clone)]
enum ThumbnailPosition {
    Percentage(u8),
    Frame(u32),
}

impl ThumbnailPosition {
    fn to_mpv_arg(&self) -> String {
        match self {
            ThumbnailPosition::Percentage(p) => format!("{}%", p),
            ThumbnailPosition::Frame(f) => format!("#{}", f),
        }
    }

    fn to_ffmpeg_arg(&self, _duration: Option<f64>) -> String {
        match self {
            ThumbnailPosition::Percentage(p) => {
                // ffmpeg uses seconds, we'll calculate from percentage if duration is known
                // For now, we'll use a simple heuristic
                format!("{}%", p)
            }
            ThumbnailPosition::Frame(f) => format!("{}", f),
        }
    }
}

fn detect_available_backend() -> Option<VideoBackend> {
    // Check ffmpeg first (better performance according to maintainer)
    if Command::new("ffmpeg").arg("-version").output().is_ok() {
        return Some(VideoBackend::FFmpeg);
    }
    
    // Fallback to mpv
    if Command::new("mpv").arg("--version").output().is_ok() {
        return Some(VideoBackend::Mpv);
    }
    
    None
}

fn get_video_backend() -> Result<VideoBackend, PipelineError> {
    let backend_env = env::var("VIDEO_BACKEND").unwrap_or_else(|_| "auto".to_string());
    
    match backend_env.to_lowercase().as_str() {
        "ffmpeg" => {
            if Command::new("ffmpeg").arg("-version").output().is_ok() {
                Ok(VideoBackend::FFmpeg)
            } else {
                Err(PipelineError("ffmpeg backend requested but not available in PATH".to_string()))
            }
        }
        "mpv" => {
            if Command::new("mpv").arg("--version").output().is_ok() {
                Ok(VideoBackend::Mpv)
            } else {
                Err(PipelineError("mpv backend requested but not available in PATH".to_string()))
            }
        }
        "auto" | _ => {
            detect_available_backend()
                .ok_or_else(|| PipelineError("No video thumbnail backend available (neither ffmpeg nor mpv found in PATH)".to_string()))
        }
    }
}

fn parse_thumbnail_positions() -> Vec<ThumbnailPosition> {
    let positions_env = env::var("VIDEO_THUMBNAIL_POSITIONS").unwrap_or_default();
    
    if positions_env.is_empty() {
        // Default positions
        return vec![
            ThumbnailPosition::Percentage(25),
            ThumbnailPosition::Percentage(20),
            ThumbnailPosition::Percentage(15),
            ThumbnailPosition::Percentage(0),
        ];
    }
    
    let mut positions = Vec::new();
    for pos_str in positions_env.split(',') {
        let pos_str = pos_str.trim();
        if pos_str.ends_with('%') {
            if let Ok(percentage) = pos_str.trim_end_matches('%').parse::<u8>() {
                if percentage <= 100 {
                    positions.push(ThumbnailPosition::Percentage(percentage));
                }
            }
        } else if let Ok(frame) = pos_str.parse::<u32>() {
            positions.push(ThumbnailPosition::Frame(frame));
        }
    }
    
    // If parsing failed completely, return defaults
    if positions.is_empty() {
        vec![
            ThumbnailPosition::Percentage(25),
            ThumbnailPosition::Percentage(20),
            ThumbnailPosition::Percentage(15),
            ThumbnailPosition::Percentage(0),
        ]
    } else {
        positions
    }
}

fn calculate_thumbnail_dimensions(url_parameters: &UrlParameters<'_>) -> (i32, i32) {
    // If neither width nor height is specified, use a reasonable default
    // Otherwise, use the get_dimensions logic from resize module
    let (width, height) = if url_parameters.width.is_none() && url_parameters.height.is_none() {
        // Return original size - we'll need to handle this in the backend implementations
        // For now, use a sensible default
        (0, 0) // 0 indicates original size
    } else {
        // Create a dummy 1x1 image to calculate dimensions
        // This is a workaround since we don't have the actual video dimensions yet
        match VipsImage::black(1920, 1080) {
            Ok(dummy_image) => get_dimensions(&dummy_image, url_parameters),
            Err(_) => {
                // Fallback to simple calculation
                let width = url_parameters.width.unwrap_or(300) as i32;
                let height = url_parameters.height.unwrap_or(300) as i32;
                (width, height)
            }
        }
    };
    
    (width, height)
}

fn get_cache_dir() -> Result<String, PipelineError> {
    let cache_path = env::var("CACHE").unwrap_or(env::temp_dir().to_string_lossy().to_string());
    let video_cache = Path::new(&cache_path).join("video");
    
    if !video_cache.exists() {
        fs::create_dir_all(&video_cache)
            .map_err(|e| PipelineError(format!("Failed to create video cache directory: {}", e)))?;
    }
    
    Ok(cache_path)
}

pub fn generate_video_thumbnail(working_file: &Path, url_parameters: &UrlParameters<'_>) -> PipelineResult<VipsImage> {
    let mut best_thumbnail = None;
    let mut best_size = 0;
    let size = url_parameters.width.unwrap_or(300).to_string();
    let path_str = working_file.to_string_lossy();
    let cache_path = env::var("CACHE").unwrap_or(env::temp_dir().to_string_lossy().to_string());
    if !Path::new(&cache_path).join("video").exists() {
        if let Err(e) = fs::create_dir_all(Path::new(&cache_path).join("video")) {
            return Err(PipelineError(format!("Failed to create video directory: {}", e)));
        }
    }
    let mpv_executable = env::var("MPV").unwrap_or("mpv".to_string());
    // Try thumbnailing at different positions
    for start in ["25%", "20%", "15%", "0"] {
        let temp_path = Path::new(&cache_path).join("video").join(format!(
            "mpv-thumbnailer-{}-{}.png",
            hash(&path_str),
            start.replace("%", "")
        ));

        let temp_path_str = temp_path.to_string_lossy().to_string();

        // Generate thumbnail with mpv
        let status = Command::new(&mpv_executable)
            .arg("--really-quiet")
            .arg("--no-config")
            .arg("--aid=no")
            .arg("--sid=no")
            .arg(format!("--vf=scale={}:{}/dar", size, size))
            .arg(format!("--start={}", start))
            .arg("--frames=1")
            .arg(format!("--o={}", temp_path_str))
            .arg(working_file)
            .status();

        match status {
            Ok(status) if status.success() && temp_path.exists() => {
                if let Ok(metadata) = temp_path.metadata() {
                    let file_size = metadata.len();
                    if file_size > best_size {
                        best_size = file_size;
                        best_thumbnail = Some(temp_path.clone());
                    }
                }
            }
            _ => continue,
        }

        if best_thumbnail.is_some() {
            break;
        }
    }

    // Clean up temp files and return result
    match best_thumbnail {
        Some(path) => {
            debug!("Using thumbnail from {}", path.to_string_lossy());
            let path_str = path.to_string_lossy().to_string();
            //check if file exists
            if !path.exists() {
                return Err(PipelineError(format!("Thumbnail file does not exist: {}", path_str)));
            }
            let result = VipsImage::new_from_file(&path_str)
                .map_err(|e| PipelineError(format!("Failed to load video thumbnail: {}", e)));
            //let _ = fs::remove_file(path);
            result
        }
        None => Err(PipelineError("Failed to generate video thumbnail".to_string())),
    }
}

fn hash<T: Hash>(data: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    hasher.finish()
}