//! Software canvas (8-bit gray, 1072x1448) + presenting regions to the e-ink panel through FBInk.
use fontdue::{Font, FontSettings, Metrics};
use std::collections::HashMap;
use std::io::Write;
use std::process::{Command, Stdio};

pub const W: i32 = 1072;
pub const H: i32 = 1448;
pub const BLACK: u8 = 0;
pub const DARK: u8 = 70;
pub const MID: u8 = 140;
pub const LIGHT: u8 = 214;
pub const PALE: u8 = 238;
pub const WHITE: u8 = 255;

pub const FBINK: &str = "/var/local/kos/bin/fbink";
const TMP_IMG: &str = "/tmp/kos_present.pgm";

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum F {
    Sans,
    SansM,
    SansB,
    Serif,
    SerifI,
    SerifB,
}

#[derive(Copy, Clone, Debug)]
pub struct Rect {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x0: x, y0: y, x1: x + w, y1: y + h }
    }
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x0 && x < self.x1 && y >= self.y0 && y < self.y1
    }
    pub fn w(&self) -> i32 {
        self.x1 - self.x0
    }
    pub fn h(&self) -> i32 {
        self.y1 - self.y0
    }
    fn union(&self, o: &Rect) -> Rect {
        Rect { x0: self.x0.min(o.x0), y0: self.y0.min(o.y0), x1: self.x1.max(o.x1), y1: self.y1.max(o.y1) }
    }
}

/// E-ink waveform choice, passed to FBInk.
#[derive(Copy, Clone, PartialEq)]
pub enum Wf {
    Auto,
    Fast, // DU: black/white only, ~120ms, for taps and handwriting
    Text, // GL16: clean text, low ghosting
    Full, // GC16: full grayscale
}

struct Fonts {
    f: Vec<Font>,
    cache: HashMap<(F, char, u32), (Metrics, Vec<u8>)>,
}

impl Fonts {
    fn new() -> Fonts {
        let load = |b: &[u8]| Font::from_bytes(b, FontSettings::default()).expect("font");
        Fonts {
            f: vec![
                load(include_bytes!("../assets/IBMPlexSans-Regular.ttf")),
                load(include_bytes!("../assets/IBMPlexSans-Medium.ttf")),
                load(include_bytes!("../assets/IBMPlexSans-Bold.ttf")),
                load(include_bytes!("../assets/IBMPlexSerif-Regular.ttf")),
                load(include_bytes!("../assets/IBMPlexSerif-Italic.ttf")),
                load(include_bytes!("../assets/IBMPlexSerif-Bold.ttf")),
            ],
            cache: HashMap::new(),
        }
    }
    fn glyph(&mut self, f: F, c: char, px: f32) -> &(Metrics, Vec<u8>) {
        let font = &self.f[f as usize];
        self.cache.entry((f, c, px as u32)).or_insert_with(|| font.rasterize(c, px))
    }
}

pub struct Canvas {
    pub buf: Vec<u8>,
    fonts: Fonts,
    dirty: Option<Rect>,
    pub flips: u32,
}

impl Canvas {
    pub fn new() -> Canvas {
        Canvas { buf: vec![WHITE; (W * H) as usize], fonts: Fonts::new(), dirty: None, flips: 0 }
    }

    fn touch(&mut self, r: Rect) {
        let r = Rect { x0: r.x0.max(0), y0: r.y0.max(0), x1: r.x1.min(W), y1: r.y1.min(H) };
        if r.x1 <= r.x0 || r.y1 <= r.y0 {
            return;
        }
        self.dirty = Some(match self.dirty {
            Some(d) => d.union(&r),
            None => r,
        });
    }

    pub fn clear(&mut self, c: u8) {
        self.buf.fill(c);
        self.touch(Rect::new(0, 0, W, H));
    }

    pub fn fill(&mut self, x: i32, y: i32, w: i32, h: i32, c: u8) {
        let (x0, y0, x1, y1) = (x.max(0), y.max(0), (x + w).min(W), (y + h).min(H));
        let x1 = x1.max(x0);
        for yy in y0..y1 {
            let o = (yy * W) as usize;
            self.buf[o + x0 as usize..o + x1 as usize].fill(c);
        }
        self.touch(Rect::new(x, y, w, h));
    }

    pub fn frame(&mut self, x: i32, y: i32, w: i32, h: i32, t: i32, c: u8) {
        self.fill(x, y, w, t, c);
        self.fill(x, y + h - t, w, t, c);
        self.fill(x, y, t, h, c);
        self.fill(x + w - t, y, t, h, c);
    }

    fn px(&mut self, x: i32, y: i32, c: u8) {
        if x >= 0 && y >= 0 && x < W && y < H {
            self.buf[(y * W + x) as usize] = c;
        }
    }

    /// Filled rounded rectangle; `ring` > 0 draws only an outline of that thickness.
    pub fn rrect(&mut self, x: i32, y: i32, w: i32, h: i32, r: i32, c: u8, ring: i32) {
        let r = r.min(w / 2).min(h / 2);
        for yy in y.max(0)..(y + h).min(H) {
            for xx in x.max(0)..(x + w).min(W) {
                let (dx, dy) = (
                    if xx < x + r { x + r - xx } else if xx >= x + w - r { xx - (x + w - r - 1) } else { 0 },
                    if yy < y + r { y + r - yy } else if yy >= y + h - r { yy - (y + h - r - 1) } else { 0 },
                );
                let inside = |rad: i32| dx * dx + dy * dy <= rad * rad;
                let out = inside(r);
                if !out {
                    continue;
                }
                if ring > 0 {
                    let edge_x = xx - x < ring || x + w - 1 - xx < ring;
                    let edge_y = yy - y < ring || y + h - 1 - yy < ring;
                    let in_corner = dx > 0 && dy > 0;
                    let hollow = if in_corner { inside(r - ring) } else { !(edge_x || edge_y) };
                    if hollow {
                        continue;
                    }
                }
                self.buf[(yy * W + xx) as usize] = c;
            }
        }
        self.touch(Rect::new(x, y, w, h));
    }

    pub fn disc(&mut self, cx: i32, cy: i32, r: i32, c: u8) {
        self.rrect(cx - r, cy - r, 2 * r, 2 * r, r, c, 0);
    }

    /// Thick line (square brush); used by handwriting and icons.
    pub fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, t: i32, c: u8) {
        let (dx, dy) = ((x1 - x0).abs(), -(y1 - y0).abs());
        let (sx, sy) = (if x0 < x1 { 1 } else { -1 }, if y0 < y1 { 1 } else { -1 });
        let (mut x, mut y, mut err) = (x0, y0, dx + dy);
        let h = t / 2;
        loop {
            for oy in -h..=(t - 1 - h) {
                for ox in -h..=(t - 1 - h) {
                    self.px(x + ox, y + oy, c);
                }
            }
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
        self.touch(Rect { x0: x0.min(x1) - t, y0: y0.min(y1) - t, x1: x0.max(x1) + t + 1, y1: y0.max(y1) + t + 1 });
    }

    pub fn text_w(&mut self, f: F, px: f32, s: &str) -> i32 {
        s.chars().map(|c| self.fonts.glyph(f, c, px).0.advance_width).sum::<f32>().round() as i32
    }

    /// Draw text with its top at `y` (ascender line). Returns the advance width.
    pub fn text(&mut self, f: F, px: f32, x: i32, y: i32, s: &str, c: u8) -> i32 {
        let ascent = self.fonts.f[f as usize].horizontal_line_metrics(px).map(|m| m.ascent).unwrap_or(px * 0.8);
        let base = y + ascent.round() as i32;
        let mut pen = x as f32;
        for ch in s.chars() {
            let (m, bm) = {
                let g = self.fonts.glyph(f, ch, px);
                (g.0, g.1.clone())
            };
            let gx = pen.round() as i32 + m.xmin;
            let gy = base - m.height as i32 - m.ymin;
            for row in 0..m.height {
                for col in 0..m.width {
                    let a = bm[row * m.width + col] as u32;
                    if a == 0 {
                        continue;
                    }
                    let (px_, py_) = (gx + col as i32, gy + row as i32);
                    if px_ >= 0 && py_ >= 0 && px_ < W && py_ < H {
                        let o = (py_ * W + px_) as usize;
                        let bg = self.buf[o] as u32;
                        self.buf[o] = ((bg * (255 - a) + c as u32 * a) / 255) as u8;
                    }
                }
            }
            pen += m.advance_width;
        }
        let w = (pen - x as f32).round() as i32;
        self.touch(Rect::new(x, y, w + 2, (px * 1.4) as i32));
        w
    }

    pub fn text_c(&mut self, f: F, px: f32, cx: i32, y: i32, s: &str, c: u8) {
        let w = self.text_w(f, px, s);
        self.text(f, px, cx - w / 2, y, s, c);
    }

    pub fn text_r(&mut self, f: F, px: f32, rx: i32, y: i32, s: &str, c: u8) {
        let w = self.text_w(f, px, s);
        self.text(f, px, rx - w, y, s, c);
    }

    /// Truncate with an ellipsis so the text fits `maxw`.
    pub fn fit(&mut self, f: F, px: f32, s: &str, maxw: i32) -> String {
        if self.text_w(f, px, s) <= maxw {
            return s.to_string();
        }
        let mut t: String = s.to_string();
        while !t.is_empty() && self.text_w(f, px, &format!("{}…", t)) > maxw {
            t.pop();
        }
        format!("{}…", t.trim_end())
    }

    /// Greedy word wrap into at most `max_lines` lines.
    pub fn wrap(&mut self, f: F, px: f32, s: &str, maxw: i32, max_lines: usize) -> Vec<String> {
        let mut lines: Vec<String> = vec![];
        let mut cur = String::new();
        for w in s.split_whitespace() {
            let t = if cur.is_empty() { w.to_string() } else { format!("{} {}", cur, w) };
            if self.text_w(f, px, &t) <= maxw || cur.is_empty() {
                cur = t;
            } else {
                lines.push(std::mem::take(&mut cur));
                cur = w.to_string();
            }
        }
        if !cur.is_empty() {
            lines.push(cur);
        }
        if lines.len() > max_lines {
            lines.truncate(max_lines);
            let last = lines.pop().unwrap();
            lines.push(self.fit(f, px, &format!("{} …", last), maxw));
        }
        lines
    }

    /// Copy an 8-bit gray image onto the canvas.
    pub fn blit(&mut self, x: i32, y: i32, w: i32, h: i32, data: &[u8]) {
        for row in 0..h {
            let yy = y + row;
            if yy < 0 || yy >= H {
                continue;
            }
            for col in 0..w {
                let xx = x + col;
                if xx >= 0 && xx < W {
                    self.buf[(yy * W + xx) as usize] = data[(row * w + col) as usize];
                }
            }
        }
        self.touch(Rect::new(x, y, w, h));
    }

    /// Nearest-neighbour scaled blit of an 8-bit image into a dw x dh box.
    pub fn blit_scaled(&mut self, x: i32, y: i32, dw: i32, dh: i32, sw: i32, sh: i32, data: &[u8]) {
        for row in 0..dh {
            let yy = y + row;
            if yy < 0 || yy >= H {
                continue;
            }
            let sy = (row * sh / dh).min(sh - 1);
            for col in 0..dw {
                let xx = x + col;
                if xx >= 0 && xx < W {
                    let sx = (col * sw / dw).min(sw - 1);
                    self.buf[(yy * W + xx) as usize] = data[(sy * sw + sx) as usize];
                }
            }
        }
        self.touch(Rect::new(x, y, dw, dh));
    }

    pub fn tint(&mut self, r: Rect, max: u8) {
        for y in r.y0.max(0)..r.y1.min(H) {
            for x in r.x0.max(0)..r.x1.min(W) {
                let o = (y * W + x) as usize;
                if self.buf[o] > max {
                    self.buf[o] = max;
                }
            }
        }
        self.touch(r);
    }

    pub fn invert(&mut self, r: Rect) {
        for y in r.y0.max(0)..r.y1.min(H) {
            for x in r.x0.max(0)..r.x1.min(W) {
                let o = (y * W + x) as usize;
                self.buf[o] = 255 - self.buf[o];
            }
        }
        self.touch(r);
    }

    /// Send the dirty region to the panel. `flash` forces a full black/white flash to clear ghosting.
    pub fn present(&mut self, wf: Wf, flash: bool) {
        let Some(mut d) = self.dirty.take() else { return };
        if flash {
            d = Rect::new(0, 0, W, H);
        }
        // FBInk is happiest with even offsets and sizes.
        d.x0 &= !1;
        d.y0 &= !1;
        d.x1 = (d.x1 + 1) & !1;
        d.y1 = (d.y1 + 1) & !1;
        d.x1 = d.x1.min(W);
        d.y1 = d.y1.min(H);
        let (w, h) = (d.w(), d.h());
        if crate::sim::enabled() {
            let mut px = Vec::with_capacity((w * h) as usize);
            for y in d.y0..d.y1 {
                let o = (y * W) as usize;
                px.extend_from_slice(&self.buf[o + d.x0 as usize..o + d.x1 as usize]);
            }
            let mode = match wf {
                Wf::Auto => 0,
                Wf::Fast => 1,
                Wf::Text => 2,
                Wf::Full => 3,
            };
            crate::sim::frame(d.x0, d.y0, w, h, mode, flash, &px);
            return;
        }
        let mut img = format!("P5\n{} {}\n255\n", w, h).into_bytes();
        for y in d.y0..d.y1 {
            let o = (y * W) as usize;
            img.extend_from_slice(&self.buf[o + d.x0 as usize..o + d.x1 as usize]);
        }
        if std::fs::write(TMP_IMG, &img).is_err() {
            return;
        }
        let mode = match wf {
            Wf::Auto => "AUTO",
            Wf::Fast => "DU",
            Wf::Text => "GL16",
            Wf::Full => "GC16",
        };
        let mut cmd = Command::new(FBINK);
        cmd.arg("-g").arg(format!("file={},x={},y={},w={},h={}", TMP_IMG, d.x0, d.y0, w, h)).arg("-W").arg(mode);
        if flash {
            cmd.arg("-f");
        }
        let _ = cmd.stdout(Stdio::null()).stderr(Stdio::null()).status();
        let _ = std::io::stdout().flush();
    }
}
