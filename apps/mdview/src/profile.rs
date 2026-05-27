use std::sync::OnceLock;
use std::time::Instant;

static START: OnceLock<Instant> = OnceLock::new();

pub fn init() {
    let _ = START.set(Instant::now());
}

pub fn enabled() -> bool {
    std::env::var("MDV_PROFILE")
        .map(|v| !v.is_empty() && v != "0")
        .unwrap_or(false)
}

pub fn log(event: &str) {
    if !enabled() {
        return;
    }
    let started = START.get_or_init(Instant::now);
    let elapsed = started.elapsed();
    eprintln!("mdv-profile: {:>7.3}s {}", elapsed.as_secs_f64(), event);
}
