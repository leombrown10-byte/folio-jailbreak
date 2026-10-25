//! Notes: handwritten notebooks (stylus-free finger/pen strokes), typed notes, and the highlights browser.
use crate::gfx::*;
use crate::ui::*;
use crate::{App, Modal, Screen, G};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Note {
    pub id: String,
    pub title: String,
    pub ink: bool,
    pub text: String,
    pub pages: Vec<Vec<Vec<(i16, i16)>>>, // pages -> strokes -> points
    pub modified: u64,
}

pub struct Pen {
    pub down: bool,
    pub last: (i32, i32),
    pub erase: bool,
    pub stroke: Vec<(i16, i16)>,
    pub last_present: Instant,
    pub touched: bool,
}

impl Pen {
    pub fn new() -> Pen {
        Pen { down: false, last: (0, 0), erase: false, stroke: vec![], last_present: Instant::now(), touched: false }
    }
}

const BAR: i32 = 120;
const PEN_W: i32 = 4;

pub fn load_all(dir: &std::path::Path) -> Vec<Note> {
    let mut v: Vec<Note> = std::fs::read_dir(dir.join("notes"))
        .map(|rd| rd.flatten().filter_map(|e| std::fs::read(e.path()).ok().and_then(|b| serde_json::from_slice(&b).ok())).collect())
        .unwrap_or_default();
    v.sort_by(|a: &Note, b: &Note| b.modified.cmp(&a.modified));
    v
}

fn crc32(data: &[u8]) -> u32 {
    let mut c = 0xffff_ffffu32;
    for &b in data {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { (c >> 1) ^ 0xedb8_8320 } else { c >> 1 };
        }
    }
    !c
}

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &x in data {
        a = (a + x as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

/// Minimal 8-bit grayscale PNG writer (stored deflate blocks), so exported pages open anywhere.
pub fn write_png(path: &std::path::Path, w: u32, h: u32, px: &[u8]) -> std::io::Result<()> {
    let mut raw = Vec::with_capacity((w as usize + 1) * h as usize);
    for y in 0..h as usize {
        raw.push(0);
        raw.extend_from_slice(&px[y * w as usize..(y + 1) * w as usize]);
    }
    let mut z = vec![0x78, 0x01];
    let mut chunks = raw.chunks(65535).peekable();
    while let Some(c) = chunks.next() {
        z.push(if chunks.peek().is_none() { 1 } else { 0 });
        z.extend_from_slice(&(c.len() as u16).to_le_bytes());
        z.extend_from_slice(&(!(c.len() as u16)).to_le_bytes());
        z.extend_from_slice(c);
    }
    z.extend_from_slice(&adler32(&raw).to_be_bytes());
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut chunk = |t: &[u8], d: &[u8]| {
        out.extend_from_slice(&(d.len() as u32).to_be_bytes());
        let mut td = t.to_vec();
        td.extend_from_slice(d);
        out.extend_from_slice(&td);
        out.extend_from_slice(&crc32(&td).to_be_bytes());
    };
    let mut ihdr = vec![];
    ihdr.extend_from_slice(&w.to_be_bytes());
    ihdr.extend_from_slice(&h.to_be_bytes());
    ihdr.extend_from_slice(&[8, 0, 0, 0, 0]);
    chunk(b"IHDR", &ihdr);
    chunk(b"IDAT", &z);
    chunk(b"IEND", &[]);
    std::fs::write(path, out)
}

impl App {
    fn note_save(&mut self, i: usize) {
        let n = &mut self.notes[i];
        n.modified = crate::sys::now();
        if let Ok(b) = serde_json::to_vec(&*n) {
            let _ = std::fs::write(self.st.dir.join("notes").join(format!("{}.json", n.id)), b);
        }
    }

    pub fn new_note(&mut self, ink: bool, title: &str) {
        let id = format!("{}", crate::sys::now());
        let t = if title.trim().is_empty() { format!("{} {}", if ink { "Notebook" } else { "Note" }, self.notes.len() + 1) } else { title.trim().to_string() };
        let n = Note { id, title: t, ink, text: String::new(), pages: if ink { vec![vec![]] } else { vec![] }, modified: crate::sys::now() };
        self.notes.insert(0, n);
        self.note_save(0);
        self.open_note(0);
    }

    pub fn open_note(&mut self, i: usize) {
        self.note_idx = Some(i);
        self.note_page = 0;
        if self.notes[i].ink {
            self.screen = Screen::Ink;
            self.ink_draw();
        } else {
            self.screen = Screen::TextNote;
            self.text_note_draw();
            self.cv.present(Wf::Text, true);
        }
    }

    // ---------------- notes tab ----------------
    pub fn draw_notes_tab(&mut self) {
        let cv = &mut self.cv;
        cv.text(F::SerifB, 72.0, 50, 100, "Notes", BLACK);
        // segmented control
        let seg = [(0, "Notebooks"), (1, "Highlights")];
        for (i, (k, label)) in seg.iter().enumerate() {
            let r = Rect::new(50 + i as i32 * 260, 214, 250, 70);
            if *k == self.notes_seg {
                cv.rrect(r.x0, r.y0, r.w(), r.h(), 35, BLACK, 0);
                cv.text_c(F::SansM, 30.0, (r.x0 + r.x1) / 2, r.y0 + 17, label, WHITE);
            } else {
                cv.rrect(r.x0, r.y0, r.w(), r.h(), 35, LIGHT, 3);
                cv.text_c(F::SansM, 30.0, (r.x0 + r.x1) / 2, r.y0 + 17, label, DARK);
            }
        }
        if self.notes_seg == 0 {
            icon_button(cv, Rect::new(W - 270, 206, 86, 86), "pen", true);
            icon_button(cv, Rect::new(W - 160, 206, 86, 86), "plus", false);
            cv.rrect(W - 160, 206, 86, 86, 43, DARK, 3);
            let top = 330;
            let per = 7;
            let n = self.notes.len();
            if n == 0 {
                cv.text_c(F::Serif, 38.0, W / 2, 560, "No notes yet", MID);
                cv.text_c(F::Sans, 28.0, W / 2, 620, "Tap the pen for a handwritten notebook", MID);
                cv.text_c(F::Sans, 28.0, W / 2, 664, "or + for a typed note", MID);
            }
            let start = self.notes_page * per;
            for (j, note) in self.notes.iter().skip(start).take(per).enumerate() {
                let y = top + j as i32 * 130;
                let t = cv.fit(F::SerifB, 38.0, &note.title, W - 260);
                cv.text(F::SerifB, 38.0, 50, y + 8, &t, BLACK);
                let sub = if note.ink { format!("Handwritten · {} page{}", note.pages.len(), if note.pages.len() == 1 { "" } else { "s" }) } else { cv.fit(F::Sans, 26.0, &note.text.replace('\n', " "), W - 200) };
                cv.text(F::Sans, 26.0, 50, y + 62, &sub, MID);
                cv.fill(50, y + 118, W - 100, 2, LIGHT);
            }
            self.draw_pager(self.notes_page, (n + per - 1) / per.max(1));
        } else {
            self.refresh_hl_list();
            let top = 330;
            let per = 6;
            let n = self.hl_list.len();
            if n == 0 {
                self.cv.text_c(F::Serif, 38.0, W / 2, 560, "No highlights yet", MID);
                self.cv.text_c(F::Sans, 28.0, W / 2, 620, "Press and hold a word while reading", MID);
            }
            let start = self.notes_page * per;
            for (j, (book, text, note)) in self.hl_list.clone().iter().skip(start).take(per).enumerate() {
                let y = top + j as i32 * 150;
                let lines = self.cv.wrap(F::Serif, 32.0, text, W - 140, 2);
                for (k, l) in lines.iter().enumerate() {
                    self.cv.text(F::Serif, 32.0, 70, y + k as i32 * 42, l, BLACK);
                }
                self.cv.fill(50, y + 4, 5, 84, BLACK);
                let b = self.cv.fit(F::Sans, 24.0, &if note.is_empty() { book.clone() } else { format!("{} — {}", book, note) }, W - 140);
                self.cv.text(F::Sans, 24.0, 70, y + 96, &b, MID);
            }
            self.draw_pager(self.notes_page, (n + per - 1) / per.max(1));
        }
    }

    fn draw_pager(&mut self, page: usize, pages: usize) {
        if pages > 1 {
            let y = 1318 - 110;
            self.cv.text_c(F::Sans, 26.0, W / 2, y + 12, &format!("{} / {}", page + 1, pages), MID);
            chevron(&mut self.cv, 120, y + 28, 20, true, if page > 0 { BLACK } else { LIGHT });
            chevron(&mut self.cv, W - 120, y + 28, 20, false, if page + 1 < pages { BLACK } else { LIGHT });
        }
    }

    fn refresh_hl_list(&mut self) {
        let mut v = vec![];
        let mut keys: Vec<_> = self.st.d.books.iter().collect();
        keys.sort_by_key(|(_, b)| std::cmp::Reverse(b.opened));
        for (_, b) in keys {
            for h in &b.highlights {
                v.push((b.title.clone(), h.text.clone(), h.note.clone()));
            }
        }
        self.hl_list = v;
    }

    pub fn notes_tab_tap(&mut self, x: i32, y: i32) {
        if y > 200 && y < 300 {
            if x < 50 + 250 && x > 50 {
                self.notes_seg = 0;
            } else if x > 310 && x < 560 {
                self.notes_seg = 1;
            } else if self.notes_seg == 0 && x > W - 270 && x < W - 180 {
                self.modal = None;
                self.pending_new = Some(true);
                self.open_kbd("Notebook name", "", KbdFor::NoteTitle);
                return;
            } else if self.notes_seg == 0 && x > W - 170 {
                self.pending_new = Some(false);
                self.open_kbd("Note title", "", KbdFor::NoteTitle);
                return;
            } else {
                return;
            }
            self.notes_page = 0;
            self.draw_screen(false);
            return;
        }
        let per = if self.notes_seg == 0 { 7 } else { 6 };
        let n = if self.notes_seg == 0 { self.notes.len() } else { self.hl_list.len() };
        let pages = (n + per - 1) / per;
        if y > 1318 - 120 && y < 1318 && pages > 1 {
            if x < W / 3 && self.notes_page > 0 {
                self.notes_page -= 1;
                self.draw_screen(false);
            } else if x > 2 * W / 3 && self.notes_page + 1 < pages {
                self.notes_page += 1;
                self.draw_screen(false);
            }
            return;
        }
        if self.notes_seg == 0 && y >= 330 {
            let j = ((y - 330) / 130) as usize;
            let idx = self.notes_page * per + j;
            if j < per && idx < self.notes.len() {
                self.open_note(idx);
            }
        }
    }

    // ---------------- typed note ----------------
    pub fn text_note_draw(&mut self) {
        let Some(i) = self.note_idx else { return };
        self.cv.clear(WHITE);
        icon(&mut self.cv, "back", 60, 62, BLACK);
        let t = self.cv.fit(F::SerifB, 44.0, &self.notes[i].title, W - 400);
        self.cv.text(F::SerifB, 44.0, 120, 34, &t, BLACK);
        self.cv.fill(36, BAR, W - 72, 3, BLACK);
        let body = self.notes[i].text.clone();
        self.draw_text_body(&body);
        if body.is_empty() {
            self.cv.text_c(F::Serif, 36.0, W / 2, 400, "Tap anywhere to write", MID);
        }
        icon(&mut self.cv, "more", W - 70, 62, BLACK);
    }

    pub fn draw_text_body(&mut self, body: &str) {
        let mut y = BAR + 30;
        for para in body.split('\n') {
            if para.trim().is_empty() {
                y += 46;
                continue;
            }
            for l in self.cv.wrap(F::Serif, 38.0, para, W - 120, 100) {
                if y > H - 120 {
                    return;
                }
                self.cv.text(F::Serif, 38.0, 60, y, &l, BLACK);
                y += 52;
            }
            y += 10;
        }
    }

    pub fn text_note_gesture(&mut self, g: G) {
        let G::Tap(x, y) = g else { return };
        let Some(i) = self.note_idx else { return };
        if y < BAR && x < 300 {
            self.note_save(i);
            self.screen = Screen::Notes;
            self.note_idx = None;
            self.draw_screen(true);
        } else if y < BAR && x > W - 160 {
            self.modal = Some(Modal::Sheet(Sheet::new("Note", vec![("Rename", Act::RenameNote), ("Export as text file", Act::ExportNote), ("Delete note", Act::DeleteNote), ("Close", Act::Close)])));
            self.draw_modal();
        } else {
            let t = self.notes[i].text.clone();
            self.open_kbd("Note", &t, KbdFor::NoteText(i));
        }
    }

    pub fn delete_note(&mut self) {
        if let Some(i) = self.note_idx.take() {
            let id = self.notes[i].id.clone();
            let _ = std::fs::remove_file(self.st.dir.join("notes").join(format!("{}.json", id)));
            self.notes.remove(i);
        }
        self.screen = Screen::Notes;
        self.modal = None;
        self.draw_screen(true);
    }

    pub fn set_note_text(&mut self, i: usize, text: String) {
        self.notes[i].text = text;
        self.note_save(i);
    }

    pub fn export_note(&mut self, i: usize) {
        let dir = std::path::Path::new("/mnt/us/documents/KOS Notes");
        let _ = std::fs::create_dir_all(dir);
        let n = self.notes[i].clone();
        let safe: String = n.title.chars().map(|c| if c.is_alphanumeric() || c == ' ' { c } else { '_' }).collect();
        let msg = if n.ink {
            let mut count = 0;
            for p in 0..n.pages.len() {
                let mut buf = vec![WHITE; (W * H) as usize];
                std::mem::swap(&mut buf, &mut self.cv.buf);
                self.cv.fill(0, 0, W, H, WHITE);
                self.paint_strokes(&n.pages[p]);
                std::mem::swap(&mut buf, &mut self.cv.buf);
                if write_png(&dir.join(format!("{} p{}.png", safe, p + 1)), W as u32, H as u32, &buf).is_ok() {
                    count += 1;
                }
            }
            format!("Exported {} page(s)", count)
        } else if std::fs::write(dir.join(format!("{}.txt", safe)), &n.text).is_ok() {
            "Exported to documents/KOS Notes".to_string()
        } else {
            "Export failed".to_string()
        };
        if n.ink {
            self.ink_draw();
        } else {
            self.text_note_draw();
            self.cv.present(Wf::Text, false);
        }
        self.toast(&msg);
    }

    // ---------------- ink notebook ----------------
    fn paint_strokes(&mut self, strokes: &[Vec<(i16, i16)>]) {
        for s in strokes {
            if s.len() == 1 {
                self.cv.disc(s[0].0 as i32, s[0].1 as i32, PEN_W / 2 + 1, BLACK);
            }
            for w in s.windows(2) {
                self.cv.line(w[0].0 as i32, w[0].1 as i32, w[1].0 as i32, w[1].1 as i32, PEN_W, BLACK);
            }
        }
    }

    pub fn ink_draw(&mut self) {
        let Some(i) = self.note_idx else { return };
        self.cv.clear(WHITE);
        // dot grid
        for gy in (BAR + 60..H - 40).step_by(60) {
            for gx in (60..W - 30).step_by(60) {
                self.cv.fill(gx, gy, 3, 3, MID);
            }
        }
        let strokes = self.notes[i].pages[self.note_page].clone();
        self.paint_strokes(&strokes);
        self.cv.fill(0, 0, W, BAR, WHITE);
        self.cv.fill(0, BAR - 3, W, 3, BLACK);
        icon(&mut self.cv, "back", 50, 62, BLACK);
        let np = self.notes[i].pages.len();
        let t = self.cv.fit(F::SerifB, 34.0, &self.notes[i].title, 330);
        self.cv.text(F::SerifB, 34.0, 100, 18, &t, BLACK);
        self.cv.text(F::Sans, 24.0, 100, 66, &format!("Page {} of {}", self.note_page + 1, np), MID);
        let tools = [("pen", !self.pen.erase), ("eraser", self.pen.erase)];
        for (k, (name, on)) in tools.iter().enumerate() {
            let r = Rect::new(500 + k as i32 * 100, 20, 80, 80);
            icon_button(&mut self.cv, r, name, *on);
        }
        icon(&mut self.cv, "undo", 740, 62, BLACK);
        icon(&mut self.cv, "prev", 830, 62, if self.note_page > 0 { BLACK } else { LIGHT });
        icon(&mut self.cv, "next", 920, 62, BLACK);
        icon(&mut self.cv, "more", W - 50, 62, BLACK);
        self.cv.present(Wf::Text, true);
    }

    /// Raw pen events from the touchscreen.
    pub fn ink_touch(&mut self, kind: u8, x: i32, y: i32) {
        let Some(i) = self.note_idx else { return };
        match kind {
            0 => {
                if y < BAR {
                    self.ink_toolbar(x, y);
                    self.pen.touched = false;
                    return;
                }
                self.pen.touched = true;
                self.pen.down = true;
                self.pen.last = (x, y);
                self.pen.stroke = vec![(x as i16, y as i16)];
                if !self.pen.erase {
                    self.cv.disc(x, y, PEN_W / 2 + 1, BLACK);
                }
            }
            1 if self.pen.down && self.pen.touched => {
                let (lx, ly) = self.pen.last;
                if self.pen.erase {
                    self.erase_at(i, x, y);
                } else if y > BAR && (x - lx).abs() + (y - ly).abs() > 2 {
                    self.cv.line(lx, ly, x, y, PEN_W, BLACK);
                    self.pen.stroke.push((x as i16, y as i16));
                    self.pen.last = (x, y);
                }
                if self.pen.last_present.elapsed().as_millis() > 70 {
                    self.cv.present(Wf::Fast, false);
                    self.pen.last_present = Instant::now();
                }
            }
            2 if self.pen.down && self.pen.touched => {
                self.pen.down = false;
                if !self.pen.erase && !self.pen.stroke.is_empty() {
                    let s = std::mem::take(&mut self.pen.stroke);
                    self.notes[i].pages[self.note_page].push(s);
                    self.note_save(i);
                }
                self.cv.present(Wf::Fast, false);
                self.pen.last_present = Instant::now();
                self.ink_idle_at = Some(Instant::now());
            }
            _ => {}
        }
    }

    fn erase_at(&mut self, i: usize, x: i32, y: i32) {
        let r = 36;
        let before = self.notes[i].pages[self.note_page].len();
        self.notes[i].pages[self.note_page].retain(|s| !s.iter().any(|p| (p.0 as i32 - x).abs() < r && (p.1 as i32 - y).abs() < r));
        if self.notes[i].pages[self.note_page].len() != before {
            self.note_save(i);
            self.ink_draw_fast();
        }
    }

    fn ink_draw_fast(&mut self) {
        let Some(i) = self.note_idx else { return };
        self.cv.fill(0, BAR, W, H - BAR, WHITE);
        for gy in (BAR + 60..H - 40).step_by(60) {
            for gx in (60..W - 30).step_by(60) {
                self.cv.fill(gx, gy, 3, 3, MID);
            }
        }
        let strokes = self.notes[i].pages[self.note_page].clone();
        self.paint_strokes(&strokes);
        self.cv.present(Wf::Fast, false);
    }

    fn ink_toolbar(&mut self, x: i32, _y: i32) {
        let Some(i) = self.note_idx else { return };
        if x < 90 {
            self.note_save(i);
            self.screen = Screen::Notes;
            self.note_idx = None;
            self.draw_screen(true);
        } else if (500..580).contains(&x) {
            self.pen.erase = false;
            self.ink_draw();
        } else if (600..680).contains(&x) {
            self.pen.erase = true;
            self.ink_draw();
        } else if (700..780).contains(&x) {
            self.notes[i].pages[self.note_page].pop();
            self.note_save(i);
            self.ink_draw();
        } else if (790..870).contains(&x) {
            if self.note_page > 0 {
                self.note_page -= 1;
                self.ink_draw();
            }
        } else if (880..960).contains(&x) {
            self.note_page += 1;
            if self.note_page >= self.notes[i].pages.len() {
                self.notes[i].pages.push(vec![]);
                self.note_save(i);
            }
            self.ink_draw();
        } else if x > W - 110 {
            self.modal = Some(Modal::Sheet(Sheet::new("Notebook", vec![("Export as PNG", Act::ExportNote), ("Rename", Act::RenameNote), ("Delete notebook", Act::DeleteNote), ("Close", Act::Close)])));
            self.draw_modal();
        }
    }
}
