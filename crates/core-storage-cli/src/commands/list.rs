use crate::cli::context::CommandContext;
use crate::cli::result::CommandResult;
use core_storage_platform::disk::{DiskInfo, enumerate};

pub fn list(ctx: CommandContext) -> CommandResult {
    if ctx.has_option("disk") {
        match enumerate::enumerate_disk_info() {
            Ok(disks) => CommandResult::Table(DiskInfo::to_table(disks)),
            Err(err) => CommandResult::Error(err.to_string()),
        }
    } else {
        match enumerate::enumerate_storage_info_to_table() {
            Ok(table) => CommandResult::Table(table),
            Err(err) => CommandResult::Error(err.to_string()),
        }
    }
}
