use std::io::{Seek, SeekFrom, Write};
use std::path::Path;

use crate::error::{BaseError, BaseResult, Codes};

pub fn wipe_signatures(path: &Path) -> BaseResult<()> {
    // Remove existing filesystem/partition signatures before formatting
    let output = std::process::Command::new("wipefs")
        .arg("--all")
        .arg("--force")
        .arg(path)
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(BaseError::system_error(
            format!("Wipefs failed: {}", stderr.trim()),
            Codes::Command,
        ));
    }
    Ok(())
}

pub fn zero_fill<W: Write + Seek>(device: &mut W, capacity_bytes: u64) -> BaseResult<()> {
    const CHUNK_SIZE: usize = 4 * 1024 * 1024; // 4 MiB / lần ghi
    let chunk = vec![0u8; CHUNK_SIZE];

    device.seek(SeekFrom::Start(0))?;

    let mut written: u64 = 0;
    while written < capacity_bytes {
        let remaining = capacity_bytes - written;
        let write_len = remaining.min(CHUNK_SIZE as u64) as usize;
        device.write_all(&chunk[..write_len]).map_err(|e| {
            BaseError::system_error(
                format!("Zero-fill failed at offset {}: {}", written, e),
                crate::error::Codes::Raw,
            )
        })?;
        written += write_len as u64;
    }

    Ok(())
}
