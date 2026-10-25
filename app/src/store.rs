//! Persistent state: settings, per-book progress/highlights/bookmarks. One JSON file, saved on change.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Cal {
    pub swap: bool,
    pub flip_x: bool,
    pub flip_y: bool,
    pub max_x: i32,
    pub max_y: i32,
    /// v2: two-point affine calibration. Raw values (after the optional axis swap) read while tapping the
    /// two on-screen targets: first axis at P1/P2 x, second axis at P1/P2 y.
    pub v: u8,
    pub a1: i32,
    pub a2: i32,
    pub b1: i32,
    pub b2: i32,
    /// v3: least-squares affine fit from five targets: x = m0*rx + m1*ry + m2, y = m3*rx + m4*ry + m5.
    pub m: [f64; 6],
}

impl Default for Cal {
    fn default() -> Self {
        Cal { swap: false, flip_x: false, flip_y: false, max_x: 1071, max_y: 1447, v: 0, a1: 0, a2: 1, b1: 0, b2: 1, m: [0.0; 6] }
    }
}

/// Screen positions of the two calibration targets.
pub const CAL_P1: (i32, i32) = (90, 90);
pub const CAL_P2: (i32, i32) = (crate::gfx::W - 90, crate::gfx::H - 90);

/// The five calibration targets: four corners and the center.
pub const CAL_TARGETS: [(i32, i32); 5] = [
    (90, 90),
    (crate::gfx::W - 90, 90),
    (crate::gfx::W - 90, crate::gfx::H - 90),
    (90, crate::gfx::H - 90),
    (crate::gfx::W / 2, crate::gfx::H / 2),
];

/// Least-squares fit of target = c0*rx + c1*ry + c2 over the touched points.
pub fn fit_affine(pts: &[(f64, f64, f64)]) -> Option<[f64; 3]> {
    let (mut a, mut b) = ([[0.0f64; 3]; 3], [0.0f64; 3]);
    for &(rx, ry, t) in pts {
        let v = [rx, ry, 1.0];
        for i in 0..3 {
            for j in 0..3 {
                a[i][j] += v[i] * v[j];
            }
            b[i] += v[i] * t;
        }
    }
    // Gaussian elimination with partial pivoting
    for i in 0..3 {
        let p = (i..3).max_by(|&x, &y| a[x][i].abs().partial_cmp(&a[y][i].abs()).unwrap())?;
        if a[p][i].abs() < 1e-9 {
            return None;
        }
        a.swap(i, p);
        b.swap(i, p);
        for r in i + 1..3 {
            let f = a[r][i] / a[i][i];
            for c in i..3 {
                a[r][c] -= f * a[i][c];
            }
            b[r] -= f * b[i];
        }
    }
    let mut x = [0.0f64; 3];
    for i in (0..3).rev() {
        let mut s = b[i];
        for j in i + 1..3 {
            s -= a[i][j] * x[j];
        }
        x[i] = s / a[i][i];
    }
    Some(x)
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Settings {
    pub cal: Option<Cal>,
    pub em: f32,
    pub margin: f32,
    pub family: i32,   // 0 publisher, 1 Plex Serif, 2 Plex Sans
    pub spacing: i32,  // line-height * 100
    pub justify: bool,
    pub light: i32,
    pub serif_ui: bool,
    pub volume: i32,
    pub last_book: Option<String>,
    pub top: i32,           // px of blank space above the text
    pub bottom: i32,        // px below the text (includes the footer)
    pub dark: bool,         // inverted (white on black) reading
    pub flash_every: i32,   // full-screen flash every N page turns (0 = never)
    pub shuffle: bool,
    pub repeat: u8,         // 0 off, 1 all, 2 one
    pub sleep_screen: u8,   // 0 player/clock, 1 book cover, 2 highlight quote
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            cal: None, em: 11.0, margin: 18.0, family: 1, spacing: 135, justify: false, light: 8, serif_ui: false, volume: 70, last_book: None,
            top: 60, bottom: 100, dark: false, flash_every: 8, shuffle: false, repeat: 0, sleep_screen: 0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Highlight {
    pub text: String, // the words, space-joined; matched against the page after any re-layout
    pub frac: f32,    // rough position in the book (0..1)
    pub note: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Book {
    pub title: String,
    pub frac: f32,
    pub opened: u64,
    pub bookmarks: Vec<f32>,
    pub highlights: Vec<Highlight>,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Data {
    pub settings: Settings,
    pub books: HashMap<String, Book>,
    /// Reading log per day number (unix days): [pages turned, seconds read].
    pub stats: HashMap<u64, [u32; 2]>,
    /// Saved position (seconds) of long recordings (audiobooks), by file path.
    pub resume: HashMap<String, u32>,
}

pub struct Store {
    pub dir: PathBuf,
    pub d: Data,
}

impl Store {
    pub fn load() -> Store {
        let dir = PathBuf::from(std::env::var("KOS_DATA").unwrap_or_else(|_| "/var/local/kos/data".into()));
        let _ = std::fs::create_dir_all(dir.join("covers"));
        let _ = std::fs::create_dir_all(dir.join("notes"));
        let _ = std::fs::create_dir_all(dir.join("conv"));
        let d = std::fs::read(dir.join("state.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        Store { dir, d }
    }

    pub fn save(&self) {
        if let Ok(b) = serde_json::to_vec_pretty(&self.d) {
            let tmp = self.dir.join("state.json.tmp");
            if std::fs::write(&tmp, b).is_ok() {
                let _ = std::fs::rename(tmp, self.dir.join("state.json"));
            }
        }
    }

    /// Count one turned page that was read for `secs` seconds.
    pub fn log_page(&mut self, secs: u32) {
        let day = crate::sys::now() / 86400;
        let e = self.d.stats.entry(day).or_default();
        e[0] += 1;
        e[1] += secs;
    }

    /// Average seconds per page over the whole log (None until there is enough to trust).
    pub fn secs_per_page(&self) -> Option<f32> {
        let (p, s) = self.d.stats.values().fold((0u32, 0u32), |a, e| (a.0 + e[0], a.1 + e[1]));
        if p >= 20 { Some(s as f32 / p as f32) } else { None }
    }

    pub fn book(&mut self, key: &str) -> &mut Book {
        self.d.books.entry(key.to_string()).or_default()
    }
}
