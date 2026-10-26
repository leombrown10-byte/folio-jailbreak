# Changelog

## Unreleased

* Easier setup: `./install.sh` (checks the Kindle, fetches or builds what's missing, installs, can restore the stock UI), `build/fetch-deps.sh` (reproducible builds of kreader/mobitool/ddjvu; kreader and ddjvu verified byte-identical to the originals), releases now carry all helpers, README install section rewritten with troubleshooting.
* Runs on any panel size: the 1072×1448 canvas is scaled to the real panel; panel size, touch device and power key are detected (see Supported devices in the README). Unscaled and unchanged on 1072×1448 models.
* Simulator hook: with `KOS_SIM=host:port` the app draws to and takes touches from [folio-sim](https://github.com/leombrown10-byte/folio-sim) (not yet published); `KOS_ROOT` redirects device paths.
* Friendlier: first-run how-to card (also under Settings > More), reader hint for new readers, confirmation before restart/power off/recalibrate/developer USB, a "Connect headphones" button on the Music tab.
* Draggable quick-panel sliders; top and bottom margin sliders; music sleep timer.
* Shuffle, repeat, audiobook resume with ±30 s skip.
* Word list with flashcards, reading stats, time left in chapter/book, highlight export (Markdown).
* Sleep screen choices, dark reading, full-refresh frequency, focus timer.
* WordNet 3.0 dictionary with Webster's 1913 fallback; tap-and-hold a word to define it.

## 0.1.0

First working release on Kindle Paperwhite 4 (5.17.1.0.3): reader, Bluetooth music, notes, USB drive mode,
lock screen player, quick panel, boot supervisor.
