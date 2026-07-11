use sysinfo::Disks;

//cargo run -p core --example test
pub fn main() {
    let disks = Disks::new_with_refreshed_list();
    for disk in disks.list() {
        println!("Name      : {:?}", disk.name());
        println!("Mount     : {:?}", disk.mount_point());
        println!("Total     : {}", disk.total_space());
        println!("Available : {}", disk.available_space());
        println!("Kind      : {:?}", disk.kind());
        println!("FS        : {:?}", disk.file_system());
        println!("is_removable        : {:?}", disk.is_removable());
        println!("is_read_only        : {:?}", disk.is_read_only());


        println!();
    }
}
