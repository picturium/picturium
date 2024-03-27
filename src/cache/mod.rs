use std::fs;
use std::fs::remove_file;
use std::os::unix::fs::MetadataExt;
use log::error;
use crate::parameters::UrlParameters;
use crate::services::formats::OutputFormat;

pub fn get_path_from_url_parameters(url_parameters: &UrlParameters<'_>, output_format: &OutputFormat) -> String {

    let env_cache = std::env::var("CACHE").unwrap_or_else(|_| "/tmp".to_string());
    let hash = crate::crypto::json_hash(url_parameters);

    let parts = [&hash[0..6], &hash[6..12], &hash[12..18]];
    let cache_path = format!("{env_cache}/{}", parts.join("/"));
    
    if let Err(e) = fs::create_dir_all(&cache_path) {
        error!("Failed to create cache directory: {}", e);
    }
    
    cache_path + "/" + &hash[18..] + "." + &output_format.to_string()

}

pub fn is_cached(cache_path: &str, url_parameters: &UrlParameters<'_>) -> bool {
    
    let cached_metadata = match fs::metadata(cache_path) {
        Ok(metadata) => metadata,
        Err(_) => return false
    };

    let file_metadata = match url_parameters.path.metadata() {
        Ok(metadata) => metadata,
        Err(_) => return true
    };
    
    if cached_metadata.mtime() < file_metadata.mtime() {
        remove_file(cache_path).unwrap_or_else(|e| error!("Failed to remove cache file: {}", e));
        return false;
    }
    
    true
    
}