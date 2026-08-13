use crate::{Context, Result};
use service::enumerate::{physical_disk_to_table, segment_to_table, storage_to_table};

pub fn handle(ctx: Context) -> Result {
    if ctx.has_option("disk") {
        return match physical_disk_to_table() {
            Ok(table) => Result::Table(table),
            Err(err) => Result::Error(err.to_string()),
        };
    }
    if ctx.has_option("segment") {
        return match segment_to_table(ctx.option("segment"), ctx.has_option("valid")) {
            Ok(table) => Result::Table(table),
            Err(err) => Result::Error(err.to_string()),
        };
    }
    match storage_to_table(ctx.has_option("valid")) {
        Ok(table) => Result::Table(table),
        Err(err) => Result::Error(err.to_string()),
    }
}
