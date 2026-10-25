//! Where things live. On a Kindle the root is "/"; the simulator (folio-sim) sets KOS_ROOT to a scratch directory
//! so the app can run on a computer without touching it.
use std::sync::OnceLock;

fn root() -> &'static str {
    static R: OnceLock<String> = OnceLock::new();
    R.get_or_init(|| std::env::var("KOS_ROOT").unwrap_or_default().trim_end_matches('/').to_string())
}

/// `p("/mnt/us/books")` -> "/mnt/us/books" on the device, "$KOS_ROOT/mnt/us/books" in the simulator.
pub fn p(path: &str) -> String {
    format!("{}{}", root(), path)
}

pub fn kreader() -> String {
    p("/var/local/kos/bin/kreader")
}
