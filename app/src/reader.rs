//! Reading engine client (kreader subprocess) and the reader screen.
use crate::gfx::*;
use crate::library::{self, Item};
use crate::store::Highlight;
use crate::ui::*;
use crate::{App, Modal, Screen, G};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

const LAYOUT_W: f32 = 360.0; // reflow layout width in "points"; 1072px / 360 = 2.98 px per point
const PAGE_TMP: &str = "/tmp/kos_page.pgm";

pub struct Kr {
    child: Child,
    sin: ChildStdin,
    sout: BufReader<ChildStdout>,
}

impl Kr {
    pub fn start() -> Option<Kr> {
        let mut child = Command::new(library::KREADER).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().ok()?;
        let sin = child.stdin.take()?;
        let sout = BufReader::new(child.stdout.take()?);
        Some(Kr { child, sin, sout })
    }
    pub fn cmd(&mut self, l: &str) -> String {
        if writeln!(self.sin, "{}", l).is_err() {
            return "err io".into();
        }
        let mut s = String::new();
        let _ = self.sout.read_line(&mut s);
        s.trim_end().to_string()
    }
    pub fn lines(&mut self, l: &str) -> Vec<String> {
        let mut v = vec![];
        if writeln!(self.sin, "{}", l).is_err() {
            return v;
        }
        loop {
            let mut s = String::new();
            if self.sout.read_line(&mut s).unwrap_or(0) == 0 {
                break;
            }
            let s = s.trim_end().to_string();
            if s == "." {
                break;
            }
            v.push(s);
        }
        v
    }
}

impl Drop for Kr {
    fn drop(&mut self) {
        let _ = writeln!(self.sin, "quit");
        let _ = self.child.wait();
    }
}

#[derive(Clone)]
pub struct Word {
    pub r: [f32; 4],
    pub t: String,
}

pub enum Panel {
    Toc(ListView, Vec<usize>),
    Marks(ListView, Vec<(bool, usize)>), // (is_highlight, index)
    Search(ListView, Vec<usize>),
    Font,
}

pub struct Reader {
    pub item: Item,
    kr: Kr,
    pub pages: usize,
    pub page: usize,
    pub toc: Vec<(u32, usize, String)>,
    pub words: Vec<Word>,
    words_page: usize,
    pub menu: bool,
    pub panel: Option<Panel>,
    pub sel: Option<usize>,
    pub scale: f32,
    pub dx: f32,
    pub top: i32,
    pub bottom: i32,
    turns: u32,
    last_turn: std::time::Instant,
}

/// The sentence containing word `sel`, limited to the words on this page.
fn sentence_around(words: &[Word], sel: usize) -> String {
    let end = |t: &str| t.ends_with(['.', '!', '?', '…']) || t.ends_with(".”") || t.ends_with("?”") || t.ends_with("!”");
    let mut lo = sel;
    while lo > 0 && sel - lo < 40 && !end(&words[lo - 1].t) {
        lo -= 1;
    }
    let mut hi = sel;
    while hi + 1 < words.len() && hi - sel < 40 && !end(&words[hi].t) {
        hi += 1;
    }
    words[lo..=hi].iter().map(|w| w.t.as_str()).collect::<Vec<_>>().join(" ")
}

fn norm(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase()
}

fn open_path(app: &App, it: &Item) -> Option<String> {
    match it.ext.as_str() {
        "mobi" | "azw" | "azw3" => crate::convert::to_epub(&app.st.dir, it),
        "djvu" => crate::convert::djvu_to_pdf(&app.st.dir, it),
        _ => Some(it.path.clone()),
    }
}

impl Reader {
    fn layout(&mut self, em: f32, margin: f32) -> bool {
        self.scale = W as f32 / LAYOUT_W;
        self.dx = margin * self.scale;
        let w_pt = LAYOUT_W - 2.0 * margin;
        let h_pt = (H - self.top - self.bottom) as f32 / self.scale;
        let _ = (w_pt, h_pt);
        true
    }

    fn open_doc(&mut self, path: &str, em: f32, margin: f32, style: (i32, i32, bool)) -> Option<usize> {
        self.layout(em, margin);
        self.kr.cmd(&format!("style {} {} {}", style.0, style.1, style.2 as i32));
        let w_pt = LAYOUT_W - 2.0 * margin;
        let h_pt = (H - self.top - self.bottom) as f32 / self.scale;
        let r = self.kr.cmd(&format!("open {} {} {} {}", w_pt, h_pt, em, path));
        r.strip_prefix("ok ").and_then(|n| n.trim().parse().ok())
    }
}

impl App {
    pub fn open_book(&mut self, it: Item) {
        let Some(path) = open_path(self, &it) else {
            self.toast("Can't open this format yet");
            return;
        };
        let Some(kr) = Kr::start() else { return };
        let mut rd = Reader {
            item: it.clone(),
            kr,
            pages: 0,
            page: 0,
            toc: vec![],
            words: vec![],
            words_page: usize::MAX,
            menu: false,
            panel: None,
            sel: None,
            scale: 1.0,
            dx: 0.0,
            top: self.st.d.settings.top,
            bottom: self.st.d.settings.bottom,
            turns: 0,
            last_turn: std::time::Instant::now(),
        };
        let (em, margin) = (self.st.d.settings.em, self.st.d.settings.margin);
        let st = &self.st.d.settings;
        let Some(pages) = rd.open_doc(&path, em, margin, (st.family, st.spacing, st.justify)) else {
            self.toast("This file could not be opened");
            return;
        };
        rd.pages = pages.max(1);
        rd.toc = rd
            .kr
            .lines("toc")
            .into_iter()
            .filter_map(|l| {
                let mut p = l.splitn(3, '\t');
                Some((p.next()?.parse().ok()?, p.next()?.parse::<i64>().ok()?.max(0) as usize, p.next()?.to_string()))
            })
            .collect();
        let t = rd.kr.cmd("title");
        let title = t.strip_prefix("ok ").unwrap_or("").trim().to_string();
        let b = self.st.book(&it.key);
        if b.title.is_empty() {
            b.title = if title.is_empty() { it.title.clone() } else { title };
        }
        b.opened = crate::sys::now();
        rd.page = ((b.frac * rd.pages as f32).round() as usize).min(rd.pages - 1);
        self.st.d.settings.last_book = Some(it.key.clone());
        self.st.save();
        self.rd = Some(rd);
        self.screen = Screen::Reader;
        self.modal = None;
        self.reader_draw(true);
    }

    pub fn close_book(&mut self) {
        self.reader_save();
        self.rd = None;
        self.screen = Screen::Books;
        self.draw_screen(true);
    }

    fn reader_save(&mut self) {
        if let Some(rd) = &self.rd {
            let f = if rd.pages > 1 { rd.page as f32 / (rd.pages - 1) as f32 } else { 0.0 };
            let key = rd.item.key.clone();
            self.st.book(&key).frac = f;
            self.st.save();
        }
    }

    pub fn reader_relayout(&mut self) {
        let Some(rd) = self.rd.as_mut() else { return };
        let frac = if rd.pages > 1 { rd.page as f32 / (rd.pages - 1) as f32 } else { 0.0 };
        let (em, margin) = (self.st.d.settings.em, self.st.d.settings.margin);
        let style = (self.st.d.settings.family, self.st.d.settings.spacing, self.st.d.settings.justify);
        let (top, bottom) = (self.st.d.settings.top, self.st.d.settings.bottom);
        let it = rd.item.clone();
        let path = match it.ext.as_str() {
            "mobi" | "azw" | "azw3" => crate::convert::to_epub(&self.st.dir, &it),
            "djvu" => crate::convert::djvu_to_pdf(&self.st.dir, &it),
            _ => Some(it.path.clone()),
        };
        let rd = self.rd.as_mut().unwrap();
        rd.top = top;
        rd.bottom = bottom;
        if let Some(p) = path {
            if let Some(n) = rd.open_doc(&p, em, margin, style) {
                rd.pages = n.max(1);
                rd.page = ((frac * (rd.pages - 1) as f32).round() as usize).min(rd.pages - 1);
                rd.words_page = usize::MAX;
                rd.toc = rd
                    .kr
                    .lines("toc")
                    .into_iter()
                    .filter_map(|l| {
                        let mut p = l.splitn(3, '\t');
                        Some((p.next()?.parse().ok()?, p.next()?.parse::<i64>().ok()?.max(0) as usize, p.next()?.to_string()))
                    })
                    .collect();
            }
        }
    }

    fn rd_words(&mut self) {
        let Some(rd) = self.rd.as_mut() else { return };
        if rd.words_page == rd.page {
            return;
        }
        rd.words = rd
            .kr
            .lines(&format!("words {}", rd.page))
            .into_iter()
            .filter_map(|l| {
                let (a, t) = l.split_once('\t')?;
                let v: Vec<f32> = a.split(' ').filter_map(|x| x.parse().ok()).collect();
                if v.len() < 4 {
                    return None;
                }
                Some(Word { r: [v[0], v[1], v[2], v[3]], t: t.to_string() })
            })
            .collect();
        rd.words_page = rd.page;
    }

    fn word_px(&self, w: &Word) -> Rect {
        let rd = self.rd.as_ref().unwrap();
        let s = rd.scale;
        Rect {
            x0: (w.r[0] * s + rd.dx) as i32 - 2,
            y0: (w.r[1] * s + rd.top as f32) as i32,
            x1: (w.r[2] * s + rd.dx) as i32 + 2,
            y1: (w.r[3] * s + rd.top as f32) as i32 + 4,
        }
    }

    /// Render the current page, overlay highlights and footer, push to the panel.
    pub fn reader_draw(&mut self, full: bool) {
        if self.rd.is_none() {
            return;
        }
        self.reader_paint();
        let turns = {
            let rd = self.rd.as_mut().unwrap();
            rd.turns += 1;
            rd.turns
        };
        // Flash every few turns to wipe ghosting; GL16 in between keeps text crisp.
        let every = self.st.d.settings.flash_every.max(0) as u32;
        let flash = full || (every > 0 && turns % every == 0);
        self.cv.present(if flash { Wf::Full } else { Wf::Text }, flash);
    }

    /// Render the current page into the canvas (no panel update).
    pub fn reader_paint(&mut self) {
        let Some(rd) = self.rd.as_mut() else { return };
        let (scale, dx) = (rd.scale, rd.dx);
        let r = rd.kr.cmd(&format!("page {} {} {} {} {} {} {}", rd.page, PAGE_TMP, scale, dx, rd.top, W, H));
        if r.starts_with("ok") {
            if let Some((w, h, px)) = library::read_pgm(std::path::Path::new(PAGE_TMP)) {
                if w == W && h == H {
                    self.cv.blit(0, 0, w, h, &px);
                }
            }
        } else {
            self.cv.clear(WHITE);
            self.cv.text_c(F::Serif, 36.0, W / 2, 600, "Could not render this page", MID);
        }
        self.reader_overlays();
    }

    fn reader_overlays(&mut self) {
        self.rd_words();
        let key = self.rd.as_ref().unwrap().item.key.clone();
        let (page, pages) = {
            let rd = self.rd.as_ref().unwrap();
            (rd.page, rd.pages)
        };
        // highlights
        let hls: Vec<Highlight> = self.st.book(&key).highlights.clone();
        let words = self.rd.as_ref().unwrap().words.clone();
        let nw: Vec<String> = words.iter().map(|w| norm(&w.t)).collect();
        for h in &hls {
            let hw: Vec<String> = h.text.split_whitespace().map(norm).filter(|s| !s.is_empty()).collect();
            if hw.is_empty() || hw.len() > nw.len() {
                continue;
            }
            let mut i = 0;
            while i + hw.len() <= nw.len() {
                if nw[i..i + hw.len()] == hw[..] {
                    let mut line: Option<Rect> = None;
                    for k in i..i + hw.len() {
                        let r = self.word_px(&words[k]);
                        match line {
                            Some(l) if (r.y0 - l.y0).abs() < 12 => line = Some(Rect { x0: l.x0.min(r.x0), y0: l.y0.min(r.y0), x1: l.x1.max(r.x1), y1: l.y1.max(r.y1) }),
                            Some(l) => {
                                self.cv.tint(l, 205);
                                self.cv.fill(l.x0, l.y1 - 3, l.w(), 3, DARK);
                                line = Some(r);
                            }
                            None => line = Some(r),
                        }
                    }
                    if let Some(l) = line {
                        self.cv.tint(l, 205);
                        self.cv.fill(l.x0, l.y1 - 3, l.w(), 3, DARK);
                    }
                    i += hw.len();
                } else {
                    i += 1;
                }
            }
        }
        // selection
        if let Some(i) = self.rd.as_ref().unwrap().sel {
            if let Some(w) = words.get(i) {
                let r = self.word_px(w);
                self.cv.invert(r);
            }
        }
        // bookmark dog-ear
        let frac = if pages > 1 { page as f32 / (pages - 1) as f32 } else { 0.0 };
        let marked = self.st.book(&key).bookmarks.iter().any(|b| (b - frac).abs() < 0.5 / pages.max(1) as f32);
        if marked {
            for i in 0..56 {
                self.cv.line(W - 56 + i, 0, W - 1, 56 - i, 1, BLACK);
            }
            self.cv.fill(W - 56, 0, 1, 1, BLACK);
        }
        // footer
        let rd = self.rd.as_ref().unwrap();
        let chapter = rd.toc.iter().rev().find(|t| t.1 <= page).map(|t| t.2.clone()).unwrap_or_default();
        // middle of the footer: focus countdown, else estimated time left in chapter / book
        let mid = if let Some(m) = self.focus_left() {
            format!("focus {} min", m)
        } else if let Some(spp) = self.st.secs_per_page() {
            let ch_end = rd.toc.iter().find(|t| t.1 > page).map(|t| t.1).unwrap_or(pages);
            format!("{} in ch. · {} left", crate::extras::fmt_left(spp * (ch_end.saturating_sub(page + 1)) as f32), crate::extras::fmt_left(spp * (pages - 1 - page) as f32))
        } else {
            String::new()
        };
        self.cv.fill(0, H - 70, W, 70, WHITE);
        let ch = self.cv.fit(F::Sans, 24.0, &chapter, 380);
        self.cv.text(F::Sans, 24.0, 40, H - 52, &ch, MID);
        self.cv.text_c(F::Sans, 22.0, 660, H - 50, &mid, MID);
        self.cv.text_r(F::Sans, 24.0, W - 40, H - 52, &format!("{} / {}", page + 1, pages), MID);
        self.cv.fill(40, H - 16, W - 80, 2, LIGHT);
        self.cv.fill(40, H - 17, ((W - 80) as f32 * frac) as i32, 4, DARK);
        // menu bars
        if self.rd.as_ref().unwrap().menu {
            self.draw_reader_menu();
        }
        if self.st.d.settings.dark {
            self.cv.invert(Rect::new(0, 0, W, H));
        }
    }

    fn draw_reader_menu(&mut self) {
        let title = {
            let rd = self.rd.as_ref().unwrap();
            self.st.d.books.get(&rd.item.key).map(|b| b.title.clone()).unwrap_or_default()
        };
        self.cv.fill(0, 0, W, 120, WHITE);
        self.cv.fill(0, 118, W, 3, BLACK);
        icon(&mut self.cv, "back", 60, 62, BLACK);
        let t = self.cv.fit(F::SerifB, 38.0, &title, W - 330);
        self.cv.text(F::SerifB, 38.0, 120, 38, &t, BLACK);
        self.cv.text_r(F::Sans, 28.0, W - 40, 42, &crate::sys::clock(), DARK);
        let y = H - 270;
        self.cv.fill(0, y, W, 270, WHITE);
        self.cv.fill(0, y, W, 3, BLACK);
        let (page, pages) = {
            let rd = self.rd.as_ref().unwrap();
            (rd.page, rd.pages)
        };
        // scrubber
        let frac = page as f32 / (pages.max(2) - 1) as f32;
        self.cv.fill(60, y + 50, W - 120, 6, LIGHT);
        self.cv.fill(60, y + 50, ((W - 120) as f32 * frac) as i32, 6, BLACK);
        self.cv.disc(60 + ((W - 120) as f32 * frac) as i32, y + 53, 16, BLACK);
        let names = [("list", "Contents"), ("search", "Search"), ("font", "Text"), ("marker", "Notes"), ("mark", "Bookmark"), ("sun", "Light")];
        for (i, (ic, label)) in names.iter().enumerate() {
            let cx = 90 + i as i32 * 178;
            icon(&mut self.cv, ic, cx, y + 150, BLACK);
            self.cv.text_c(F::Sans, 22.0, cx, y + 196, label, DARK);
        }
    }

    fn rd_jump(&mut self, page: usize) {
        if let Some(rd) = self.rd.as_mut() {
            rd.page = page.min(rd.pages - 1);
            rd.sel = None;
        }
    }

    pub fn reader_gesture(&mut self, g: G) {
        if self.rd.is_none() {
            return;
        }
        // panels (TOC, search, ...) take over the whole screen
        if self.rd.as_ref().unwrap().panel.is_some() {
            self.reader_panel_gesture(g);
            return;
        }
        let menu = self.rd.as_ref().unwrap().menu;
        match g {
            G::Tap(x, y) if menu => {
                let by = H - 270;
                if y < 120 && x < 300 {
                    self.close_book();
                } else if y > by + 20 && y < by + 90 {
                    let pages = self.rd.as_ref().unwrap().pages;
                    let f = ((x - 60) as f32 / (W - 120) as f32).clamp(0.0, 1.0);
                    self.rd_jump((f * (pages - 1) as f32).round() as usize);
                    self.reader_draw(false);
                } else if y > by + 100 {
                    match ((x - 0) / 178).min(5) {
                        0 => self.reader_toc(),
                        1 => self.open_kbd("Search in book", "", KbdFor::Search),
                        2 => self.reader_panel(Panel::Font),
                        3 => self.reader_marks(),
                        4 => self.reader_bookmark(),
                        _ => self.light_sheet(),
                    }
                } else {
                    self.rd.as_mut().unwrap().menu = false;
                    self.reader_draw(false);
                }
            }
            G::Tap(x, y) => {
                if self.rd.as_ref().unwrap().sel.is_some() {
                    self.rd.as_mut().unwrap().sel = None;
                    self.reader_draw(false);
                } else if x < W * 3 / 10 {
                    self.rd_turn(-1);
                } else if x > W * 7 / 10 && y > 140 {
                    self.rd_turn(1);
                } else if y < 140 && x > W - 200 {
                    self.reader_bookmark();
                } else {
                    self.rd.as_mut().unwrap().menu = true;
                    self.reader_draw(false);
                }
            }
            G::Swipe { dx, dy, .. } => {
                if dx.abs() > dy.abs() {
                    self.rd_turn(if dx < 0 { 1 } else { -1 });
                } else if dy > 250 {
                    self.rd.as_mut().unwrap().menu = true;
                    self.reader_draw(false);
                }
            }
            G::Long(x, y) => {
                if menu {
                    return;
                }
                self.rd_words();
                let words = self.rd.as_ref().unwrap().words.clone();
                let hit = words.iter().position(|w| self.word_px(w).contains(x, y)).or_else(|| {
                    // nearest word within 40px
                    let mut best: Option<(usize, i32)> = None;
                    for (i, w) in words.iter().enumerate() {
                        let r = self.word_px(w);
                        let d = (((r.x0 + r.x1) / 2 - x).pow(2) + ((r.y0 + r.y1) / 2 - y).pow(2)) as i32;
                        if d < 60 * 60 && best.map_or(true, |b| d < b.1) {
                            best = Some((i, d));
                        }
                    }
                    best.map(|b| b.0)
                });
                if let Some(i) = hit {
                    self.rd.as_mut().unwrap().sel = Some(i);
                    self.reader_define();
                }
            }
        }
    }

    fn rd_turn(&mut self, d: i32) {
        let rd = self.rd.as_mut().unwrap();
        let np = rd.page as i32 + d;
        if np < 0 || np >= rd.pages as i32 {
            return;
        }
        // reading log: a forward turn counts the time spent on the page (idle gaps capped at 2 min)
        let secs = rd.last_turn.elapsed().as_secs().min(120) as u32;
        rd.last_turn = std::time::Instant::now();
        rd.page = np as usize;
        if d > 0 {
            self.st.log_page(secs);
        }
        let rd = self.rd.as_mut().unwrap();
        rd.sel = None;
        rd.menu = false;
        if rd.turns % 6 == 0 {
            self.reader_save();
        }
        self.reader_draw(false);
    }

    fn reader_bookmark(&mut self) {
        let rd = self.rd.as_ref().unwrap();
        let (page, pages, key) = (rd.page, rd.pages, rd.item.key.clone());
        let frac = if pages > 1 { page as f32 / (pages - 1) as f32 } else { 0.0 };
        let tol = 0.5 / pages.max(1) as f32;
        let b = self.st.book(&key);
        if let Some(i) = b.bookmarks.iter().position(|x| (x - frac).abs() < tol) {
            b.bookmarks.remove(i);
        } else {
            b.bookmarks.push(frac);
        }
        self.st.save();
        self.reader_draw(false);
    }

    fn reader_panel(&mut self, p: Panel) {
        self.rd.as_mut().unwrap().panel = Some(p);
        self.reader_panel_draw();
    }

    fn reader_toc(&mut self) {
        let rd = self.rd.as_ref().unwrap();
        let items: Vec<(String, String)> = rd.toc.iter().map(|t| (t.2.clone(), format!("{}", t.1 + 1))).collect();
        let pages: Vec<usize> = rd.toc.iter().map(|t| t.1).collect();
        let cur = rd.toc.iter().rposition(|t| t.1 <= rd.page).unwrap_or(0);
        let mut lv = ListView::new("Contents", items);
        lv.page = cur / lv.per_page;
        self.reader_panel(Panel::Toc(lv, pages));
    }

    fn reader_marks(&mut self) {
        let rd = self.rd.as_ref().unwrap();
        let pages = rd.pages;
        let key = rd.item.key.clone();
        let b = self.st.book(&key).clone();
        let mut items = vec![];
        let mut refs = vec![];
        for (i, f) in b.bookmarks.iter().enumerate() {
            items.push((format!("Bookmark"), format!("p. {}", (f * (pages - 1) as f32).round() as usize + 1)));
            refs.push((false, i));
        }
        for (i, h) in b.highlights.iter().enumerate() {
            let label = if h.note.is_empty() { h.text.clone() } else { format!("{}  — {}", h.text, h.note) };
            items.push((label, format!("p. {}", (h.frac * (pages - 1) as f32).round() as usize + 1)));
            refs.push((true, i));
        }
        self.reader_panel(Panel::Marks(ListView::new("Bookmarks & highlights", items), refs));
    }

    pub fn reader_search(&mut self, q: &str) {
        let rd = self.rd.as_mut().unwrap();
        let hits: Vec<usize> = rd.kr.lines(&format!("search {}", q)).into_iter().filter_map(|l| l.parse().ok()).collect();
        let items = hits.iter().map(|p| (format!("“{}”", q), format!("p. {}", p + 1))).collect();
        self.reader_panel(Panel::Search(ListView::new(&format!("Search: {}", q), items), hits));
    }

    fn reader_panel_draw(&mut self) {
        let Some(rd) = self.rd.as_ref() else { return };
        match rd.panel.as_ref() {
            Some(Panel::Toc(lv, _)) | Some(Panel::Marks(lv, _)) | Some(Panel::Search(lv, _)) => {
                let toc_levels: Vec<u32> = rd.toc.iter().map(|t| t.0).collect();
                let is_toc = matches!(rd.panel, Some(Panel::Toc(..)));
                let lv_ptr = lv as *const ListView;
                // SAFETY-free alternative: clone view data for drawing
                let lv = unsafe { &*lv_ptr };
                lv.draw(&mut self.cv, &|i| if is_toc { toc_levels.get(i).copied().unwrap_or(0) as i32 * 36 } else { 0 });
            }
            Some(Panel::Font) => self.draw_font_panel(),
            None => return,
        }
        self.cv.present(Wf::Text, true);
    }

    fn draw_font_panel(&mut self) {
        let st = self.st.d.settings.clone();
        self.cv.clear(WHITE);
        icon(&mut self.cv, "back", 60, 78, BLACK);
        self.cv.text(F::SerifB, 46.0, 120, 48, "Text", BLACK);
        self.cv.fill(36, 128, W - 72, 3, BLACK);
        let rows = [
            ("Font size", format!("{:.0} pt", st.em)),
            ("Margins", format!("{:.0}", st.margin)),
            ("Line spacing", format!("{:.2}", st.spacing as f32 / 100.0)),
        ];
        for (i, (label, val)) in rows.iter().enumerate() {
            let y = 160 + i as i32 * 170;
            self.cv.text(F::SansM, 34.0, 60, y + 40, label, BLACK);
            self.cv.text_c(F::SerifB, 46.0, W / 2 + 20, y + 30, val, BLACK);
            for (x, name) in [(W - 360, "minus"), (W - 210, "plus")] {
                self.cv.rrect(x, y, 130, 100, 50, DARK, 3);
                icon(&mut self.cv, name, x + 65, y + 50, BLACK);
            }
        }
        let fam = ["Publisher default", "IBM Plex Serif", "IBM Plex Sans"][st.family.clamp(0, 2) as usize];
        button(&mut self.cv, Rect::new(60, 690, W - 120, 90), &format!("Typeface: {}", fam), false);
        button(&mut self.cv, Rect::new(60, 800, W - 120, 90), &format!("Justify text: {}", if st.justify { "on" } else { "off" }), false);
        self.cv.text(F::SansM, 26.0, 60, 925, "Preview", MID);
        let sample = "The quick brown fox jumps over the lazy dog. Pack my box with five dozen liquor jugs.";
        let px = st.em * (W as f32 / LAYOUT_W);
        let f = if st.family == 2 { F::Sans } else { F::Serif };
        let lines = self.cv.wrap(f, px, sample, W - 120, 4);
        for (i, l) in lines.iter().enumerate() {
            self.cv.text(f, px, 60, 965 + i as i32 * (px * st.spacing as f32 / 100.0) as i32, l, BLACK);
        }
        button(&mut self.cv, Rect::new(60, H - 150, W - 120, 100), "Apply", true);
    }

    fn reader_panel_gesture(&mut self, g: G) {
        let G::Tap(x, y) = g else { return };
        let is_font = matches!(self.rd.as_ref().unwrap().panel, Some(Panel::Font));
        if is_font {
            let s = &mut self.st.d.settings;
            for row in 0..3 {
                let yy = 160 + row * 170;
                let minus = Rect::new(W - 360, yy, 130, 100).contains(x, y);
                let plus = Rect::new(W - 210, yy, 130, 100).contains(x, y);
                if minus || plus {
                    let d = if plus { 1.0 } else { -1.0 };
                    match row {
                        0 => s.em = (s.em + d).clamp(7.0, 22.0),
                        1 => s.margin = (s.margin + 4.0 * d).clamp(0.0, 60.0),
                        _ => s.spacing = (s.spacing + (10.0 * d) as i32).clamp(100, 200),
                    }
                    self.draw_font_panel();
                    self.cv.present(Wf::Text, false);
                    return;
                }
            }
            if Rect::new(60, 690, W - 120, 90).contains(x, y) {
                s.family = (s.family + 1) % 3;
                self.draw_font_panel();
                self.cv.present(Wf::Text, false);
                return;
            }
            if Rect::new(60, 800, W - 120, 90).contains(x, y) {
                s.justify = !s.justify;
                self.draw_font_panel();
                self.cv.present(Wf::Text, false);
                return;
            }
            if Rect::new(60, H - 150, W - 120, 100).contains(x, y) || (y < 130 && x < 300) {
                self.st.save();
                self.reader_relayout();
                self.rd.as_mut().unwrap().panel = None;
                self.rd.as_mut().unwrap().menu = false;
                self.reader_draw(true);
            }
            return;
        }
        // list panels
        let hit = {
            let rd = self.rd.as_mut().unwrap();
            match rd.panel.as_mut() {
                Some(Panel::Toc(lv, _)) | Some(Panel::Marks(lv, _)) | Some(Panel::Search(lv, _)) => lv.hit(x, y),
                _ => None,
            }
        };
        match hit {
            Some(ListHit::Back) => {
                self.rd.as_mut().unwrap().panel = None;
                self.reader_draw(true);
            }
            Some(ListHit::Prev) | Some(ListHit::Next) => self.reader_panel_draw(),
            Some(ListHit::Item(i)) => {
                let rd = self.rd.as_mut().unwrap();
                let pages = rd.pages;
                let target = match rd.panel.as_ref() {
                    Some(Panel::Toc(_, p)) | Some(Panel::Search(_, p)) => p.get(i).copied(),
                    Some(Panel::Marks(_, refs)) => {
                        let key = rd.item.key.clone();
                        let (is_h, idx) = refs[i];
                        let b = self.st.book(&key);
                        let f = if is_h { b.highlights[idx].frac } else { b.bookmarks[idx] };
                        Some((f * (pages - 1) as f32).round() as usize)
                    }
                    _ => None,
                };
                if let Some(t) = target {
                    self.rd_jump(t);
                    let rd = self.rd.as_mut().unwrap();
                    rd.panel = None;
                    rd.menu = false;
                    self.reader_draw(true);
                }
            }
            None => {}
        }
    }

    // ---- selection / highlight actions (from the long-press sheet) ----
    pub fn reader_hl_action(&mut self, act: Act) {
        let Some(rd) = self.rd.as_ref() else { return };
        let Some(sel) = rd.sel else { return };
        let words = rd.words.clone();
        let (lo, hi) = match act {
            Act::HlWord => (sel, sel),
            _ => {
                let end = |t: &str| t.ends_with(['.', '!', '?', '…']) || t.ends_with(".”") || t.ends_with("?”") || t.ends_with("!”");
                // a big vertical gap between consecutive words means a new block (heading, paragraph)
                let gap = |a: &Word, b: &Word| (b.r[1] - a.r[3]) > (b.r[3] - b.r[1]) * 0.7;
                let mut lo = sel;
                while lo > 0 && sel - lo < 60 && !end(&words[lo - 1].t) && !gap(&words[lo - 1], &words[lo]) {
                    lo -= 1;
                }
                let mut hi = sel;
                while hi + 1 < words.len() && hi - sel < 60 && !end(&words[hi].t) && !gap(&words[hi], &words[hi + 1]) {
                    hi += 1;
                }
                (lo, hi)
            }
        };
        let text = words[lo..=hi].iter().map(|w| w.t.clone()).collect::<Vec<_>>().join(" ");
        let rd = self.rd.as_ref().unwrap();
        let frac = if rd.pages > 1 { rd.page as f32 / (rd.pages - 1) as f32 } else { 0.0 };
        let key = rd.item.key.clone();
        let b = self.st.book(&key);
        b.highlights.push(Highlight { text, frac, note: String::new() });
        let idx = b.highlights.len() - 1;
        self.st.save();
        self.rd.as_mut().unwrap().sel = None;
        if act == Act::HlNote {
            self.open_kbd("Note for this highlight", "", KbdFor::HlNote(idx));
        } else {
            self.reader_draw(false);
        }
    }

    pub fn reader_define(&mut self) {
        let Some(rd) = self.rd.as_ref() else { return };
        let Some(sel) = rd.sel else { return };
        let raw = rd.words[sel].t.clone();
        let w = raw.split(|c| c == '—' || c == '–').next().unwrap_or("").trim_matches(|c: char| !c.is_alphanumeric()).to_string();
        let body = crate::dict::lookup(&self.st.dir, &w);
        if body.is_some() {
            let rd = self.rd.as_ref().unwrap();
            let sentence = sentence_around(&rd.words, sel);
            let book = self.st.d.books.get(&rd.item.key).map(|b| b.title.clone()).unwrap_or_default();
            self.vocab_add(&w, &sentence, &book);
        }
        self.reader_paint(); // wipe whatever was open, without a refresh
        let card = Card::new(
            &mut self.cv,
            &w,
            &body.unwrap_or_else(|| "No definition found in the offline dictionary.".into()),
            vec![("Highlight", Act::HlWord), ("Sentence", Act::HlSentence), ("Note", Act::HlNote)],
        );
        self.modal = Some(Modal::Card(card));
        self.draw_modal();
    }

    pub fn light_sheet(&mut self) {
        self.modal = Some(Modal::Sheet(Sheet::new(
            &format!("Light: {}", self.st.d.settings.light),
            vec![("Brighter", Act::LightUp), ("Dimmer", Act::LightDown), ("Close", Act::Close)],
        )));
        self.draw_modal();
    }

    pub fn reader_hl_note(&mut self, idx: usize, note: String) {
        if let Some(rd) = self.rd.as_ref() {
            let key = rd.item.key.clone();
            if let Some(h) = self.st.book(&key).highlights.get_mut(idx) {
                h.note = note;
            }
            self.st.save();
        }
        self.reader_draw(false);
    }
}
