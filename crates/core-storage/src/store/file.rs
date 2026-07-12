use std::fs;
use std::io::Result;
use std::path::Path;

pub fn read_string<P: AsRef<Path>>(path: P) -> Result<String> {
    let s = fs::read_to_string(path)?;
    Ok(s.trim().to_string())
}