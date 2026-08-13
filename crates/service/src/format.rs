use std::io::{Seek, SeekFrom, Write};

use chrono::DateTime;
use model::{Segment, Storage, segment_bin, storage_bin};
use platform::{
    disk::{DiskEntry, TempMount},
    erase::{wipe_signatures, zero_fill},
};

use crate::{BaseError, BaseResult, ErrorCode, };

pub const KB: u64 = 1024;
pub const MB: u64 = 1024 * KB;
pub const GB: u64 = 1024 * MB;
pub const TB: u64 = 1024 * GB;

pub fn format_size(bytes: u64) -> String {
    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

pub fn format_date(ms: u64) -> String {
    let default = "--/--/----".to_string();
    if ms == 0 {
        return default;
    }

    DateTime::from_timestamp_millis(ms as i64)
        .map(|dt| dt.format("%d/%m/%Y").to_string())
        .unwrap_or_else(|| default)
}
//
fn allowed_format_disk(forced: bool, disk_entry: &DiskEntry) -> BaseResult<()> {
    let volume_paths = disk_entry.volume_paths()?;
    if volume_paths.len() > 0 {
        for path in &volume_paths {
            match TempMount::mount(path) {
                Ok(mount) => {
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
                }
                Err(e) => {
                    if !forced {
                        return Err(BaseError::system_warning(
                            format!(
                                "[{}] Cannot mount volume <{}>. Try --forced",
                                disk_entry.name, e
                            ),
                            ErrorCode::PermissionDenied,
                        ));
                    }
                }
            }
            // Out this already unmout temp path
        }
        wipe_signatures(&disk_entry.device_path)?;
    } else {
        if storage_bin::has_valid_magic(&mut disk_entry.open_device(0)?)? && !forced {
            return Err(BaseError::system_warning(
                format!("[{}] Contains core format. Try --forced", disk_entry.name),
                ErrorCode::PermissionDenied,
            ));
        }
    }

    Ok(())
}

pub fn format_disk(forced: bool, zero_mode: bool, name: String) -> BaseResult<()> {
    let disk_entry = DiskEntry::verify(name)?;
    let logical_sector_size = disk_entry.logical_sector_size();
    let capacity_bytes = disk_entry.capacity_bytes(logical_sector_size);
    allowed_format_disk(forced, &disk_entry)?;
    let storage = Storage::new(
        capacity_bytes,
        disk_entry.physical_sector_size(logical_sector_size),
        logical_sector_size,
    );
    let header_bytes = storage_bin::to_bytes(&storage);
    let mut device = disk_entry.open_device(1)?;
    if zero_mode {
        zero_fill(&mut device, capacity_bytes)?;
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
