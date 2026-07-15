use core_storage_platform::{disk::enumerate::*, error::BaseResult};

//cargo run -p core-storage-platform --example test
pub fn main() -> BaseResult<()> {
    let disks = enumerate_physical_disks()?;

    for d in disks {
        let a = has_directory(d.volume_paths);
        println!(">>{:?}", a);

    }
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
