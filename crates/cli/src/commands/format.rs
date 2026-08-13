use service::format::format_disk;

use crate::{Context, Result};


/// --forced là command flag, chỉ format mới có, khai báo trong Command.flags.
pub fn handle(ctx: Context) -> Result {
    let name = ctx.prm(0).expect("Dispatcher đã đảm bảo đủ argument");
    
    let zero_mode = ctx.has_option("zero");
    let forced = ctx.has_option("forced");
    match format_disk(forced, zero_mode, name) {
        Ok(_) => Result::Text(format!("Formatted success")),
        Err(err) => Result::Error(err.to_string()),
    }
}
