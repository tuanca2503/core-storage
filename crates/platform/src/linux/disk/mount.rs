use std::{
    fs,
    io::{Error, Result},
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

pub struct TempMount(PathBuf);

impl TempMount {
    pub fn mount(volume: &Path) -> Result<Self> {
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System clock is before UNIX_EPOCH")
            .as_millis();
        let mount_point = std::env::temp_dir().join(format!("core-storage-{id}"));
        fs::create_dir_all(&mount_point)?;
        let output = Command::new("mount")
            .arg(volume)
            .arg(&mount_point)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::other(format!(
                "Failed to mount '{}' at '{}': {}",
                volume.display(),
                mount_point.display(),
                stderr.trim()
            )));
        }
        Ok(Self(mount_point))
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

    pub fn unmount(volume: &Path) -> Result<()> {
        let output = Command::new("umount").arg(volume).output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            if !stderr.contains("not mounted") {
                return Err(Error::other(format!("Unmount failed {}", stderr)));
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
