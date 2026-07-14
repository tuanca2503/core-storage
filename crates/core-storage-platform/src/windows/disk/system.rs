use std::mem::size_of;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_GENERIC_READ, FILE_SHARE_READ, FILE_SHARE_WRITE,
    GetLogicalDrives, OPEN_EXISTING, PARTITION_SYSTEM_GUID, ReadFile,
};
use windows::Win32::System::IO::DeviceIoControl;
use windows::Win32::System::Ioctl::{
    DRIVE_LAYOUT_INFORMATION_EX, IOCTL_DISK_GET_DRIVE_LAYOUT_EX, IOCTL_STORAGE_GET_DEVICE_NUMBER,
    PARTITION_INFORMATION_EX, PARTITION_STYLE_GPT, PARTITION_STYLE_MBR, STORAGE_DEVICE_NUMBER,
};
use windows::core::{PCWSTR, Result};

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
fn read_first_sector(device_path: &str) -> Result<[u8; 512]> {
    let wide: Vec<u16> = device_path
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let handle: HANDLE = CreateFileW(
            PCWSTR(wide.as_ptr()),
            FILE_GENERIC_READ.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )?;

        let mut sector = [0u8; 512];
        let mut bytes_read = 0u32;
        ReadFile(handle, Some(&mut sector), Some(&mut bytes_read), None)?;
        let _ = CloseHandle(handle);
        Ok(sector)
    }
}

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

/// Disk này có phải ổ mà firmware (BIOS/UEFI) dùng để boot không.
/// MBR: có partition nào bật BootIndicator.
/// GPT: có partition nào là EFI System Partition (PartitionType == PARTITION_SYSTEM_GUID).
pub fn is_boot_disk(device_path: &str) -> Result<bool> {
    let wide: Vec<u16> = device_path
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let handle: HANDLE = CreateFileW(
            PCWSTR(wide.as_ptr()),
            FILE_GENERIC_READ.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )?;

        // DRIVE_LAYOUT_INFORMATION_EX là struct có "mảng đuôi" (PartitionEntry),
        // nên phải tự cấp buffer đủ lớn cho nhiều partition rồi tự tính offset.
        const MAX_PARTITIONS: usize = 128;
        let buffer_size = size_of::<DRIVE_LAYOUT_INFORMATION_EX>()
            + MAX_PARTITIONS * size_of::<PARTITION_INFORMATION_EX>();
        let mut buffer = vec![0u8; buffer_size];

        let mut bytes_returned = 0u32;
        let ioctl_result = DeviceIoControl(
            handle,
            IOCTL_DISK_GET_DRIVE_LAYOUT_EX,
            None,
            0,
            Some(buffer.as_mut_ptr() as *mut _),
            buffer_size as u32,
            Some(&mut bytes_returned),
            None,
        );
        let _ = CloseHandle(handle);
        ioctl_result?;

        let layout = &*(buffer.as_ptr() as *const DRIVE_LAYOUT_INFORMATION_EX);
        let entry_ptr = layout.PartitionEntry.as_ptr();

        for i in 0..layout.PartitionCount as usize {
            let entry = &*entry_ptr.add(i);
            let is_boot = if entry.PartitionStyle == PARTITION_STYLE_MBR {
                entry.Anonymous.Mbr.BootIndicator
            } else if entry.PartitionStyle == PARTITION_STYLE_GPT {
                entry.Anonymous.Gpt.PartitionType == PARTITION_SYSTEM_GUID
            } else {
                false
            };

            if is_boot {
                return Ok(true);
            }
        }
    }

    Ok(false)
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

    let sector = read_first_sector(device_path)?;
    Ok(sector[510] == 0x55 && sector[511] == 0xAA)
}
