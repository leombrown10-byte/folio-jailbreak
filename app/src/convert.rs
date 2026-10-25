//! Format bridges: MOBI/AZW3 -> EPUB (libmobi's mobitool), DjVu -> PDF (djvulibre's ddjvu). Results are cached.
use crate::library::Item;
use std::path::Path;
use std::process::{Command, Stdio};

const MOBITOOL: &str = "/var/local/kos/bin/mobitool";
const DDJVU: &str = "/var/local/kos/bin/ddjvu";

pub fn to_epub(data: &Path, it: &Item) -> Option<String> {
    let out = data.join("conv").join(format!("{}.epub", it.key));
    if out.exists() {
        return Some(out.to_string_lossy().to_string());
    }
    let tmp = data.join("conv").join(format!("{}_work", it.key));
    let _ = std::fs::create_dir_all(&tmp);
    let ok = Command::new(MOBITOOL).arg("-e").arg("-o").arg(&tmp).arg(&it.path).stdout(Stdio::null()).stderr(Stdio::null()).status().map(|s| s.success()).unwrap_or(false);
    let mut found = None;
    if ok {
        if let Ok(rd) = std::fs::read_dir(&tmp) {
            for e in rd.flatten() {
                if e.path().extension().map_or(false, |x| x == "epub") {
                    found = Some(e.path());
                }
            }
        }
    }
    let r = found.and_then(|f| std::fs::rename(f, &out).ok().map(|_| out.to_string_lossy().to_string()));
    let _ = std::fs::remove_dir_all(&tmp);
    r
}

pub fn djvu_to_pdf(data: &Path, it: &Item) -> Option<String> {
    let out = data.join("conv").join(format!("{}.pdf", it.key));
    if out.exists() {
        return Some(out.to_string_lossy().to_string());
    }
    let ok = Command::new(DDJVU).arg("-format=pdf").arg(&it.path).arg(&out).stdout(Stdio::null()).stderr(Stdio::null()).status().map(|s| s.success()).unwrap_or(false);
    if ok && out.exists() { Some(out.to_string_lossy().to_string()) } else { None }
}
