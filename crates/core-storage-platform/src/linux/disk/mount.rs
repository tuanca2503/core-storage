use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::error::{BaseError, BaseResult, Codes};
pub struct TempMount(PathBuf);

impl TempMount {
    pub fn mount(volume: &Path) -> BaseResult<Option<Self>> {
        let id = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
        let mount_point = std::env::temp_dir().join(format!("core-storage-{id}"));
        fs::create_dir_all(&mount_point)?;
        let status = Command::new("mount")
            .arg(volume)
            .arg(&mount_point)
            .status()?;
        if !status.success() {
            return Ok(None);
        }
        Ok(Some(Self(mount_point)))
    }

    pub fn has_directory_entries(&self) -> bool {
        let mut entries = match std::fs::read_dir(&self.0) {
            Ok(entries) => entries,
            Err(_) => return true,
        };
        entries.next().is_some()
    }
    pub fn has_system_directory(&self) -> bool {
        let root_indicators = ["boot", "etc", "bin", "usr", "var"];

        for name in root_indicators {
            if self.0.join(name).exists() {
                return true;
            }
        }

        false
    }

    pub fn unmount(volume: &Path) -> BaseResult<()> {
        let output = Command::new("umount").arg(volume).output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            if !stderr.contains("not mounted") {
                return Err(BaseError::system_error(
                    format!("Unmount failed {}", stderr),
                    Codes::Command,
                ));
            }
        }

        _ = fs::remove_dir_all(volume);
        Ok(())
    }
}

impl Drop for TempMount {
    fn drop(&mut self) {
        let _ = TempMount::unmount(&self.0);
    }
}
