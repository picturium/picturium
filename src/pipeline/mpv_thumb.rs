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