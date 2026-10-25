//! Tab screens: Home, Library, Music, Settings ("More"), calibration, and the shared chrome (status bar, tabs, mini player).
use crate::gfx::*;
use crate::library;
use crate::music::Track;
use crate::ui::*;
use crate::{App, Screen};

pub const TAB_Y: i32 = H - 130;
pub const MINI_Y: i32 = TAB_Y - 100;
/// Recordings longer than this (20 min) are treated as audiobooks: the position is remembered, ±30 s buttons.
pub const AUDIOBOOK_SECS: u32 = 1200;
const TABS: [(&str, &str, Screen); 5] = [
    ("home", "Home", Screen::Home),
    ("books", "Library", Screen::Books),
    ("music", "Music", Screen::Music),
    ("notes", "Notes", Screen::Notes),
    ("more", "More", Screen::More),
];

pub fn greeting() -> &'static str {
    match crate::sys::hour() {
        5..=11 => "Good morning",
        12..=17 => "Good afternoon",
        18..=22 => "Good evening",
        _ => "Good night",
    }
}

impl App {
    // ---------------- chrome ----------------
    pub fn status_bar(&mut self) {
        let bat = crate::sys::battery();
        self.cv.text(F::SansM, 26.0, 40, 22, &crate::sys::clock(), DARK);
        let mut right = W - 40;
        if bat >= 0 {
            // battery glyph
            self.cv.frame(right - 58, 24, 52, 24, 3, DARK);
            self.cv.fill(right - 6, 31, 5, 10, DARK);
            self.cv.fill(right - 54, 28, (44 * bat / 100).max(2), 16, DARK);
            right -= 76;
            self.cv.text_r(F::Sans, 24.0, right, 24, &format!("{}%", bat), DARK);
            right -= 80;
        }
        if crate::sys::audio_output_is_bt() {
            self.cv.text_r(F::SansM, 22.0, right, 26, "BT audio", DARK);
        }
    }

    pub fn draw_chrome(&mut self) {
        self.status_bar();
        // tab bar
        self.cv.fill(0, TAB_Y, W, 130, WHITE);
        self.cv.fill(0, TAB_Y, W, 2, LIGHT);
        let tw = W / 5;
        for (i, (ic, label, scr)) in TABS.iter().enumerate() {
            let on = self.screen == *scr;
            let cx = i as i32 * tw + tw / 2;
            let col = if on { BLACK } else { MID };
            icon(&mut self.cv, ic, cx, TAB_Y + 48, col);
            self.cv.text_c(if on { F::SansB } else { F::Sans }, 22.0, cx, TAB_Y + 88, label, col);
            if on {
                self.cv.rrect(cx - 30, TAB_Y, 60, 6, 3, BLACK, 0);
            }
        }
        self.draw_mini_player();
    }

    fn mini_visible(&self) -> bool {
        let s = self.player.st.lock().unwrap();
        s.idx.is_some() && self.screen != Screen::Music
    }

    fn draw_mini_player(&mut self) {
        if !self.mini_visible() {
            return;
        }
        let (t, playing) = {
            let s = self.player.st.lock().unwrap();
            (s.queue.get(s.idx.unwrap()).cloned(), s.playing)
        };
        let Some(t) = t else { return };
        self.cv.fill(0, MINI_Y, W, 100, PALE);
        self.cv.fill(0, MINI_Y, W, 2, LIGHT);
        let title = self.cv.fit(F::SansM, 30.0, &t.title, W - 330);
        self.cv.text(F::SansM, 30.0, 40, MINI_Y + 16, &title, BLACK);
        let artist = self.cv.fit(F::Sans, 24.0, &t.artist, W - 330);
        self.cv.text(F::Sans, 24.0, 40, MINI_Y + 58, &artist, MID);
        icon(&mut self.cv, if playing { "pause" } else { "play" }, W - 170, MINI_Y + 50, BLACK);
        icon(&mut self.cv, "next", W - 70, MINI_Y + 50, BLACK);
    }

    /// Returns true if the tap was consumed by chrome.
    pub fn chrome_tap(&mut self, x: i32, y: i32) -> bool {
        if y >= TAB_Y {
            let i = (x / (W / 5)).min(4) as usize;
            let scr = TABS[i].2;
            if self.screen != scr {
                self.screen = scr;
                self.draw_screen(false);
            }
            return true;
        }
        if self.mini_visible() && y >= MINI_Y {
            if x > W - 120 {
                let _ = self.player.tx.send(crate::music::Cmd::Next);
            } else if x > W - 240 {
                let playing = self.player.st.lock().unwrap().playing;
                let _ = self.player.tx.send(if playing { crate::music::Cmd::Pause } else { crate::music::Cmd::Resume });
            } else {
                self.screen = Screen::Music;
                self.draw_screen(false);
            }
            return true;
        }
        false
    }

    pub fn draw_screen(&mut self, flash: bool) {
        match self.screen {
            Screen::Reader => return self.reader_draw(flash),
            Screen::Ink => return self.ink_draw(),
            Screen::TextNote => {
                self.text_note_draw();
                self.cv.present(Wf::Text, flash);
                return;
            }
            Screen::Calibrate { step, .. } => return self.draw_calibrate(step),
            Screen::Bluetooth => return self.draw_bt(),
            Screen::Usb => return self.draw_usb(),
            Screen::Lock => return self.draw_lock(),
            _ => {}
        }
        self.cv.clear(WHITE);
        match self.screen {
            Screen::Home => self.draw_home(),
            Screen::Books => self.draw_books(),
            Screen::Music => self.draw_music(),
            Screen::Notes => self.draw_notes_tab(),
            Screen::More => self.draw_more(),
            _ => {}
        }
        self.draw_chrome();
        self.cv.present(if flash { Wf::Full } else { Wf::Text }, flash);
    }

    // ---------------- covers ----------------
    pub fn draw_cover(&mut self, it: &library::Item, x: i32, y: i32, w: i32, h: i32) {
        let cp = library::cover_path(&self.st.dir, &it.key);
        if let Some((cw, ch, px)) = library::read_pgm(&cp).filter(|(_, _, p)| !library::mostly_blank(p)) {
            self.cv.blit_scaled(x, y, w, h, cw, ch, &px);
            self.cv.frame(x, y, w, h, 2, LIGHT);
            return;
        }
        // typographic placeholder
        self.cv.fill(x, y, w, h, PALE);
        self.cv.frame(x, y, w, h, 3, DARK);
        self.cv.frame(x + 12, y + 12, w - 24, h - 24, 1, MID);
        let initial: String = it.title.chars().find(|c| c.is_alphanumeric()).unwrap_or('•').to_uppercase().collect();
        self.cv.text_c(F::SerifB, (h as f32) * 0.28, x + w / 2, y + h / 12, &initial, DARK);
        let px = (w as f32 * 0.085).max(18.0);
        let lines = self.cv.wrap(F::SerifB, px, &it.title, w - 50, 4);
        for (i, l) in lines.iter().enumerate() {
            self.cv.text_c(F::SerifB, px, x + w / 2, y + h / 2 + 10 + i as i32 * (px * 1.25) as i32, l, BLACK);
        }
        self.cv.text_c(F::Sans, px * 0.8, x + w / 2, y + h - 54, &it.ext.to_uppercase(), MID);
    }

    fn book_frac(&self, key: &str) -> f32 {
        self.st.d.books.get(key).map(|b| b.frac).unwrap_or(0.0)
    }

    // ---------------- home ----------------
    pub fn draw_home(&mut self) {
        self.cv.text(F::SerifB, 66.0, 50, 90, greeting(), BLACK);
        self.cv.text(F::Sans, 30.0, 52, 176, &crate::sys::date_long(), MID);
        // continue reading
        let last = self.st.d.settings.last_book.clone().and_then(|k| self.items.iter().find(|i| i.key == k).cloned());
        let cur = last.or_else(|| self.items.first().cloned());
        self.cv.fill(50, 240, W - 100, 3, BLACK);
        match cur {
            Some(it) => {
                self.draw_cover(&it, 50, 280, 300, 400);
                self.cv.text(F::SansM, 24.0, 390, 288, "CONTINUE READING", MID);
                let title = self.st.d.books.get(&it.key).map(|b| b.title.clone()).filter(|t| !t.is_empty()).unwrap_or(it.title.clone());
                let lines = self.cv.wrap(F::SerifB, 44.0, &title, W - 440, 4);
                for (i, l) in lines.iter().enumerate() {
                    self.cv.text(F::SerifB, 44.0, 390, 336 + i as i32 * 58, l, BLACK);
                }
                let f = self.book_frac(&it.key);
                progress(&mut self.cv, 390, 540, W - 440, f);
                self.cv.text(F::Sans, 26.0, 390, 560, &format!("{}% read", (f * 100.0) as i32), MID);
                button(&mut self.cv, Rect::new(390, 616, 300, 74), if f > 0.0 { "Resume" } else { "Start reading" }, true);
            }
            None => {
                self.cv.text(F::SerifB, 48.0, 50, 300, "Your library is empty", BLACK);
                let lines = self.cv.wrap(F::Serif, 34.0, "Copy EPUB, PDF, MOBI, AZW3, CBZ, FB2 or DjVu files into the “books” folder when Folio is plugged into a computer.", W - 100, 6);
                for (i, l) in lines.iter().enumerate() {
                    self.cv.text(F::Serif, 34.0, 50, 380 + i as i32 * 50, l, DARK);
                }
            }
        }
        // recent row
        let mut recent: Vec<library::Item> = self.items.clone();
        recent.sort_by_key(|i| std::cmp::Reverse(self.st.d.books.get(&i.key).map(|b| b.opened).unwrap_or(0)));
        if recent.len() > 1 {
            self.cv.text(F::SerifB, 40.0, 50, 750, "Your books", BLACK);
            let (cw, ch) = (224, 299);
            let gap = (W - 100 - 4 * cw) / 3;
            for (i, it) in recent.iter().take(4).enumerate() {
                let x = 50 + i as i32 * (cw + gap);
                self.draw_cover(it, x, 820, cw, ch);
                let t = self.cv.fit(F::Sans, 24.0, &it.title, cw);
                self.cv.text(F::Sans, 24.0, x, 1130, &t, DARK);
                let f = self.book_frac(&it.key);
                if f > 0.0 {
                    progress(&mut self.cv, x, 1170, cw, f);
                }
            }
        }
    }

    pub fn home_tap(&mut self, x: i32, y: i32) {
        let last = self.st.d.settings.last_book.clone().and_then(|k| self.items.iter().find(|i| i.key == k).cloned());
        let cur = last.or_else(|| self.items.first().cloned());
        if let Some(it) = cur {
            if (y > 270 && y < 710 && x < 700) || (Rect::new(390, 616, 300, 74).contains(x, y)) {
                return self.open_book(it);
            }
        }
        if y >= 820 && y < 1200 {
            let mut recent: Vec<library::Item> = self.items.clone();
            recent.sort_by_key(|i| std::cmp::Reverse(self.st.d.books.get(&i.key).map(|b| b.opened).unwrap_or(0)));
            let (cw, ch) = (224, 299);
            let gap = (W - 100 - 4 * cw) / 3;
            for (i, it) in recent.iter().take(4).enumerate() {
                let rx = 50 + i as i32 * (cw + gap);
                if Rect::new(rx, 820, cw, ch + 60).contains(x, y) {
                    return self.open_book(it.clone());
                }
            }
        }
    }

    // ---------------- library ----------------
    fn filtered(&self) -> Vec<library::Item> {
        let q = self.lib_query.to_lowercase();
        let mut v: Vec<library::Item> = self.items.iter().filter(|i| q.is_empty() || i.title.to_lowercase().contains(&q)).cloned().collect();
        if self.lib_sort == 1 {
            v.sort_by_key(|i| std::cmp::Reverse(self.st.d.books.get(&i.key).map(|b| b.opened).unwrap_or(0)));
        }
        v
    }

    pub fn draw_books(&mut self) {
        let v = self.filtered();
        let pages = ((v.len() + 5) / 6).max(1);
        self.lib_page = self.lib_page.min(pages - 1);
        self.cv.text(F::SerifB, 66.0, 50, 90, "Library", BLACK);
        let sub = if self.lib_query.is_empty() { format!("{} {} · sorted by {}", v.len(), if v.len() == 1 { "book" } else { "books" }, if self.lib_sort == 0 { "title" } else { "recent" }) } else { format!("{} matching “{}”", v.len(), self.lib_query) };
        self.cv.text(F::Sans, 26.0, 52, 176, &sub, MID);
        icon(&mut self.cv, "search", W - 330, 150, BLACK);
        chevron(&mut self.cv, W - 230, 150, 18, true, if self.lib_page > 0 { BLACK } else { LIGHT });
        self.cv.text_c(F::Sans, 26.0, W - 150, 134, &format!("{}/{}", self.lib_page + 1, pages), DARK);
        chevron(&mut self.cv, W - 70, 150, 18, false, if self.lib_page + 1 < pages { BLACK } else { LIGHT });
        self.cv.fill(50, 220, W - 100, 3, BLACK);
        if v.is_empty() {
            self.cv.text_c(F::Serif, 38.0, W / 2, 500, "No books found", MID);
        }
        let gap = (W - 3 * 300) / 4;
        for (i, it) in v.iter().skip(self.lib_page * 6).take(6).enumerate() {
            let (col, row) = ((i % 3) as i32, (i / 3) as i32);
            let x = gap + col * (300 + gap);
            let y = 250 + row * 480;
            self.draw_cover(it, x, y, 300, 400);
            let t = self.cv.wrap(F::Sans, 24.0, &it.title, 300, 2);
            for (k, l) in t.iter().enumerate() {
                self.cv.text(F::Sans, 24.0, x, y + 408 + k as i32 * 30, l, DARK);
            }
            let f = self.book_frac(&it.key);
            if f > 0.0 {
                let r = Rect::new(x + 208, y + 8, 84, 40);
                self.cv.rrect(r.x0, r.y0, r.w(), r.h(), 20, BLACK, 0);
                self.cv.text_c(F::SansM, 22.0, (r.x0 + r.x1) / 2, r.y0 + 8, &format!("{}%", (f * 100.0) as i32), WHITE);
            }
        }
    }

    pub fn books_tap(&mut self, x: i32, y: i32) {
        let v = self.filtered();
        let pages = ((v.len() + 5) / 6).max(1);
        if y < 220 {
            if x > W - 380 && x < W - 280 {
                if self.lib_query.is_empty() {
                    let q = self.lib_query.clone();
                    self.open_kbd("Search library", &q, KbdFor::LibFilter);
                } else {
                    self.lib_query.clear();
                    self.lib_page = 0;
                    self.draw_screen(false);
                }
            } else if x > W - 280 && x < W - 190 && self.lib_page > 0 {
                self.lib_page -= 1;
                self.draw_screen(false);
            } else if x > W - 120 && self.lib_page + 1 < pages {
                self.lib_page += 1;
                self.draw_screen(false);
            } else if x < 600 && y > 150 {
                self.lib_sort = 1 - self.lib_sort;
                self.draw_screen(false);
            }
            return;
        }
        let gap = (W - 3 * 300) / 4;
        for (i, it) in v.iter().skip(self.lib_page * 6).take(6).enumerate() {
            let (col, row) = ((i % 3) as i32, (i / 3) as i32);
            let r = Rect::new(gap + col * (300 + gap), 250 + row * 480, 300, 460);
            if r.contains(x, y) {
                return self.open_book(it.clone());
            }
        }
    }

    // ---------------- music ----------------
    pub fn draw_music(&mut self) {
        self.cv.text(F::SerifB, 66.0, 50, 90, "Music", BLACK);
        let bt = crate::sys::bt_device();
        let out_bt = crate::sys::audio_output_is_bt();
        let status = if out_bt { if bt.is_empty() { "Bluetooth audio connected".to_string() } else { format!("Playing to {}", bt) } } else { "No Bluetooth headphones connected".to_string() };
        self.cv.text(F::Sans, 26.0, 52, 176, &status, if out_bt { MID } else { DARK });
        self.cv.fill(50, 220, W - 100, 3, BLACK);
        let (cur, playing, pos, dur, vol) = {
            let s = self.player.st.lock().unwrap();
            (s.idx.and_then(|i| s.queue.get(i).cloned()), s.playing, s.pos, s.dur, s.volume)
        };
        let n = self.tracks.len();
        let top;
        if let Some(t) = cur {
            self.draw_art(&t, 50, 250, 340);
            let lines = self.cv.wrap(F::SerifB, 42.0, &t.title, W - 470, 3);
            for (i, l) in lines.iter().enumerate() {
                self.cv.text(F::SerifB, 42.0, 430, 256 + i as i32 * 54, l, BLACK);
            }
            let a = self.cv.fit(F::Sans, 30.0, &t.artist, W - 470);
            self.cv.text(F::Sans, 30.0, 430, 256 + lines.len() as i32 * 54 + 10, &a, MID);
            let frac = if dur > 0 { pos as f32 / dur as f32 } else { 0.0 };
            progress(&mut self.cv, 430, 520, W - 480, frac);
            self.cv.text(F::Sans, 24.0, 430, 540, &fmt_time(pos), MID);
            if dur > 0 {
                self.cv.text_r(F::Sans, 24.0, W - 50, 540, &fmt_time(dur), MID);
            }
            icon(&mut self.cv, "prev", 480, 630, BLACK);
            icon_button(&mut self.cv, Rect::new(600, 575, 110, 110), if playing { "pause" } else { "play" }, true);
            icon(&mut self.cv, "next", 830, 630, BLACK);
            if dur > AUDIOBOOK_SECS {
                button(&mut self.cv, Rect::new(50, 600, 170, 70), "− 30 s", false);
                button(&mut self.cv, Rect::new(235, 600, 185, 70), "+ 30 s", false);
            } else {
                let (sh, rp) = (self.st.d.settings.shuffle, self.st.d.settings.repeat);
                button(&mut self.cv, Rect::new(50, 600, 170, 70), "Shuffle", sh);
                button(&mut self.cv, Rect::new(235, 600, 185, 70), ["Repeat", "Repeat all", "Repeat 1"][rp as usize % 3], rp > 0);
            }
            // volume
            self.cv.text(F::Sans, 24.0, 52, 716, "Volume", MID);
            icon(&mut self.cv, "minus", 200, 730, BLACK);
            progress(&mut self.cv, 260, 726, W - 520, vol as f32 / 100.0);
            icon(&mut self.cv, "plus", W - 200, 730, BLACK);
            top = 800;
        } else {
            self.cv.text_c(F::Serif, 38.0, W / 2, 330, if n == 0 { "No music found" } else { "Pick a track to play" }, MID);
            if n == 0 {
                self.cv.text_c(F::Sans, 28.0, W / 2, 390, "Copy MP3 or FLAC files into the “music” folder", MID);
            }
            top = 440;
        }
        let per = ((MINI_Y - 40 - top) / 88).max(1) as usize;
        let pages = ((n + per - 1) / per).max(1);
        self.music_page = self.music_page.min(pages - 1);
        if n > 0 {
            self.cv.fill(50, top - 14, W - 100, 2, LIGHT);
        }
        let cur_path: Option<String> = {
            let s = self.player.st.lock().unwrap();
            s.idx.and_then(|c| s.queue.get(c)).map(|q| q.path.clone())
        };
        for (i, t) in self.tracks.clone().iter().enumerate().skip(self.music_page * per).take(per) {
            let y = top + (i - self.music_page * per) as i32 * 88;
            let now = cur_path.as_deref() == Some(t.path.as_str());
            let l = self.cv.fit(if now { F::SansB } else { F::SansM }, 30.0, &t.title, W - 140);
            self.cv.text(if now { F::SansB } else { F::SansM }, 30.0, 70, y + 4, &l, BLACK);
            let a = self.cv.fit(F::Sans, 23.0, &t.artist, W - 140);
            self.cv.text(F::Sans, 23.0, 70, y + 44, &a, MID);
            if now {
                self.cv.rrect(50, y + 6, 6, 62, 3, BLACK, 0);
            }
        }
        if pages > 1 {
            chevron(&mut self.cv, W - 230, 140, 18, true, if self.music_page > 0 { BLACK } else { LIGHT });
            self.cv.text_c(F::Sans, 26.0, W - 150, 124, &format!("{}/{}", self.music_page + 1, pages), DARK);
            chevron(&mut self.cv, W - 70, 140, 18, false, if self.music_page + 1 < pages { BLACK } else { LIGHT });
        }
    }

    /// Update just the progress bar and times on the Music tab (cheap, so audio is never interrupted).
    pub fn music_progress_partial(&mut self) {
        let (pos, dur) = {
            let s = self.player.st.lock().unwrap();
            (s.pos, s.dur)
        };
        self.cv.fill(425, 505, W - 450, 70, WHITE);
        let frac = if dur > 0 { pos as f32 / dur as f32 } else { 0.0 };
        progress(&mut self.cv, 430, 520, W - 480, frac);
        self.cv.text(F::Sans, 24.0, 430, 540, &fmt_time(pos), MID);
        if dur > 0 {
            self.cv.text_r(F::Sans, 24.0, W - 50, 540, &fmt_time(dur), MID);
        }
        self.cv.present(Wf::Fast, false);
    }

    /// Generative album art from the track title: concentric rings with a distinct rhythm per track.
    pub fn draw_art(&mut self, t: &Track, x: i32, y: i32, s: i32) {
        let h = library::fnv(&format!("{}{}", t.title, t.artist)).bytes().map(|b| b as u32).collect::<Vec<_>>();
        self.cv.fill(x, y, s, s, PALE);
        self.cv.frame(x, y, s, s, 3, DARK);
        let (cx, cy) = (x + s / 2, y + s / 2);
        let rings = 6 + h[0] % 5;
        for k in 0..rings {
            let r = (s / 2 - 14) - k as i32 * ((s / 2 - 30) / rings as i32);
            let thick = 3 + (h[(k as usize + 1) % h.len()] % 9) as i32;
            let gray = [BLACK, DARK, MID][(h[(k as usize + 3) % h.len()] % 3) as usize];
            self.cv.rrect(cx - r, cy - r, 2 * r, 2 * r, r, gray, thick);
        }
        self.cv.disc(cx, cy, 12 + (h[2] % 14) as i32, BLACK);
    }

    pub fn music_tap(&mut self, x: i32, y: i32) {
        use crate::music::Cmd;
        let (has_cur, playing, dur, vol) = {
            let s = self.player.st.lock().unwrap();
            (s.idx.is_some(), s.playing, s.dur, s.volume)
        };
        let per = if has_cur { ((MINI_Y - 40 - 800) / 88).max(1) as usize } else { ((MINI_Y - 40 - 440) / 88).max(1) as usize };
        let pages = ((self.tracks.len() + per - 1) / per).max(1);
        if y < 220 {
            if x > W - 280 && x < W - 190 && self.music_page > 0 {
                self.music_page -= 1;
                self.draw_screen(false);
            } else if x > W - 120 && self.music_page + 1 < pages {
                self.music_page += 1;
                self.draw_screen(false);
            }
            return;
        }
        if has_cur {
            if y > 590 && y < 680 && x < 430 && dur > AUDIOBOOK_SECS {
                let pos = self.player.st.lock().unwrap().pos;
                let t = if x < 228 { pos.saturating_sub(30) } else { (pos + 30).min(dur.saturating_sub(1)) };
                let _ = self.player.tx.send(Cmd::Seek(t));
                return;
            }
            if y > 590 && y < 680 && x < 430 {
                let s = &mut self.st.d.settings;
                if x < 228 {
                    s.shuffle = !s.shuffle;
                } else {
                    s.repeat = (s.repeat + 1) % 3;
                }
                let (sh, rp) = (s.shuffle, s.repeat);
                self.st.save();
                let mut p = self.player.st.lock().unwrap();
                p.shuffle = sh;
                p.repeat = rp;
                drop(p);
                self.draw_screen(false);
                return;
            }
            if y > 560 && y < 700 {
                if x < 560 {
                    let _ = self.player.tx.send(Cmd::Prev);
                } else if x < 760 {
                    let _ = self.player.tx.send(if playing { Cmd::Pause } else { Cmd::Resume });
                } else {
                    let _ = self.player.tx.send(Cmd::Next);
                }
                return;
            }
            if y > 490 && y < 560 && x > 430 && dur > 0 {
                let f = ((x - 430) as f32 / (W - 480) as f32).clamp(0.0, 1.0);
                let _ = self.player.tx.send(Cmd::Seek((f * dur as f32) as u32));
                return;
            }
            if y > 700 && y < 770 {
                let nv = if x < 230 { vol - 10 } else if x > W - 230 { vol + 10 } else { vol };
                self.player.st.lock().unwrap().volume = nv.clamp(0, 100);
                self.st.d.settings.volume = nv.clamp(0, 100);
                self.st.save();
                self.draw_screen(false);
                return;
            }
        }
        let top = if has_cur { 800 } else { 440 };
        if y >= top {
            let i = self.music_page * per + ((y - top) / 88) as usize;
            if i < self.tracks.len() {
                if !crate::sys::audio_output_is_bt() {
                    self.toast("Connect Bluetooth headphones first (More → Bluetooth)");
                    return;
                }
                {
                    let mut s = self.player.st.lock().unwrap();
                    s.queue = self.tracks.clone();
                }
                let path = self.tracks[i].path.clone();
                let _ = self.player.tx.send(match self.st.d.resume.get(&path) {
                    Some(&sec) => Cmd::PlayAt(i, sec),
                    None => Cmd::Play(i),
                });
            }
        }
    }

    // ---------------- settings ----------------
    pub fn draw_more(&mut self) {
        self.cv.text(F::SerifB, 66.0, 50, 90, "Settings", BLACK);
        self.cv.fill(50, 220, W - 100, 3, BLACK);
        let s = self.st.d.settings.clone();
        let bt = crate::sys::bt_device();
        let rows: Vec<(String, String)> = if self.more_page == 0 {
            vec![
                ("Brightness".into(), format!("{} / 24", s.light)),
                ("Default text size".into(), format!("{:.0} pt", s.em)),
                ("Bluetooth audio".into(), if crate::sys::audio_output_is_bt() { if bt.is_empty() { "connected".into() } else { bt } } else { "not connected".into() }),
                ("Rescan library".into(), format!("{} {} · {} {}", self.items.len(), if self.items.len() == 1 { "book" } else { "books" }, self.tracks.len(), if self.tracks.len() == 1 { "track" } else { "tracks" })),
                ("Calibrate touch".into(), "".into()),
                ("Sleep now".into(), "".into()),
                ("Dark reading".into(), (if s.dark { "on" } else { "off" }).into()),
                ("Full refresh while reading".into(), match s.flash_every { 0 => "never".into(), 1 => "every page".into(), n => format!("every {} pages", n) }),
                ("Word list".into(), format!("{} saved", self.vocab_load().len())),
                ("More  ›".into(), "".into()),
            ]
        } else {
            vec![
                ("Reading stats".into(), "".into()),
                ("Export highlights".into(), "to the exports folder".into()),
                ("Sleep screen".into(), ["player and clock", "book cover", "highlight quote"][s.sleep_screen as usize % 3].into()),
                ("Focus timer".into(), if self.focus_until.is_some() { "running".into() } else { "25 min".into() }),
                ("USB network (developer)".into(), if std::path::Path::new("/mnt/us/usbnetlite/auto").exists() { "on at boot".into() } else { "off".into() }),
                ("Restart Folio".into(), "".into()),
                ("Restart device".into(), "".into()),
                ("Power off".into(), "".into()),
                ("".into(), "".into()),
                ("‹  Back".into(), "".into()),
            ]
        };
        for (i, (a, b)) in rows.iter().enumerate() {
            let y = 240 + i as i32 * 104;
            self.cv.text(F::SansM, 34.0, 50, y + 28, a, BLACK);
            self.cv.text_r(F::Sans, 28.0, W - 50, y + 34, b, MID);
            self.cv.fill(50, y + 100, W - 100, 2, LIGHT);
        }
    }

    pub fn more_tap(&mut self, x: i32, y: i32) {
        if y < 240 {
            return;
        }
        let row = (y - 240) / 104;
        if row == 9 {
            self.more_page = 1 - self.more_page;
            return self.draw_screen(false);
        }
        if self.more_page == 1 {
            match row {
                0 => self.stats_card(),
                1 => self.export_highlights(),
                2 => {
                    let s = &mut self.st.d.settings;
                    s.sleep_screen = (s.sleep_screen + 1) % 3;
                    self.st.save();
                    self.draw_screen(false);
                }
                3 => {
                    self.focus_toggle();
                    self.draw_screen(false);
                }
                4 => self.do_act(Act::UsbNet),
                5 => self.do_act(Act::Quit),
                6 => self.do_act(Act::Reboot),
                7 => self.do_act(Act::Poweroff),
                _ => {}
            }
            return;
        }
        match row {
            0 => {
                let d = if x < W / 2 { -2 } else { 2 };
                let n = (self.st.d.settings.light + d).clamp(0, 24);
                self.st.d.settings.light = n;
                crate::sys::set_light(n);
                self.st.save();
                self.draw_screen(false);
            }
            1 => {
                let d = if x < W / 2 { -1.0 } else { 1.0 };
                self.st.d.settings.em = (self.st.d.settings.em + d).clamp(7.0, 22.0);
                self.st.save();
                self.draw_screen(false);
            }
            2 => self.bt_open(),
            3 => self.do_act(Act::Rescan),
            4 => self.do_act(Act::Calibrate),
            5 => self.do_act(Act::Sleep),
            6 => {
                self.st.d.settings.dark = !self.st.d.settings.dark;
                self.st.save();
                self.draw_screen(false);
            }
            7 => {
                let s = &mut self.st.d.settings;
                s.flash_every = match s.flash_every {
                    1 => 4,
                    4 => 8,
                    8 => 16,
                    16 => 0,
                    _ => 1,
                };
                self.st.save();
                self.draw_screen(false);
            }
            8 => self.open_words(),
            _ => {}
        }
    }

    // ---------------- calibration ----------------
    pub fn draw_calibrate(&mut self, step: u8) {
        self.cv.clear(WHITE);
        self.cv.text_c(F::SerifB, 56.0, W / 2, 300, "Touch setup", BLACK);
        self.cv.text_c(F::Serif, 34.0, W / 2, 390, "Press the exact center of the circle with one fingertip.", DARK);
        let (x, y) = crate::store::CAL_TARGETS[(step as usize).min(4)];
        self.cv.rrect(x - 60, y - 60, 120, 120, 60, BLACK, 6);
        self.cv.disc(x, y, 10, BLACK);
        self.cv.text_c(F::Sans, 26.0, W / 2, 470, &format!("Circle {} of 5", step + 1), MID);
        self.cv.present(Wf::Full, true);
    }
}
