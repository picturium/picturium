use crate::process::pipeline::request::PipelineRequest;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

const INTERMEDIATE_DIR: &str = "intermediate";

pub fn generate_intermediate_path(
    request: &PipelineRequest,
    source_path: &PathBuf,
    intermediate_format: &str,
) -> String {
    let cache_dir = &request.state.config.cache.dir;

    let mut hasher = DefaultHasher::new();
    source_path.hash(&mut hasher);
    let hash = format!("{:016x}", hasher.finish());

    let intermediate_dir = PathBuf::from(cache_dir).join(INTERMEDIATE_DIR);
    let first_dir = &hash[0..2];
    let second_dir = &hash[2..4];
    let filename = format!("{}.{}", hash, intermediate_format);

    intermediate_dir
        .join(first_dir)
        .join(second_dir)
        .join(filename)
        .to_string_lossy()
        .into_owned()
}
