use std::io::{Error, Result, Seek, SeekFrom, Write};
use std::path::Path;

pub fn wipe_signatures(path: &Path) -> Result<()> {
    // Remove existing filesystem/partition signatures before formatting
    let output = std::process::Command::new("wipefs")
        .arg("--all")
        .arg("--force")
        .arg(path)
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(Error::other(format!(
            "wipefs failed ({}): {}",
            output.status,
            stderr.trim()
        )));
    }
    Ok(())
}

pub fn zero_fill<W: Write + Seek>(device: &mut W, capacity_bytes: u64) -> Result<()> {
    let chunk_size: usize = 4 * 1024 * 1024; // 4 MiB / lần ghi
    let chunk = vec![0u8; chunk_size];

    device.seek(SeekFrom::Start(0))?;

    let mut written: u64 = 0;
    while written < capacity_bytes {
        let remaining = capacity_bytes - written;
        let write_len = remaining.min(chunk_size as u64) as usize;
        device.write_all(&chunk[..write_len])?;
        written += write_len as u64;
    }

    Ok(())
}
