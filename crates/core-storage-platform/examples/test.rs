use core_storage_platform::{
    disk::{allowed_format,enumerate_physical_disks},
    error::BaseResult,
};

//cargo run -p core-storage-platform --example test
pub fn main() -> BaseResult<()> {
    let disks = enumerate_physical_disks()?;
    allowed_format(true, &disks[1])?;

    // println!("== Danh sách ổ đĩa vật lý (SetupAPI) ==");
    // if disks.is_empty() {
    //     println!("  (Không thấy ổ nào — chạy lại với quyền Administrator.)");
    // }
    // for d in &disks {
    //     let gb = d.capacity_bytes as f64 / 1_073_741_824.0;
    //     println!("  | {:>9.2} GB ", gb);
    // }
    Ok(())
}
