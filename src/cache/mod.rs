use std::{cmp, fs};
use std::fs::remove_file;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use log::error;

use crate::parameters::UrlParameters;
use crate::services::formats::OutputFormat;

pub mod buster;

pub fn get_path_from_url_parameters(url_parameters: &UrlParameters<'_>, output_format: &OutputFormat) -> String {

    let env_cache = std::env::var("CACHE").unwrap_or("/tmp".to_string());
    let params_hash = crate::crypto::json_hash(url_parameters);
    let filename_hash = crate::crypto::string_hash(&url_parameters.path.to_string_lossy());

    let parts = [&params_hash[0..2], &params_hash[2..4], &params_hash[4..6]];
    let cache_path = format!("{env_cache}/{}", parts.join("/"));
    
    if let Err(e) = fs::create_dir_all(&cache_path) {
        error!("Failed to create cache directory: {}", e);
    }
    
    cache_path + "/" + &filename_hash + "." + &output_format.to_string()

}

pub fn is_cached(cache_path: &str, url_parameters: &UrlParameters<'_>) -> bool {
    
    let cached_max_time = match fs::metadata(cache_path) {
        Ok(metadata) => cmp::max(metadata.mtime(), metadata.ctime()),
        Err(_) => return false
    };

    let original_max_time = match url_parameters.path.metadata() {
        Ok(metadata) => cmp::max(metadata.mtime(), metadata.ctime()),
        Err(_) => return true
    };  
    
    if cached_max_time < original_max_time {
        remove_file(cache_path).unwrap_or_else(|e| error!("Failed to remove cache file: {}", e));
        return false;
    }
    
    true
    
}

pub fn index(cache_path: String, file_path: PathBuf) {
    
    let cache_path = Path::new(&cache_path);
    let cache_path_stem = cache_path.file_stem().unwrap().to_string_lossy();
    
    // Create index file
    let index_path = cache_path.with_file_name(format!("{cache_path_stem}.index"));
    let index_content = file_path.to_string_lossy();
    
    if let Err(e) = fs::write(index_path, index_content.as_bytes()) {
        error!("Failed to write cache index file: {}", e);
    }
    
}
