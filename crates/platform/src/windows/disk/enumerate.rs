use std::mem::size_of;
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    DIGCF_DEVICEINTERFACE, DIGCF_PRESENT, SP_DEVICE_INTERFACE_DATA,
    SP_DEVICE_INTERFACE_DETAIL_DATA_W, SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInterfaces,
    SetupDiGetClassDevsW, SetupDiGetDeviceInterfaceDetailW,
};
use windows::core::{GUID, PCWSTR, Result};

use crate::disk::PhysicalDisk;
use crate::disk::information::DiskHandle;
const GUID_DEVINTERFACE_DISK: GUID = GUID::from_u128(0x53F56307_B6BF_11D0_94F2_00A0C91EFB8B);

/*
disk::enumerate()
disk::find(id)
disk::exists(id)
disk::refresh()
*/
fn read_device_path(detail_ptr: *mut SP_DEVICE_INTERFACE_DETAIL_DATA_W) -> String {
    unsafe {
        let wide_ptr = (*detail_ptr).DevicePath.as_ptr();
        let mut len = 0usize;
        while *wide_ptr.add(len) != 0 {
            len += 1;
        }
        let slice = std::slice::from_raw_parts(wide_ptr, len);
        String::from_utf16_lossy(slice)
    }
}
//
pub fn enumerate_physical_disks() -> Result<Vec<PhysicalDisk>> {
    let mut disks = Vec::new();

    unsafe {
        // 1) Lấy device-info-set chứa mọi device interface thuộc lớp "Disk" đang PRESENT.
        let device_info_set = SetupDiGetClassDevsW(
            Some(&GUID_DEVINTERFACE_DISK),
            PCWSTR::null(),
            None,
            DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
        )?;

        let mut index = 0u32;
        loop {
            let mut interface_data = SP_DEVICE_INTERFACE_DATA {
                cbSize: size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
                ..Default::default()
            };

            // Hết danh sách -> SetupDiEnumDeviceInterfaces trả lỗi (ERROR_NO_MORE_ITEMS) -> dừng.
            if SetupDiEnumDeviceInterfaces(
                device_info_set,
                None,
                &GUID_DEVINTERFACE_DISK,
                index,
                &mut interface_data,
            )
            .is_err()
            {
                break;
            }

            // 2) Gọi lần 1 với buffer rỗng để Windows cho biết cần bao nhiêu byte.
            let mut required_size: u32 = 0;
            let _ = SetupDiGetDeviceInterfaceDetailW(
                device_info_set,
                &interface_data,
                None,
                0,
                Some(&mut required_size),
                None,
            );

            // 3) Cấp đúng buffer cần thiết rồi gọi lại lấy device path thật.
            let mut buffer = vec![0u8; required_size as usize];
            let detail_ptr = buffer.as_mut_ptr() as *mut SP_DEVICE_INTERFACE_DETAIL_DATA_W;
            // Lưu ý: cbSize phải tính theo kích thước struct CỐ ĐỊNH (không tính phần
            // path biến đổi độ dài) — đây là quy ước bắt buộc của chính API này.
            (*detail_ptr).cbSize = size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32;

            if SetupDiGetDeviceInterfaceDetailW(
                device_info_set,
                &interface_data,
                Some(detail_ptr),
                required_size,
                None,
                None,
            )
            .is_ok()
            {
                let device_path = read_device_path(detail_ptr);
                if let Ok(info) = DiskHandle::open(&device_path) {
                    disks.push(info.to_physical_disk()?);
                }
            }

            index += 1;
        }

        let _ = SetupDiDestroyDeviceInfoList(device_info_set);
    }

    Ok(disks)
}
