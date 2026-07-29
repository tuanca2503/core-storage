#[cfg(target_os = "windows")]
pub use crate::windows::disk::*;

#[cfg(target_os = "linux")]
pub use crate::linux::disk::*;

#[cfg(target_os = "macos")]
pub use crate::macos::disk::*;

use model::{ BaseResult, Segment, Storage, segment_bin, storage_bin};
use service::format::{format_date, format_size};

pub fn physical_disk_info() -> BaseResult<Vec<Vec<String>>> {
    let mut disks = Vec::new();
    disks.push(vec![
        "SERIAL".into(),
        "MODULE".into(),
        "SP".into(),
        "DP".into(),
        "VOLS".into(),
        "REMOVE".into(),
        "RO".into(),
        "TOTAL".into(),
        "LS".into(),
        "PS".into(),
    ]);

    DiskEntry::for_each_disk(|disk_entry| {
        let logical_sector_size = disk_entry.logical_sector_size();
        disks.push(vec![
            disk_entry.serial(),
            disk_entry.model(),
            disk_entry.sysfs_path.display().to_string(),
            disk_entry.device_path.display().to_string(),
            disk_entry
                .volume_paths()?
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", "),
            disk_entry.removable().to_string(),
            disk_entry.read_only().to_string(),
            disk_entry.capacity_bytes(logical_sector_size).to_string(),
            logical_sector_size.to_string(),
            disk_entry
                .physical_sector_size(logical_sector_size)
                .to_string(),
        ]);
        Ok(())
    })?;

    Ok(disks)
}

pub fn storage_to_table(valid: bool) -> BaseResult<Vec<Vec<String>>> {
    let mut disks = Vec::new();
    disks.push(vec![
        "NAME".into(),
        "STATE".into(),
        "UUID".into(),
        "VERS".into(),
        "SEG_COUNT".into(),
        "TOTAL".into(),
        "CREATE_AT".into(),
    ]);
    DiskEntry::for_each_disk(|disk_entry| {
        if !valid || !disk_entry.has_volumes()? {
            let storage = storage_bin::from_device(&mut disk_entry.open_device(0)?);
            disks.push(vec![
                disk_entry.name.clone(),
                storage.state.to_string(),
                uuid::Uuid::from_bytes(storage.uuid).to_string(),
                storage.version.to_string(),
                storage.segment_count.to_string(),
                format_size(storage.capacity_bytes),
                format_date(storage.created_at_ms),
            ]);
        }
        Ok(())
    })?;

    Ok(disks)
}

pub fn segment_to_table(name: String) -> BaseResult<Vec<Vec<String>>> {
    let mut segments = Vec::new();
    segments.push(vec![
        "INDEX".into(),
        "CHUNK_COUNT".into(),
        "CHUNK_CAPACITY".into(),
    ]);
    let disk_entry = DiskEntry::verify(name)?;
    let mut device = disk_entry.open_device(0)?;
    let storage = storage_bin::from_device(&mut device);

    for index in 0..storage.segment_count {
        let segment = segment_bin::from_device(index, &mut device);
        segments.push(vec![
            format!("{}", index + 1).into(),
            segment.chunk_count.to_string(),
            segment.chunk_capacity.to_string(),
        ]);
    }

    Ok(segments)
}
