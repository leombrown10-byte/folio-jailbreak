//! Panel geometry. Folio always draws on a fixed logical canvas (W x H = 1072 x 1448); on a Kindle whose panel
//! has another resolution, each update is scaled to the panel on its way out and touches are scaled back, so
//! the same app runs on every model. On the 1072x1448 models (Paperwhite 3/4, Voyage, ...) nothing is scaled.
use crate::gfx::{H, W};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// Physical panel size in the upright orientation.
pub fn size() -> (i32, i32) {
    static S: OnceLock<(i32, i32)> = OnceLock::new();
    *S.get_or_init(detect)
}

pub fn scaled() -> bool {
    size() != (W, H)
}

fn detect() -> (i32, i32) {
    // simulator: KOS_SIM_SIZE=600x800
    if let Ok(v) = std::env::var("KOS_SIM_SIZE") {
        if let Some((a, b)) = v.split_once('x') {
            if let (Ok(w), Ok(h)) = (a.trim().parse::<i32>(), b.trim().parse::<i32>()) {
                if sane(w, h) {
                    return (w, h);
                }
            }
        }
    }
    if crate::sim::enabled() {
        return (W, H);
    }
    from_fbink().unwrap_or((W, H))
}

fn sane(w: i32, h: i32) -> bool {
    (300..=4000).contains(&w) && (400..=5000).contains(&h) && h > w
}

/// `fbink -e` prints the screen state as shell variables (viewWidth=..;viewHeight=..).
fn from_fbink() -> Option<(i32, i32)> {
    let out = Command::new(crate::gfx::FBINK).arg("-e").stdout(Stdio::piped()).stderr(Stdio::null()).output().ok()?;
    let s = String::from_utf8_lossy(&out.stdout).to_string();
    let get = |key: &str| -> Option<i32> {
        let i = s.find(&format!("{}=", key))? + key.len() + 1;
        s[i..].trim_start_matches(['\'', '"']).chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse().ok()
    };
    let (w, h) = (get("viewWidth")?, get("viewHeight")?);
    // a panel within a few pixels of the logical canvas (e.g. a padded stride) is the canvas: never scale those
    if (w - W).abs() <= 16 && (h - H).abs() <= 16 {
        return Some((W, H));
    }
    if sane(w, h) { Some((w, h)) } else { None }
}

/// Physical pixel -> logical pixel (touch input from the simulator).
pub fn to_logical(x: i32, y: i32) -> (i32, i32) {
    let (pw, ph) = size();
    ((x * W / pw).clamp(0, W - 1), (y * H / ph).clamp(0, H - 1))
}

/// A scaled piece of the canvas, in physical pixel coordinates.
pub struct Piece {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub px: Vec<u8>,
}

/// Scale the logical region `[x0,x1) x [y0,y1)` of the canvas to the panel. Upscaling is bilinear, downscaling
/// averages the covered source pixels. The result is aligned to even offsets and sizes (FBInk likes that).
pub fn scale_region(buf: &[u8], x0: i32, y0: i32, x1: i32, y1: i32) -> Piece {
    let (pw, ph) = size();
    let px0 = ((x0 * pw / W) & !1).max(0);
    let py0 = ((y0 * ph / H) & !1).max(0);
    let px1 = (((x1 * pw + W - 1) / W + 1) & !1).min(pw);
    let py1 = (((y1 * ph + H - 1) / H + 1) & !1).min(ph);
    let (w, h) = (px1 - px0, py1 - py0);
    let (sx, sy) = (W as f32 / pw as f32, H as f32 / ph as f32);
    let mut px = Vec::with_capacity((w * h) as usize);
    // source column / row ranges per output pixel, computed once
    let span = |o: i32, s: f32, max: i32| -> (i32, i32, f32) {
        if s <= 1.0 {
            // upscale: interpolate between the two nearest source pixels
            let c = ((o as f32 + 0.5) * s - 0.5).clamp(0.0, (max - 1) as f32);
            let i = c.floor() as i32;
            (i, (i + 1).min(max - 1), c - i as f32)
        } else {
            let a = (o as f32 * s).floor() as i32;
            let b = (((o + 1) as f32 * s).ceil() as i32).min(max).max(a + 1);
            (a, b, -1.0)
        }
    };
    let cols: Vec<(i32, i32, f32)> = (px0..px1).map(|o| span(o, sx, W)).collect();
    for oy in py0..py1 {
        let (ya, yb, yf) = span(oy, sy, H);
        for &(xa, xb, xf) in &cols {
            let v = if xf >= 0.0 && yf >= 0.0 {
                let g = |x: i32, y: i32| buf[(y * W + x) as usize] as f32;
                let top = g(xa, ya) * (1.0 - xf) + g(xb, ya) * xf;
                let bot = g(xa, yb) * (1.0 - xf) + g(xb, yb) * xf;
                top * (1.0 - yf) + bot * yf
            } else {
                // box average over the source rectangle (when only one axis downscales, the other axis
                // degenerates to a 1-pixel range, which is exactly the bilinear case's nearest pixel)
                let (xa, xb) = if xf >= 0.0 { (xa, xa + 1) } else { (xa, xb) };
                let (ya2, yb2) = if yf >= 0.0 { (ya, ya + 1) } else { (ya, yb) };
                let mut sum = 0u32;
                for y in ya2..yb2 {
                    let o = (y * W) as usize;
                    for &p in &buf[o + xa as usize..o + xb as usize] {
                        sum += p as u32;
                    }
                }
                sum as f32 / ((xb - xa) * (yb2 - ya2)) as f32
            };
            px.push(v.round().clamp(0.0, 255.0) as u8);
        }
    }
    Piece { x: px0, y: py0, w, h, px }
}
