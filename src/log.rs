//! Logging init (colorized, with timestamps).

use tracing_subscriber::fmt::time::UtcTime;

/// Map a numeric verbosity (0..2) to a tracing level string.
pub fn level_from_verbosity(v: u8) -> &'static str {
    match v {
        0 => "warn",
        2 => "debug",
        _ => "info",
    }
}

/// Initialize global logging with color and RFC3339 timestamps.
/// Accepts a level string like "warn" | "info" | "debug".
pub fn init(level: &str) {
    // If a global subscriber is already set, ignore errors.
    let _ = tracing_subscriber::fmt()
        // e.g. 2025-09-23T13:37:42Z
        .with_timer(UtcTime::rfc_3339())
        .with_env_filter(level)
        .with_target(false) // cleaner lines
        .with_ansi(true) // force colors when TTY
        .with_level(true)
        .try_init();
}
