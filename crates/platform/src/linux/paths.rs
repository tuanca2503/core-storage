use std::path::{PathBuf};
use std::io::Result;

const APP_DIR: &str = "/var/lib/core-storage";

pub fn app_directory() -> Result<PathBuf> {
    let dir = PathBuf::from(APP_DIR);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn app_join(path: &str) -> Result<PathBuf> {
    let dir = app_directory()?;
    Ok(dir.join(path))
}