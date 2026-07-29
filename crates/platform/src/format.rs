use crate::disk::{DiskEntry, TempMount};

use model::{BaseError, BaseResult, ErrorCode, Segment, Storage, segment_bin, storage_bin};
use std::io::{Seek, SeekFrom, Write};

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

fn allowed_format(forced: bool, disk_entry: &DiskEntry) -> BaseResult<()> {
    let volume_paths = disk_entry.volume_paths()?;
    if volume_paths.len() > 0 {
        for path in &volume_paths {
            if let Some(mount) = TempMount::mount(path)? {
                // Mout success
                if mount.has_system_directory() {
                    return Err(BaseError::system_warning(
                        format!("[{}] Contains system directory.", disk_entry.name),
                        ErrorCode::PermissionDenied,
                    ));
                }
                if mount.has_directory_entries() && !forced {
                    return Err(BaseError::system_warning(
                        format!(
                            "[{}] Contains directory entries. Try --forced",
                            disk_entry.name
                        ),
                        ErrorCode::PermissionDenied,
                    ));
                }
                //Passed unmmout volume path
                TempMount::unmount(path)?
            } else if !forced {
                // Mout fail and not forced
                return Err(BaseError::system_warning(
                    format!("[{}] Cannot mount volume. Try --forced", disk_entry.name),
                    ErrorCode::PermissionDenied,
                ));
            }
            // Out this already unmout temp path
        }
        erase::wipe_signatures(&disk_entry.device_path)?;
    } else {
        if storage_bin::has_valid_magic(&mut disk_entry.open_device(0)?) && !forced {
            return Err(BaseError::system_warning(
                format!("[{}] Contains core format. Try --forced", disk_entry.name),
                ErrorCode::PermissionDenied,
            ));
        }
    }

    Ok(())
}

pub fn format_disk(forced: bool, name: String, mode: FormatMode) -> BaseResult<()> {
    let disk_entry = DiskEntry::verify(name)?;
    let logical_sector_size = disk_entry.logical_sector_size();
    let capacity_bytes = disk_entry.capacity_bytes(logical_sector_size);
    allowed_format(forced, &disk_entry)?;
    let storage = Storage::new(
        capacity_bytes,
        disk_entry.physical_sector_size(logical_sector_size),
        logical_sector_size,
    );
    let header_bytes = storage_bin::to_bytes(&storage);
    let mut device = disk_entry.open_device(1)?;

    match mode {
        FormatMode::ZeroFill => {
            erase::zero_fill(&mut device, capacity_bytes)?;
        }
        FormatMode::Quick => {}
    }

    device.seek(SeekFrom::Start(0))?;
    device.write_all(&header_bytes)?;

    for index in 0..storage.segment_count {
        let segment = if index == storage.segment_count - 1 {
            Segment::new(storage.last_segment_size_bytes)
        } else {
            Segment::default()
        };
        device.seek(SeekFrom::Start(segment_bin::offset(index)))?;
        device.write_all(&segment_bin::to_bytes(&segment))?;
    }

    device.seek(SeekFrom::Start(storage.mirror_offset))?;
    device.write_all(&header_bytes)?;

    device.flush()?;
    device.sync_all()?;

    Ok(())
}
