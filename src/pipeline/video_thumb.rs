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

#[cfg(feature = "native-ffmpeg")]
use ffmpeg_next as ffmpeg;

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
    // Check native-ffmpeg first (best performance, no process spawning)
    #[cfg(feature = "native-ffmpeg")]
    {
        return Some(VideoBackend::NativeFFmpeg);
    }
    
    // Check command-line ffmpeg (better performance than mpv)
    #[cfg(not(feature = "native-ffmpeg"))]
    {
        if Command::new("ffmpeg").arg("-version").output().is_ok() {
            return Some(VideoBackend::FFmpeg);
        }
        
        // Fallback to mpv
        if Command::new("mpv").arg("--version").output().is_ok() {
            return Some(VideoBackend::Mpv);
        }
        
        None
    }
}

fn get_video_backend() -> Result<VideoBackend, PipelineError> {
    let backend_env = env::var("VIDEO_BACKEND").unwrap_or_else(|_| "auto".to_string());
    
    match backend_env.to_lowercase().as_str() {
        "native" => {
            #[cfg(feature = "native-ffmpeg")]
            {
                Ok(VideoBackend::NativeFFmpeg)
            }
            #[cfg(not(feature = "native-ffmpeg"))]
            {
                Err(PipelineError("native backend requested but native-ffmpeg feature not enabled".to_string()))
            }
        }
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

#[cfg(feature = "native-ffmpeg")]
fn generate_thumbnail_native_ffmpeg(
    working_file: &Path,
    url_parameters: &UrlParameters<'_>,
    positions: &[ThumbnailPosition],
    cache_path: &str,
) -> PipelineResult<VipsImage> {
    use ffmpeg::format::{input, Pixel};
    use ffmpeg::media::Type;
    use ffmpeg::software::scaling::{context::Context, flag::Flags};
    use ffmpeg::util::frame::video::Video;
    use std::fs::File;
    use std::io::Write;
    
    // Initialize ffmpeg (safe to call multiple times)
    ffmpeg::init().map_err(|e| PipelineError(format!("Failed to initialize ffmpeg: {}", e)))?;
    
    let mut best_thumbnail = None;
    let mut best_size = 0;
    
    let (width, height) = calculate_thumbnail_dimensions(url_parameters);
    let target_width = if width > 0 { width as u32 } else { 0 };
    let target_height = if height > 0 { height as u32 } else { 0 };
    
    let path_str = working_file.to_string_lossy();
    let working_file_str = working_file.to_str()
        .ok_or_else(|| PipelineError("Invalid file path".to_string()))?;
    
    // Open input video
    let mut ictx = input(&working_file_str)
        .map_err(|e| PipelineError(format!("Failed to open video: {}", e)))?;
    
    // Find best video stream and extract needed info before borrowing mutably
    let (video_stream_index, time_base, avg_frame_rate, decoder_params) = {
        let video_stream = ictx.streams()
            .best(Type::Video)
            .ok_or_else(|| PipelineError("No video stream found".to_string()))?;
        (
            video_stream.index(),
            video_stream.time_base(),
            video_stream.avg_frame_rate(),
            video_stream.parameters(),
        )
    };
    
    // Get video duration for percentage calculations
    let duration_secs = ictx.duration() as f64 / f64::from(ffmpeg::ffi::AV_TIME_BASE);
    
    // Create decoder
    let context_decoder = ffmpeg::codec::context::Context::from_parameters(decoder_params)
        .map_err(|e| PipelineError(format!("Failed to create decoder context: {}", e)))?;
    let mut decoder = context_decoder.decoder().video()
        .map_err(|e| PipelineError(format!("Failed to create video decoder: {}", e)))?;
    
    // Determine output dimensions
    let (out_width, out_height) = if target_width == 0 && target_height == 0 {
        (decoder.width(), decoder.height())
    } else if target_width == 0 {
        let ratio = decoder.width() as f64 / decoder.height() as f64;
        ((target_height as f64 * ratio) as u32, target_height)
    } else if target_height == 0 {
        let ratio = decoder.height() as f64 / decoder.width() as f64;
        (target_width, (target_width as f64 * ratio) as u32)
    } else {
        (target_width, target_height)
    };
    
    // Try each position
    for position in positions {
        // Calculate target timestamp
        let target_timestamp = match position {
            ThumbnailPosition::Percentage(p) => {
                let target_secs = duration_secs * (*p as f64 / 100.0);
                (target_secs * time_base.denominator() as f64 / time_base.numerator() as f64) as i64
            }
            ThumbnailPosition::Frame(f) => {
                // Calculate timestamp from frame number
                let frame_duration = avg_frame_rate.denominator() as f64 / avg_frame_rate.numerator() as f64;
                let target_secs = *f as f64 * frame_duration;
                (target_secs * time_base.denominator() as f64 / time_base.numerator() as f64) as i64
            }
        };
        
        // Seek to position
        if let Err(e) = ictx.seek(target_timestamp, ..target_timestamp) {
            debug!("Failed to seek to position {:?}: {}", position, e);
            continue;
        }
        
        // Decode frame at position
        for (stream, packet) in ictx.packets() {
            if stream.index() == video_stream_index {
                if let Err(e) = decoder.send_packet(&packet) {
                    debug!("Failed to send packet: {}", e);
                    continue;
                }
                
                let mut decoded = Video::empty();
                if decoder.receive_frame(&mut decoded).is_ok() {
                    // Create scaler
                    let mut scaler = Context::get(
                        decoder.format(),
                        decoder.width(),
                        decoder.height(),
                        Pixel::RGB24,
                        out_width,
                        out_height,
                        Flags::BILINEAR,
                    ).map_err(|e| PipelineError(format!("Failed to create scaler: {}", e)))?;
                    
                    // Scale frame
                    let mut rgb_frame = Video::empty();
                    scaler.run(&decoded, &mut rgb_frame)
                        .map_err(|e| PipelineError(format!("Failed to scale frame: {}", e)))?;
                    
                    // Save as PNG
                    let temp_path = Path::new(cache_path).join("video").join(format!(
                        "native-thumbnailer-{}-{}.png",
                        hash(&path_str),
                        match position {
                            ThumbnailPosition::Percentage(p) => format!("{}pct", p),
                            ThumbnailPosition::Frame(f) => format!("{}f", f),
                        }
                    ));
                    
                    // Save frame as PPM first (simple format), then we'll convert
                    // Actually, let's save as raw RGB and use libvips to create PNG
                    let temp_path_str = temp_path.to_string_lossy().to_string();
                    
                    // For now, save as PPM and convert with libvips
                    let ppm_path = temp_path.with_extension("ppm");
                    let ppm_path_str = ppm_path.to_string_lossy().to_string();
                    
                    let mut file = File::create(&ppm_path)
                        .map_err(|e| PipelineError(format!("Failed to create temp file: {}", e)))?;
                    file.write_all(format!("P6\n{} {}\n255\n", rgb_frame.width(), rgb_frame.height()).as_bytes())
                        .map_err(|e| PipelineError(format!("Failed to write PPM header: {}", e)))?;
                    file.write_all(rgb_frame.data(0))
                        .map_err(|e| PipelineError(format!("Failed to write frame data: {}", e)))?;
                    drop(file);
                    
                    // Load with libvips and save as PNG
                    let vips_img = VipsImage::new_from_file(&ppm_path_str)
                        .map_err(|e| PipelineError(format!("Failed to load frame with libvips: {}", e)))?;
                    vips_img.image_write_to_file(&temp_path_str)
                        .map_err(|e| PipelineError(format!("Failed to save PNG: {}", e)))?;
                    
                    // Clean up PPM
                    let _ = fs::remove_file(&ppm_path);
                    
                    // Check file size
                    if let Ok(metadata) = temp_path.metadata() {
                        let file_size = metadata.len();
                        if file_size > 0 && file_size > best_size {
                            best_size = file_size;
                            best_thumbnail = Some(temp_path.clone());
                        }
                    }
                    
                    // Found a frame, move to next position
                    break;
                }
            }
        }
        
        // Flush decoder
        let _ = decoder.send_eof();
    }
    
    // Return best thumbnail
    match best_thumbnail {
        Some(path) => {
            debug!("Using native ffmpeg thumbnail from {}", path.to_string_lossy());
            let path_str = path.to_string_lossy().to_string();
            if !path.exists() {
                return Err(PipelineError(format!("Thumbnail file does not exist: {}", path_str)));
            }
            let result = VipsImage::new_from_file(&path_str)
                .map_err(|e| PipelineError(format!("Failed to load video thumbnail: {}", e)));
            result
        }
        None => Err(PipelineError("Failed to generate video thumbnail with native ffmpeg".to_string())),
    }
}

pub fn generate_video_thumbnail(working_file: &Path, url_parameters: &UrlParameters<'_>) -> PipelineResult<VipsImage> {
    let backend = get_video_backend()?;
    let positions = parse_thumbnail_positions();
    let cache_path = get_cache_dir()?;
    
    match backend {
        VideoBackend::FFmpeg => generate_thumbnail_ffmpeg(working_file, url_parameters, &positions, &cache_path),
        VideoBackend::Mpv => generate_thumbnail_mpv(working_file, url_parameters, &positions, &cache_path),
        #[cfg(feature = "native-ffmpeg")]
        VideoBackend::NativeFFmpeg => generate_thumbnail_native_ffmpeg(working_file, url_parameters, &positions, &cache_path),
    }
}

fn hash<T: Hash>(data: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    hasher.finish()
}