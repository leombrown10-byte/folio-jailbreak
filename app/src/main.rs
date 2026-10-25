mod bt;
mod convert;
mod dict;
mod extras;
mod gfx;
mod input;
mod library;
mod music;
mod notes;
mod quick;
mod reader;
mod screens;
mod store;
mod usb;
mod sys;
mod ui;

use gfx::*;
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use store::{Cal, Store};
use ui::*;

pub enum Ev {
    Down(i32, i32),
    Move(i32, i32),
    Up(i32, i32),
    Raw(i32, i32),
    Power,
    Cover,
    Music(music::MEv),
    SkipCal,
    Bt,
    Usb(String),
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Screen {
    Home,
    Books,
    Music,
    Notes,
    More,
    Bluetooth,
    Usb,
    Lock,
    Reader,
    Ink,
    TextNote,
    Calibrate { step: u8, first: (i32, i32) },
}

pub enum Modal {
    Sheet(Sheet),
    Kbd(Kbd),
    Card(Card),
    Quick,
}

#[derive(Clone, Copy)]
pub enum G {
    Tap(i32, i32),
    Long(i32, i32),
    Swipe { dx: i32, dy: i32, x: i32, y: i32 },
}

pub struct Touch {
    pub down: bool,
    pub x0: i32,
    pub y0: i32,
    pub x: i32,
    pub y: i32,
    pub t0: Instant,
    pub moved: bool,
    pub long: bool,
    pub from_top: bool,
    pub drag: Option<usize>, // slider row being dragged in the quick panel
    pub last_draw: Instant,
}

pub struct App {
    pub cv: Canvas,
    pub st: Store,
    pub tx: Sender<Ev>,
    pub cal: Arc<Mutex<Cal>>,
    pub screen: Screen,
    pub modal: Option<Modal>,
    pub items: Vec<library::Item>,
    pub lib_page: usize,
    pub lib_query: String,
    pub lib_sort: u8,
    pub rd: Option<reader::Reader>,
    pub player: music::Player,
    pub tracks: Vec<music::Track>,
    pub music_page: usize,
    pub notes: Vec<notes::Note>,
    pub note_idx: Option<usize>,
    pub note_page: usize,
    pub notes_seg: u8,
    pub notes_page: usize,
    pub hl_list: Vec<(String, String, String)>,
    pub pen: notes::Pen,
    pub ink_idle_at: Option<Instant>,
    pub pending_new: Option<bool>,
    pub bt_devs: Vec<bt::BtDev>,
    pub bt_scanning: bool,
    pub bt_busy: bool,
    pub bt_sel: Option<usize>,
    pub touch: Touch,
    last_raw: (i32, i32),
    raw_down: (i32, i32),
    cal_pts: Vec<((i32, i32), (i32, i32))>,
    last_min: String,
    pub last_input: Instant,
    pub last_wake: Instant,
    pub lock_prev: Screen,
    pub lock_at: Instant,
    pub qs_em0: f32,
    pub qs_tb: (i32, i32),
    pub sleep_deadline: Option<Instant>,
    pub sleep_mins: u32,
    pub fc: Vec<(String, String, String)>, // flashcards: word, sentence, book
    pub fc_i: usize,
    pub fc_shown: bool,
    pub focus_until: Option<Instant>,
    pub focus_pages0: u32,
    pub more_page: u8,
    last_prog: u32,
    toast_until: Option<Instant>,
    started: Instant,
    boot_marked: bool,
    pub quit: bool,
    pub exit_code: i32,
}

impl App {
    fn new(tx: Sender<Ev>) -> App {
        let st = Store::load();
        let (mx, my) = input::touch_range();
        let cal = st.d.settings.cal.clone().unwrap_or(Cal { max_x: mx, max_y: my, ..Cal::default() });
        let player = music::start(tx.clone(), st.d.settings.volume);
        {
            let mut p = player.st.lock().unwrap();
            p.shuffle = st.d.settings.shuffle;
            p.repeat = st.d.settings.repeat;
        }
        let notes = notes::load_all(&st.dir);
        App {
            cv: Canvas::new(),
            tx,
            cal: Arc::new(Mutex::new(cal)),
            screen: Screen::Home,
            modal: None,
            items: vec![],
            lib_page: 0,
            lib_query: String::new(),
            lib_sort: 0,
            rd: None,
            player,
            tracks: vec![],
            music_page: 0,
            notes,
            note_idx: None,
            note_page: 0,
            notes_seg: 0,
            notes_page: 0,
            hl_list: vec![],
            pen: notes::Pen::new(),
            ink_idle_at: None,
            pending_new: None,
            bt_devs: vec![],
            bt_scanning: false,
            bt_busy: false,
            bt_sel: None,
            touch: Touch { down: false, x0: 0, y0: 0, x: 0, y: 0, t0: Instant::now(), moved: false, long: false, from_top: false, drag: None, last_draw: Instant::now() },
            last_raw: (0, 0),
            raw_down: (0, 0),
            cal_pts: vec![],
            last_min: String::new(),
            last_input: Instant::now(),
            lock_prev: Screen::Home,
            lock_at: Instant::now(),
            qs_em0: 11.0,
            qs_tb: (60, 100),
            sleep_deadline: None,
            sleep_mins: 0,
            fc: vec![],
            fc_i: 0,
            fc_shown: false,
            focus_until: None,
            focus_pages0: 0,
            more_page: 0,
            last_wake: Instant::now() - Duration::from_secs(60),
            last_prog: 0,
            toast_until: None,
            started: Instant::now(),
            boot_marked: false,
            quit: false,
            exit_code: 0,
            st,
        }
    }

    fn boot(&mut self) {
        sys::set_light(self.st.d.settings.light);
        self.items = library::scan();
        library::spawn_covers(self.items.clone(), self.st.dir.clone(), self.tx.clone());
        music::scan(self.tx.clone());
        usb::spawn(self.tx.clone());
        self.screen = if self.st.d.settings.cal.as_ref().map_or(true, |c| c.v < 3) { Screen::Calibrate { step: 0, first: (0, 0) } } else { Screen::Home };
        self.draw_screen(true);
        if !usb::storage_available() {
            self.usb_enter();
        }
        self.first_run_help();
    }

    /// Show the how-to card once, as soon as the touch setup is done.
    fn first_run_help(&mut self) {
        if self.screen == Screen::Home && !self.st.d.settings.seen_help {
            self.st.d.settings.seen_help = true;
            self.st.save();
            self.show_help();
        }
    }

    // ---------------- events ----------------
    fn handle(&mut self, ev: Ev) {
        match ev {
            Ev::Raw(x, y) => {
                if !self.touch.down {
                    self.raw_down = (x, y); // first contact of a new touch
                }
                self.last_raw = (x, y);
            }
            Ev::Down(x, y) => {
                self.last_input = Instant::now();
                self.touch = Touch { down: true, x0: x, y0: y, x, y, t0: Instant::now(), moved: false, long: false, from_top: y < 45, drag: None, last_draw: Instant::now() };
                if matches!(self.modal, Some(Modal::Quick)) {
                    if let Some(i) = self.quick_row_at(x, y) {
                        self.touch.drag = Some(i);
                        self.quick_drag(i, x, false);
                        return;
                    }
                }
                if self.wants_pen() && !self.touch.from_top {
                    self.ink_touch(0, x, y);
                }
            }
            Ev::Move(x, y) => {
                if !self.touch.down {
                    return;
                }
                self.touch.x = x;
                self.touch.y = y;
                if let Some(i) = self.touch.drag {
                    if self.touch.last_draw.elapsed() > Duration::from_millis(120) {
                        self.quick_drag(i, x, false);
                    }
                    return;
                }
                if (x - self.touch.x0).abs() + (y - self.touch.y0).abs() > 45 {
                    self.touch.moved = true;
                }
                if self.wants_pen() && !self.touch.from_top {
                    self.ink_touch(1, x, y);
                }
            }
            Ev::Up(x, y) => {
                if !self.touch.down {
                    return;
                }
                self.touch.down = false;
                if let Some(i) = self.touch.drag.take() {
                    self.quick_drag(i, x, true);
                    return;
                }
                let (dx, dy) = (x - self.touch.x0, y - self.touch.y0);
                // pull-down from the very top edge opens the quick panel, from any screen
                if self.touch.from_top && dy > 160 && dy * 2 > dx.abs() * 3 {
                    self.open_quick();
                    return;
                }
                if self.wants_pen() {
                    if !self.touch.from_top {
                        self.ink_touch(2, x, y);
                    }
                    return;
                }
                if self.touch.long {
                    return;
                }
                if dx.abs() + dy.abs() > 90 {
                    self.gesture(G::Swipe { dx, dy, x: self.touch.x0, y: self.touch.y0 });
                } else {
                    self.gesture(G::Tap(x, y));
                }
            }
            Ev::Power => {
                if self.last_wake.elapsed() > Duration::from_secs(3) {
                    if self.screen == Screen::Lock {
                        self.unlock();
                    } else {
                        self.lock_now();
                    }
                }
            }
            Ev::Cover => {
                if matches!(self.screen, Screen::Home | Screen::Books) && self.modal.is_none() && !self.touch.down {
                    self.draw_screen(false);
                }
            }
            Ev::Music(m) => self.music_event(m),
            Ev::Bt => self.bt_event(),
            Ev::Usb(l) => self.usb_event(&l),
            Ev::SkipCal => {
                if let Screen::Calibrate { .. } = self.screen {
                    self.screen = Screen::Home;
                    self.draw_screen(true);
                }
            }
        }
    }

    fn wants_pen(&self) -> bool {
        self.screen == Screen::Ink && self.modal.is_none()
    }

    fn music_event(&mut self, m: music::MEv) {
        use music::MEv::*;
        let on_music = self.screen == Screen::Music && self.modal.is_none();
        match m {
            Library(t) => {
                self.tracks = t;
                if on_music {
                    self.draw_screen(false);
                }
            }
            Track(_) | Ended => {
                if matches!(m, Ended) {
                    // finished: the next listen starts from the beginning
                    let p = { let s = self.player.st.lock().unwrap(); s.idx.and_then(|i| s.queue.get(i)).map(|t| t.path.clone()) };
                    if let Some(p) = p {
                        self.st.d.resume.remove(&p);
                    }
                }
                if self.screen == Screen::Lock {
                    self.draw_lock();
                } else if self.modal.is_none() && matches!(self.screen, Screen::Music | Screen::Home | Screen::Books | Screen::Notes | Screen::More) {
                    self.draw_screen(false);
                }
            }
            Progress => {
                let (pos, dur, playing, path) = {
                    let s = self.player.st.lock().unwrap();
                    (s.pos, s.dur, s.playing, s.idx.and_then(|i| s.queue.get(i)).map(|t| t.path.clone()))
                };
                // audiobooks: remember the position (when paused, and every 30 s while playing)
                if let (true, Some(p)) = (dur > screens::AUDIOBOOK_SECS, path) {
                    if self.st.d.resume.insert(p, pos) != Some(pos) && (!playing || pos % 30 == 0) {
                        self.st.save();
                    }
                }
                if self.screen == Screen::Lock && pos / 15 != self.last_prog / 15 {
                    self.last_prog = pos;
                    self.lock_progress();
                }
                if on_music && pos / 10 != self.last_prog / 10 {
                    self.last_prog = pos;
                    self.music_progress_partial(); // a full redraw here would starve the Bluetooth encoder and click
                }
            }
            NoOutput => {
                self.toast("Bluetooth headphones disconnected");
                if on_music {
                    self.draw_screen(false);
                }
            }
            Failed(msg) => self.toast(&format!("Playback failed: {}", msg)),
        }
    }

    fn tick(&mut self) {
        // healthy for 20s: clear the autostart crash guard
        if !self.boot_marked && self.started.elapsed() > Duration::from_secs(20) {
            self.boot_marked = true;
            let _ = std::fs::remove_file("/var/local/kos/.starting");
        }
        // long press
        if self.touch.down && !self.touch.moved && !self.touch.long && self.touch.t0.elapsed() > Duration::from_millis(650) {
            if self.screen == Screen::Reader && self.modal.is_none() {
                self.touch.long = true;
                let (x, y) = (self.touch.x, self.touch.y);
                self.gesture(G::Long(x, y));
            }
        }
        // settle ink after a pause: refresh the written region with a cleaner waveform
        if let Some(t) = self.ink_idle_at {
            if t.elapsed() > Duration::from_millis(1800) && !self.touch.down {
                self.ink_idle_at = None;
                self.cv.present(Wf::Text, false);
            }
        }
        if let Some(t) = self.toast_until {
            if Instant::now() > t {
                self.toast_until = None;
                if self.modal.is_none() {
                    self.draw_screen(false);
                }
            }
        }
        // status bar clock
        let m = sys::clock();
        if m != self.last_min {
            self.last_min = m;
            if matches!(self.screen, Screen::Home | Screen::Books | Screen::Music | Screen::Notes | Screen::More) && self.modal.is_none() && !self.touch.down {
                self.cv.fill(0, 0, W, 70, WHITE);
                self.status_bar();
                self.cv.present(Wf::Auto, false);
            }
        }
        // idle: lock after 15 minutes (the lock screen suspends the device unless music is playing)
        if self.last_input.elapsed() > Duration::from_secs(15 * 60) && self.screen != Screen::Lock {
            self.lock_now();
        }
        self.lock_idle_check();
        self.sleep_timer_check();
        self.focus_check();
    }

    // ---------------- gestures ----------------
    fn gesture(&mut self, g: G) {
        if self.modal.is_some() {
            if matches!(self.modal, Some(Modal::Quick)) {
                return self.quick_gesture(g);
            }
            if let G::Tap(x, y) = g {
                self.modal_tap(x, y);
            }
            return;
        }
        match self.screen {
            Screen::Calibrate { step, first } => {
                if let G::Tap(..) = g {
                    self.calibrate_tap(step, first);
                }
            }
            Screen::Reader => self.reader_gesture(g),
            Screen::TextNote => self.text_note_gesture(g),
            Screen::Usb => {}
            Screen::Lock => self.lock_gesture(g),
            Screen::Bluetooth => {
                if let G::Tap(x, y) = g {
                    self.bt_tap(x, y);
                }
            }
            Screen::Ink => {}
            _ => {
                if let G::Tap(x, y) = g {
                    if self.chrome_tap(x, y) {
                        return;
                    }
                    match self.screen {
                        Screen::Home => self.home_tap(x, y),
                        Screen::Books => self.books_tap(x, y),
                        Screen::Music => self.music_tap(x, y),
                        Screen::Notes => self.notes_tab_tap(x, y),
                        Screen::More => self.more_tap(x, y),
                        _ => {}
                    }
                } else if let G::Swipe { dx, dy, .. } = g {
                    if dx.abs() > dy.abs() && dx.abs() > 150 {
                        // swipe between pages in paged tabs
                        let d: i32 = if dx < 0 { 1 } else { -1 };
                        let (page, pages) = match self.screen {
                            Screen::Books => (&mut self.lib_page, 99usize),
                            Screen::Music => (&mut self.music_page, 99),
                            Screen::Notes => (&mut self.notes_page, 99),
                            _ => return,
                        };
                        let np = *page as i32 + d;
                        if np >= 0 && (np as usize) < pages {
                            *page = np as usize;
                            self.draw_screen(false);
                        }
                    }
                }
            }
        }
    }

    /// Five-point touch calibration. Each tap records the raw touch position (mean of first contact and
    /// release) against its on-screen target; after the fifth, a least-squares affine fit is stored.
    fn calibrate_tap(&mut self, step: u8, _first: (i32, i32)) {
        let (d, u) = (self.raw_down, self.last_raw);
        let raw = ((d.0 + u.0) / 2, (d.1 + u.1) / 2);
        if step == 0 {
            self.cal_pts.clear();
        }
        self.cal_pts.push((raw, crate::store::CAL_TARGETS[step as usize]));
        if (step as usize) + 1 < crate::store::CAL_TARGETS.len() {
            self.screen = Screen::Calibrate { step: step + 1, first: (0, 0) };
            self.draw_screen(false);
            return;
        }
        let xs: Vec<(f64, f64, f64)> = self.cal_pts.iter().map(|(r, t)| (r.0 as f64, r.1 as f64, t.0 as f64)).collect();
        let ys: Vec<(f64, f64, f64)> = self.cal_pts.iter().map(|(r, t)| (r.0 as f64, r.1 as f64, t.1 as f64)).collect();
        match (store::fit_affine(&xs), store::fit_affine(&ys)) {
            (Some(fx), Some(fy)) => {
                let mut c = self.cal.lock().unwrap().clone();
                c.v = 3;
                c.m = [fx[0], fx[1], fx[2], fy[0], fy[1], fy[2]];
                *self.cal.lock().unwrap() = c.clone();
                self.st.d.settings.cal = Some(c);
                self.st.save();
                self.screen = Screen::Home;
                self.draw_screen(true);
                self.first_run_help();
            }
            _ => {
                self.toast("Touch setup failed, try again");
                self.screen = Screen::Calibrate { step: 0, first: (0, 0) };
                self.draw_screen(false);
            }
        }
    }

    // ---------------- modals ----------------
    pub fn draw_modal(&mut self) {
        let Some(m) = self.modal.take() else { return };
        match &m {
            Modal::Sheet(s) => s.draw(&mut self.cv),
            Modal::Card(c) => c.draw(&mut self.cv),
            Modal::Quick => {}
            Modal::Kbd(k) => {
                let mut k2 = Kbd { prompt: k.prompt.clone(), text: k.text.clone(), shift: k.shift, num: k.num, purpose: k.purpose, keys: vec![] };
                if k.multiline() {
                    self.cv.fill(0, 0, W, KBD_TOP - 6, WHITE);
                    self.cv.text(F::SerifB, 40.0, 50, 40, "Note", BLACK);
                    self.cv.fill(36, 120, W - 72, 3, BLACK);
                    let body = format!("{}|", k.text);
                    self.draw_text_body(&body);
                }
                k2.draw(&mut self.cv);
            }
        }
        self.cv.present(Wf::Auto, false);
        self.modal = Some(m);
    }

    pub fn open_kbd(&mut self, prompt: &str, text: &str, purpose: KbdFor) {
        self.modal = Some(Modal::Kbd(Kbd::new(prompt, text, purpose)));
        self.draw_modal();
    }

    fn modal_tap(&mut self, x: i32, y: i32) {
        let Some(m) = self.modal.take() else { return };
        match m {
            Modal::Sheet(s) => {
                if let Some(a) = s.hit(x, y) {
                    self.do_act(a);
                } else if !s.panel.contains(x, y) {
                    self.do_act(Act::Close);
                } else {
                    self.modal = Some(Modal::Sheet(s));
                }
            }
            Modal::Quick => {
                self.modal = Some(Modal::Quick);
                self.quick_tap(x, y);
            }
            Modal::Card(mut c) => match c.tap(x, y) {
                Some(Act::Noop) => {
                    self.modal = Some(Modal::Card(c));
                    self.draw_modal_card_redraw();
                }
                Some(a) => self.do_act(a),
                None => self.modal = Some(Modal::Card(c)),
            },
            Modal::Kbd(mut k) => match k.tap(x, y) {
                KbdRes::Nothing => self.modal = Some(Modal::Kbd(k)),
                KbdRes::Input => {
                    if k.multiline() {
                        self.modal = Some(Modal::Kbd(k));
                        self.draw_modal();
                    } else {
                        k.draw_input(&mut self.cv);
                        self.cv.present(Wf::Fast, false);
                        self.modal = Some(Modal::Kbd(k));
                    }
                }
                KbdRes::Keys => {
                    self.modal = Some(Modal::Kbd(k));
                    self.draw_modal();
                }
                KbdRes::Submit(ok) => {
                    let (purpose, text) = (k.purpose, k.text.clone());
                    self.kbd_done(purpose, text, ok);
                }
            },
        }
    }

    fn draw_modal_card_redraw(&mut self) {
        // Re-render the underlying screen, then the card on top.
        let m = self.modal.take();
        self.redraw_under();
        self.modal = m;
        self.draw_modal();
    }

    /// Repaint whatever is under a modal without presenting.
    fn redraw_under(&mut self) {
        match self.screen {
            Screen::Reader => self.reader_paint(),
            _ => self.draw_screen(false),
        }
    }

    fn kbd_done(&mut self, purpose: KbdFor, text: String, ok: bool) {
        self.modal = None;
        match purpose {
            KbdFor::Search => {
                if ok && !text.trim().is_empty() {
                    self.reader_search(text.trim());
                } else {
                    self.draw_screen(false);
                }
            }
            KbdFor::NoteTitle => {
                if ok {
                    match self.pending_new.take() {
                        Some(ink) => self.new_note(ink, &text),
                        None => {
                            if let Some(i) = self.note_idx {
                                if !text.trim().is_empty() {
                                    self.notes[i].title = text.trim().to_string();
                                    self.set_note_text(i, self.notes[i].text.clone());
                                }
                                self.draw_screen(false);
                            }
                        }
                    }
                } else {
                    self.pending_new = None;
                    self.draw_screen(false);
                }
            }
            KbdFor::NoteText(i) => {
                if ok {
                    self.set_note_text(i, text);
                }
                self.draw_screen(false);
            }
            KbdFor::HlNote(idx) => {
                if ok {
                    self.reader_hl_note(idx, text);
                } else {
                    self.draw_screen(false);
                }
            }
            KbdFor::LibFilter => {
                if ok {
                    self.lib_query = text;
                    self.lib_page = 0;
                }
                self.draw_screen(false);
            }
        }
    }

    pub fn do_act(&mut self, a: Act) {
        match a {
            Act::Close | Act::Noop => {
                self.modal = None;
                if let Some(rd) = self.rd.as_mut() {
                    rd.sel = None;
                }
                self.draw_screen(false);
            }
            Act::Define => {
                self.modal = None;
                self.reader_define();
            }
            Act::HlWord | Act::HlSentence | Act::HlNote => {
                self.modal = None;
                self.reader_hl_action(a);
            }
            Act::LightUp | Act::LightDown => {
                let s = &mut self.st.d.settings;
                s.light = (s.light + if a == Act::LightUp { 2 } else { -2 }).clamp(0, 24);
                sys::set_light(s.light);
                self.st.save();
                self.light_sheet();
            }
            Act::WordShow | Act::WordNext | Act::WordDrop => self.word_act(a),
            Act::BtForget => self.bt_forget(),
            Act::DeleteNote => self.delete_note(),
            Act::RenameNote => {
                self.modal = None;
                if let Some(i) = self.note_idx {
                    let t = self.notes[i].title.clone();
                    self.pending_new = None;
                    self.open_kbd("Rename", &t, KbdFor::NoteTitle);
                }
            }
            Act::ExportNote => {
                self.modal = None;
                if let Some(i) = self.note_idx {
                    self.export_note(i);
                }
            }
            Act::Rescan => {
                self.modal = None;
                self.items = library::scan();
                library::spawn_covers(self.items.clone(), self.st.dir.clone(), self.tx.clone());
                music::scan(self.tx.clone());
                self.draw_screen(false);
                self.toast("Rescanning books and music…");
            }
            Act::Calibrate => {
                self.modal = None;
                self.screen = Screen::Calibrate { step: 0, first: (0, 0) };
                self.draw_screen(false);
            }
            Act::Sleep => {
                self.modal = None;
                self.lock_now();
            }
            Act::UsbNet => {
                // Developer SSH over USB is chosen at boot by this marker file; normal mode is a plain USB drive.
                let auto = std::path::Path::new("/mnt/us/usbnetlite/auto");
                let msg = if auto.exists() {
                    let _ = std::fs::remove_file(auto);
                    "USB network off. Restart to apply."
                } else if std::fs::File::create(auto).is_ok() {
                    "USB network on. Restart to apply."
                } else {
                    "Could not change the setting."
                };
                self.draw_screen(false);
                self.toast(msg);
            }
            Act::Quit | Act::ToStock => self.quit = true,
            Act::Reboot => {
                self.exit_code = 42;
                self.quit = true;
            }
            Act::Poweroff => {
                self.exit_code = 43;
                self.quit = true;
            }
            _ => {
                self.modal = None;
                self.draw_screen(false);
            }
        }
    }

    pub fn toast(&mut self, msg: &str) {
        let w = self.cv.text_w(F::SansM, 28.0, msg) + 70;
        let w = w.min(W - 60);
        let x = (W - w) / 2;
        let y = screens::TAB_Y - 190;
        self.cv.rrect(x, y, w, 80, 40, BLACK, 0);
        let m = self.cv.fit(F::SansM, 28.0, msg, w - 50);
        self.cv.text_c(F::SansM, 28.0, W / 2, y + 22, &m, WHITE);
        self.cv.present(Wf::Auto, false);
        self.toast_until = Some(Instant::now() + Duration::from_millis(2600));
    }

    pub fn reader_save_pub(&mut self) {
        if let Some(rd) = &self.rd {
            let f = if rd.pages > 1 { rd.page as f32 / (rd.pages - 1) as f32 } else { 0.0 };
            let key = rd.item.key.clone();
            self.st.book(&key).frac = f;
            self.st.save();
        }
    }
}

/// Drive-based updates: files in /mnt/us/kos-update/ replace the installed ones at start-up, then the app restarts.
fn self_update() {
    let dir = std::path::Path::new("/mnt/us/kos-update");
    let mut restart = false;
    let d = "/var/local/kos/data";
    let files = [("dict_old.tsv", format!("{d}/dict_old.tsv")), ("dict.tsv", format!("{d}/dict.tsv")), ("kreader", "/var/local/kos/bin/kreader".into()), ("boot.sh", "/var/local/kos/boot.sh".into()), ("kos", "/var/local/kos/bin/kos".into())];
    for (name, dest) in files.iter().map(|(a, b)| (*a, b.as_str())) {
        let src = dir.join(name);
        if !src.exists() {
            continue;
        }
        let tmp = format!("{}.new", dest);
        if std::fs::copy(&src, &tmp).is_ok() {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755));
            if std::fs::rename(&tmp, dest).is_ok() {
                let _ = std::fs::remove_file(&src);
                restart |= name == "kos";
            }
        }
    }
    if restart {
        std::process::exit(0); // the supervisor starts the new binary
    }
}

fn main() {
    self_update();
    // leftovers from an app that was killed mid-song
    if let Ok(rd) = std::fs::read_dir("/tmp") {
        for e in rd.flatten() {
            if e.file_name().to_string_lossy().starts_with("kos_seg_") {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
    let (tx, rx) = channel::<Ev>();
    let mut app = App::new(tx.clone());
    input::spawn_cmd_fifo(tx.clone());
    input::spawn(tx, app.cal.clone());
    app.boot();
    while !app.quit {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(ev) => app.handle(ev),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        app.tick();
    }
    app.reader_save_pub();
    app.st.save();
    std::process::exit(app.exit_code);
}
