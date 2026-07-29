use chrono::DateTime;

pub const KB: u64 = 1024;
pub const MB: u64 = 1024 * KB;
pub const GB: u64 = 1024 * MB;
pub const TB: u64 = 1024 * GB;

pub fn format_size(bytes: u64) -> String {
    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

pub fn format_date(ms: u64) -> String {
    let default = "--/--/----".to_string();
    if ms == 0 {
        return default;
    }

    DateTime::from_timestamp_millis(ms as i64)
        .map(|dt| dt.format("%d/%m/%Y").to_string())
        .unwrap_or_else(|| default)
}
