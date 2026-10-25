//! Shared widgets: buttons, icons, action sheet, on-screen keyboard, info card, paged list.
use crate::gfx::*;

// ---------- icons (drawn with lines so no icon font is needed) ----------
pub fn chevron(cv: &mut Canvas, cx: i32, cy: i32, s: i32, left: bool, c: u8) {
    let d = if left { -1 } else { 1 };
    cv.line(cx - d * s / 2, cy - s, cx + d * s / 2, cy, 5, c);
    cv.line(cx + d * s / 2, cy, cx - d * s / 2, cy + s, 5, c);
}

pub fn icon(cv: &mut Canvas, name: &str, cx: i32, cy: i32, c: u8) {
    match name {
        "home" => {
            cv.line(cx - 26, cy + 2, cx, cy - 22, 5, c);
            cv.line(cx, cy - 22, cx + 26, cy + 2, 5, c);
            cv.frame(cx - 18, cy + 2, 36, 22, 5, c);
        }
        "books" => {
            cv.frame(cx - 24, cy - 22, 18, 46, 5, c);
            cv.frame(cx - 4, cy - 22, 18, 46, 5, c);
            cv.line(cx + 18, cy - 20, cx + 28, cy + 22, 5, c);
        }
        "music" => {
            cv.disc(cx - 10, cy + 16, 9, c);
            cv.disc(cx + 18, cy + 10, 9, c);
            cv.line(cx - 2, cy + 14, cx - 2, cy - 22, 5, c);
            cv.line(cx + 26, cy + 8, cx + 26, cy - 28, 5, c);
            cv.line(cx - 2, cy - 22, cx + 26, cy - 28, 6, c);
        }
        "notes" => {
            cv.frame(cx - 20, cy - 24, 40, 50, 5, c);
            cv.line(cx - 10, cy - 8, cx + 10, cy - 8, 4, c);
            cv.line(cx - 10, cy + 4, cx + 10, cy + 4, 4, c);
            cv.line(cx - 10, cy + 16, cx + 2, cy + 16, 4, c);
        }
        "more" => {
            for i in -1..=1 {
                cv.disc(cx + i * 22, cy, 6, c);
            }
        }
        "play" => {
            for dy in -24..=24i32 {
                let w = 38 - 38 * dy.abs() / 25;
                cv.fill(cx - 15, cy + dy, w.max(1), 1, c);
            }
        }
        "pause" => {
            cv.fill(cx - 16, cy - 20, 10, 40, c);
            cv.fill(cx + 6, cy - 20, 10, 40, c);
        }
        "next" => {
            for i in 0..22 {
                cv.line(cx - 16 + i / 2, cy - 18 + i * 18 / 22, cx - 16 + i / 2, cy + 18 - i * 18 / 22, 2, c);
            }
            cv.fill(cx + 10, cy - 18, 6, 36, c);
        }
        "prev" => {
            for i in 0..22 {
                cv.line(cx + 16 - i / 2, cy - 18 + i * 18 / 22, cx + 16 - i / 2, cy + 18 - i * 18 / 22, 2, c);
            }
            cv.fill(cx - 16, cy - 18, 6, 36, c);
        }
        "search" => {
            cv.rrect(cx - 18, cy - 20, 32, 32, 16, c, 5);
            cv.line(cx + 8, cy + 10, cx + 22, cy + 24, 6, c);
        }
        "list" => {
            for i in -1..=1 {
                cv.line(cx - 22, cy + i * 14, cx + 22, cy + i * 14, 5, c);
            }
        }
        "font" => {
            cv.line(cx - 22, cy + 20, cx - 8, cy - 22, 5, c);
            cv.line(cx - 8, cy - 22, cx + 6, cy + 20, 5, c);
            cv.line(cx - 16, cy + 6, cx, cy + 6, 4, c);
            cv.line(cx + 12, cy + 20, cx + 20, cy - 6, 4, c);
            cv.line(cx + 20, cy - 6, cx + 28, cy + 20, 4, c);
        }
        "mark" => {
            cv.frame(cx - 16, cy - 24, 32, 48, 5, c);
            cv.fill(cx - 6, cy + 4, 12, 20, WHITE);
            cv.line(cx - 6, cy + 4, cx, cy + 14, 5, c);
            cv.line(cx + 6, cy + 4, cx, cy + 14, 5, c);
        }
        "marker" => {
            cv.line(cx - 20, cy + 22, cx + 14, cy - 12, 8, c);
            cv.line(cx + 14, cy - 12, cx + 22, cy - 20, 5, c);
            cv.line(cx - 24, cy + 24, cx - 6, cy + 24, 4, c);
        }
        "sun" => {
            cv.rrect(cx - 10, cy - 10, 20, 20, 10, c, 0);
            for k in 0..8 {
                let a = k as f32 * std::f32::consts::FRAC_PI_4;
                cv.line(cx + (a.cos() * 17.0) as i32, cy + (a.sin() * 17.0) as i32, cx + (a.cos() * 25.0) as i32, cy + (a.sin() * 25.0) as i32, 4, c);
            }
        }
        "close" => {
            cv.line(cx - 16, cy - 16, cx + 16, cy + 16, 6, c);
            cv.line(cx + 16, cy - 16, cx - 16, cy + 16, 6, c);
        }
        "back" => chevron(cv, cx, cy, 20, true, c),
        "pen" => {
            cv.line(cx - 20, cy + 22, cx + 16, cy - 14, 7, c);
            cv.line(cx - 22, cy + 24, cx - 14, cy + 22, 4, c);
        }
        "eraser" => {
            cv.rrect(cx - 22, cy - 12, 44, 28, 6, c, 5);
            cv.line(cx - 4, cy - 12, cx - 4, cy + 16, 4, c);
        }
        "plus" => {
            cv.line(cx - 18, cy, cx + 18, cy, 6, c);
            cv.line(cx, cy - 18, cx, cy + 18, 6, c);
        }
        "minus" => cv.line(cx - 18, cy, cx + 18, cy, 6, c),
        "undo" => {
            cv.line(cx - 18, cy - 4, cx + 14, cy - 4, 5, c);
            cv.line(cx + 14, cy - 4, cx + 14, cy + 14, 5, c);
            cv.line(cx - 18, cy - 4, cx - 8, cy - 14, 5, c);
            cv.line(cx - 18, cy - 4, cx - 8, cy + 6, 5, c);
        }
        _ => {}
    }
}

// ---------- buttons ----------
pub fn button(cv: &mut Canvas, r: Rect, label: &str, primary: bool) {
    if primary {
        cv.rrect(r.x0, r.y0, r.w(), r.h(), 14, BLACK, 0);
        cv.text_c(F::SansM, 32.0, (r.x0 + r.x1) / 2, r.y0 + (r.h() - 40) / 2, label, WHITE);
    } else {
        cv.rrect(r.x0, r.y0, r.w(), r.h(), 14, DARK, 3);
        cv.text_c(F::SansM, 32.0, (r.x0 + r.x1) / 2, r.y0 + (r.h() - 40) / 2, label, BLACK);
    }
}

pub fn icon_button(cv: &mut Canvas, r: Rect, name: &str, filled: bool) {
    if filled {
        cv.rrect(r.x0, r.y0, r.w(), r.h(), r.h() / 2, BLACK, 0);
        icon(cv, name, (r.x0 + r.x1) / 2, (r.y0 + r.y1) / 2, WHITE);
    } else {
        icon(cv, name, (r.x0 + r.x1) / 2, (r.y0 + r.y1) / 2, BLACK);
    }
}

pub fn progress(cv: &mut Canvas, x: i32, y: i32, w: i32, frac: f32) {
    cv.rrect(x, y, w, 8, 4, LIGHT, 0);
    let fw = ((w as f32) * frac.clamp(0.0, 1.0)) as i32;
    if fw > 8 {
        cv.rrect(x, y, fw, 8, 4, BLACK, 0);
    }
}

pub fn fmt_time(secs: u32) -> String {
    format!("{}:{:02}", secs / 60, secs % 60)
}

// ---------- actions ----------
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Act {
    Close,
    Toc,
    Search,
    FontPanel,
    Marks,
    Notes,
    Library,
    EmUp,
    EmDown,
    MarginUp,
    MarginDown,
    LightUp,
    LightDown,
    Bookmark,
    HlWord,
    HlSentence,
    HlNote,
    Define,
    Unhighlight,
    DeleteNote,
    RenameNote,
    NewNotebook,
    NewText,
    ExportNote,
    BtForget,
    Calibrate,
    Rescan,
    Sleep,
    Quit,
    UsbNet,
    Reboot,
    Poweroff,
    ToStock,
    WordShow,
    WordNext,
    WordDrop,
    Noop,
}

// ---------- action sheet ----------
pub struct Sheet {
    pub title: String,
    pub items: Vec<(String, Act)>,
    pub rects: Vec<Rect>,
    pub panel: Rect,
}

impl Sheet {
    pub fn new(title: &str, items: Vec<(&str, Act)>) -> Sheet {
        let rowh = 112;
        let hh = if title.is_empty() { 0 } else { 100 };
        let h = hh + rowh * items.len() as i32 + 40;
        let y = H - h;
        let rects = (0..items.len()).map(|i| Rect::new(40, y + hh + i as i32 * rowh, W - 80, rowh)).collect();
        Sheet { title: title.into(), items: items.into_iter().map(|(a, b)| (a.to_string(), b)).collect(), rects, panel: Rect::new(0, y - 6, W, h + 6) }
    }

    pub fn draw(&self, cv: &mut Canvas) {
        let p = self.panel;
        cv.fill(p.x0, p.y0, p.w(), p.h(), WHITE);
        cv.fill(p.x0, p.y0, p.w(), 5, BLACK);
        if !self.title.is_empty() {
            cv.text(F::SerifB, 38.0, 44, p.y0 + 30, &self.title, BLACK);
        }
        for (i, (label, _)) in self.items.iter().enumerate() {
            let r = self.rects[i];
            cv.text(F::SansM, 36.0, r.x0 + 4, r.y0 + 32, label, BLACK);
            if i + 1 < self.items.len() {
                cv.fill(r.x0, r.y1 - 2, r.w(), 2, LIGHT);
            }
        }
    }

    pub fn hit(&self, x: i32, y: i32) -> Option<Act> {
        self.rects.iter().position(|r| r.contains(x, y)).map(|i| self.items[i].1)
    }
}

// ---------- info card (dictionary entries, messages) ----------
pub struct Card {
    pub title: String,
    pub lines: Vec<String>,
    pub scroll: usize,
    pub actions: Vec<(String, Act)>,
}

pub const CARD_LINES: usize = 12;

impl Card {
    pub fn new(cv: &mut Canvas, title: &str, body: &str, actions: Vec<(&str, Act)>) -> Card {
        let mut lines = vec![];
        for para in body.split('\n') {
            if para.trim().is_empty() {
                lines.push(String::new());
            } else {
                lines.extend(cv.wrap(F::Serif, 34.0, para, W - 140, 200));
            }
        }
        Card { title: title.into(), lines, scroll: 0, actions: actions.into_iter().map(|(a, b)| (a.to_string(), b)).collect() }
    }

    pub fn panel() -> Rect {
        Rect::new(40, 200, W - 80, 1000)
    }

    pub fn btn_rects(&self) -> Vec<Rect> {
        let p = Card::panel();
        let n = self.actions.len() as i32 + 1;
        let w = (p.w() - 60 - (n - 1) * 20) / n;
        (0..n).map(|i| Rect::new(p.x0 + 30 + i * (w + 20), p.y1 - 120, w, 90)).collect()
    }

    pub fn draw(&self, cv: &mut Canvas) {
        let p = Card::panel();
        cv.rrect(p.x0 - 4, p.y0 - 4, p.w() + 8, p.h() + 8, 22, BLACK, 0);
        cv.rrect(p.x0, p.y0, p.w(), p.h(), 20, WHITE, 0);
        cv.text(F::SerifB, 50.0, p.x0 + 40, p.y0 + 30, &self.title, BLACK);
        cv.fill(p.x0 + 40, p.y0 + 104, p.w() - 80, 3, LIGHT);
        for (i, l) in self.lines.iter().skip(self.scroll).take(CARD_LINES).enumerate() {
            cv.text(F::Serif, 34.0, p.x0 + 40, p.y0 + 130 + i as i32 * 54, l, DARK);
        }
        if self.lines.len() > CARD_LINES {
            let more = self.scroll + CARD_LINES < self.lines.len();
            cv.text_r(F::Sans, 26.0, p.x1 - 40, p.y1 - 160, if more { "tap text to continue" } else { "end" }, MID);
        }
        let rects = self.btn_rects();
        for (i, a) in self.actions.iter().enumerate() {
            button(cv, rects[i], &a.0, false);
        }
        button(cv, *rects.last().unwrap(), "Close", true);
    }

    /// Returns Some(act) when a button is pressed; scrolls on body taps.
    pub fn tap(&mut self, x: i32, y: i32) -> Option<Act> {
        let rects = self.btn_rects();
        for (i, r) in rects.iter().enumerate() {
            if r.contains(x, y) {
                return Some(if i < self.actions.len() { self.actions[i].1 } else { Act::Close });
            }
        }
        if Card::panel().contains(x, y) {
            if self.scroll + CARD_LINES < self.lines.len() {
                self.scroll += CARD_LINES - 2;
            } else {
                self.scroll = 0;
            }
            return Some(Act::Noop);
        }
        Some(Act::Close)
    }
}

// ---------- on-screen keyboard ----------
#[derive(Clone, Copy, PartialEq)]
pub enum Key {
    Ch(char),
    Shift,
    Back,
    Num,
    Space,
    Enter,
    Done,
    Cancel,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum KbdFor {
    Search,
    NoteTitle,
    NoteText(usize),
    HlNote(usize),
    LibFilter,
}

pub enum KbdRes {
    Nothing,
    Input,
    Keys,
    Submit(bool),
}

pub struct Kbd {
    pub prompt: String,
    pub text: String,
    pub shift: bool,
    pub num: bool,
    pub purpose: KbdFor,
    pub keys: Vec<(Rect, Key)>,
}

pub const KBD_TOP: i32 = H - 660;

impl Kbd {
    pub fn new(prompt: &str, text: &str, purpose: KbdFor) -> Kbd {
        let mut k = Kbd { prompt: prompt.into(), text: text.into(), shift: false, num: false, purpose, keys: vec![] };
        k.layout();
        k
    }

    pub fn multiline(&self) -> bool {
        matches!(self.purpose, KbdFor::NoteText(_))
    }

    fn layout(&mut self) {
        let rows: [&str; 3] = if self.num { ["1234567890", "-/:;()&@\"'", "#+=.,?!%*_"] } else { ["qwertyuiop", "asdfghjkl", "zxcvbnm"] };
        let kh = 118;
        let top = KBD_TOP + 140;
        self.keys.clear();
        for (ri, row) in rows.iter().enumerate() {
            let n = row.chars().count() as i32;
            let kw = 104;
            let x0 = (W - n * kw) / 2;
            for (i, ch) in row.chars().enumerate() {
                let r = Rect::new(x0 + i as i32 * kw + 4, top + ri as i32 * kh + 4, kw - 8, kh - 8);
                self.keys.push((r, Key::Ch(ch)));
            }
        }
        let y = top + 3 * kh + 4;
        self.keys.push((Rect::new(8, top + 2 * kh + 4, 130, kh - 8), Key::Shift));
        self.keys.push((Rect::new(W - 138, top + 2 * kh + 4, 130, kh - 8), Key::Back));
        self.keys.push((Rect::new(8, y, 150, kh - 8), Key::Num));
        self.keys.push((Rect::new(170, y, W - 170 - 250, kh - 8), Key::Space));
        self.keys.push((Rect::new(W - 238, y, 230, kh - 8), Key::Enter));
        self.keys.push((Rect::new(W - 190, KBD_TOP + 16, 170, 90), Key::Cancel));
        if self.multiline() {
            self.keys.push((Rect::new(W - 380, KBD_TOP + 16, 170, 90), Key::Done));
        }
    }

    pub fn input_rect(&self) -> Rect {
        Rect::new(0, KBD_TOP, W - 200, 130)
    }

    pub fn draw_input(&self, cv: &mut Canvas) {
        let r = self.input_rect();
        cv.fill(r.x0, r.y0, r.w(), r.h(), WHITE);
        cv.text(F::Sans, 26.0, 36, KBD_TOP + 16, &self.prompt, MID);
        let mut t = self.text.replace('\n', " ¶ ");
        while cv.text_w(F::SansM, 40.0, &t) > W - 300 && !t.is_empty() {
            t.remove(0);
        }
        cv.text(F::SansM, 40.0, 36, KBD_TOP + 62, &format!("{}|", t), BLACK);
    }

    pub fn draw(&mut self, cv: &mut Canvas) {
        self.layout();
        cv.fill(0, KBD_TOP - 6, W, H - KBD_TOP + 6, WHITE);
        cv.fill(0, KBD_TOP - 6, W, 5, BLACK);
        self.draw_input(cv);
        for (r, k) in self.keys.clone() {
            let (label, dark) = match k {
                Key::Ch(c) => (c.to_string(), false),
                Key::Shift => ("Shift".to_string(), self.shift),
                Key::Back => ("Del".to_string(), false),
                Key::Num => (if self.num { "abc".to_string() } else { "123".to_string() }, false),
                Key::Space => ("space".to_string(), false),
                Key::Enter => (if self.multiline() { "Return".to_string() } else { "Go".to_string() }, !self.multiline()),
                Key::Done => ("Done".to_string(), true),
                Key::Cancel => ("Cancel".to_string(), false),
            };
            if dark {
                cv.rrect(r.x0, r.y0, r.w(), r.h(), 12, BLACK, 0);
                cv.text_c(F::SansM, 34.0, (r.x0 + r.x1) / 2, r.y0 + (r.h() - 44) / 2, &label, WHITE);
            } else if k == Key::Cancel {
                cv.text_c(F::SansM, 30.0, (r.x0 + r.x1) / 2, r.y0 + (r.h() - 40) / 2, &label, DARK);
            } else {
                cv.rrect(r.x0, r.y0, r.w(), r.h(), 12, PALE, 0);
                cv.rrect(r.x0, r.y0, r.w(), r.h(), 12, LIGHT, 2);
                cv.text_c(F::SansM, 40.0, (r.x0 + r.x1) / 2, r.y0 + (r.h() - 50) / 2, &label, BLACK);
            }
        }
    }

    pub fn tap(&mut self, x: i32, y: i32) -> KbdRes {
        let Some(hit) = self.keys.iter().find(|(r, _)| r.contains(x, y)).map(|(_, k)| *k) else { return KbdRes::Nothing };
        match hit {
            Key::Ch(c) => {
                self.text.push(if self.shift && !self.num { c.to_ascii_uppercase() } else { c });
                if self.shift {
                    self.shift = false;
                    return KbdRes::Keys;
                }
                KbdRes::Input
            }
            Key::Shift => {
                self.shift = !self.shift;
                KbdRes::Keys
            }
            Key::Back => {
                self.text.pop();
                KbdRes::Input
            }
            Key::Num => {
                self.num = !self.num;
                KbdRes::Keys
            }
            Key::Space => {
                self.text.push(' ');
                KbdRes::Input
            }
            Key::Enter => {
                if self.multiline() {
                    self.text.push('\n');
                    KbdRes::Input
                } else {
                    KbdRes::Submit(true)
                }
            }
            Key::Done => KbdRes::Submit(true),
            Key::Cancel => KbdRes::Submit(false),
        }
    }
}

// ---------- paged list ----------
pub struct ListView {
    pub title: String,
    pub items: Vec<(String, String)>, // (label, right-aligned detail)
    pub page: usize,
    pub per_page: usize,
    pub top: i32,
    pub rowh: i32,
}

pub enum ListHit {
    Item(usize),
    Back,
    Prev,
    Next,
}

impl ListView {
    pub fn new(title: &str, items: Vec<(String, String)>) -> ListView {
        ListView { title: title.into(), items, page: 0, per_page: 11, top: 150, rowh: 100 }
    }

    pub fn pages(&self) -> usize {
        ((self.items.len() + self.per_page - 1) / self.per_page).max(1)
    }

    pub fn draw(&self, cv: &mut Canvas, indent_of: &dyn Fn(usize) -> i32) {
        cv.fill(0, 0, W, H, WHITE);
        icon(cv, "back", 60, 78, BLACK);
        cv.text(F::SerifB, 46.0, 120, 48, &self.title, BLACK);
        cv.fill(36, 128, W - 72, 3, BLACK);
        let start = self.page * self.per_page;
        if self.items.is_empty() {
            cv.text_c(F::Serif, 38.0, W / 2, 400, "Nothing here yet", MID);
        }
        for (i, (label, detail)) in self.items.iter().skip(start).take(self.per_page).enumerate() {
            let y = self.top + i as i32 * self.rowh;
            let ind = indent_of(start + i);
            let dw = cv.text_w(F::Sans, 28.0, detail);
            let l = cv.fit(F::Serif, 34.0, label, W - 120 - ind - dw - 30);
            cv.text(F::Serif, 34.0, 50 + ind, y + 26, &l, BLACK);
            cv.text_r(F::Sans, 28.0, W - 50, y + 32, detail, MID);
            cv.fill(50, y + self.rowh - 2, W - 100, 2, LIGHT);
        }
        let ny = H - 120;
        if self.pages() > 1 {
            cv.text_c(F::Sans, 28.0, W / 2, ny + 30, &format!("{} / {}", self.page + 1, self.pages()), MID);
            chevron(cv, 130, ny + 44, 22, true, if self.page > 0 { BLACK } else { LIGHT });
            chevron(cv, W - 130, ny + 44, 22, false, if self.page + 1 < self.pages() { BLACK } else { LIGHT });
        }
    }

    pub fn hit(&mut self, x: i32, y: i32) -> Option<ListHit> {
        if y < 130 && x < 400 {
            return Some(ListHit::Back);
        }
        if y > H - 150 && self.pages() > 1 {
            if x < W / 3 && self.page > 0 {
                self.page -= 1;
                return Some(ListHit::Prev);
            }
            if x > 2 * W / 3 && self.page + 1 < self.pages() {
                self.page += 1;
                return Some(ListHit::Next);
            }
            return None;
        }
        if y >= self.top {
            let i = ((y - self.top) / self.rowh) as usize;
            let idx = self.page * self.per_page + i;
            if i < self.per_page && idx < self.items.len() {
                return Some(ListHit::Item(idx));
            }
        }
        None
    }
}
