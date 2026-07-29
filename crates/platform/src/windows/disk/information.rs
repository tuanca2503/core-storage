use std::ffi::CStr;

use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_GENERIC_READ, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING, PARTITION_SYSTEM_GUID, ReadFile,
};
use windows::Win32::System::IO::DeviceIoControl;
use windows::Win32::System::Ioctl::{
    DRIVE_LAYOUT_INFORMATION_EX, GET_LENGTH_INFORMATION, IOCTL_DISK_GET_DRIVE_LAYOUT_EX, IOCTL_DISK_GET_LENGTH_INFO, IOCTL_DISK_IS_WRITABLE, IOCTL_STORAGE_GET_DEVICE_NUMBER, IOCTL_STORAGE_QUERY_PROPERTY, PARTITION_INFORMATION_EX, PARTITION_STYLE_GPT, PARTITION_STYLE_MBR, PropertyStandardQuery, STORAGE_ACCESS_ALIGNMENT_DESCRIPTOR, STORAGE_DEVICE_DESCRIPTOR, STORAGE_DEVICE_NUMBER, STORAGE_PROPERTY_QUERY, StorageAccessAlignmentProperty, StorageDeviceProperty,
};
use windows::core::{PCWSTR, Result};

use crate::disk::PhysicalDisk;
pub struct DiskHandle(HANDLE);

impl DiskHandle {
    pub fn open(device_path: &str) -> Result<Self> {
        let wide_path: Vec<u16> = device_path
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let handle = unsafe {
            CreateFileW(
                PCWSTR(wide_path.as_ptr()),
                FILE_GENERIC_READ.0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            )
        }?;
        Ok(Self(handle))
    }
    pub fn to_physical_disk(&self) -> Result<PhysicalDisk> {
        let (model, serial, bus_type, removable) = self.descriptor()?;
        let (logical_sector_size, physical_sector_size) = self.sector_size()?;
        let disk_number = self.disk_number()?;
        Ok(PhysicalDisk {
            model,
            serial,
            bus_type,
            removable,
            disk_number,
            logical_sector_size,
            physical_sector_size,
            read_only: self.read_only()?,
            device_path: self.device_path(disk_number)?,
            capacity_bytes: self.capacity_bytes()?,
        })
    }
    pub fn disk_number(&self) -> Result<u32> {
        let mut bytes_returned = 0u32;
        let mut device_number = STORAGE_DEVICE_NUMBER::default();
        unsafe {
            (DeviceIoControl(
                self.0,
                IOCTL_STORAGE_GET_DEVICE_NUMBER,
                None,
                0,
                Some(&mut device_number as *mut _ as *mut _),
                size_of::<STORAGE_DEVICE_NUMBER>() as u32,
                Some(&mut bytes_returned),
                None,
            ))?;
            Ok(device_number.DeviceNumber)
        }
    }
    pub fn read_first_sector(&self) -> Result<[u8; 512]> {
        let mut sector = [0u8; 512];
        let mut bytes_read = 0u32;
        unsafe {
            ReadFile(self.0, Some(&mut sector), Some(&mut bytes_read), None)?;
            Ok(sector)
        }
    }
    pub fn is_boot_disk(&self) -> Result<bool> {
        unsafe {
            // DRIVE_LAYOUT_INFORMATION_EX là struct có "mảng đuôi" (PartitionEntry),
            // nên phải tự cấp buffer đủ lớn cho nhiều partition rồi tự tính offset.
            const MAX_PARTITIONS: usize = 128;
            let buffer_size = size_of::<DRIVE_LAYOUT_INFORMATION_EX>()
                + MAX_PARTITIONS * size_of::<PARTITION_INFORMATION_EX>();
            let mut buffer = vec![0u8; buffer_size];

            let mut bytes_returned = 0u32;
            let ioctl_result = DeviceIoControl(
                self.0,
                IOCTL_DISK_GET_DRIVE_LAYOUT_EX,
                None,
                0,
                Some(buffer.as_mut_ptr() as *mut _),
                buffer_size as u32,
                Some(&mut bytes_returned),
                None,
            );
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
    //
    fn descriptor(&self) -> Result<(String, String, i32, bool)> {
        let mut bytes_returned = 0u32;
        let mut buffer = vec![0u8; 1024];
        let query = STORAGE_PROPERTY_QUERY {
            PropertyId: StorageDeviceProperty,
            QueryType: PropertyStandardQuery,
            ..Default::default()
        };
        unsafe {
            (DeviceIoControl(
                self.0,
                IOCTL_STORAGE_QUERY_PROPERTY,
                Some(&query as *const _ as *const _),
                size_of::<STORAGE_PROPERTY_QUERY>() as u32,
                Some(buffer.as_mut_ptr() as *mut _),
                buffer.len() as u32,
                Some(&mut bytes_returned),
                None,
            ))?;
        }
        let descriptor = unsafe { &*(buffer.as_ptr() as *const STORAGE_DEVICE_DESCRIPTOR) };
        let model = self.read_string(&buffer, descriptor.ProductIdOffset);
        let serial = self.read_string(&buffer, descriptor.SerialNumberOffset);
        let bus_type = descriptor.BusType.0;
        let removable = descriptor.RemovableMedia;
        Ok((model, serial, bus_type, removable))
    }
    fn capacity_bytes(&self) -> Result<u64> {
        let mut bytes_returned = 0u32;
        let mut length_info = GET_LENGTH_INFORMATION::default();
        unsafe {
            (DeviceIoControl(
                self.0,
                IOCTL_DISK_GET_LENGTH_INFO,
                None,
                0,
                Some(&mut length_info as *mut _ as *mut _),
                size_of::<GET_LENGTH_INFORMATION>() as u32,
                Some(&mut bytes_returned),
                None,
            ))?;
        }
        Ok(length_info.Length as u64)
    }
    fn device_path(&self, disk_number: u32) -> Result<String> {
        Ok(format!(r"\\.\PhysicalDrive{disk_number}"))
    }
    fn read_only(&self) -> Result<bool> {
        let mut bytes_returned = 0u32;
        let result = unsafe {
            DeviceIoControl(
                self.0,
                IOCTL_DISK_IS_WRITABLE,
                None,
                0,
                None,
                0,
                Some(&mut bytes_returned),
                None,
            )
        };
        Ok(result.is_err())
    }
    fn sector_size(&self) -> Result<(u32, u32)> {
        let mut query = STORAGE_PROPERTY_QUERY {
            PropertyId: StorageAccessAlignmentProperty,
            QueryType: PropertyStandardQuery,
            AdditionalParameters: [0],
        };
        let mut alignment = STORAGE_ACCESS_ALIGNMENT_DESCRIPTOR::default();
        let mut bytes_returned = 0u32;
        unsafe {
            DeviceIoControl(
                self.0,
                IOCTL_STORAGE_QUERY_PROPERTY,
                Some(&mut query as *mut _ as *mut _),
                size_of::<STORAGE_PROPERTY_QUERY>() as u32,
                Some(&mut alignment as *mut _ as *mut _),
                size_of::<STORAGE_ACCESS_ALIGNMENT_DESCRIPTOR>() as u32,
                Some(&mut bytes_returned),
                None,
            )?;
        }

        Ok((
            alignment.BytesPerLogicalSector,
            alignment.BytesPerPhysicalSector,
        ))
    }
    //
    fn read_string(&self, buffer: &[u8], offset: u32) -> String {
        if offset == 0 {
            return String::new();
        }

        let ptr = unsafe { buffer.as_ptr().add(offset as usize) };

        let value = unsafe { CStr::from_ptr(ptr as *const i8) }
            .to_string_lossy()
            .trim()
            .to_string();

        value
    }
    //
}

impl Drop for DiskHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}
