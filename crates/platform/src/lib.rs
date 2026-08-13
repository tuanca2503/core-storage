cfg_if::cfg_if! {
    if #[cfg(target_os = "linux")] {
        mod linux;
        pub use linux::{disk,erase,paths};
    } else if #[cfg(target_os = "windows")] {
        compile_error!("Unsupported platform");
        mod windows;
    } else if #[cfg(target_os = "macos")] {
        compile_error!("Unsupported platform");
        mod macos;
    } else {
        compile_error!("Unsupported platform");
    }
}
