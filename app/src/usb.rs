//! USB drive mode. When a computer is connected, volumd takes the user storage away from the device.
//! Before it does, it asks apps to let go (userstoreIsLikelyToUnMount) and waits for userstoreReadyToUnMount.
use crate::sys::sh;
use crate::{App, Ev, Screen};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;

/// Forward volumd's drive-mode events to the app. lipc-wait-event is restarted if it ever exits.
pub fn spawn(tx: Sender<Ev>) {
    std::thread::spawn(move || loop {
        if let Ok(mut child) = Command::new("lipc-wait-event")
            .args(["-m", "com.lab126.volumd", "userstoreIsLikelyToUnMount,driveModeStateChanged"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        {
            if let Some(out) = child.stdout.take() {
                for line in BufReader::new(out).lines().flatten() {
                    let _ = tx.send(Ev::Usb(line));
                }
            }
            let _ = child.wait();
        }
        std::thread::sleep(std::time::Duration::from_secs(2));
    });
}

pub fn storage_available() -> bool {
    sh("lipc-get-prop -i com.lab126.volumd userstoreIsAvailable") != "0"
}

impl App {
    /// Called at start-up (cable may already be connected) and after each drive-mode event.
    pub fn usb_sync(&mut self) {
        let avail = storage_available();
        if !avail && self.screen != Screen::Usb {
            self.usb_enter();
        } else if avail && self.screen == Screen::Usb {
            self.usb_leave();
        }
    }

    /// Let go of everything on the user storage, tell volumd it may unmount, and show the drive-mode screen.
    pub fn usb_enter(&mut self) {
        let _ = self.player.tx.send(crate::music::Cmd::Stop);
        self.reader_save_pub();
        self.rd = None; // closes kreader and the open book
        self.modal = None;
        self.note_idx = None;
        std::thread::sleep(std::time::Duration::from_millis(400)); // let the player thread close its file
        sh("lipc-set-prop -i com.lab126.volumd userstoreReadyToUnMount 1");
        self.screen = Screen::Usb;
        self.draw_screen(true);
    }

    pub fn usb_leave(&mut self) {
        self.items = crate::library::scan();
        crate::library::spawn_covers(self.items.clone(), self.st.dir.clone(), self.tx.clone());
        crate::music::scan(self.tx.clone());
        self.notes = crate::notes::load_all(&self.st.dir);
        self.screen = Screen::Home;
        self.draw_screen(true);
    }

    pub fn usb_event(&mut self, line: &str) {
        if line.contains("LikelyToUnMount") {
            if self.screen != Screen::Usb {
                self.usb_enter();
            }
        } else {
            // driveModeStateChanged: wait a moment for the mount to settle, then look at the storage
            std::thread::sleep(std::time::Duration::from_millis(1500));
            self.usb_sync();
        }
    }

    pub fn draw_usb(&mut self) {
        use crate::gfx::*;
        self.cv.clear(WHITE);
        // a simple USB trident-free glyph: plug outline
        let (cx, cy) = (W / 2, 520);
        self.cv.rrect(cx - 90, cy - 130, 180, 260, 24, BLACK, 8);
        self.cv.fill(cx - 50, cy - 175, 28, 46, BLACK);
        self.cv.fill(cx + 22, cy - 175, 28, 46, BLACK);
        self.cv.disc(cx, cy + 10, 26, BLACK);
        self.cv.text_c(F::SerifB, 64.0, W / 2, 780, "USB drive mode", BLACK);
        let lines = self.cv.wrap(F::Serif, 36.0, "Folio is connected to a computer. Copy books and music, then eject it there or unplug the cable to keep reading.", W - 200, 5);
        for (i, l) in lines.iter().enumerate() {
            self.cv.text_c(F::Serif, 36.0, W / 2, 880 + i as i32 * 52, l, DARK);
        }
        self.cv.present(Wf::Full, true);
    }
}
