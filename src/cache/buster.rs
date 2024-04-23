use std::{cmp, env};
use std::fs::File;
use std::io::Read;
use std::os::unix::fs::MetadataExt;
use std::process::Command;
use std::path::Path;
use log::info;
use walkdir::WalkDir;

pub fn bust_cache() {

    info!("[SCHEDULER] Busting cache...");

    let cache_path = env::var("CACHE").unwrap_or("/tmp".to_string());

    if !max_size_exceeded(&cache_path) {
        info!("[SCHEDULER] Cache size is within limits, no need to bust");
        return;
    }

    let busted = detect_out_of_date(&cache_path);
    remove_out_of_date(&cache_path, busted);

    info!("[SCHEDULER] Cache busted!");

}

fn detect_out_of_date(cache_path: &str) -> Vec<String> {

    let mut bust = vec![];

    for entry in WalkDir::new(cache_path).follow_links(true) {

        if entry.is_err() {
            continue;
        }

        let entry = entry.unwrap();

        if !entry.file_type().is_file() || !entry.file_name().to_string_lossy().ends_with(".index") {
            continue;
        }

        let mut original_path = String::new();

        match File::open(entry.path()) {
            Ok(mut file) => if file.read_to_string(&mut original_path).is_err() { continue },
            _ => continue
        }

        let original_max_time = match Path::new(&original_path).metadata() {
            Ok(metadata) => cmp::max(metadata.mtime(), metadata.ctime()),
            _ => continue
        };

        let cache_max_time = match entry.metadata() {
            Ok(metadata) => cmp::max(metadata.mtime(), metadata.ctime()),
            _ => continue
        };

        if original_max_time > cache_max_time {
            bust.push(match entry.path().file_stem() {
                Some(stem) => stem.to_string_lossy().to_string(),
                _ => continue
            });
        }

    }

    bust

}

fn remove_out_of_date(cache_path: &str, busted: Vec<String>) {

    for entry in WalkDir::new(cache_path).follow_links(true) {

        if entry.is_err() {
            continue;
        }

        let entry = entry.unwrap();

        if !entry.file_type().is_file() {
            continue;
        }

        let stem = match entry.path().file_stem() {
            Some(stem) => stem.to_string_lossy().to_string(),
            _ => continue
        };

        if busted.contains(&stem) {
            let _ = std::fs::remove_file(entry.path());
        }

    }

}

fn max_size_exceeded(cache_path: &str) -> bool {

    let max_size = env::var("CACHE_CAPACITY").unwrap_or("10".to_string()); // in GB
    let max_size = max_size.parse::<u64>().unwrap() * 1024 * 1024; // in kB

    let command = Command::new("du")
        .arg("-s")
        .arg(cache_path)
        .output()
        .expect("Failed to execute command");

    let output = String::from_utf8_lossy(&command.stdout);
    let size = output.split_whitespace().next().unwrap();
    let size = size.parse::<u64>().unwrap(); // in kB

    size > max_size

}