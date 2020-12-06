#![allow(non_upper_case_globals)]
const KiB: u64 = 1024;
const MiB: u64 = KiB * 1024;
const GiB: u64 = MiB * 1024;
const TiB: u64 = GiB * 1024;

/// Format a byte number as a human-readable string.
pub fn format_bytes(bytes: u64) -> String {
    if bytes >= TiB {
        format!("{:.2}TiB", (bytes as f64 / TiB as f64))
    } else if bytes >= GiB {
        format!("{:.2}GiB", (bytes as f64 / GiB as f64))
    } else if bytes >= MiB {
        format!("{:.2}MiB", (bytes as f64 / MiB as f64))
    } else if bytes >= KiB {
        format!("{:.2}KiB", (bytes as f64 / KiB as f64))
    } else {
        format!("{} bytes", bytes)
    }
}
