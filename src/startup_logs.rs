use tracing::{info, warn};
use crate::config::SharedConfig;
use crate::state::AppState;

const ART: [&str; 7] = [
    "███████    ██    ██████    ████████   ██    ██   ███████    ██   ██    ██   █████████",
    "██    ██   ██   ██    ██      ██      ██    ██   ██    ██   ██   ██    ██   ██  ██  ██",
    "██    ██   ██   ██            ██      ██    ██   ██    ██   ██   ██    ██   ██  ██  ██",
    "███████    ██   ██            ██      ██    ██   ████████   ██   ██    ██   ██  ██  ██",
    "██         ██   ██            ██      ██    ██   ██   ██    ██   ██    ██   ██  ██  ██",
    "██         ██   ██    ██      ██      ██    ██   ██    ██   ██   ██    ██   ██  ██  ██",
    "██         ██    ██████       ██       ██████    ██    ██   ██    ██████    ██  ██  ██",
];

/// Hue in degrees for each non-space character of the matching ART row.
const HUES: [&[u16]; 7] = [
    &[40, 53, 57, 66, 72, 79, 91, 345, 3, 4, 26, 34, 45, 56, 66, 90, 107, 121, 147, 158, 165, 178, 187, 56, 58, 68, 77, 33, 29, 19, 4, 346, 329, 317, 194, 195, 195, 196, 270, 280, 353, 1, 12, 24, 38, 43, 55, 67, 86],
    &[54, 60, 93, 112, 3, 12, 350, 31, 66, 73, 151, 159, 58, 64, 73, 84, 33, 24, 323, 322, 191, 193, 197, 201, 276, 290, 5, 18, 44, 51, 100, 116],
    &[61, 71, 119, 129, 16, 23, 15, 43, 160, 163, 61, 70, 79, 96, 34, 25, 328, 333, 190, 192, 203, 209, 284, 299, 13, 22, 49, 57, 114, 122],
    &[64, 72, 77, 97, 116, 124, 131, 26, 35, 37, 54, 167, 169, 65, 77, 95, 115, 34, 27, 21, 7, 346, 332, 339, 346, 190, 193, 209, 214, 289, 304, 20, 29, 58, 66, 122, 131],
    &[69, 84, 33, 40, 43, 56, 182, 186, 68, 80, 111, 119, 33, 22, 348, 360, 192, 195, 212, 220, 293, 308, 26, 36, 63, 71, 132, 137],
    &[79, 103, 36, 45, 51, 61, 94, 119, 189, 195, 71, 81, 122, 122, 30, 21, 9, 9, 195, 203, 215, 226, 298, 309, 32, 43, 66, 77, 140, 146],
    &[90, 118, 41, 50, 56, 66, 77, 86, 102, 118, 196, 201, 72, 80, 94, 116, 128, 128, 29, 21, 16, 17, 202, 208, 241, 258, 267, 281, 298, 305, 39, 47, 72, 82, 147, 155],
];

fn hue_rgb(hue: u16) -> (u8, u8, u8) {
    let hue = hue % 360;
    let hue_fraction = (hue % 60) as f32 / 60.0;
    let up = (hue_fraction * 255.0) as u8;
    let down = 255 - up;

    match hue / 60 {
        0 => (255, up, 0),
        1 => (down, 255, 0),
        2 => (0, 255, up),
        3 => (0, down, 255),
        4 => (up, 0, 255),
        _ => (255, 0, down),
    }
}

pub fn print_startup_logs(config: &SharedConfig, state: &AppState) {
    println!();

    for (row, line) in ART.iter().enumerate() {
        let mut hues = HUES[row].iter();

        for char in line.chars() {
            if char == ' ' {
                print!(" ");
                continue;
            }

            let (red, green, blue) = hue_rgb(*hues.next().unwrap());
            print!("\x1b[38;2;{red};{green};{blue}m{char}");
        }

        println!("\x1b[0m");
    }

    println!();

    let version = format!(" Version: {} ", env!("CARGO_PKG_VERSION"));
    let bar = ART[0].chars().count().saturating_sub(version.chars().count());
    println!("{}{version}{}\n", "█".repeat(7), "█".repeat(bar.saturating_sub(6)));

    info!("Starting Picturium v{}", env!("CARGO_PKG_VERSION"));
    info!("Configuration loaded:");
    info!("  Server: {}", config.server.get_address());
    info!("  Data directory: {}", config.data.dir);
    info!("  Cache directory: {}", config.cache.dir);
    info!(
        "  Memory cache: {} ({} MiB total, {} MiB per entry)",
        config.cache.memory.enabled,
        config.cache.memory.limit,
        config.cache.memory.entry_limit
    );
    info!(
        "  Disk cache: {} ({} MiB effective capacity)",
        config.cache.disk.enabled,
        config.cache.disk.limit
    );
    info!("  CORS origins: {}", config.cors.allowed_origins.join(", "));
    info!("  Signature verification: {}", config.security.signature_enabled);
    info!("  Vips debug: {}", config.vips.debug);
    info!("  Vips concurrency: {}", config.vips.concurrency);
    info!("  Multithreading: {} workers, {} queue size", state.multithreading.get_available_workers(), state.multithreading.get_available_queue_size());

    oversized_memory_cache_warning(config);
}

/// The memory cache budget is not the whole footprint: libvips working memory per in-flight
/// request comes on top of it, so warn well before the budget reaches the limit.
const MEMORY_WARN_RATIO: f64 = 0.75;

fn oversized_memory_cache_warning(config: &SharedConfig) {
    if !config.cache.memory.enabled {
        return;
    }

    let Some(available) = available_memory() else {
        return;
    };

    let budget = config.cache.memory.limit as u64 * 1024 * 1024;

    if budget as f64 > available as f64 * MEMORY_WARN_RATIO {
        warn!(
            "cache.memory.limit is {} MiB but only {} MiB are available to this process; \
             libvips needs memory per in-flight request on top of the cache",
            config.cache.memory.limit,
            available / (1024 * 1024)
        );
    }
}

/// Inside a container `/proc/meminfo` reports the host's memory, so the cgroup limit is the
/// only source that sees the real ceiling. Take whichever is lower.
fn available_memory() -> Option<u64> {
    let cgroup = ["/sys/fs/cgroup/memory.max", "/sys/fs/cgroup/memory/memory.limit_in_bytes"]
        .iter()
        .find_map(|path| std::fs::read_to_string(path).ok())
        .and_then(|raw| parse_cgroup_limit(&raw));

    let total = std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|raw| parse_mem_total(&raw));

    match (cgroup, total) {
        (Some(cgroup), Some(total)) => Some(cgroup.min(total)),
        (cgroup, total) => cgroup.or(total),
    }
}

fn parse_cgroup_limit(raw: &str) -> Option<u64> {
    raw.trim().parse().ok()
}

fn parse_mem_total(raw: &str) -> Option<u64> {
    raw.lines()
        .find_map(|line| line.strip_prefix("MemTotal:"))
        .and_then(|value| value.split_whitespace().next())
        .and_then(|kib| kib.parse::<u64>().ok())
        .map(|kib| kib * 1024)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_mem_total_in_bytes() {
        let raw = "MemFree:         1000 kB\nMemTotal:        2048 kB\n";
        assert_eq!(parse_mem_total(raw), Some(2048 * 1024));
        assert_eq!(parse_mem_total("SwapTotal: 0 kB"), None);
    }

    #[test]
    fn unlimited_cgroup_is_not_a_limit() {
        assert_eq!(parse_cgroup_limit("max\n"), None);
        assert_eq!(parse_cgroup_limit("2097152\n"), Some(2097152));
    }
}
