//! Simulator link. With KOS_SIM=host:port the app sends its screen updates to folio-sim over TCP instead of
//! drawing with FBInk, and receives touches / power presses from it instead of reading /dev/input.
//!
//! app -> sim:  b'H', pw, ph (u16 LE) once at connect: the panel size being simulated (default 1072x1448);
//!              then b'F', x, y, w, h (u16 LE), mode (0 auto, 1 fast, 2 text, 3 full), flash (0/1), w*h gray bytes
//!              (frames are in panel pixels)
//! sim -> app:  text lines: "D x y" | "M x y" | "U x y" (touch, in panel pixels) | "P" (power button)
use crate::Ev;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::mpsc::Sender;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

static CONN: OnceLock<Mutex<TcpStream>> = OnceLock::new();

pub fn enabled() -> bool {
    std::env::var("KOS_SIM").is_ok()
}

/// Connect to the simulator (retrying for a while: it may still be starting).
pub fn connect() {
    let Ok(addr) = std::env::var("KOS_SIM") else { return };
    for _ in 0..60 {
        if let Ok(mut s) = TcpStream::connect(&addr) {
            let _ = s.set_nodelay(true);
            // handshake: the panel size this run simulates (frames and touches use panel pixels)
            let (pw, ph) = crate::display::size();
            let mut h = vec![b'H'];
            h.extend_from_slice(&(pw as u16).to_le_bytes());
            h.extend_from_slice(&(ph as u16).to_le_bytes());
            let _ = s.write_all(&h);
            let _ = CONN.set(Mutex::new(s));
            return;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    eprintln!("kos: cannot reach the simulator at {}", addr);
    std::process::exit(1);
}

pub fn frame(x: i32, y: i32, w: i32, h: i32, mode: u8, flash: bool, px: &[u8]) {
    let Some(c) = CONN.get() else { return };
    let mut head = vec![b'F'];
    for v in [x, y, w, h] {
        head.extend_from_slice(&(v as u16).to_le_bytes());
    }
    head.push(mode);
    head.push(flash as u8);
    let mut s = c.lock().unwrap();
    if s.write_all(&head).and_then(|_| s.write_all(px)).is_err() {
        std::process::exit(0); // simulator closed
    }
}

/// Touch and power events from the simulator window.
pub fn spawn_input(tx: Sender<Ev>) {
    let Some(c) = CONN.get() else { return };
    let Ok(read) = c.lock().unwrap().try_clone() else { return };
    std::thread::spawn(move || {
        for line in BufReader::new(read).lines().flatten() {
            let p: Vec<&str> = line.split_whitespace().collect();
            let n = |i: usize| p.get(i).and_then(|x| x.parse::<i32>().ok()).unwrap_or(0);
            let l = crate::display::to_logical(n(1), n(2)); // touches arrive in panel pixels
            let _ = match p.first().copied() {
                Some("D") => tx.send(Ev::Down(l.0, l.1)),
                Some("M") => tx.send(Ev::Move(l.0, l.1)),
                Some("U") => tx.send(Ev::Up(l.0, l.1)),
                Some("P") => tx.send(Ev::Power),
                _ => Ok(()),
            };
        }
        std::process::exit(0);
    });
}
