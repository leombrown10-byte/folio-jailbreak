//! Offline dictionary. data/dict.tsv: "word<TAB>definition" lines sorted by lowercase word; binary-searched on disk.
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

fn line_at(f: &mut File, pos: u64, len: u64) -> Option<(u64, String)> {
    // Return the first full line starting at or after `pos`.
    let mut start = pos;
    if pos > 0 {
        f.seek(SeekFrom::Start(pos - 1)).ok()?;
        let mut b = [0u8; 1];
        f.read_exact(&mut b).ok()?;
        if b[0] != b'\n' {
            // skip partial line
            let mut buf = [0u8; 1];
            start = pos;
            loop {
                if f.read(&mut buf).ok()? == 0 {
                    return None;
                }
                start += 1;
                if buf[0] == b'\n' {
                    break;
                }
            }
        }
    }
    if start >= len {
        return None;
    }
    f.seek(SeekFrom::Start(start)).ok()?;
    let mut s = Vec::new();
    let mut buf = [0u8; 1];
    while f.read(&mut buf).ok()? == 1 {
        if buf[0] == b'\n' {
            break;
        }
        s.push(buf[0]);
    }
    Some((start, String::from_utf8_lossy(&s).to_string()))
}

/// Modern WordNet first; Webster's 1913 (dict_old.tsv) covers archaic words old books use.
pub fn lookup(data: &Path, word: &str) -> Option<String> {
    lookup_in(data, "dict.tsv", word).or_else(|| lookup_in(data, "dict_old.tsv", word).map(|d| format!("(Webster, 1913)\n{}", d)))
}

fn lookup_in(data: &Path, file: &str, word: &str) -> Option<String> {
    let w = word.to_lowercase();
    let mut f = File::open(data.join(file)).ok()?;
    let len = f.metadata().ok()?.len();
    for cand in stems(&w) {
        let (mut lo, mut hi) = (0u64, len);
        while lo < hi {
            let mid = (lo + hi) / 2;
            match line_at(&mut f, mid, len) {
                Some((start, line)) => {
                    let (k, v) = line.split_once('\t').unwrap_or((&line, ""));
                    match k.cmp(cand.as_str()) {
                        std::cmp::Ordering::Equal => return Some(v.replace("\\n", "\n")),
                        std::cmp::Ordering::Less => lo = start + line.len() as u64 + 1,
                        std::cmp::Ordering::Greater => hi = mid,
                    }
                }
                None => hi = mid,
            }
        }
    }
    None
}

/// The word itself, then crude de-inflections.
fn stems(w: &str) -> Vec<String> {
    let mut v = vec![w.to_string()];
    for (suf, rep) in [("ies", "y"), ("ied", "y"), ("es", ""), ("s", ""), ("ed", ""), ("ed", "e"), ("ing", ""), ("ing", "e"), ("ly", ""), ("'s", "")] {
        if let Some(b) = w.strip_suffix(suf) {
            if b.len() > 2 {
                v.push(format!("{}{}", b, rep));
            }
        }
    }
    v
}
