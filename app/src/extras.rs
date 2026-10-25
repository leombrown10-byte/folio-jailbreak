//! Extras: vocabulary word list + flashcards, reading stats and time-left, highlight export, focus timer,
//! sleep-screen content.
use crate::gfx::*;
use crate::ui::*;
use crate::{App, Modal};
use std::time::{Duration, Instant};

/// "5 min" / "2 h 05 m"
pub fn fmt_left(secs: f32) -> String {
    let m = (secs / 60.0).ceil() as u32;
    if m < 60 { format!("{} min", m.max(1)) } else { format!("{} h {:02} m", m / 60, m % 60) }
}

impl App {
    // ---------------------------------------------------------------- help and confirmations
    /// Ask before anything disruptive: a card with the action button and Close.
    pub fn confirm(&mut self, title: &str, body: &str, label: &str, act: Act) {
        let c = Card::new(&mut self.cv, title, body, vec![(label, act)]);
        self.modal = Some(Modal::Card(c));
        self.draw_modal();
    }

    pub fn show_help(&mut self) {
        let body = "Reading\n• Tap the right edge for the next page, the left edge for the previous one.\n• Tap the middle for the menu: contents, search, text, notes, bookmark, light.\n• Hold a word to see what it means. It is saved to your word list.\n\nAnywhere\n• Pull down from the very top edge for brightness, volume, text size, margins, music and sleep.\n• The power button sleeps and wakes.\n\nBooks and music\n• Plug the Kindle into a computer and copy files into the books and music folders.\n• Music plays to Bluetooth headphones: connect them from the Music tab.\n\nThis help is always in Settings > More > How to use Folio.";
        let c = Card::new(&mut self.cv, "Welcome to Folio", body, vec![]);
        self.modal = Some(Modal::Card(c));
        self.draw_modal();
    }

    // ---------------------------------------------------------------- vocabulary
    fn vocab_path(&self) -> std::path::PathBuf {
        self.st.dir.join("vocab.tsv")
    }

    pub fn vocab_load(&self) -> Vec<(String, String, String)> {
        let s = std::fs::read_to_string(self.vocab_path()).unwrap_or_default();
        s.lines()
            .filter_map(|l| {
                let mut p = l.splitn(3, '\t');
                Some((p.next()?.to_string(), p.next()?.to_string(), p.next().unwrap_or("").to_string()))
            })
            .collect()
    }

    fn vocab_save(&self, v: &[(String, String, String)]) {
        let s: String = v.iter().map(|(a, b, c)| format!("{}\t{}\t{}\n", a, b, c)).collect();
        let _ = std::fs::write(self.vocab_path(), s);
    }

    /// Remember a looked-up word with the sentence it appeared in.
    pub fn vocab_add(&mut self, word: &str, sentence: &str, book: &str) {
        let clean = |s: &str| s.replace(['\t', '\n'], " ");
        let w = word.to_lowercase();
        let mut v = self.vocab_load();
        if w.is_empty() || v.iter().any(|e| e.0 == w) {
            return;
        }
        v.push((w, clean(sentence), clean(book)));
        self.vocab_save(&v);
    }

    /// Flashcards over every word looked up so far.
    pub fn open_words(&mut self) {
        self.fc = self.vocab_load();
        if self.fc.is_empty() {
            let c = Card::new(&mut self.cv, "Word list", "Words you look up (tap and hold a word while reading) are saved here, with the sentence they came from, so you can review them.", vec![]);
            self.modal = Some(Modal::Card(c));
            return self.draw_modal();
        }
        self.fc_i = (crate::sys::now() as usize) % self.fc.len();
        self.fc_shown = false;
        self.word_card();
    }

    pub fn word_card(&mut self) {
        let Some((w, sentence, book)) = self.fc.get(self.fc_i).cloned() else { return };
        let title = format!("{}  ({}/{})", w, self.fc_i + 1, self.fc.len());
        let (body, actions) = if self.fc_shown {
            let def = crate::dict::lookup(&self.st.dir, &w).unwrap_or_else(|| "No definition found.".into());
            (format!("{}\n\n“{}”\n— {}", def, sentence, book), vec![("Next", Act::WordNext), ("Remove", Act::WordDrop)])
        } else {
            (format!("“{}”\n— {}\n\nDo you remember what it means?", sentence.replacen(&w, "_____", 1), book), vec![("Show", Act::WordShow), ("Next", Act::WordNext)])
        };
        let c = Card::new(&mut self.cv, &title, &body, actions);
        self.modal = Some(Modal::Card(c));
        self.draw_modal();
    }

    pub fn word_act(&mut self, a: Act) {
        match a {
            Act::WordShow => self.fc_shown = true,
            Act::WordNext => {
                self.fc_i = (self.fc_i + 1) % self.fc.len().max(1);
                self.fc_shown = false;
            }
            _ => {
                if self.fc_i < self.fc.len() {
                    self.fc.remove(self.fc_i);
                    let v = self.fc.clone();
                    self.vocab_save(&v);
                }
                if self.fc.is_empty() {
                    self.modal = None;
                    return self.draw_screen(false);
                }
                self.fc_i %= self.fc.len();
                self.fc_shown = false;
            }
        }
        self.word_card();
    }

    // ---------------------------------------------------------------- stats
    pub fn stats_card(&mut self) {
        let today = crate::sys::now() / 86400;
        let get = |d: u64| self.st.d.stats.get(&d).copied().unwrap_or([0, 0]);
        let fmt = |e: [u32; 2]| format!("{} pages · {}", e[0], fmt_left(e[1] as f32));
        let week = (0..7).fold([0u32, 0], |a, i| {
            let e = get(today - i);
            [a[0] + e[0], a[1] + e[1]]
        });
        let all = self.st.d.stats.values().fold([0u32, 0], |a, e| [a[0] + e[0], a[1] + e[1]]);
        let mut streak = 0;
        let mut d = if get(today)[0] > 0 { today } else { today.saturating_sub(1) };
        while get(d)[0] > 0 {
            streak += 1;
            d -= 1;
        }
        let finished = self.st.d.books.values().filter(|b| b.frac >= 0.97).count();
        let speed = self.st.secs_per_page().map(|s| format!("{:.0} s per page", s)).unwrap_or_else(|| "not enough data yet".into());
        let body = format!(
            "Today: {}\nThis week: {}\nAll time: {}\n\nStreak: {} {}\nBooks finished: {}\nReading speed: {}",
            fmt(get(today)),
            fmt(week),
            fmt(all),
            streak,
            if streak == 1 { "day" } else { "days" },
            finished,
            speed
        );
        let c = Card::new(&mut self.cv, "Reading stats", &body, vec![]);
        self.modal = Some(Modal::Card(c));
        self.draw_modal();
    }

    // ---------------------------------------------------------------- export
    /// One Markdown file per book (highlights and notes) in /mnt/us/exports.
    pub fn export_highlights(&mut self) {
        let dir = std::path::PathBuf::from(crate::paths::p("/mnt/us/exports"));
        let dir = dir.as_path();
        if std::fs::create_dir_all(dir).is_err() {
            return self.toast("Could not write to the drive");
        }
        let mut n = 0;
        for b in self.st.d.books.values() {
            if b.highlights.is_empty() {
                continue;
            }
            let mut md = format!("# {}\n\n", b.title);
            for h in &b.highlights {
                md += &format!("> {}\n\n", h.text);
                if !h.note.is_empty() {
                    md += &format!("*{}*\n\n", h.note);
                }
            }
            let name: String = b.title.chars().map(|c| if c.is_alphanumeric() || c == ' ' { c } else { '_' }).collect();
            if std::fs::write(dir.join(format!("{}.md", name.trim())), md).is_ok() {
                n += 1;
            }
        }
        self.toast(&if n == 0 { "No highlights to export".to_string() } else { format!("Exported {} {} to exports/", n, if n == 1 { "book" } else { "books" }) });
    }

    // ---------------------------------------------------------------- focus timer
    pub fn focus_toggle(&mut self) {
        if self.focus_until.is_some() {
            self.focus_until = None;
            self.toast("Focus timer off");
        } else {
            self.focus_until = Some(Instant::now() + Duration::from_secs(25 * 60));
            self.focus_pages0 = self.pages_today();
            self.toast("Focus: 25 minutes");
        }
    }

    fn pages_today(&self) -> u32 {
        self.st.d.stats.get(&(crate::sys::now() / 86400)).map_or(0, |e| e[0])
    }

    pub fn focus_check(&mut self) {
        let Some(t) = self.focus_until else { return };
        if Instant::now() < t || self.modal.is_some() || self.touch.down || self.screen == crate::Screen::Lock {
            return;
        }
        self.focus_until = None;
        let n = self.pages_today().saturating_sub(self.focus_pages0);
        let body = format!("25 minutes done. You read {} {}. Take a short break.", n, if n == 1 { "page" } else { "pages" });
        let c = Card::new(&mut self.cv, "Focus session", &body, vec![]);
        self.modal = Some(Modal::Card(c));
        self.draw_modal();
    }

    /// Minutes left in the focus session, for the reader footer.
    pub fn focus_left(&self) -> Option<u64> {
        self.focus_until.map(|t| (t.saturating_duration_since(Instant::now()).as_secs() + 59) / 60)
    }

    // ---------------------------------------------------------------- sleep screen
    /// A highlight chosen at random from all books: (text, book title).
    pub fn random_quote(&self) -> Option<(String, String)> {
        let all: Vec<(String, String)> = self.st.d.books.values().flat_map(|b| b.highlights.iter().map(move |h| (h.text.clone(), b.title.clone()))).collect();
        if all.is_empty() {
            None
        } else {
            Some(all[crate::sys::now() as usize % all.len()].clone())
        }
    }
}
