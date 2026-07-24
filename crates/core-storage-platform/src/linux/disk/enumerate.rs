use crate::{
    StorageInfo,
    disk::{DiskInfo, entry::DiskEntry},
    ensure_root,
    error::BaseResult,
    format_date, format_size,
    header::Header,
};

///////////////////////////////////////////////////////////////////////////////////////////

pub fn enumerate_storage_info_to_table() -> BaseResult<Vec<Vec<String>>> {
    ensure_root()?;
    let mut disks = Vec::new();
    disks.push(vec![
        "NAME".into(),
        "UUID".into(),
        "VERS".into(),
        "STATE".into(),
        "SEG_COUNT".into(),
        "TOTAL".into(),
        "CREATE_AT".into(),
    ]);

    DiskEntry::for_each_disk(|disk_entry| {
        let volume_paths = disk_entry.volume_paths()?;
        let header = Header::try_load(&volume_paths, &disk_entry.device_path);
        disks.push(vec![
            disk_entry.name.clone(),
            uuid::Uuid::from_bytes(header.uuid).to_string(),
            header.version.to_string(),
            header.state.to_string(),
            header.segment_count.to_string(),
            format_size(header.total_bytes),
            format_date(header.created_at_ms),
        ]);
        Ok(())
    })?;

    Ok(disks)
}

pub fn enumerate_disk_info() -> BaseResult<Vec<DiskInfo>> {
    let mut disks = Vec::new();
    DiskEntry::for_each_disk(|disk_entry| {
        let logical_sector_size = disk_entry.logical_sector_size();
        disks.push(DiskInfo::new(
            disk_entry.serial(),
            disk_entry.model(),
            disk_entry.sysfs_path.clone(),
            disk_entry.device_path.clone(),
            disk_entry.volume_paths()?,
            disk_entry.removable(),
            disk_entry.read_only(),
            disk_entry.capacity_bytes(logical_sector_size),
            logical_sector_size,
            disk_entry.physical_sector_size(logical_sector_size),
        ));
        Ok(())
    })?;

    Ok(disks)
}

pub fn enumerate_storage_info() -> BaseResult<Vec<StorageInfo>> {
    ensure_root()?;
    let mut disks = Vec::new();
    DiskEntry::for_each_disk(|disk_entry| {
        let logical_sector_size = disk_entry.logical_sector_size();
        let device_path = disk_entry.device_path.clone();
        let volume_paths = disk_entry.volume_paths()?;
        let header = Header::try_load(&volume_paths, &device_path);
        let disk = DiskInfo::new(
            disk_entry.serial(),
            disk_entry.model(),
            disk_entry.sysfs_path.clone(),
            disk_entry.device_path.clone(),
            disk_entry.volume_paths()?,
            disk_entry.removable(),
            disk_entry.read_only(),
            disk_entry.capacity_bytes(logical_sector_size),
            logical_sector_size,
            disk_entry.physical_sector_size(logical_sector_size),
        );
        disks.push(StorageInfo::new(disk, header));
        Ok(())
    })?;
    Ok(disks)
}
