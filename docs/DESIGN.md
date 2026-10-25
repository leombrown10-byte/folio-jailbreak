# Kindle OS: Design

Status: original design notes from before the build. The implemented system differs in places (see README); kept for rationale.
Legend: **[You]** = stated by the owner. **[Call]** = my decision, open to change. **[Open]** = blocked on information we don't have yet.

## Intent

Replace the stock software on an older Kindle e-reader with a custom, offline system that reads books, plays music, and takes notes. It should look nicer and run much less laggily than the stock software. [You]

## Goals

- Read all common ebook formats. [You]
  - Libraries, not hand-written parsers. [Call]
  - MuPDF: EPUB, PDF, CBZ, FB2, XPS.
  - libmobi: MOBI, AZW, AZW3.
  - DjVu via djvulibre; TXT, HTML, Markdown directly.
  - "All formats" means every format those libraries cover, not literally everything. [Call]
- Play music, including FLAC and MP3. [You] Decoders: dr_flac and minimp3 (small, permissively licensed). [Call]
- Play music in the background while reading. [You] Implemented as a tiny separate audio service. [Call]
- Take notes on the device. [You] Input method depends on the model (see Open items).
- Keep the stock offline features, with a nicer look. [You]
  - Library and collections, highlights, search, font and layout settings.
  - Dictionary, bookmarks, reading position. [Call: guessed from "keep all features"]
- Significantly less lag than stock. [You]
  - A small native program (C or Rust) drawing directly to the framebuffer, instead of Amazon's Java UI. [Call]
  - Render only the visible page and load books lazily. [Call]
  - Partial e-ink refreshes with a full flash every few pages. [Call]
- "Eye candy": polished typography, a clean home screen and library, tasteful transitions within e-ink limits. [You, interpreted by me]

## Non-goals

- Kindle Store, buying books, and Amazon cloud sync / Whispersync. [You]
- Opening DRM-locked files (Amazon purchases). [Call] Users load their own DRM-free files.
- Wiping Amazon's kernel and bootloader to install a different Linux distribution. [Call] Too much brick risk for little visible gain.

## Anti-goals (what would make this a failure even if it technically runs)

- A bricked device. Every step must be recoverable, and the install must be reversible. [Call]
- Slower or laggier than stock.
- Music and reading interfering with each other (stutter, crashes under memory pressure). [Call]

## Approach

1. Jailbreak the Kindle. Older models have well-documented methods; which one depends on the firmware. [Call]
2. Keep Amazon's kernel and bootloader. Stop Amazon's UI from starting and launch our own program in its place. [Call]
3. Build the system on top, as separate small pieces:
   - Launcher / library (home screen)
   - Reader (uses the format libraries above)
   - Music service plus player UI
   - Notes app
   - Shared e-ink drawing layer (refresh strategy, fonts, theme)
4. Transfer books, music, and notes over USB mass storage or SSH/Wi-Fi, depending on the model. [Call]

## Constraints

- Low CPU and little RAM on an older Kindle. Big PDFs and comic archives need careful, page-at-a-time loading.
- E-ink panels have a physical refresh limit. A page turn will never be instant, only much better than stock.
- Fully offline, no accounts, no cloud. [You, from dropping the Store]

## Open items (need the Kindle model)

The owner doesn't know the exact model yet, only that it is an older one. Find it via Settings, Device Options, Device Info, or the back of the device, or the first four characters of the serial number. These decisions are blocked until then:

- Which jailbreak method and firmware version applies.
- Screen resolution and framebuffer details (affects the drawing layer).
- Input: touchscreen, keyboard, or buttons only. This decides how notes work (typed, handwritten, or limited).
- Audio hardware: headphone jack, speaker, or Bluetooth. If there is none, music playback is not possible on this device. This is the largest risk to the music goals.
- Available RAM and storage, which sets limits for large files.

## Left to the builder

Language choice (C vs Rust), exact UI design, file layout, the dictionary source, and the notes data format are the builder's choice within the goals above.

## Next steps

1. Owner identifies the model.
2. Resolve the Open items and revise this document.
3. Owner approves the design.
4. Write an implementation plan (first milestone: jailbreak the device and boot a hello-world screen).

## Hardware findings (2026-10-24, from the actual device)

- Device: Kindle Paperwhite 4 (10th gen), firmware 5.17.1.0.3, already jailbroken with WinterBreak. KUAL, MRPI and USBNetwork-lite are installed. SSH is reachable as root over USB (RNDIS, 192.168.15.244).
- SoC: Freescale i.MX6 SoloLite (ARMv7, Cortex-A9), about 500 MB RAM, kernel 4.1.15-lab126. Root filesystem is nearly full (37 MB free), so our code goes on the user partition.
- Display: `/dev/fb0` via `mxc_epdc_v2_fb`. Touch: `gt9xx_ts`.
- Audio: no ALSA sound card. There is no wired audio and no speaker. Sound can only go out over Bluetooth.
- Bluetooth: the kernel has BR/EDR, RFCOMM, BNEP, HID and UART HCI drivers, and Amazon's `btfd` and `btpowercontrol` daemons are running. There is no BlueZ and no A2DP userspace on the device. Open question: whether we can drive the Bluetooth chip and stream A2DP audio ourselves, or must reuse Amazon's daemon. This is the first spike.

## Implementation status (2026-10-24)

Built and verified on the device (screenshots of the real e-ink framebuffer): home, library with covers, reader (EPUB, MOBI via
conversion, PDF), contents, search, text settings, bookmarks, highlights, dictionary, typed and handwritten notes, on-screen keyboard,
settings, KUAL launcher entries. See README.md.

Written but needing hands-on confirmation: touch calibration, Bluetooth pairing screen, music over Bluetooth, sleep/wake, boot-to-OS.
Not done: DjVu is built (ddjvu) but untested for lack of a sample file; CBZ/FB2/XPS go through MuPDF but were not individually tested.
