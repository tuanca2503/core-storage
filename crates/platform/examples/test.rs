
use platform::{
    disk::enumerate, 
};
use model::{BaseError, BaseResult, ErrorCode};

//cargo run -p core-storage-platform --example test
pub fn main() -> BaseResult<()> {
    // let disks = enumerate::storage_info()?;
    // println!(">>>{:?}",disks);
    // let a = &disks[1];

    // let mut device = OpenOptions::new()
    //     .read(true)
    //     .write(false)
    //     .open(&a.device_path)?;
    // // format_disk(false,FormatMode::Quick,a)?;
    // let b = Header::load(&mut device)?;
    // let c = b.total_bytes == a.capacity_bytes;
    // println!("done {:?} > {}", b,c);
    
    // allowed_format(true, &disks[1])?;

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
