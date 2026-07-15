use std::{
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use crate::{
    disk::{PhysicalDisk, mout::TempMount},
    error::BaseResult,
};

pub fn enumerate_physical_disks() -> BaseResult<Vec<PhysicalDisk>> {
    let mut disks = Vec::new();
    let block_dir = Path::new("/sys/block");
    if !block_dir.exists() {
        return Ok(disks);
    }
    for entry in fs::read_dir(block_dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        let sysfs_path = entry.path();
        // Bỏ qua các thiết bị ảo: loop, ram, dm-*, sr (CD/DVD) nếu không cần
        if name.starts_with("loop")
            || name.starts_with("ram")
            || name.starts_with("dm-")
            || name.starts_with("zram")
            || name.starts_with("md")
            || name.starts_with("sr")
            || !sysfs_path.join("device").exists()
        {
            continue;
        }
        let sectors = read_as::<u64>(&sysfs_path.join("size")).unwrap_or(0);
        if sectors == 0 {
            continue;
        }
        let logical_sector_size =
            read_as::<u32>(&sysfs_path.join("queue/logical_block_size")).unwrap_or(512);
        let physical_sector_size = read_as::<u32>(&sysfs_path.join("queue/physical_block_size"))
            .unwrap_or(logical_sector_size);
        let removable = read_as::<u32>(&sysfs_path.join("removable")).unwrap_or(0) != 0;
        let read_only = read_as::<u32>(&sysfs_path.join("ro")).unwrap_or(0) != 0;
        let capacity_bytes = sectors.saturating_mul(logical_sector_size as u64);
        let vendor = read_as::<String>(&sysfs_path.join("device/vendor")).unwrap_or_default();
        let device_model = read_as::<String>(&sysfs_path.join("device/model")).unwrap_or_default();
        let model = format!("{} {}", vendor.trim(), device_model.trim());
        let serial = read_as::<String>(&sysfs_path.join("device/serial")).unwrap_or_default();
        let device_path = PathBuf::from("/dev").join(&name);

        disks.push(PhysicalDisk {
            volume_paths: enumerate_volumes(&sysfs_path, &name)?,
            name,
            device_path,
            sysfs_path,
            model,
            serial,
            removable,
            read_only,
            capacity_bytes,
            logical_sector_size,
            physical_sector_size,
        });
    }
    Ok(disks)
}

pub fn has_directory(volume_paths: Vec<PathBuf>) -> bool {
    if volume_paths.len() == 0 {
        //to-do check custom format here if match return true else false
        return false;
    }
    for volume in volume_paths {
        let mount = match TempMount::mount(&volume) {
            Ok(m) => m,
            Err(_) => return true,
        };
        if mount.has_directory_entries() {
            return true;
        }
    }

    false
}
//
fn read_as<T>(path: &Path) -> Option<T>
where
    T: FromStr,
{
    fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse::<T>().ok())
}
fn enumerate_volumes(sysfs_path: &PathBuf, name: &str) -> BaseResult<Vec<PathBuf>> {
    let mut volumes = Vec::new();
    for entry in fs::read_dir(sysfs_path)? {
        let entry = entry?;

        if !entry.file_type()?.is_dir() {
            continue;
        }
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if file_name.starts_with(name) {
            volumes.push(PathBuf::from("/dev").join(file_name.as_ref()));
        }
    }

    Ok(volumes)
}

