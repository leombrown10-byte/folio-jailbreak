//! Touchscreen (Goodix, multitouch protocol B, first finger only) and power-key readers.
use crate::store::Cal;
use crate::Ev;
use std::fs::File;
use std::io::Read;
use std::sync::mpsc::Sender;

const TOUCH_DEV: &str = "/dev/input/event2";
const KEY_DEVS: [&str; 2] = ["/dev/input/event0", "/dev/input/event1"];

#[repr(C)]
#[derive(Default)]
struct AbsInfo {
    value: i32,
    min: i32,
    max: i32,
    fuzz: i32,
    flat: i32,
    res: i32,
}

/// Raw (max_x, max_y) of the touch panel.
pub fn touch_range() -> (i32, i32) {
    use std::os::unix::io::AsRawFd;
    let Ok(f) = File::open(TOUCH_DEV) else { return (1071, 1447) };
    let get = |axis: u32| -> i32 {
        let mut a = AbsInfo::default();
        let req: u32 = (2u32 << 30) | (24u32 << 16) | (0x45u32 << 8) | (0x40 + axis);
        let r = unsafe { libc::ioctl(f.as_raw_fd(), req as _, &mut a as *mut AbsInfo) };
        if r < 0 { 0 } else { a.max }
    };
    let (mx, my) = (get(0x35), get(0x36));
    (if mx > 0 { mx } else { 1071 }, if my > 0 { my } else { 1447 })
}

pub fn map(cal: &Cal, rx: i32, ry: i32) -> (i32, i32) {
    use crate::store::{CAL_P1, CAL_P2};
    if cal.v >= 3 {
        let (fx, fy) = (rx as f64, ry as f64);
        let x = cal.m[0] * fx + cal.m[1] * fy + cal.m[2];
        let y = cal.m[3] * fx + cal.m[4] * fy + cal.m[5];
        return ((x.round() as i32).clamp(0, crate::gfx::W - 1), (y.round() as i32).clamp(0, crate::gfx::H - 1));
    }
    let (a, b) = if cal.swap { (ry, rx) } else { (rx, ry) };
    if cal.v == 2 && cal.a2 != cal.a1 && cal.b2 != cal.b1 {
        let x = CAL_P1.0 + (a - cal.a1) * (CAL_P2.0 - CAL_P1.0) / (cal.a2 - cal.a1);
        let y = CAL_P1.1 + (b - cal.b1) * (CAL_P2.1 - CAL_P1.1) / (cal.b2 - cal.b1);
        return (x.clamp(0, crate::gfx::W - 1), y.clamp(0, crate::gfx::H - 1));
    }
    let (ra, rb) = if cal.swap { (cal.max_y, cal.max_x) } else { (cal.max_x, cal.max_y) };
    let mut x = a * crate::gfx::W / (ra + 1);
    let mut y = b * crate::gfx::H / (rb + 1);
    if cal.flip_x {
        x = crate::gfx::W - 1 - x;
    }
    if cal.flip_y {
        y = crate::gfx::H - 1 - y;
    }
    (x.clamp(0, crate::gfx::W - 1), y.clamp(0, crate::gfx::H - 1))
}

/// Spawn the reader threads. The shared calibration is looked up per event so the setup screen can change it live.
pub fn spawn(tx: Sender<Ev>, cal: std::sync::Arc<std::sync::Mutex<Cal>>) {
    let txt = tx.clone();
    std::thread::spawn(move || {
        let Ok(mut f) = File::open(TOUCH_DEV) else { return };
        let (mut rx, mut ry) = (0, 0);
        let (mut down, mut pdown, mut up, mut moved) = (false, false, false, false);
        let mut buf = [0u8; 16];
        while f.read_exact(&mut buf).is_ok() {
            let t = u16::from_le_bytes([buf[8], buf[9]]);
            let c = u16::from_le_bytes([buf[10], buf[11]]);
            let v = i32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]);
            match (t, c) {
                (3, 0x35) => { rx = v; moved = true; }
                (3, 0x36) => { ry = v; moved = true; }
                (3, 0x39) => { if v < 0 { up = true } else if !down { pdown = true } }
                (1, 0x14a) => { if v == 0 { up = true } else if !down { pdown = true } }
                (0, 0) => {
                    let (x, y) = map(&cal.lock().unwrap(), rx, ry);
                    if pdown && !down {
                        down = true;
                        let _ = txt.send(Ev::Raw(rx, ry));
                        let _ = txt.send(Ev::Down(x, y));
                    } else if up {
                        if down {
                            let _ = txt.send(Ev::Raw(rx, ry));
                            let _ = txt.send(Ev::Up(x, y));
                        }
                        down = false;
                    } else if moved && down {
                        let _ = txt.send(Ev::Move(x, y));
                    }
                    pdown = false;
                    up = false;
                    moved = false;
                }
                _ => {}
            }
        }
    });
    for dev in KEY_DEVS {
        let tx = tx.clone();
        std::thread::spawn(move || {
            let Ok(mut f) = File::open(dev) else { return };
            let mut buf = [0u8; 16];
            while f.read_exact(&mut buf).is_ok() {
                let t = u16::from_le_bytes([buf[8], buf[9]]);
                let c = u16::from_le_bytes([buf[10], buf[11]]);
                let v = i32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]);
                if t == 1 && c == 116 && v == 1 {
                    let _ = tx.send(Ev::Power);
                }
            }
        });
    }
}

/// Developer test channel: `echo "tap 500 700" > /tmp/kos.cmd` injects pixel-space touches (bypasses calibration).
/// Commands: tap X Y | long X Y | swipe X0 Y0 X1 Y1 | power | skipcal
pub fn spawn_cmd_fifo(tx: Sender<Ev>) {
    const PATH: &str = "/tmp/kos.cmd";
    std::thread::spawn(move || {
        use std::io::{BufRead, BufReader};
        let c = std::ffi::CString::new(PATH).unwrap();
        unsafe { libc::mkfifo(c.as_ptr(), 0o666) };
        loop {
            let Ok(f) = File::open(PATH) else { return };
            for line in BufReader::new(f).lines().flatten() {
                let p: Vec<&str> = line.split_whitespace().collect();
                let n = |i: usize| p.get(i).and_then(|x| x.parse::<i32>().ok()).unwrap_or(0);
                match p.first().copied() {
                    Some("tap") => {
                        let _ = tx.send(Ev::Down(n(1), n(2)));
                        std::thread::sleep(std::time::Duration::from_millis(60));
                        let _ = tx.send(Ev::Up(n(1), n(2)));
                    }
                    Some("long") => {
                        let _ = tx.send(Ev::Down(n(1), n(2)));
                        std::thread::sleep(std::time::Duration::from_millis(1100));
                        let _ = tx.send(Ev::Up(n(1), n(2)));
                    }
                    Some("swipe") => {
                        let _ = tx.send(Ev::Down(n(1), n(2)));
                        for k in 1..=5 {
                            let _ = tx.send(Ev::Move(n(1) + (n(3) - n(1)) * k / 5, n(2) + (n(4) - n(2)) * k / 5));
                            std::thread::sleep(std::time::Duration::from_millis(20));
                        }
                        let _ = tx.send(Ev::Up(n(3), n(4)));
                    }
                    Some("power") => {
                        let _ = tx.send(Ev::Power);
                    }
                    Some("skipcal") => {
                        let _ = tx.send(Ev::SkipCal);
                    }
                    _ => {}
                }
            }
        }
    });
}
