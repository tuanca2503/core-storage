use crate::{
    disk::{entry::DiskEntry, mount::TempMount},
    ensure_root,
    error::{BaseError, BaseResult, Codes},
    header::Header,
};
use std::{
    fs::OpenOptions,
    io::{Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

#[cfg(target_os = "windows")]
pub use crate::windows::format::*;

#[cfg(target_os = "linux")]
pub use crate::linux::format::*;

#[cfg(target_os = "macos")]
pub use crate::macos::format::*;

pub enum FormatMode {
    Quick,
    ZeroFill,
}
impl FormatMode {
    pub fn from_str(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "zerofill" | "zero-fill" | "zero_fill" => Self::ZeroFill,
            _ => Self::Quick,
        }
    }
}



fn allowed_format(
    forced: bool,
    device_path: &Path,
    volume_paths: &Vec<PathBuf>,
) -> BaseResult<()> {
    ensure_root()?;

    if volume_paths.len() > 0 {
        for path in volume_paths {
            if let Some(mount) = TempMount::mount(path)? {
                // Mout success
                if mount.has_system_directory() {
                    return Err(BaseError::system_warning(
                        format!(
                            "[{}] Contains system directory.",
                            device_path.to_string_lossy()
                        ),
                        Codes::PermissionDenied,
                    ));
                }
                if mount.has_directory_entries() && !forced {
                    return Err(BaseError::system_warning(
                        format!(
                            "[{}] Contains directory entries. Try --forced",
                            device_path.to_string_lossy()
                        ),
                        Codes::PermissionDenied,
                    ));
                }
                //Passed unmmout volume path
                TempMount::unmount(path)?
            } else if !forced {
                // Mout fail and not forced
                return Err(BaseError::system_warning(
                    format!(
                        "[{}] Cannot mount volume. Try --forced",
                        device_path.to_string_lossy()
                    ),
                    Codes::PermissionDenied,
                ));
            }
            // Out this already unmout temp path
        }
        erase::wipe_signatures(&device_path)?;
    } else {
        if Header::is_magic(&device_path)? && !forced {
            return Err(BaseError::system_warning(
                format!(
                    "[{}] Contains core format. Try --forced",
                    device_path.to_string_lossy()
                ),
                Codes::PermissionDenied,
            ));
        }
    }

    Ok(())
}

pub fn format_disk(forced: bool, name: String, mode: FormatMode) -> BaseResult<()> {
    let disk_entry = DiskEntry::verify(name)?;
    let logical_sector_size = disk_entry.logical_sector_size();
    let capacity_bytes = disk_entry.capacity_bytes(logical_sector_size);
    allowed_format(
        forced,
        &disk_entry.device_path,
        &disk_entry.volume_paths,
    )?;

    let header = Header::create(
        capacity_bytes,
        disk_entry.physical_sector_size(logical_sector_size),
    )?;
    let header_bytes = header.to_bytes();
    let bitmap = vec![0u8; header.bitmap_size_bytes as usize]; // 0 = segment trống

    let mut device: std::fs::File = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&disk_entry.device_path)?;

    match mode {
        FormatMode::ZeroFill => {
            erase::zero_fill(&mut device, capacity_bytes)?;
        }
        FormatMode::Quick => {}
    }

    // Ghi primary header
    device.seek(SeekFrom::Start(0))?;
    device.write_all(&header_bytes)?;

    // Ghi bitmap
    device.seek(SeekFrom::Start(header.bitmap_offset))?;
    device.write_all(&bitmap)?;

    // Ghi mirror header (cuối device)
    device.seek(SeekFrom::Start(header.mirror_offset))?;
    device.write_all(&header_bytes)?;

    device.flush()?;
    device.sync_all()?;

    Ok(())
}
