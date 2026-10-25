//! Bluetooth headphone management through Amazon's btfd (lipc): scan, pair, connect, disconnect, forget.
//! Audio itself flows through audiomgrd once a device is connected (see music.rs).
use crate::gfx::*;
use crate::sys::sh;
use crate::ui::*;
use crate::{App, Ev, Modal, Screen};

#[derive(Clone, Debug, Default)]
pub struct BtDev {
    pub name: String,
    pub addr: String,
    pub paired: bool,
    pub connected: bool,
}

fn find_mac(s: &str) -> Option<String> {
    let b: Vec<char> = s.chars().collect();
    let is_hex = |c: char| c.is_ascii_hexdigit();
    let mut i = 0;
    while i + 17 <= b.len() {
        let w = &b[i..i + 17];
        if (0..17).all(|k| if k % 3 == 2 { w[k] == ':' } else { is_hex(w[k]) }) {
            return Some(w.iter().collect());
        }
        i += 1;
    }
    None
}

/// Parse `lipc-hash-prop` output generously: a device starts at each line with a MAC or a name key.
pub fn parse_list(out: &str) -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = vec![];
    let mut cur_name = String::new();
    let mut cur_addr = String::new();
    let mut flush = |name: &mut String, addr: &mut String, v: &mut Vec<(String, String)>| {
        if !addr.is_empty() {
            v.push((std::mem::take(name), std::mem::take(addr)));
        } else {
            name.clear();
        }
    };
    for l in out.lines() {
        let low = l.to_lowercase();
        let val = l.split(|c| c == '=' || c == ':').skip(1).collect::<Vec<_>>().join(":").trim().trim_matches(|c| c == '"' || c == '\'' || c == ',').to_string();
        if let Some(m) = find_mac(l) {
            if !cur_addr.is_empty() {
                flush(&mut cur_name, &mut cur_addr, &mut v);
            }
            cur_addr = m;
        } else if low.contains("name") && !val.is_empty() {
            cur_name = val;
        }
    }
    flush(&mut cur_name, &mut cur_addr, &mut v);
    v
}

impl App {
    pub fn bt_refresh(&mut self) {
        let get = |p: &str| sh(&format!("lipc-hash-prop com.lab126.btfd {} </dev/null", p));
        let paired = parse_list(&get("ListPaired"));
        let conn = parse_list(&get("ListConnected"));
        let disc = parse_list(&get("ListDiscovered"));
        let mut devs: Vec<BtDev> = vec![];
        for (n, a) in paired.iter() {
            devs.push(BtDev { name: n.clone(), addr: a.clone(), paired: true, connected: conn.iter().any(|c| c.1 == *a) });
        }
        for (n, a) in disc.iter() {
            if !devs.iter().any(|d| d.addr == *a) {
                devs.push(BtDev { name: n.clone(), addr: a.clone(), paired: false, connected: false });
            }
        }
        for d in devs.iter_mut() {
            if d.name.is_empty() {
                d.name = format!("Headphones {}", &d.addr[d.addr.len().saturating_sub(5)..]);
            }
        }
        self.bt_devs = devs;
    }

    pub fn bt_open(&mut self) {
        self.screen = Screen::Bluetooth;
        self.bt_refresh();
        self.draw_screen(false);
    }

    pub fn bt_scan(&mut self) {
        self.bt_scanning = true;
        self.draw_screen(false);
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            sh("lipc-set-prop -i com.lab126.btfd BTenable 1; lipc-set-prop -i com.lab126.btfd triggerBTscan 1");
            std::thread::sleep(std::time::Duration::from_secs(14));
            let _ = tx.send(Ev::Bt);
        });
    }

    fn bt_action(&mut self, args: &'static str, addr: String, wait: u64) {
        let tx = self.tx.clone();
        self.bt_busy = true;
        self.draw_screen(false);
        std::thread::spawn(move || {
            for step in args.split(';') {
                match step {
                    "bond" => {
                        sh(&format!("lipc-set-prop com.lab126.btfd Bond '{}'", addr));
                        std::thread::sleep(std::time::Duration::from_secs(4));
                        sh("lipc-set-prop -i com.lab126.btfd PairUserConfirmation 1");
                        std::thread::sleep(std::time::Duration::from_secs(3));
                    }
                    "connect" => {
                        sh(&format!("lipc-set-prop com.lab126.btfd Connect '{}'", addr));
                    }
                    "disconnect" => {
                        sh(&format!("lipc-set-prop com.lab126.btfd Disconnect '{}'", addr));
                    }
                    "forget" => {
                        sh(&format!("lipc-set-prop com.lab126.btfd Unbond '{}'", addr));
                    }
                    _ => {}
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(wait));
            let _ = tx.send(Ev::Bt);
        });
    }

    pub fn bt_event(&mut self) {
        self.bt_scanning = false;
        self.bt_busy = false;
        self.bt_refresh();
        if self.screen == Screen::Bluetooth && self.modal.is_none() {
            self.draw_screen(false);
        }
    }

    pub fn draw_bt(&mut self) {
        self.cv.fill(0, 0, W, H, WHITE);
        icon(&mut self.cv, "back", 60, 78, BLACK);
        self.cv.text(F::SerifB, 56.0, 120, 48, "Bluetooth audio", BLACK);
        self.cv.fill(36, 128, W - 72, 3, BLACK);
        let on = crate::sys::audio_output_is_bt();
        let name = crate::sys::bt_device();
        let status = if self.bt_busy {
            "Working…".to_string()
        } else if self.bt_scanning {
            "Scanning… put your headphones in pairing mode".to_string()
        } else if on {
            format!("Connected{}", if name.is_empty() { String::new() } else { format!(": {}", name) })
        } else {
            "Not connected".to_string()
        };
        self.cv.text(F::Sans, 30.0, 50, 160, &status, if on { BLACK } else { MID });
        button(&mut self.cv, Rect::new(50, 220, W - 100, 100), if self.bt_scanning { "Scanning…" } else { "Scan for headphones" }, true);
        if self.bt_devs.is_empty() {
            self.cv.text_c(F::Serif, 34.0, W / 2, 440, "No devices yet", MID);
            let lines = self.cv.wrap(F::Sans, 28.0, "Put your headphones in pairing mode, then tap Scan. Only audio devices are shown.", W - 160, 4);
            for (i, l) in lines.iter().enumerate() {
                self.cv.text_c(F::Sans, 28.0, W / 2, 500 + i as i32 * 40, l, MID);
            }
        }
        for (i, d) in self.bt_devs.clone().iter().enumerate().take(8) {
            let y = 360 + i as i32 * 130;
            let n = self.cv.fit(F::SansM, 36.0, &d.name, W - 340);
            self.cv.text(F::SansM, 36.0, 50, y + 22, &n, BLACK);
            let st = if d.connected { "Connected" } else if d.paired { "Paired" } else { "Available" };
            self.cv.text(F::Sans, 26.0, 50, y + 72, st, MID);
            let r = Rect::new(W - 270, y + 14, 220, 80);
            button(&mut self.cv, r, if d.connected { "Disconnect" } else if d.paired { "Connect" } else { "Pair" }, !d.connected);
            self.cv.fill(50, y + 120, W - 100, 2, LIGHT);
        }
        self.cv.present(Wf::Text, true);
    }

    pub fn bt_tap(&mut self, x: i32, y: i32) {
        if y < 130 && x < 400 {
            self.screen = Screen::More;
            return self.draw_screen(false);
        }
        if self.bt_busy || self.bt_scanning {
            return;
        }
        if Rect::new(50, 220, W - 100, 100).contains(x, y) {
            return self.bt_scan();
        }
        for (i, d) in self.bt_devs.clone().iter().enumerate().take(8) {
            let y0 = 360 + i as i32 * 130;
            if Rect::new(W - 270, y0 + 14, 220, 80).contains(x, y) {
                if d.connected {
                    self.bt_action("disconnect", d.addr.clone(), 3);
                } else if d.paired {
                    self.bt_action("connect", d.addr.clone(), 8);
                } else {
                    self.bt_action("bond;connect", d.addr.clone(), 8);
                }
                return;
            }
            if Rect::new(50, y0, W - 340, 110).contains(x, y) && d.paired {
                self.bt_sel = Some(i);
                self.modal = Some(Modal::Sheet(Sheet::new(&d.name, vec![("Forget this device", Act::BtForget), ("Cancel", Act::Close)])));
                self.draw_modal();
                return;
            }
        }
    }

    pub fn bt_forget(&mut self) {
        if let Some(i) = self.bt_sel.take() {
            if let Some(d) = self.bt_devs.get(i).cloned() {
                self.modal = None;
                self.bt_action("forget", d.addr, 2);
            }
        }
    }
}
