pub mod store;
pub mod error;


pub fn hello() {
    let disks = Disks::new_with_refreshed_list();
    for disk in disks.list() {
        println!("Name      : {:?}", disk.name());
        println!("Mount     : {:?}", disk.mount_point());
        println!("Total     : {}", disk.total_space());
        println!("Available : {}", disk.available_space());
        println!("Kind      : {:?}", disk.kind());
        println!("FS        : {:?}", disk.file_system());
        println!();
    }
    println!("Hello, world!");
}
