use std::path::{Path, PathBuf};

const APP_DIR: &str = "/var/lib/core-storage";

pub fn get_database_path() -> PathBuf {
    std::fs::create_dir_all(APP_DIR)
        .unwrap_or_else(|e| panic!("Can not create dir {}: {}", APP_DIR, e));

    Path::new(APP_DIR).join("metadata.db")
}
