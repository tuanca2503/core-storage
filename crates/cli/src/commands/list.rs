use crate::cli::context::CommandContext;
use crate::cli::result::CommandResult;
use platform::disk::{physical_disk_info, segment_to_table, storage_to_table};

pub fn list(ctx: CommandContext) -> CommandResult {
    let name = ctx.prm(0).unwrap_or_default();

    match () {
        _ if ctx.has_option("disk") => match physical_disk_info() {
            Ok(table) => CommandResult::Table(table),
            Err(err) => CommandResult::Error(err.to_string()),
        },

        _ if ctx.has_option("segment") => match segment_to_table(name) {
            Ok(table) => CommandResult::Table(table),
            Err(err) => CommandResult::Error(err.to_string()),
        },

        _ => match storage_to_table(ctx.has_option("valid")) {
            Ok(table) => CommandResult::Table(table),
            Err(err) => CommandResult::Error(err.to_string()),
        },
    }
}
