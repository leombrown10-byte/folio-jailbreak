//! Book discovery and cover thumbnails (rendered by kreader, cached as small PGMs).
use crate::Ev;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;

pub const COVER_W: i32 = 300;
pub const COVER_H: i32 = 400;
pub const KREADER: &str = "/var/local/kos/bin/kreader";
const DIRS: [&str; 2] = ["/mnt/us/books", "/mnt/us/documents"];
pub const EXTS: [&str; 12] = ["epub", "pdf", "cbz", "fb2", "xps", "mobi", "azw", "azw3", "txt", "html", "htm", "djvu"];

#[derive(Clone)]
pub struct Item {
    pub path: String,
    pub title: String,
    pub ext: String,
    pub key: String,
}

pub fn fnv(s: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", h)
}

fn walk(dir: &Path, out: &mut Vec<Item>, depth: u32) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || name.ends_with(".sdr") {
            continue;
        }
        if p.is_dir() {
            if depth < 4 {
                walk(&p, out, depth + 1);
            }
            continue;
        }
        let ext = p.extension().map(|x| x.to_string_lossy().to_lowercase()).unwrap_or_default();
        if !EXTS.contains(&ext.as_str()) {
            continue;
        }
        let stem = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let path = p.to_string_lossy().to_string();
        out.push(Item { key: fnv(&path), title: stem.replace('_', " "), ext, path });
    }
}

pub fn scan() -> Vec<Item> {
    let mut v = vec![];
    for d in DIRS {
        walk(Path::new(d), &mut v, 0);
    }
    v.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    v
}

pub fn cover_path(data: &Path, key: &str) -> PathBuf {
    data.join("covers").join(format!("{}.pgm", key))
}

/// True when an image has (almost) no ink: a blank cover page that should be replaced by a typographic cover.
pub fn mostly_blank(px: &[u8]) -> bool {
    let ink = px.iter().filter(|&&p| p < 200).count();
    let mean = px.iter().map(|&p| p as u64).sum::<u64>() / px.len().max(1) as u64;
    ink * 200 < px.len() || mean >= 228 // a plain text page is not a cover
}

/// Parse a binary PGM (P5) into (w, h, pixels).
pub fn read_pgm(p: &Path) -> Option<(i32, i32, Vec<u8>)> {
    let b = std::fs::read(p).ok()?;
    let mut i = 0;
    let mut tok = vec![];
    while tok.len() < 4 && i < b.len() {
        while i < b.len() && b[i].is_ascii_whitespace() {
            i += 1;
        }
        let s = i;
        while i < b.len() && !b[i].is_ascii_whitespace() {
            i += 1;
        }
        tok.push(String::from_utf8_lossy(&b[s..i]).to_string());
    }
    i += 1;
    if tok.len() < 4 || tok[0] != "P5" {
        return None;
    }
    let (w, h): (i32, i32) = (tok[1].parse().ok()?, tok[2].parse().ok()?);
    if b.len() < i + (w * h) as usize {
        return None;
    }
    Some((w, h, b[i..i + (w * h) as usize].to_vec()))
}

/// Background worker: renders missing covers one by one and notifies the UI.
pub fn spawn_covers(items: Vec<Item>, data: PathBuf, tx: Sender<Ev>) {
    std::thread::spawn(move || {
        let Ok(mut child) = Command::new(KREADER).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn() else {
            return;
        };
        let mut sin = child.stdin.take().unwrap();
        let mut sout = BufReader::new(child.stdout.take().unwrap());
        let mut line = String::new();
        for it in items {
            let cp = cover_path(&data, &it.key);
            if cp.exists() {
                continue;
            }
            let path = match it.ext.as_str() {
                "epub" | "pdf" | "cbz" | "fb2" | "xps" | "html" | "htm" | "txt" => it.path.clone(),
                "mobi" | "azw" | "azw3" => match crate::convert::to_epub(&data, &it) {
                    Some(p) => p,
                    None => continue,
                },
                "djvu" => match crate::convert::djvu_to_pdf(&data, &it) {
                    Some(p) => p,
                    None => continue,
                },
                _ => continue,
            };
            let _ = writeln!(sin, "open 360 480 10 {}", path);
            line.clear();
            if sout.read_line(&mut line).is_err() || !line.starts_with("ok") {
                continue;
            }
            let tmp = cp.with_extension("tmp");
            let s = COVER_W as f32 / 360.0;
            let _ = writeln!(sin, "page 0 {} {} 0 0 {} {}", tmp.display(), s, COVER_W, COVER_H);
            line.clear();
            if sout.read_line(&mut line).is_ok() && line.starts_with("ok") {
                // many EPUBs start with a blank page; the real cover is the next one
                if read_pgm(&tmp).map_or(false, |(_, _, p)| mostly_blank(&p)) {
                    let _ = writeln!(sin, "page 1 {} {} 0 0 {} {}", tmp.display(), s, COVER_W, COVER_H);
                    line.clear();
                    let _ = sout.read_line(&mut line);
                }
                let _ = std::fs::rename(&tmp, &cp);
                let _ = tx.send(Ev::Cover);
            }
        }
        let _ = writeln!(sin, "quit");
    });
}
