use core_storage::store::models::disks::Disks;
use sysinfo::Disks as SysDisk;

//cargo run -p core --example test
pub fn main() {
    let disks = SysDisk::new_with_refreshed_list();
    for disk in disks.list() {
        let mount_path = disk.mount_point();
        if disk.is_read_only() || disk.is_removable() || mount_path.as_os_str().is_empty() {
            continue;
        }
        //
        let disk = Disks::new(
            disk.name().to_string_lossy().into_owned(),
            mount_path,
            disk.total_space(),
            disk.available_space(),
            disk.kind().to_string(),
            disk.file_system().to_string_lossy().into_owned(),
        );
        println!("Name      : {:?}", disk);

        println!();
    }
}
