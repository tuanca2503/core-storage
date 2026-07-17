use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::error::{BaseError, BaseResult, Codes};
pub struct TempMount(PathBuf);

impl TempMount {
    pub fn mount(volume: &Path) -> BaseResult<Option<Self>> {
        if unsafe { libc::geteuid() } != 0 {
            return Err(BaseError::system_warning(
                "This operation requires root privileges.".to_string(),
                Codes::PermissionDenied,
            ));
        }

        let id = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
        let mount_point = std::env::temp_dir().join(format!("core-storage-{id}"));
        fs::create_dir_all(&mount_point)?;
        let status = Command::new("mount")
            .arg(volume)
            .arg(&mount_point)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
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

    fn unmount(&self) -> BaseResult<()> {
        Command::new("umount")
            .arg(&self.0)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        fs::remove_dir_all(&self.0)?;
        Ok(())
    }
}

impl Drop for TempMount {
    fn drop(&mut self) {
        let _ = self.unmount();
    }
}
