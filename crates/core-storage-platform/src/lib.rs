#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "macos")]
mod macos;

pub mod disk;
pub mod raw;
pub mod filesystem;
pub mod format;
