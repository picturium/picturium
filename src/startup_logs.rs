use tracing::info;
use crate::config::SharedConfig;
use crate::state::AppState;

pub fn  print_startup_logs(config: &SharedConfig, state: &AppState) {
    let gradient: &[(u8, u8, u8)] = &[
        (255,  40,   0),
        (255, 120,   0),
        (255, 180,   0),
        (255, 220,   0),
        (220, 255,   0),
        (120, 255,   0),
        (  0, 255,  80),
        (  0, 255, 160),
        (  0, 200, 255),
        (  0, 120, 255),
        (120,  80, 255),
        (255,   0, 200),
    ];

    let ascii_art = [
        "░▒▓███████▓▒░░▒▓█▓▒░░▒▓██████▓▒░▒▓████████▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓███████▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓██████████████▓▒░  ",
        "░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░ ░▒▓█▓▒░   ░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░░▒▓█▓▒░",
        "░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░▒▓█▓▒░        ░▒▓█▓▒░   ░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░░▒▓█▓▒░",
        "░▒▓███████▓▒░░▒▓█▓▒░▒▓█▓▒░        ░▒▓█▓▒░   ░▒▓█▓▒░░▒▓█▓▒░▒▓███████▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░░▒▓█▓▒░",
        "░▒▓█▓▒░      ░▒▓█▓▒░▒▓█▓▒░        ░▒▓█▓▒░   ░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░░▒▓█▓▒░",
        "░▒▓█▓▒░      ░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░ ░▒▓█▓▒░   ░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░░▒▓█▓▒░",
        "░▒▓█▓▒░      ░▒▓█▓▒░░▒▓██████▓▒░  ░▒▓█▓▒░    ░▒▓██████▓▒░░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓██████▓▒░░▒▓█▓▒░░▒▓█▓▒░░▒▓█▓▒░",
    ];

    println!();

    for (line_idx, line) in ascii_art.iter().enumerate() {
        for (char_idx, ch) in line.chars().enumerate() {
            let color_idx = (line_idx + char_idx + if line_idx % 2 == 1 { 1 } else { 0 }) / 2;
            let (r, g, b) = gradient[color_idx % gradient.len()];
            let opacity = match ch {
                '█' => 1.00,
                '▓' => 0.80,
                '▒' => 0.60,
                '░' => 0.40,
                _   => 1.00,
            };
            let (r, g, b) = (
                (r as f32 * opacity) as u8,
                (g as f32 * opacity) as u8,
                (b as f32 * opacity) as u8,
            );
            print!("\x1b[38;2;{r};{g};{b}m{ch}");
        }
        println!("\x1b[0m");
    }

    println!();
    println!("███████ Version: {} ███████████████████████████████████████████████████████████████████████████████████████\n", env!("CARGO_PKG_VERSION"));

    info!("Starting Picturium v{}", env!("CARGO_PKG_VERSION"));
    info!("Configuration loaded:");
    info!("  Server: {}", config.server.get_address());
    info!("  Data directory: {}", config.data.dir);
    info!("  Cache directory: {}", config.cache.dir);
    info!(
        "  Memory cache: {} ({} entries, {} MiB per entry)",
        config.cache.memory.enabled,
        config.cache.memory.capacity,
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
}
