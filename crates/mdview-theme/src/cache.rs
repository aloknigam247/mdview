use crate::_stubs::Theme;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("no data directory available on this platform")]
    NoDataDir,
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

static OVERRIDE: OnceLock<PathBuf> = OnceLock::new();

pub fn set_cache_dir_for_tests(p: PathBuf) {
    let _ = OVERRIDE.set(p);
}

pub fn cache_dir() -> Result<PathBuf, CacheError> {
    if let Some(p) = OVERRIDE.get() {
        return Ok(p.clone());
    }
    let base = dirs::data_dir().ok_or(CacheError::NoDataDir)?;
    Ok(base.join("mdview").join("theme-cache"))
}

pub fn cache_key(colorscheme: &str, nvim_version: &str) -> String {
    format!("{colorscheme}-{nvim_version}")
}

fn path_for(key: &str) -> Result<PathBuf, CacheError> {
    Ok(cache_dir()?.join(format!("{key}.json")))
}

pub fn load(key: &str) -> Option<Theme> {
    let path = path_for(key).ok()?;
    let bytes = fs::read(&path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

pub fn store(key: &str, theme: &Theme) -> Result<(), CacheError> {
    let dir = cache_dir()?;
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{key}.json"));
    let bytes = serde_json::to_vec_pretty(theme)?;
    write_atomic(&path, &bytes)?;
    Ok(())
}

pub fn clear_all() -> Result<(), CacheError> {
    let dir = cache_dir()?;
    match fs::remove_dir_all(&dir) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), CacheError> {
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path)?;
    Ok(())
}
