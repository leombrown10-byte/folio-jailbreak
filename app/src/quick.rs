//! Quick panel (pull down from the top edge, from anywhere) and the lock screen with a mini player.
use crate::gfx::*;
use crate::music::Cmd;
use crate::ui::*;
use crate::{App, Modal, Screen, G};
use std::time::{Duration, Instant};

const QH: i32 = 850; // panel height
const TRACK: (i32, i32) = (215, 855); // slider track x-range
const ROW_MUSIC: i32 = 610;
const ROW_BTNS: i32 = 730;
/// Slider rows: top y, label (icon name for the first two), min, max, step.
const QROWS: [(i32, &str, f32, f32, f32); 5] = [
    (90, "sun", 0.0, 24.0, 2.0),
    (190, "music", 0.0, 100.0, 10.0),
    (290, "Aa", 7.0, 22.0, 1.0),
    (390, "Top", 20.0, 200.0, 10.0),
    (490, "Bottom", 70.0, 250.0, 10.0),
];

impl App {
    // ------------------------------------------------------------------ quick panel
    pub fn open_quick(&mut self) {
        if self.modal.is_some() || matches!(self.screen, Screen::Usb | Screen::Calibrate { .. } | Screen::Lock) {
            return;
        }
        self.qs_em0 = self.st.d.settings.em;
        self.qs_tb = (self.st.d.settings.top, self.st.d.settings.bottom);
        self.modal = Some(Modal::Quick);
        self.draw_quick();
    }

    fn qget(&self, i: usize) -> f32 {
        let s = &self.st.d.settings;
        match i {
            0 => s.light as f32,
            1 => s.volume as f32,
            2 => s.em,
            3 => s.top as f32,
            _ => s.bottom as f32,
        }
    }

    /// Store a slider value. `apply` pushes it to the hardware (frontlight / player volume).
    fn qset(&mut self, i: usize, v: f32, apply: bool) {
        let s = &mut self.st.d.settings;
        match i {
            0 => {
                s.light = v as i32;
                if apply {
                    crate::sys::set_light(s.light);
                }
            }
            1 => {
                s.volume = v as i32;
                if apply {
                    self.player.st.lock().unwrap().volume = v as i32;
                }
            }
            2 => s.em = v,
            3 => s.top = v as i32,
            _ => s.bottom = v as i32,
        }
    }

    /// Which slider's track (if any) is under this point.
    pub fn quick_row_at(&self, x: i32, y: i32) -> Option<usize> {
        if x < TRACK.0 - 25 || x > TRACK.1 + 25 {
            return None;
        }
        QROWS.iter().position(|r| y >= r.0 && y < r.0 + 100)
    }

    /// Drag a slider to x. Cheap partial redraw of just that row; volume is applied when the finger lifts
    /// (changing it restarts the audio segment).
    pub fn quick_drag(&mut self, i: usize, x: i32, fin: bool) {
        let (_, _, lo, hi, step) = QROWS[i];
        let f = ((x - TRACK.0) as f32 / (TRACK.1 - TRACK.0) as f32).clamp(0.0, 1.0);
        let v = (lo + (hi - lo) * f) / step;
        let v = v.round() * step;
        let v = v.clamp(lo, hi);
        if v != self.qget(i) || fin {
            self.qset(i, v, i != 1 || fin);
            self.draw_slider(i);
            self.cv.present(Wf::Fast, false);
        }
        self.touch.last_draw = Instant::now();
    }

    fn draw_slider(&mut self, i: usize) {
        let (y, label, lo, hi, _) = QROWS[i];
        let v = self.qget(i);
        let frac = ((v - lo) / (hi - lo)).clamp(0.0, 1.0);
        let val = match i {
            2 => format!("{:.0} pt", v),
            3 | 4 => format!("{} px", v as i32),
            _ => format!("{}", v as i32),
        };
        let cv = &mut self.cv;
        cv.fill(0, y, W, 100, WHITE);
        if i < 2 {
            icon(cv, label, 70, y + 55, BLACK);
        } else {
            cv.text_c(F::SerifB, if i == 2 { 40.0 } else { 24.0 }, 62, y + if i == 2 { 32 } else { 42 }, label, BLACK);
        }
        cv.rrect(110, y + 15, 80, 80, 40, DARK, 3);
        icon(cv, "minus", 150, y + 55, BLACK);
        cv.text_r(F::Sans, 22.0, TRACK.1, y + 6, &val, MID);
        cv.rrect(TRACK.0, y + 49, TRACK.1 - TRACK.0, 12, 6, LIGHT, 0);
        let fx = TRACK.0 + ((TRACK.1 - TRACK.0) as f32 * frac) as i32;
        cv.rrect(TRACK.0, y + 49, fx - TRACK.0, 12, 6, BLACK, 0);
        cv.disc(fx, y + 55, 22, BLACK);
        cv.rrect(890, y + 15, 80, 80, 40, DARK, 3);
        icon(cv, "plus", 930, y + 55, BLACK);
    }

    pub fn draw_quick(&mut self) {
        let (track, playing) = {
            let s = self.player.st.lock().unwrap();
            (s.idx.and_then(|i| s.queue.get(i).cloned()), s.playing)
        };
        let bat = crate::sys::battery();
        let timer = self.sleep_label();
        let cv = &mut self.cv;
        cv.fill(0, 0, W, QH, WHITE);
        cv.fill(0, QH - 6, W, 6, BLACK);
        cv.rrect(W / 2 - 50, QH + 14, 100, 8, 4, DARK, 0); // grab handle below the panel
        // status line
        cv.text(F::SansM, 32.0, 40, 28, &crate::sys::clock(), BLACK);
        cv.text_c(F::Sans, 28.0, W / 2, 32, &crate::sys::date_long(), MID);
        if bat >= 0 {
            cv.text_r(F::SansM, 30.0, W - 40, 28, &format!("{}%", bat), BLACK);
        }
        for i in 0..QROWS.len() {
            self.draw_slider(i);
        }
        let cv = &mut self.cv;
        // music
        let y = ROW_MUSIC;
        cv.fill(40, y - 6, W - 80, 2, LIGHT);
        match track {
            Some(t) => {
                let title = cv.fit(F::SansM, 30.0, &t.title, W - 460);
                cv.text(F::SansM, 30.0, 40, y + 10, &title, BLACK);
                let artist = cv.fit(F::Sans, 24.0, &t.artist, W - 460);
                cv.text(F::Sans, 24.0, 40, y + 54, &artist, MID);
                icon(cv, "prev", W - 330, y + 50, BLACK);
                icon_button(cv, Rect::new(W - 270, y + 5, 90, 90), if playing { "pause" } else { "play" }, true);
                icon(cv, "next", W - 90, y + 50, BLACK);
            }
            None => {
                cv.text(F::Sans, 28.0, 40, y + 32, "Nothing playing", MID);
            }
        }
        // buttons
        let y = ROW_BTNS;
        let bw = (W - 80 - 3 * 16) / 4;
        for (i, label) in ["Sleep", timer.as_str(), "Settings", "Bluetooth"].iter().enumerate() {
            button(cv, Rect::new(40 + i as i32 * (bw + 16), y, bw, 80), label, false);
        }
        self.cv.present(Wf::Auto, false);
    }

    pub fn close_quick(&mut self) {
        self.modal = None;
        let s = &self.st.d.settings;
        let changed = (s.em - self.qs_em0).abs() > 0.01 || (s.top, s.bottom) != self.qs_tb;
        self.st.save();
        if self.screen == Screen::Reader && changed {
            self.reader_relayout();
            self.reader_draw(true);
        } else {
            self.draw_screen(false);
        }
    }

    /// `true` if the tap was handled by the panel (it may have closed it).
    pub fn quick_tap(&mut self, x: i32, y: i32) {
        if y > QH {
            return self.close_quick();
        }
        if let Some(i) = QROWS.iter().position(|r| y >= r.0 && y < r.0 + 100) {
            // the track itself is handled by dragging; here only the minus / plus buttons
            let (_, _, lo, hi, step) = QROWS[i];
            let d = if x < 200 { -step } else if x > 880 { step } else { return };
            let v = (self.qget(i) + d).clamp(lo, hi);
            self.qset(i, v, true);
            self.draw_slider(i);
            self.cv.present(Wf::Fast, false);
            return;
        } else if y >= ROW_MUSIC && y < ROW_MUSIC + 100 {
            let playing = self.player.st.lock().unwrap().playing;
            if x >= W - 390 && x < W - 270 {
                let _ = self.player.tx.send(Cmd::Prev);
            } else if x >= W - 270 && x < W - 150 {
                let _ = self.player.tx.send(if playing { Cmd::Pause } else { Cmd::Resume });
            } else if x >= W - 150 {
                let _ = self.player.tx.send(Cmd::Next);
            }
        } else if y >= ROW_BTNS && y < ROW_BTNS + 90 {
            let bw = (W - 80 - 3 * 16) / 4;
            let which = ((x - 40) / (bw + 16)).clamp(0, 3);
            self.st.save();
            match which {
                0 => {
                    self.modal = None;
                    return self.lock_now();
                }
                1 => {
                    self.cycle_sleep_timer();
                }
                2 => {
                    self.modal = None;
                    if self.screen == Screen::Reader {
                        self.reader_save_pub();
                        self.rd = None;
                    }
                    self.screen = Screen::More;
                    return self.draw_screen(false);
                }
                _ => {
                    self.modal = None;
                    return self.bt_open();
                }
            }
        } else {
            return;
        }
        self.draw_quick();
    }

    // ------------------------------------------------------------------ music sleep timer
    pub fn sleep_label(&self) -> String {
        match self.sleep_deadline {
            Some(t) => format!("Timer {}m", (t.saturating_duration_since(Instant::now()).as_secs() + 59) / 60),
            None => "Timer off".into(),
        }
    }

    /// Off -> 15 -> 30 -> 60 -> off.
    pub fn cycle_sleep_timer(&mut self) {
        let next = match self.sleep_mins {
            0 => 15,
            15 => 30,
            30 => 60,
            _ => 0,
        };
        self.sleep_mins = next;
        self.sleep_deadline = if next > 0 { Some(Instant::now() + Duration::from_secs(next as u64 * 60)) } else { None };
    }

    /// Called from the tick: when the timer runs out, music stops.
    pub fn sleep_timer_check(&mut self) {
        if let Some(t) = self.sleep_deadline {
            if Instant::now() >= t {
                self.sleep_deadline = None;
                self.sleep_mins = 0;
                let _ = self.player.tx.send(Cmd::Pause);
            }
        }
    }

    pub fn quick_gesture(&mut self, g: G) {
        match g {
            G::Tap(x, y) => self.quick_tap(x, y),
            G::Swipe { dy, .. } if dy < -100 => self.close_quick(),
            _ => {}
        }
    }

    // ------------------------------------------------------------------ lock screen
    pub fn lock_now(&mut self) {
        if self.screen == Screen::Lock {
            return;
        }
        self.modal = None;
        if self.rd.is_some() {
            self.reader_save_pub();
        }
        self.lock_prev = self.screen;
        self.screen = Screen::Lock;
        self.lock_at = Instant::now();
        self.draw_lock();
        if !self.player.st.lock().unwrap().playing {
            self.lock_suspend();
        }
    }

    /// Suspend to RAM until the power button wakes the device, then unlock.
    pub fn lock_suspend(&mut self) {
        let t = Instant::now();
        crate::sys::sh_wait("sync; echo mem > /sys/power/state");
        self.last_wake = Instant::now();
        self.last_input = Instant::now();
        self.touch.down = false;
        if t.elapsed() < Duration::from_secs(3) {
            // the suspend did not hold: stay on the lock screen and wait for the next power press
            self.lock_at = Instant::now();
            return;
        }
        self.unlock();
    }

    pub fn unlock(&mut self) {
        if self.screen != Screen::Lock {
            return;
        }
        self.screen = self.lock_prev;
        self.draw_screen(true);
    }

    pub fn lock_idle_check(&mut self) {
        let playing = self.player.st.lock().unwrap().playing;
        if self.screen == Screen::Lock && !playing && self.lock_at.elapsed() > Duration::from_secs(60) {
            self.lock_at = Instant::now();
            self.lock_suspend();
        }
    }

    pub fn draw_lock(&mut self) {
        let (track, playing, pos, dur, vol) = {
            let s = self.player.st.lock().unwrap();
            (s.idx.and_then(|i| s.queue.get(i).cloned()), s.playing, s.pos, s.dur, s.volume)
        };
        self.cv.clear(WHITE);
        let bat = crate::sys::battery();
        self.cv.text_c(F::SerifB, 130.0, W / 2, 90, &crate::sys::clock(), BLACK);
        self.cv.text_c(F::Sans, 34.0, W / 2, 250, &crate::sys::date_long(), MID);
        if bat >= 0 {
            self.cv.text_r(F::Sans, 28.0, W - 40, 30, &format!("{}%", bat), MID);
        }
        match track {
            Some(t) => {
                self.draw_art(&t, W / 2 - 170, 330, 340);
                let lines = self.cv.wrap(F::SerifB, 46.0, &t.title, W - 160, 2);
                for (i, l) in lines.iter().enumerate() {
                    self.cv.text_c(F::SerifB, 46.0, W / 2, 700 + i as i32 * 58, l, BLACK);
                }
                let ay = 700 + lines.len() as i32 * 58 + 6;
                self.cv.text_c(F::Sans, 30.0, W / 2, ay, &t.artist, MID);
                self.draw_lock_progress(pos, dur);
                icon(&mut self.cv, "prev", 300, 1120, BLACK);
                icon_button(&mut self.cv, Rect::new(W / 2 - 60, 1060, 120, 120), if playing { "pause" } else { "play" }, true);
                icon(&mut self.cv, "next", W - 300, 1120, BLACK);
                icon(&mut self.cv, "minus", 200, 1250, BLACK);
                self.cv.rrect(260, 1245, W - 520, 10, 5, LIGHT, 0);
                self.cv.rrect(260, 1245, ((W - 520) as f32 * vol as f32 / 100.0) as i32, 10, 5, BLACK, 0);
                icon(&mut self.cv, "plus", W - 200, 1250, BLACK);
            }
            None => {
                let cur = self.st.d.settings.last_book.clone().and_then(|k| self.items.iter().find(|i| i.key == k).cloned());
                let mode = self.st.d.settings.sleep_screen;
                if let (2, Some((q, book))) = (mode, self.random_quote()) {
                    self.cv.text(F::SerifB, 160.0, 90, 330, "“", MID);
                    let lines = self.cv.wrap(F::SerifI, 50.0, &q, W - 200, 12);
                    for (i, l) in lines.iter().enumerate() {
                        self.cv.text(F::SerifI, 50.0, 100, 470 + i as i32 * 70, l, BLACK);
                    }
                    let ay = 470 + lines.len() as i32 * 70 + 30;
                    self.cv.text(F::Sans, 30.0, 100, ay, &format!("— {}", book), MID);
                } else if let (1, Some(it)) = (mode, cur.clone()) {
                    self.draw_cover(&it, W / 2 - 280, 330, 560, 746);
                } else {
                    if let Some(it) = cur {
                        self.draw_cover(&it, W / 2 - 200, 340, 400, 533);
                    }
                    self.cv.text_c(F::SerifB, 54.0, W / 2, 940, "Folio is asleep", BLACK);
                }
            }
        }
        self.cv.text_c(F::Sans, 26.0, W / 2, 1340, "Press the power button to unlock", MID);
        self.cv.present(Wf::Full, true);
    }

    fn draw_lock_progress(&mut self, pos: u32, dur: u32) {
        self.cv.fill(100, 900, W - 200, 90, WHITE);
        let frac = if dur > 0 { pos as f32 / dur as f32 } else { 0.0 };
        progress(&mut self.cv, 120, 925, W - 240, frac);
        self.cv.text(F::Sans, 24.0, 120, 945, &fmt_time(pos), MID);
        if dur > 0 {
            self.cv.text_r(F::Sans, 24.0, W - 120, 945, &fmt_time(dur), MID);
        }
    }

    /// Update only the progress area (called every ~15 s while music plays on the lock screen).
    pub fn lock_progress(&mut self) {
        let (pos, dur) = {
            let s = self.player.st.lock().unwrap();
            (s.pos, s.dur)
        };
        self.draw_lock_progress(pos, dur);
        self.cv.present(Wf::Fast, false);
    }

    pub fn lock_gesture(&mut self, g: G) {
        let G::Tap(x, y) = g else { return };
        let (playing, vol, has) = {
            let s = self.player.st.lock().unwrap();
            (s.playing, s.volume, s.idx.is_some())
        };
        if !has {
            return;
        }
        if y > 1040 && y < 1200 {
            if x < W / 2 - 80 {
                let _ = self.player.tx.send(Cmd::Prev);
            } else if x > W / 2 + 80 {
                let _ = self.player.tx.send(Cmd::Next);
            } else {
                let _ = self.player.tx.send(if playing { Cmd::Pause } else { Cmd::Resume });
            }
        } else if y > 1200 && y < 1310 {
            let nv = (if x < W / 2 { vol - 10 } else { vol + 10 }).clamp(0, 100);
            self.player.st.lock().unwrap().volume = nv;
            self.st.d.settings.volume = nv;
            self.st.save();
            self.draw_lock();
        }
    }
}

fn min_f(a: f32, b: f32) -> f32 {
    if a < b { a } else { b }
}
