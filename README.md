# Folio

A replacement interface for jailbroken Kindle e-readers. Folio keeps the Kindle's kernel and low-level services
(display, Bluetooth, power, USB) and replaces Amazon's Java UI with a small native app that draws straight to
the e-ink panel: fast page turns, a typeset reader, Bluetooth music, handwritten and typed notes, and no store.

> **Status: experimental, tested on exactly one device** — Kindle Paperwhite 4 (10th gen, PQ94WIF), firmware
> 5.17.1.0.3, jailbroken with WinterBreak. Other models are designed for but untested; see
> [Supported devices](#supported-devices). Not affiliated with or endorsed by Amazon. "Kindle" is a trademark of
> Amazon.com, Inc., used here only to name the hardware.

**Contents:** [Features](#features) · [Supported devices](#supported-devices) · [Install](#install) ·
[Using Folio](#using-folio) · [Building](#building) · [Developing](#developing) · [Known limits](#known-limits) ·
[Risks](#risks) · [Contributing](#contributing) · [License](#license)

## Features

| Area | What you get |
|---|---|
| Library | Covers, progress, search, sort. EPUB, PDF, CBZ, FB2, XPS, HTML, TXT natively; MOBI/AZW/AZW3 via libmobi and DjVu via djvulibre (converted once, cached). DRM-locked files can't be opened. |
| Reader | IBM Plex typesetting; size, side/top/bottom margins, line spacing, justify; contents, in-book search, bookmarks, highlights with notes; tap-and-hold a word for its definition (WordNet 3.0, Webster's 1913 as fallback); page scrubber; time left in chapter/book; dark reading. |
| Learning | Every looked-up word is saved with its sentence and reviewable as flashcards; reading stats and streaks. |
| Music | MP3, FLAC, OGG, WAV, AAC/M4A, ALAC to Bluetooth headphones through Amazon's audio mixer, in the background while you read. Shuffle, repeat, sleep timer, audiobook resume (recordings over 20 min). |
| Quick panel | Pull down from the top edge on any screen: draggable brightness, volume, text size, top and bottom margin; music controls; sleep, timer, settings, Bluetooth. |
| Notes | Handwritten notebooks (pen/eraser/undo, multi-page), typed notes, highlight browser, export to PNG/TXT/Markdown. |
| System | Lock screen with mini player (or book cover / random highlight), Bluetooth pairing, touch calibration, USB drive mode, drive-based self-update, crash-safe boot supervisor, first-run help and confirmations before anything disruptive. |

## Supported devices

Folio is meant to run on any touchscreen Kindle that can be jailbroken, but **only the Paperwhite 4 has been
tried on real hardware.** Everything else is expected to work, not known to work. If you try one, please open an
issue with what happened.

| Model | Panel | Expected | Notes |
|---|---|---|---|
| Paperwhite 4 (10th gen) | 1072×1448 | **works** | author's device, firmware 5.17.1.0.3 |
| Paperwhite 3, Voyage, Kindle 11th gen | 1072×1448 | likely | same panel size; Bluetooth audio needs a model that has Bluetooth |
| Paperwhite 5 / Signature | 1236×1648 | scaled up | UI is upscaled (slightly soft); text is physically larger |
| Oasis 1/2/3 | 1264×1680 | scaled up | as above; the physical page-turn buttons are not used |
| Scribe | 1860×2480 | scaled up | very soft UI; the pen is not used |
| Paperwhite 1/2 | 758×1024 | scaled down | older kernel (3.0.x); see below |
| Kindle 7/8/10 (basic) | 600×800 | scaled down | no Bluetooth on most; the 10th gen has a light |
| Kindle Touch (4th gen) | 600×800 | scaled down | very old firmware and kernel; likely needs work |
| Kindles without a touchscreen | — | **no** | Folio needs touch |

### How one build covers every panel size

Folio always draws on a fixed 1072×1448 canvas. On each screen update `app/src/display.rs` scales the changed
region to the real panel (bilinear up, averaging down) before handing it to FBInk, and touches are mapped back
through the touch calibration, so the five-circle setup also absorbs any difference in touch panel size or
orientation. On a 1072×1448 panel nothing is scaled and the output is identical to a build without this.

* The panel size is read with `fbink -e` at start-up (`viewWidth` / `viewHeight`). A panel within 16 px of
  1072×1448 counts as 1072×1448; if the size can't be read, Folio assumes 1072×1448.
* The touch panel is found by looking for an input device that reports multitouch axes (or plain X/Y axes),
  trying `event2` first; the power key is listened for on every other input device.

### Known unknowns for other models

* **Old kernels.** Folio is a Rust program and Rust's standard library officially needs Linux ≥ 3.2. Kindles
  before the Paperwhite 3 run older kernels; it may still work, or it may not start.
* **Music** needs Bluetooth audio, which many older Kindles lack. Without it the Music tab is only a library.
* **Frontlight** controls do nothing on models without a light.
* **Jailbreak and install steps differ by model and firmware.** The install section below describes what worked on the Paperwhite 4.
* **Large panels look soft.** Book pages are rendered at 1072 px wide and then scaled; rendering natively at the
  panel's resolution would make text crisp on the Scribe / Paperwhite 5 / Oasis. Not done yet.

## Install

Written from what was done on one device. Treat every step as untested on anything else, and keep a second way
to read: you can brick a Kindle.

**0. Before you start.** You need a jailbreakable firmware (check <https://kindlemodding.org> for your model and
firmware; do *not* update the firmware afterwards), a Mac/Linux computer with Docker (to build) and `ssh`, and a
backup of the Kindle's contents.

**1. Jailbreak and tools.** Follow the community guide for your firmware (the author used WinterBreak), then
install KUAL, MRPI and USBNetwork (the "lite" variant is fine). Enable USB networking and confirm you can
`ssh root@192.168.15.244`. The password the author's setup uses is in `build/askpass.sh`; change it if you care.
Folio only uses USB networking as a developer link; Wi-Fi stays off.

**2. Get the binaries.** Take `kos` (and `kreader`, `fbink`, `mobitool`, `ddjvu`) from a GitHub Release, or build
them ([Building](#building)). Put them in `build/dist/`; the fonts are already in `build/fonts/`.

**3. Push Folio to the Kindle.**

```sh
build/deploy.sh          # copies everything to /var/local/kos on the Kindle's system partition
```

Folio lives in `/var/local/kos`, not on the USB drive: the drive disappears while a computer is connected.

**4. Make Folio the interface (reversible).**

```sh
build/install-boot.sh    # renames Amazon's UI start-up jobs to *.amazon and installs /etc/upstart/kos.conf
```

Restart the Kindle. Folio starts with a five-circle touch setup, then a short how-to card. If it crashes three
times in a row the boot supervisor shows a recovery message and brings USB networking up so you can log in.
To go back to the stock UI: `build/restore-stock-ui.sh`.

**5. Optional: remove Amazon's software (NOT reversible).** `build/remove-amazon.sh` deletes Amazon's UI,
services and data and installs Folio's branding. Afterwards the stock UI cannot be restored without a firmware
reinstall. It refuses to run unless you set `FOLIO_I_UNDERSTAND_THIS_IS_PERMANENT=yes`. Back up first, for example
`ssh root@192.168.15.244 'tar cf - /opt/amazon /usr/java /etc/upstart' > amazon-backup.tar`, and keep that file
private: it is Amazon's software and must never be committed or shared.

Known trap: deleting `/var/local/kindleforeink_weblabs.json` or other Amazon dynamic-config data can stop the X
server from starting (`FormatEPDC` empty). The script no longer deletes that file; if you hit it, make
`/etc/upstart/makexconfig` fall back to `epdcFormat=Y8`.

**Troubleshooting.** Blank screen with the cable plugged in at boot: boot with the cable out and plug in after
Home shows. `/var/log/messages` and `/var/log/Xorg.0.log` are the first places to look. Music clicks or freezes:
see the notes at the top of `app/src/music.rs` (the mixer only plays complete files, so tracks are decoded into
segment files).

## Using Folio

* **First time:** a how-to card appears after touch setup; it is always under *Settings → More → How to use Folio*.
* **Reading:** tap the right edge for the next page and the left edge for the previous one; tap the middle for the
  menu (contents, search, text, notes, bookmark, light); hold a word for its meaning.
* **Anywhere:** pull down from the very top edge for the quick panel; the power button sleeps and wakes.
* **Books and music:** plug the Kindle into a computer. It shows "USB drive mode" and a drive named FOLIO. Copy
  files into `books/` and `music/`, eject, unplug; the library rescans by itself. Music plays to Bluetooth
  headphones: connect them from the Music tab.
* **Power:** *Settings → More → Restart device / Power off* (each asks first). Hold the power button ~40 s to force a reboot.
* **Developer access:** *Settings → More → USB network (developer)*, restart with the cable out, plug in after Home
  shows. SSH `root@192.168.15.244` over USB only (Wi-Fi stays off). While it is on, the FOLIO drive does not appear.
* **Update without a shell:** drop `kos`, `kreader`, `boot.sh` (and `dict.tsv`, `dict_old.tsv`) into `kos-update/`
  on the drive; they are installed at the next app start.
* **Dictionary:** `data/dict.tsv` is WordNet 3.0 and `dict_old.tsv` is Webster's 1913 for archaic words (optional);
  both are `word<TAB>definition`, sorted by lowercase word.

## Building

Everything is cross-compiled for ARMv7 (hard-float, musl, static) inside Docker image
`messense/rust-musl-cross:armv7-musleabihf`. Nothing is built on the Kindle.

**The app (`kos`, Rust):**

```sh
build/build-app.sh      # -> build/dist/kos   (this is also what CI runs)
```

**Native helpers (not built by CI).** `kreader` (C, `native/kreader.c`) links a static MuPDF; `mobitool`
(libmobi) and `ddjvu` (djvulibre) convert MOBI/AZW and DjVu; `fbink` draws to the e-ink panel. Notes from the
original build, reconstructed from memory of what worked on the author's machine (expect to adjust):

* MuPDF 1.24.11: built static in the same container with the TOFU font flags set (`-DTOFU`, `-DTOFU_CJK`, … to drop
  large font data), `HAVE_OBJCOPY=no`, no X11/GL; IBM Plex fonts were substituted for the built-in Nimbus fonts.
  Then `build/build-native.sh` links `native/kreader.c` against `libmupdf.a` and `libmupdf-third.a`.
* libmobi: autotools/CMake, static, only `mobitool` is used.
* djvulibre 3.5.28: static `ddjvu` only.
* FBInk: use the release binary for Kindle (<https://github.com/NiLuJe/FBInk>).

If you get these building cleanly, please contribute a `build/fetch-deps.sh`.

**Dictionary:**

```sh
curl -L -o wn.tgz https://wordnetcode.princeton.edu/3.0/WordNet-3.0.tar.gz && tar xzf wn.tgz
python3 build/dict/wordnet.py WordNet-3.0/dict dict.tsv
```

Put `dict.tsv` (and optionally `dict_old.tsv`) under `/mnt/us/kos-update/` on the drive, or in `/var/local/kos/data/`.

**Releases.** Pushing a tag `vX.Y.Z` makes CI build `kos` and attach it to a GitHub Release.

## Developing

```sh
build/build-app.sh && build/deploy.sh && build/rerun.sh   # build, push to the Kindle, restart (needs developer USB networking)
python3 build/shot.py out.png                              # screenshot the e-ink framebuffer
build/t.sh "tap 540 700" "swipe 800 700 200 700"           # inject touches (test channel /tmp/kos.cmd)
```

**Simulator.** A companion project, *folio-sim* (a separate repository, not published yet), runs the real app on
your computer against fake Kindle system tools and shows the e-ink screen in a browser, at any model's
resolution. Folio supports it through two environment variables: `KOS_SIM=host:port` makes the app send its
screen updates to the simulator and take touches from it (protocol described in `app/src/sim.rs`), and
`KOS_ROOT=/some/dir` redirects the device paths (`/mnt/us`, `/var/local/kos`). The simulator exercises the
scaling and touch mapping, not model-specific hardware, kernels or firmware.

**Layout.**

```
app/src/        the Rust app (main.rs events, gfx.rs canvas + drawing, display.rs panel scaling, input.rs touch,
                reader.rs, screens.rs, quick.rs, music.rs, notes.rs, extras.rs, dict.rs, sim.rs, ...)
native/         kreader.c: MuPDF line protocol used by the reader
build/          build, deploy, install and removal scripts, boot supervisor, branding images, fonts
docs/DESIGN.md  the original design notes (kept for rationale; the implementation differs in places)
```

## Known limits

* No Kindle Store, Amazon sync or DRM, by design.
* The Kindle has no speaker or headphone jack: music needs Bluetooth headphones.
* Highlights are matched by their words, so they survive re-layout, but only when the same words land on the page.
* Not built yet: footnote pop-ups, playlists, audiobook chapter lists, handwriting-to-text, Wi-Fi sync, natively
  rendered pages on high-resolution panels.

## Risks

Jailbreaking and modifying a device can brick it and voids the warranty. This project is provided as-is, without
warranty. Keep a backup of anything you remove, and don't run `remove-amazon.sh` on a device you can't afford to lose.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). The most valuable contributions are reports from other models and
firmware, reproducible native builds (`build/fetch-deps.sh`), and fixes for anything under *Known limits*.
Changes are listed in [CHANGELOG.md](CHANGELOG.md).

## License

[AGPL-3.0](LICENSE) (Folio links MuPDF, which is AGPL). Third-party components and data are listed in
[THIRD_PARTY.md](THIRD_PARTY.md).
