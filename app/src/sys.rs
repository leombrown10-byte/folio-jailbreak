//! Thin wrappers over the Kindle's shell tools (lipc, date) and libc.
use std::process::{Command, Stdio};

/// Run a shell command and return its trimmed stdout. Gives up after 3 s so a stuck system service can never
/// freeze the UI; slow or timed-out commands are noted in data/slow.log.
pub fn sh(cmd: &str) -> String {
    sh_timeout(cmd, Some(std::time::Duration::from_secs(3)))
}

/// Same, with no time limit (used for suspend-to-RAM, which blocks for the whole sleep).
pub fn sh_wait(cmd: &str) -> String {
    sh_timeout(cmd, None)
}

fn sh_timeout(cmd: &str, limit: Option<std::time::Duration>) -> String {
    use std::io::Read;
    let Ok(mut child) = Command::new("sh").arg("-c").arg(cmd).stdout(Stdio::piped()).stderr(Stdio::null()).spawn() else {
        return String::new();
    };
    let mut out = child.stdout.take().unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut s = String::new();
        let _ = out.read_to_string(&mut s);
        let _ = tx.send(s);
    });
    let t0 = std::time::Instant::now();
    let res = match limit {
        Some(l) => rx.recv_timeout(l).ok(),
        None => rx.recv().ok(),
    };
    match res {
        Some(s) => {
            let _ = child.wait();
            if t0.elapsed() > std::time::Duration::from_secs(1) {
                slow_log(&format!("slow {:?}: {}", t0.elapsed(), cmd));
            }
            s.trim().to_string()
        }
        None => {
            let _ = child.kill();
            let _ = child.wait();
            slow_log(&format!("TIMEOUT: {}", cmd));
            String::new()
        }
    }
}

fn slow_log(msg: &str) {
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("/var/local/kos/data/slow.log") {
        let _ = writeln!(f, "{} {}", now(), msg);
    }
}

pub fn battery() -> i32 {
    sh("lipc-get-prop -i com.lab126.powerd battLevel").parse().unwrap_or(-1)
}

pub fn charging() -> bool {
    sh("lipc-get-prop -i com.lab126.powerd isCharging") == "1"
}

pub fn light() -> i32 {
    sh("lipc-get-prop -i com.lab126.powerd flIntensity").parse().unwrap_or(0)
}

pub fn set_light(n: i32) {
    sh(&format!("lipc-set-prop -i com.lab126.powerd flIntensity {}", n.clamp(0, 24)));
}

pub fn bt_device() -> String {
    sh("lipc-get-prop com.lab126.btfd BTconnectedDevName")
}

pub fn audio_output_is_bt() -> bool {
    sh("lipc-get-prop -i com.lab126.audiomgrd audioCurrentOutput") != "0"
}

fn tm() -> libc::tm {
    unsafe {
        let t = libc::time(std::ptr::null_mut());
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&t, &mut tm);
        tm
    }
}

pub fn clock() -> String {
    let t = tm();
    format!("{}:{:02}", t.tm_hour, t.tm_min)
}

pub fn hour() -> i32 {
    tm().tm_hour
}

pub fn date_long() -> String {
    const D: [&str; 7] = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
    const M: [&str; 12] =
        ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
    let t = tm();
    format!("{}, {} {}", D[t.tm_wday as usize % 7], M[t.tm_mon as usize % 12], t.tm_mday)
}

pub fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}
