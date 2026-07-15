use windows::{Win32::Storage::FileSystem::GetLogicalDrives, core::Result};

use crate::disk::information::DiskHandle;

// ---------------------------------------------------------------------------
// Helper nội bộ — sau này nên tách ra raw/read.rs (mở raw device, đọc byte thô)
// và volume/enumerate.rs (map drive letter <-> disk_id). Để tạm ở đây cho gọn.
// ---------------------------------------------------------------------------

/// Mở 1 path bất kỳ (volume "\\.\C:" hoặc physical disk "\\.\PhysicalDriveN"),
/// hỏi Windows: cái này đang nằm trên physical disk số mấy.

/// Tìm drive letter (nếu có) đang mount trên đúng disk_id truyền vào.
fn drive_letter_for_disk(disk_id: u32) -> Option<char> {
    let drives_mask = unsafe { GetLogicalDrives() };

    for i in 0..26u32 {
        if drives_mask & (1 << i) == 0 {
            continue;
        }
        let letter = (b'A' + i as u8) as char;
        let volume_path = format!(r"\\.\{letter}:");

        if let Ok(handle) = DiskHandle::open(&volume_path) {
            if let Ok(mapped_id) = handle.disk_number() {
                if mapped_id == disk_id {
                    return Some(letter);
                }
            }
        }
    }
    None
}

/// Đọc 512 byte đầu tiên (sector 0) của device_path — dùng khi đĩa chưa mount được.
// ---------------------------------------------------------------------------
// 3 hàm ra quyết định
// ---------------------------------------------------------------------------

/// Disk này có phải nơi Windows đang chạy không (chứa %SystemDrive%, thường là C:).
pub fn is_system_disk(disk_id: u32) -> Result<bool> {
    let letter = std::env::var("SystemDrive")
        .ok()
        .and_then(|s| s.chars().next())
        .unwrap_or('C');

    let volume_path = format!(r"\\.\{letter}:");
    match DiskHandle::open(&volume_path)?.disk_number() {
        Ok(system_disk_id) => Ok(system_disk_id == disk_id),
        Err(_) => Ok(false), // không xác định được -> để disk_has_data/is_boot_disk lo phần còn lại
    }
}


/// Disk này có đang chứa file/thư mục thật hay không.
/// Ưu tiên 1: nếu Windows mount được (có drive letter) -> read_dir thật, chính xác nhất.
/// Fallback: chưa mount được (RAW/Unknown) -> đoán qua signature MBR/GPT ở sector 0.
pub fn disk_has_data(disk_id: u32, device_path: &str) -> Result<bool> {
    if let Some(letter) = drive_letter_for_disk(disk_id) {
        let root = format!(r"{letter}:\");
        if let Ok(mut entries) = std::fs::read_dir(&root) {
            return Ok(entries.next().is_some());
        }
    }

    let sector = DiskHandle::open(device_path)?.read_first_sector()?;
    Ok(sector[510] == 0x55 && sector[511] == 0xAA)
}
