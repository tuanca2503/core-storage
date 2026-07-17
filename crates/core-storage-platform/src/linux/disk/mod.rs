mod enumerate;
mod mout;


use crate::{
    disk::PhysicalDisk,
    error::{BaseError, BaseResult, Codes},
};

pub fn allowed_format(forced: bool, ps: &PhysicalDisk) -> BaseResult<()> {
    for volume in ps.volume_paths.clone() {
        if let Some(mount) = mout::TempMount::mount(&volume)? {
            // Mout success
            if mount.has_system_directory() {
                return Err(BaseError::system_warning(
                    format!("[{}] Contains system directory.", ps.name),
                    Codes::PermissionDenied,
                ));
            }
            if mount.has_directory_entries() && !forced {
                return Err(BaseError::system_warning(
                    format!("[{}] Has directory entries. Try --forced", ps.name),
                    Codes::PermissionDenied,
                ));
            }
        } else if !forced {
            // Mout fail and not forced
            return Err(BaseError::system_warning(
                format!("[{}] Cannot mount volume. Try --forced", ps.name),
                Codes::PermissionDenied,
            ));
        }
    }
    Ok(())
}

pub fn enumerate_physical_disks() -> BaseResult<Vec<PhysicalDisk>> {
    Ok(enumerate::enumerate_physical_disks()?)
}
