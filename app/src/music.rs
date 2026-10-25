//! Music: library scan, symphonia decoding (MP3/FLAC/OGG/WAV/AAC/M4A/ALAC) and playback through Amazon's
//! mixer (`mplay`, raw PCM on stdin), which routes to the paired Bluetooth headphones.
use crate::Ev;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_AAC, CODEC_TYPE_MP3, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymErr;
use symphonia::core::formats::{FormatOptions, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::{MetadataOptions, StandardTagKey};
use symphonia::core::probe::Hint;
use symphonia::core::units::Time;

pub const MUSIC_DIR: &str = "/mnt/us/music";
const EXTS: [&str; 8] = ["mp3", "flac", "ogg", "oga", "wav", "m4a", "aac", "mp4"];

#[derive(Clone, Debug)]
pub struct Track {
    pub path: String,
    pub title: String,
    pub artist: String,
}

#[derive(Debug)]
pub enum MEv {
    Library(Vec<Track>),
    Progress,
    Track(usize),
    Ended,
    NoOutput,
    Failed(String),
}

pub enum Cmd {
    Play(usize),
    PlayAt(usize, u32),
    Pause,
    Resume,
    Next,
    Prev,
    Seek(u32),
    Stop,
}

#[derive(Default, Clone)]
pub struct PState {
    pub queue: Vec<Track>,
    pub idx: Option<usize>,
    pub playing: bool,
    pub pos: u32,
    pub dur: u32,
    pub volume: i32,
    pub shuffle: bool,
    pub repeat: u8, // 0 off, 1 all, 2 one
}

/// The track to play after `idx`. `auto` is true when the song ended by itself (repeat-one only applies then).
fn pick(s: &PState, idx: usize, auto: bool) -> Option<usize> {
    let n = s.queue.len();
    if n == 0 {
        return None;
    }
    if auto && s.repeat == 2 {
        return Some(idx);
    }
    if s.shuffle && n > 1 {
        let mut x = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(7) as u64 | 1;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        let r = (x % (n as u64 - 1)) as usize; // any track but this one
        return Some(if r >= idx { r + 1 } else { r });
    }
    if idx + 1 < n {
        Some(idx + 1)
    } else if s.repeat == 1 {
        Some(0)
    } else {
        None
    }
}

pub struct Player {
    pub tx: Sender<Cmd>,
    pub st: Arc<Mutex<PState>>,
}

fn clean(s: &str) -> String {
    s.replace('_', " ").trim().to_string()
}

fn read_tags(path: &str) -> Track {
    let p = std::path::Path::new(path);
    let stem = clean(&p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default());
    let (mut artist, mut title) = match stem.split_once(" - ") {
        Some((a, t)) => (a.trim().to_string(), t.trim().to_string()),
        None => (String::new(), stem.clone()),
    };
    if artist.is_empty() {
        artist = p.parent().and_then(|d| d.file_name()).map(|s| clean(&s.to_string_lossy())).filter(|s| s != "music").unwrap_or_default();
    }
    if let Ok(file) = std::fs::File::open(path) {
        let mss = MediaSourceStream::new(Box::new(file), Default::default());
        let mut hint = Hint::new();
        if let Some(e) = p.extension() {
            hint.with_extension(&e.to_string_lossy());
        }
        if let Ok(mut probed) = symphonia::default::get_probe().format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default()) {
            let mut grab = |tags: &[symphonia::core::meta::Tag]| {
                for t in tags {
                    match t.std_key {
                        Some(StandardTagKey::TrackTitle) => title = t.value.to_string(),
                        Some(StandardTagKey::Artist) => artist = t.value.to_string(),
                        _ => {}
                    }
                }
            };
            if let Some(m) = probed.metadata.get().as_ref().and_then(|m| m.current()) {
                grab(m.tags());
            }
            if let Some(m) = probed.format.metadata().current() {
                grab(m.tags());
            }
        }
    }
    Track { path: path.to_string(), title, artist }
}

fn walk(dir: &std::path::Path, out: &mut Vec<String>, depth: u32) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = rd.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let p = e.path();
        if e.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        if p.is_dir() {
            if depth < 4 {
                walk(&p, out, depth + 1);
            }
        } else if p.extension().map_or(false, |x| EXTS.contains(&x.to_string_lossy().to_lowercase().as_str())) {
            out.push(p.to_string_lossy().to_string());
        }
    }
}

pub fn scan(evtx: Sender<Ev>) {
    std::thread::spawn(move || {
        let mut paths = vec![];
        walk(std::path::Path::new(MUSIC_DIR), &mut paths, 0);
        let tracks: Vec<Track> = paths.iter().map(|p| read_tags(p)).collect();
        let _ = evtx.send(Ev::Music(MEv::Library(tracks)));
    });
}

struct Resampler {
    ratio: f64,
    pos: f64,
    last: Vec<i16>,
    ch: usize,
}

impl Resampler {
    fn new(from: u32, to: u32, ch: usize) -> Resampler {
        Resampler { ratio: from as f64 / to as f64, pos: 0.0, last: vec![0; ch], ch }
    }
    /// Linear interpolation; good enough for a handheld's Bluetooth path.
    fn process(&mut self, input: &[i16], out: &mut Vec<i16>) {
        let n = input.len() / self.ch;
        if n == 0 {
            return;
        }
        while self.pos < n as f64 {
            let i = self.pos.floor() as isize;
            let f = (self.pos - i as f64) as f32;
            for c in 0..self.ch {
                let a = (if i < 0 { self.last[c] } else { input[i as usize * self.ch + c] }) as f32;
                let b = if (i + 1) < n as isize { input[(i + 1) as usize * self.ch + c] as f32 } else { a };
                out.push((a + (b - a) * f) as i16);
            }
            self.pos += self.ratio;
        }
        self.pos -= n as f64;
        self.last.copy_from_slice(&input[(n - 1) * self.ch..n * self.ch]);
    }
}

/// Amazon's mixer plays complete files (it will not read pipes or growing files), so a track is decoded into
/// short raw-PCM segment files that are handed to successive `mplay` processes. Each later segment starts with
/// SEG_PAD of silence and is launched SEG_PAD early, so its audio begins exactly when the previous one ends
/// (both processes have the same start-up latency).
const SEG_PAD: f64 = 0.5;
/// The mixer drops its last buffered half-second when a file ends; silence there keeps the cut inaudible.
const TAIL: f64 = 1.2;
/// Silence before the first sample: the Bluetooth stream wakes up and the mixer ramps in during this time.
const LEAD: f64 = 0.8;
/// Songs up to this long play as one file (no joins to click); longer ones are cut into SEG_SECS pieces.
const WHOLE_TRACK_MAX: u32 = 900;
const SEG_SECS: f64 = 60.0;

fn played_secs(t: &Instant) -> u32 {
    (t.elapsed().as_secs_f64() - LEAD).max(0.0) as u32
}

fn spawn_mplay(rate: u32, ch: u32, file: &str) -> Option<Child> {
    Command::new("mplay")
        .args(["-r", &rate.to_string(), "-c", &ch.to_string(), file])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .ok()
}

enum Outcome {
    Ended,
    Cmd(Cmd),
    Failed(String),
}

struct Playing {
    kids: Vec<(Child, String)>,
}

impl Playing {
    fn reap(&mut self) {
        self.kids.retain_mut(|(c, f)| match c.try_wait() {
            Ok(Some(_)) => {
                let _ = std::fs::remove_file(&*f);
                false
            }
            _ => true,
        });
    }
    fn stop(&mut self) {
        for (c, f) in self.kids.iter_mut() {
            let _ = c.kill();
            let _ = c.wait();
            let _ = std::fs::remove_file(&*f);
        }
        self.kids.clear();
    }
}

/// Keeps Amazon's mixer output and the Bluetooth stream open with silence during a listening session. Without it
/// the first song after a pause pops while the stream wakes up.
struct Keep {
    kids: Vec<Child>,
    since: Option<Instant>,
    last_use: Instant,
}

const KEEP_FILE: &str = "/tmp/kos_sil.raw";
const KEEP_SECS: u64 = 120; // length of the silence file
const KEEP_IDLE: Duration = Duration::from_secs(90); // stop after this long without playback
const KEEP_WARM: Duration = Duration::from_millis(3000);

impl Keep {
    fn new() -> Keep {
        Keep { kids: vec![], since: None, last_use: Instant::now() }
    }
    fn reap(&mut self) {
        self.kids.retain_mut(|c| matches!(c.try_wait(), Ok(None)));
        if self.kids.is_empty() {
            self.since = None;
        }
    }
    /// Make sure a silent stream is playing; `used` marks real playback activity.
    fn ensure(&mut self, used: bool) {
        if used {
            self.last_use = Instant::now();
        }
        self.reap();
        let renew = self.since.map_or(true, |s| s.elapsed() > Duration::from_secs(KEEP_SECS - 20));
        if renew && self.last_use.elapsed() < KEEP_IDLE {
            if !std::path::Path::new(KEEP_FILE).exists() {
                let _ = std::fs::write(KEEP_FILE, vec![0u8; 44100 * 4 * KEEP_SECS as usize]);
            }
            if let Some(c) = spawn_mplay(44100, 2, KEEP_FILE) {
                self.kids.push(c);
                if self.since.is_none() || self.kids.len() == 1 {
                    self.since = Some(Instant::now());
                }
            }
        }
    }
    fn warm(&self) -> bool {
        self.since.map_or(false, |s| s.elapsed() >= KEEP_WARM)
    }
    fn tick_idle(&mut self) {
        self.reap();
        if !self.kids.is_empty() && self.last_use.elapsed() > KEEP_IDLE {
            for c in self.kids.iter_mut() {
                let _ = c.kill();
                let _ = c.wait();
            }
            self.kids.clear();
            self.since = None;
        }
    }
}

fn play_track(t: &Track, start: u32, rx: &Receiver<Cmd>, st: &Arc<Mutex<PState>>, evtx: &Sender<Ev>, ka: &mut Keep) -> Outcome {
    let Ok(file) = std::fs::File::open(&t.path) else { return Outcome::Failed("can't open file".into()) };
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(e) = std::path::Path::new(&t.path).extension() {
        hint.with_extension(&e.to_string_lossy());
    }
    let probed = match symphonia::default::get_probe().format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default()) {
        Ok(p) => p,
        Err(e) => return Outcome::Failed(format!("{}", e)),
    };
    let mut format = probed.format;
    let Some(track) = format.tracks().iter().find(|t| t.codec_params.codec != CODEC_TYPE_NULL).cloned() else {
        return Outcome::Failed("no audio track".into());
    };
    let Ok(mut decoder) = symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default()) else {
        return Outcome::Failed("unsupported codec".into());
    };
    let src_rate = track.codec_params.sample_rate.unwrap_or(44100);
    let src_ch = track.codec_params.channels.map(|c| c.count()).unwrap_or(2).max(1);
    let dur = track.codec_params.n_frames.map(|n| (n / src_rate as u64) as u32).unwrap_or(0);
    st.lock().unwrap().dur = dur;
    if start > 0 {
        let _ = format.seek(SeekMode::Coarse, SeekTo::Time { time: Time::from(start as f64), track_id: Some(track.id) });
        decoder.reset();
    }
    let out_rate: u32 = if src_rate == 48000 { 48000 } else { 44100 };
    let out_ch = src_ch.min(2);
    let mut rs = if src_rate != out_rate { Some(Resampler::new(src_rate, out_rate, out_ch)) } else { None };
    let bytes_per_sec = (out_rate as usize) * out_ch * 2;
    let id = std::process::id();

    let mut play = Playing { kids: vec![] };
    let mut sbuf: Option<SampleBuffer<i16>> = None;
    let mut seg: Vec<u8> = Vec::new();
    let mut pcm: Vec<i16> = Vec::new();
    let mut k = 0usize; // segment index
    let mut cum = 0.0f64; // seconds of audio already scheduled
    let mut t0: Option<Instant> = None;
    let mut last_report = Instant::now();
    let mut last_bt_check = Instant::now();
    let mut eof = false;
    // Lossy decoders (AAC, MP3) emit a few rough frames while their state warms up: drop them, then fade in.
    let mut skip_frames: usize = match track.codec_params.codec {
        c if c == CODEC_TYPE_AAC => 2112,
        c if c == CODEC_TYPE_MP3 => 1105,
        _ => 0,
    };
    let fade_total = (src_rate as usize) / 12; // ~80 ms
    let mut fade_done: usize = if skip_frames == 0 { fade_total } else { 0 };
    let mut gain_vol = st.lock().unwrap().volume; // volume baked into the audio already handed to the mixer

    // Poll for a command; returns an outcome that ends playback.
    macro_rules! check_cmd {
        ($play:expr) => {
            match rx.try_recv() {
                Ok(c) => {
                    $play.stop();
                    return Outcome::Cmd(c);
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    $play.stop();
                    return Outcome::Cmd(Cmd::Stop);
                }
            }
        };
    }

    // A volume change restarts playback from the current second with the new gain (the mixer holds finished files).
    macro_rules! check_volume {
        ($play:expr) => {
            if let Some(t) = t0 {
                if (st.lock().unwrap().volume - gain_vol).abs() >= 5 {
                    $play.stop();
                    return Outcome::Cmd(Cmd::Seek(start + played_secs(&t)));
                }
            }
        };
    }

    loop {
        check_cmd!(play);
        check_volume!(play);
        play.reap();
        ka.ensure(true);
        if last_bt_check.elapsed() > Duration::from_secs(20) {
            last_bt_check = Instant::now();
            if !crate::sys::audio_output_is_bt() {
                play.stop();
                let _ = evtx.send(Ev::Music(MEv::NoOutput));
                return Outcome::Cmd(Cmd::Pause);
            }
        }
        if let Some(t0) = t0 {
            if last_report.elapsed() > Duration::from_secs(1) {
                last_report = Instant::now();
                st.lock().unwrap().pos = start + played_secs(&t0);
                let _ = evtx.send(Ev::Music(MEv::Progress));
            }
        }

        // 1. decode until a segment is full
        let first_secs = if dur > 0 && dur <= WHOLE_TRACK_MAX { (dur as f64 - start as f64).max(1.0) + 5.0 } else { SEG_SECS };
        let target = ((if k == 0 { first_secs } else { SEG_SECS }) * bytes_per_sec as f64) as usize;
        while seg.len() < target && !eof {
            let packet = match format.next_packet() {
                Ok(p) => p,
                Err(_) => {
                    eof = true;
                    break;
                }
            };
            if packet.track_id() != track.id {
                continue;
            }
            let decoded = match decoder.decode(&packet) {
                Ok(d) => d,
                Err(SymErr::DecodeError(_)) => continue,
                Err(_) => {
                    eof = true;
                    break;
                }
            };
            let spec = *decoded.spec();
            if sbuf.is_none() {
                sbuf = Some(SampleBuffer::<i16>::new(decoded.capacity() as u64, spec));
            }
            let sb = sbuf.as_mut().unwrap();
            sb.copy_interleaved_ref(decoded);
            let samples = sb.samples();
            let sc = spec.channels.count().max(1);
            pcm.clear();
            if sc == out_ch {
                pcm.extend_from_slice(samples);
            } else if out_ch == 2 && sc == 1 {
                for s in samples {
                    pcm.push(*s);
                    pcm.push(*s);
                }
            } else {
                for f in samples.chunks(sc) {
                    pcm.push(f[0]);
                    if out_ch == 2 {
                        pcm.push(f[1.min(f.len() - 1)]);
                    }
                }
            }
            // trim the decoder's warm-up frames, then ramp the volume up over ~80 ms
            if skip_frames > 0 {
                let frames = pcm.len() / out_ch;
                let drop = skip_frames.min(frames);
                pcm.drain(..drop * out_ch);
                skip_frames -= drop;
            }
            if fade_done < fade_total && !pcm.is_empty() {
                let frames = pcm.len() / out_ch;
                for f in 0..frames {
                    if fade_done >= fade_total {
                        break;
                    }
                    let m = fade_done as f32 / fade_total as f32;
                    for c in 0..out_ch {
                        pcm[f * out_ch + c] = (pcm[f * out_ch + c] as f32 * m) as i16;
                    }
                    fade_done += 1;
                }
            }
            gain_vol = st.lock().unwrap().volume;
            let vol = gain_vol.clamp(0, 100) as f32 / 100.0;
            let g = vol * vol;
            let mut push = |s: i16| seg.extend_from_slice(&((s as f32 * g) as i16).to_le_bytes());
            if let Some(r) = rs.as_mut() {
                let mut o = Vec::with_capacity(pcm.len() * 2);
                r.process(&pcm, &mut o);
                o.into_iter().for_each(&mut push);
            } else {
                pcm.iter().copied().for_each(&mut push);
            }
            check_cmd!(play);
        }
        if seg.is_empty() {
            break; // nothing left to schedule
        }

        // 2. write the segment file (later segments get SEG_PAD of leading silence)
        if eof {
            // fade the last ~40 ms to zero so the file does not end on a click
            let n = (bytes_per_sec / 25) / (out_ch * 2); // frames in 40 ms
            let total = seg.len() / (out_ch * 2);
            let n = n.min(total);
            for f in 0..n {
                let m = (n - 1 - f) as f32 / n as f32;
                for c in 0..out_ch {
                    let o = ((total - n + f) * out_ch + c) * 2;
                    let v = i16::from_le_bytes([seg[o], seg[o + 1]]);
                    seg[o..o + 2].copy_from_slice(&((v as f32 * m) as i16).to_le_bytes());
                }
            }
        }
        let frames_bytes = seg.len();
        let path = format!("/tmp/kos_seg_{}_{}.raw", id, k);
        let mut data: Vec<u8> = Vec::with_capacity(frames_bytes + (LEAD * bytes_per_sec as f64) as usize);
        let lead = if k == 0 { LEAD } else { SEG_PAD };
        data.resize((lead * bytes_per_sec as f64) as usize / (out_ch * 2) * (out_ch * 2), 0);
        data.extend_from_slice(&seg);
        data.resize(data.len() + (TAIL * bytes_per_sec as f64) as usize / (out_ch * 2) * (out_ch * 2), 0); // trailing silence
        if std::fs::write(&path, &data).is_err() {
            play.stop();
            return Outcome::Failed("can't write audio buffer".into());
        }
        let seg_secs = frames_bytes as f64 / bytes_per_sec as f64;
        seg.clear();

        // 3. launch it at the right moment
        match t0 {
            None => {
                let Some(c) = spawn_mplay(out_rate, out_ch as u32, &path) else {
                    return Outcome::Failed("mixer unavailable".into());
                };
                play.kids.push((c, path));
                t0 = Some(Instant::now());
            }
            Some(t) => {
                let due = Duration::from_secs_f64((cum - SEG_PAD).max(0.0));
                while t.elapsed() < due {
                    check_cmd!(play);
                    check_volume!(play);
                    ka.ensure(true);
                    std::thread::sleep(Duration::from_millis(40));
                    if last_report.elapsed() > Duration::from_secs(1) {
                        last_report = Instant::now();
                        st.lock().unwrap().pos = start + played_secs(&t);
                        let _ = evtx.send(Ev::Music(MEv::Progress));
                    }
                }
                let Some(c) = spawn_mplay(out_rate, out_ch as u32, &path) else {
                    play.stop();
                    return Outcome::Failed("mixer unavailable".into());
                };
                play.kids.push((c, path));
            }
        }
        cum += seg_secs + if k == 0 { LEAD } else { 0.0 };
        k += 1;
        if eof {
            break;
        }
    }

    // wait for the last segment to finish playing
    while !play.kids.is_empty() {
        check_cmd!(play);
        play.reap();
        ka.ensure(true);
        if let Some(t) = t0 {
            st.lock().unwrap().pos = start + played_secs(&t);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Outcome::Ended
}

pub fn start(evtx: Sender<Ev>, volume: i32) -> Player {
    let (tx, rx) = channel::<Cmd>();
    let st = Arc::new(Mutex::new(PState { volume, ..Default::default() }));
    let st2 = st.clone();
    std::thread::spawn(move || {
        let mut pending: Option<Cmd> = None;
        let mut resume_at: u32 = 0;
        let mut ka = Keep::new();
        loop {
            let cmd = match pending.take() {
                Some(c) => c,
                None => loop {
                    match rx.recv_timeout(Duration::from_millis(500)) {
                        Ok(c) => break c,
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => ka.tick_idle(),
                        Err(_) => return,
                    }
                },
            };
            let (idx, start) = match cmd {
                Cmd::Play(i) => (i, 0),
                Cmd::PlayAt(i, s) => (i, s),
                Cmd::Resume => match st2.lock().unwrap().idx {
                    Some(i) => (i, resume_at),
                    None => continue,
                },
                Cmd::Next => {
                    let s = st2.lock().unwrap();
                    match s.idx.and_then(|i| pick(&s, i, false)) {
                        Some(i) => (i, 0),
                        None => continue,
                    }
                }
                Cmd::Prev => {
                    let s = st2.lock().unwrap();
                    match s.idx {
                        Some(i) if s.pos > 5 => (i, 0),
                        Some(i) if i > 0 => (i - 1, 0),
                        Some(i) => (i, 0),
                        _ => continue,
                    }
                }
                Cmd::Seek(sec) => match st2.lock().unwrap().idx {
                    Some(i) => (i, sec),
                    None => continue,
                },
                Cmd::Stop | Cmd::Pause => {
                    st2.lock().unwrap().playing = false;
                    continue;
                }
            };
            if !crate::sys::audio_output_is_bt() {
                st2.lock().unwrap().playing = false;
                let _ = evtx.send(Ev::Music(MEv::NoOutput));
                continue;
            }
            let track = {
                let mut s = st2.lock().unwrap();
                let Some(t) = s.queue.get(idx).cloned() else { continue };
                s.idx = Some(idx);
                s.playing = true;
                s.pos = start;
                t
            };
            let _ = evtx.send(Ev::Music(MEv::Track(idx)));
            // open the Bluetooth stream with silence first, so the song does not pop in
            ka.ensure(true);
            let mut interrupted: Option<Cmd> = None;
            while !ka.warm() {
                match rx.try_recv() {
                    Ok(c) => {
                        interrupted = Some(c);
                        break;
                    }
                    Err(_) => std::thread::sleep(Duration::from_millis(60)),
                }
                ka.ensure(true);
            }
            if let Some(c) = interrupted {
                pending = Some(c);
                continue;
            }
            match play_track(&track, start, &rx, &st2, &evtx, &mut ka) {
                Outcome::Ended => {
                    let next = pick(&st2.lock().unwrap(), idx, true);
                    if let Some(n) = next {
                        pending = Some(Cmd::Play(n));
                    } else {
                        let mut s = st2.lock().unwrap();
                        s.playing = false;
                        s.pos = 0;
                        let _ = evtx.send(Ev::Music(MEv::Ended));
                    }
                }
                Outcome::Cmd(c) => {
                    match c {
                        Cmd::Pause => {
                            let mut s = st2.lock().unwrap();
                            s.playing = false;
                            resume_at = s.pos;
                        }
                        Cmd::Stop => {
                            let mut s = st2.lock().unwrap();
                            s.playing = false;
                            s.pos = 0;
                            resume_at = 0;
                        }
                        other => pending = Some(other),
                    }
                    let _ = evtx.send(Ev::Music(MEv::Progress));
                }
                Outcome::Failed(m) => {
                    st2.lock().unwrap().playing = false;
                    let _ = evtx.send(Ev::Music(MEv::Failed(m)));
                }
            }
        }
    });
    Player { tx, st }
}
