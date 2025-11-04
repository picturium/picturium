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

#[derive(Debug, Clone, Copy, PartialEq)]
enum VideoBackend {
    FFmpeg,
    Mpv,
    #[cfg(feature = "native-ffmpeg")]
    NativeFFmpeg,
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
    // Calculate dimensions similar to get_dimensions in resize.rs
    // If neither width nor height is specified, return 0 to indicate original size
    if url_parameters.width.is_none() && url_parameters.height.is_none() {
        return (0, 0); // 0 indicates original size should be used
    }
    
    let width = url_parameters.width.map(|w| w as i32);
    let height = url_parameters.height.map(|h| h as i32);
    
    // If only one dimension is specified, the backend will maintain aspect ratio
    // If both are specified, use both
    match (width, height) {
        (Some(w), Some(h)) => (w, h),
        (Some(w), None) => (w, -1), // -1 indicates auto-calculate to maintain aspect ratio
        (None, Some(h)) => (-1, h),
        (None, None) => (0, 0), // Original size
    }
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

fn generate_thumbnail_mpv(
    working_file: &Path,
    url_parameters: &UrlParameters<'_>,
    positions: &[ThumbnailPosition],
    cache_path: &str,
) -> PipelineResult<VipsImage> {
    let mut best_thumbnail = None;
    let mut best_size = 0;
    
    let (width, height) = calculate_thumbnail_dimensions(url_parameters);
    
    // Determine scale filter for mpv
    // mpv format: scale=width:height or scale=width:height/dar to preserve aspect ratio
    let scale_filter = match (width, height) {
        (0, 0) => String::new(), // Original size, no scaling
        (w, -1) | (w, 0) if w > 0 => format!("--vf=scale={}:-1", w), // Width specified, auto height
        (-1, h) | (0, h) if h > 0 => format!("--vf=scale=-1:{}", h), // Height specified, auto width
        (w, h) if w > 0 && h > 0 => format!("--vf=scale={}:{}", w, h), // Both specified
        _ => "--vf=scale=300:-1".to_string(), // Fallback
    };
    
    let path_str = working_file.to_string_lossy();
    
    // Try thumbnailing at different positions
    for position in positions {
        let position_str = position.to_mpv_arg();
        let temp_path = Path::new(cache_path).join("video").join(format!(
            "mpv-thumbnailer-{}-{}.png",
            hash(&path_str),
            position_str.replace("%", "").replace("#", "f")
        ));

        let temp_path_str = temp_path.to_string_lossy().to_string();

        // Generate thumbnail with mpv
        let mut cmd = Command::new("mpv");
        cmd.arg("--really-quiet")
            .arg("--no-config")
            .arg("--aid=no")
            .arg("--sid=no");
        
        if !scale_filter.is_empty() {
            cmd.arg(&scale_filter);
        }
        
        let status = cmd
            .arg(format!("--start={}", position_str))
            .arg("--frames=1")
            .arg(format!("--o={}", temp_path_str))
            .arg(working_file)
            .status();

        match status {
            Ok(status) if status.success() && temp_path.exists() => {
                if let Ok(metadata) = temp_path.metadata() {
                    let file_size = metadata.len();
                    // Fix: properly compare file sizes (not just > 0)
                    if file_size > 0 && file_size > best_size {
                        best_size = file_size;
                        best_thumbnail = Some(temp_path.clone());
                    }
                }
            }
            _ => continue,
        }
    }

    // Clean up temp files and return result
    match best_thumbnail {
        Some(path) => {
            debug!("Using mpv thumbnail from {}", path.to_string_lossy());
            let path_str = path.to_string_lossy().to_string();
            if !path.exists() {
                return Err(PipelineError(format!("Thumbnail file does not exist: {}", path_str)));
            }
            let result = VipsImage::new_from_file(&path_str)
                .map_err(|e| PipelineError(format!("Failed to load video thumbnail: {}", e)));
            result
        }
        None => Err(PipelineError("Failed to generate video thumbnail with mpv".to_string())),
    }
}

fn generate_thumbnail_ffmpeg(
    working_file: &Path,
    url_parameters: &UrlParameters<'_>,
    positions: &[ThumbnailPosition],
    cache_path: &str,
) -> PipelineResult<VipsImage> {
    let mut best_thumbnail = None;
    let mut best_size = 0;
    
    let (width, height) = calculate_thumbnail_dimensions(url_parameters);
    
    let path_str = working_file.to_string_lossy();
    
    // Try thumbnailing at different positions
    for position in positions {
        let temp_path = Path::new(cache_path).join("video").join(format!(
            "ffmpeg-thumbnailer-{}-{}.png",
            hash(&path_str),
            match position {
                ThumbnailPosition::Percentage(p) => format!("{}pct", p),
                ThumbnailPosition::Frame(f) => format!("{}f", f),
            }
        ));

        let temp_path_str = temp_path.to_string_lossy().to_string();

        // Build ffmpeg command based on position type
        let mut cmd = Command::new("ffmpeg");
        cmd.arg("-v").arg("quiet"); // Suppress output
        
        // Build video filter chain
        let mut vfilters = Vec::new();
        
        match position {
            ThumbnailPosition::Percentage(p) => {
                // For percentages, we use -ss with a percentage
                // Note: ffmpeg doesn't directly support percentages, so we approximate
                // For now, use the percentage as a rough time estimate (assuming typical video length)
                cmd.arg("-ss").arg(format!("{}%", p));
            }
            ThumbnailPosition::Frame(f) => {
                // For frames, seek to specific frame
                vfilters.push(format!("select='eq(n\\,{})'", f));
            }
        }
        
        // Add scale filter if dimensions are specified
        let scale_filter = match (width, height) {
            (0, 0) => None, // Original size, no scaling
            (w, -1) | (w, 0) if w > 0 => Some(format!("scale={}:-1", w)),
            (-1, h) | (0, h) if h > 0 => Some(format!("scale=-1:{}", h)),
            (w, h) if w > 0 && h > 0 => Some(format!("scale={}:{}", w, h)),
            _ => Some("scale=300:-1".to_string()),
        };
        
        if let Some(scale) = scale_filter {
            vfilters.push(scale);
        }
        
        cmd.arg("-i").arg(working_file)
            .arg("-vframes").arg("1");
        
        // Apply video filters if any
        if !vfilters.is_empty() {
            cmd.arg("-vf").arg(vfilters.join(","));
        }
        
        cmd.arg("-y") // Overwrite output file
            .arg(&temp_path_str);

        let status = cmd.status();

        match status {
            Ok(status) if status.success() && temp_path.exists() => {
                if let Ok(metadata) = temp_path.metadata() {
                    let file_size = metadata.len();
                    // Fix: properly compare file sizes
                    if file_size > 0 && file_size > best_size {
                        best_size = file_size;
                        best_thumbnail = Some(temp_path.clone());
                    }
                }
            }
            _ => continue,
        }
    }

    // Clean up temp files and return result
    match best_thumbnail {
        Some(path) => {
            debug!("Using ffmpeg thumbnail from {}", path.to_string_lossy());
            let path_str = path.to_string_lossy().to_string();
            if !path.exists() {
                return Err(PipelineError(format!("Thumbnail file does not exist: {}", path_str)));
            }
            let result = VipsImage::new_from_file(&path_str)
                .map_err(|e| PipelineError(format!("Failed to load video thumbnail: {}", e)));
            result
        }
        None => Err(PipelineError("Failed to generate video thumbnail with ffmpeg".to_string())),
    }
}

pub fn generate_video_thumbnail(working_file: &Path, url_parameters: &UrlParameters<'_>) -> PipelineResult<VipsImage> {
    let backend = get_video_backend()?;
    let positions = parse_thumbnail_positions();
    let cache_path = get_cache_dir()?;
    
    match backend {
        VideoBackend::FFmpeg => generate_thumbnail_ffmpeg(working_file, url_parameters, &positions, &cache_path),
        VideoBackend::Mpv => generate_thumbnail_mpv(working_file, url_parameters, &positions, &cache_path),
    }
}

fn hash<T: Hash>(data: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    hasher.finish()
}