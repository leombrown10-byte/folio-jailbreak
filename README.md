# Folio

A replacement interface for jailbroken Kindle e-readers. Folio keeps the Kindle's kernel and low-level services
(display, Bluetooth, power, USB) and replaces Amazon's Java UI with a small native app that draws straight to
the e-ink panel: fast page turns, a typeset reader, Bluetooth music, handwritten and typed notes, and no store.

> **Status: experimental, tested on exactly one device** — Kindle Paperwhite 4 (10th gen, PQ94WIF), firmware
> 5.17.1.0.3, jailbroken with WinterBreak. Other models and firmware are untested; see
> [Supported devices](#supported-devices). Not affiliated with or endorsed by Amazon. "Kindle" is a trademark of
> Amazon.com, Inc., used here only to name the hardware.

## Features

| Area | What you get |
|---|---|
| Library | Covers, progress, search, sort. EPUB, PDF, CBZ, FB2, XPS, HTML, TXT natively; MOBI/AZW/AZW3 via libmobi and DjVu via djvulibre (converted once, cached). DRM-locked files can't be opened. |
| Reader | IBM Plex typesetting; size, side/top/bottom margins, line spacing, justify; contents, in-book search, bookmarks, highlights with notes; tap-and-hold a word for its definition (WordNet 3.0, Webster's 1913 as fallback); page scrubber; time left in chapter/book; dark reading. |
| Learning | Every looked-up word is saved with its sentence and reviewable as flashcards; reading stats and streaks. |
| Music | MP3, FLAC, OGG, WAV, AAC/M4A, ALAC to Bluetooth headphones through Amazon's audio mixer, in the background while you read. Shuffle, repeat, sleep timer, audiobook resume (recordings over 20 min). |
| Quick panel | Pull down from the top edge on any screen: draggable brightness, volume, text size, top and bottom margin; music controls; sleep, timer, settings, Bluetooth. |
| Notes | Handwritten notebooks (pen/eraser/undo, multi-page), typed notes, highlight browser, export to PNG/TXT/Markdown. |
| System | Lock screen with mini player (or book cover / random highlight), Bluetooth pairing, touch calibration, USB drive mode, drive-based self-update, crash-safe boot supervisor. |

## Supported devices

| Device | Firmware | Status |
|---|---|---|
| Kindle Paperwhite 4 (PW4, 10th gen) | 5.17.1.0.3 | works (author's device) |
| Anything else | — | **untested.** Likely needs changes (screen size, touch device, framebuffer format). Reports and patches welcome. |

## Install (summary)

Read [docs/INSTALL.md](docs/INSTALL.md) first. In short: jailbreak the Kindle (see <https://kindlemodding.org>),
install KUAL and USBNetwork, build or download the Folio binaries ([Releases](../../releases)), push them with
`build/deploy.sh`, then switch the boot job with `build/install-boot.sh`. That step is reversible
(`build/restore-stock-ui.sh`). `build/remove-amazon.sh` is **not** reversible and is optional.

## Use

* **Books and music:** plug in a computer; the Kindle shows "USB drive mode" and a drive named FOLIO. Copy files into `books/` and `music/`, eject, unplug.
* **Power:** the power button sleeps/wakes. *More → Restart device / Power off* are in the app.
* **Developer access:** *More → More → USB network (developer)*, restart with the cable out, plug in after Home shows. SSH `root@192.168.15.244` over USB only (Wi-Fi stays off).
* **Update without a computer shell:** drop `kos`, `kreader`, `boot.sh` (and `dict.tsv`, `dict_old.tsv`) into `kos-update/` on the drive; they are installed at the next app start.

## Develop

```sh
build/build-app.sh                       # cross-compile the app (needs Docker)
build/deploy.sh && build/rerun.sh        # push to the Kindle and restart (needs developer USB networking)
python3 build/shot.py out.png            # screenshot the e-ink framebuffer
build/t.sh "tap 540 700" "swipe 800 700 200 700"   # inject touches
```

See [docs/BUILDING.md](docs/BUILDING.md), [docs/DESIGN.md](docs/DESIGN.md) and [CONTRIBUTING.md](CONTRIBUTING.md).

## Known limits

* No Kindle Store, Amazon sync or DRM support, by design.
* The Kindle has no speaker or headphone jack: music needs Bluetooth headphones.
* Highlights are matched by their words, so they survive re-layout, but only when the same words land on the page.
* Not built yet: footnote pop-ups, playlists, audiobook chapter lists, handwriting-to-text, Wi-Fi sync.

## Risks

Jailbreaking and modifying a device can brick it and voids the warranty. This project is provided as-is, without
warranty. Keep a backup of anything you remove, and don't run `remove-amazon.sh` on a device you can't afford to lose.

## License

[AGPL-3.0](LICENSE) (Folio links MuPDF, which is AGPL). Third-party components and data are listed in
[THIRD_PARTY.md](THIRD_PARTY.md).
