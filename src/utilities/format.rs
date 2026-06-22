pub(crate) fn format_bytes(bytes: u64) -> String {
    format!("{:.1} MB", bytes as f64 / 1024.0 / 1024.0)
}

pub(crate) fn format_percent(done: usize, total: usize) -> String {
    if total == 0 {
        return "unknown".to_owned();
    }
    format!("{:.1}%", done as f64 * 100.0 / total as f64)
}
