use platform::format::{format_disk, FormatMode};

use crate::cli::context::CommandContext;
use crate::cli::result::CommandResult;

/// --forced là command flag, chỉ format mới có, khai báo trong Command.flags.
pub fn format(ctx: CommandContext) -> CommandResult {
    let name = ctx.prm(0).expect("Dispatcher đã đảm bảo đủ argument");
    let mode = ctx.prm(1).unwrap_or_default();
    let forced = ctx.has_option("forced");
    match format_disk(forced, name, FormatMode::from_str(&mode)) {
        Ok(_) => CommandResult::Text(format!("Formatted success")),
        Err(err) => CommandResult::Error(err.to_string()),
    }
}
