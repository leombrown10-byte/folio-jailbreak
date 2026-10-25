# Devices

Folio is written to run on any touchscreen Kindle that can be jailbroken, but **only the Paperwhite 4 has been
tried on real hardware**. Everything else below is expected to work, not known to work. If you try one, please
open an issue with what happened.

## How one build covers every panel size

Folio always draws on a fixed 1072×1448 canvas. At each screen update `app/src/display.rs` scales the changed
region to the real panel (bilinear up, averaging down) before handing it to FBInk, and touches are mapped back
through the touch calibration, so the five-circle setup also absorbs any difference in touch panel size or
orientation. On a 1072×1448 panel nothing is scaled and the output is identical to before.

The panel size is read with `fbink -e` at start-up (`viewWidth` / `viewHeight`). A panel within 16 px of
1072×1448 counts as 1072×1448; if the size can't be read, Folio assumes 1072×1448.

The touch panel is found by looking for a device that reports multitouch axes (or plain X/Y axes) under
`/dev/input`, trying `event2` first; the power key is listened for on every other input device.

## Matrix

| Model | Panel | Expected | Notes |
|---|---|---|---|
| Paperwhite 4 (10th gen) | 1072×1448 | **works** | author's device, firmware 5.17.1.0.3 |
| Paperwhite 3, Voyage, Kindle 11th gen | 1072×1448 | likely | same panel size; Bluetooth audio needs the model to have Bluetooth |
| Paperwhite 5 / Signature | 1236×1648 | scaled up | UI is upscaled (slightly soft); text sizes are physically larger |
| Oasis 1/2/3 | 1264×1680 | scaled up | as above; two physical page-turn buttons are not used |
| Scribe | 1860×2480 | scaled up | very soft UI; Folio does not use the pen |
| Paperwhite 1/2 | 758×1024 | scaled down | older kernel (3.0.x); see below |
| Kindle 7/8/10 (basic) | 600×800 | scaled down | no Bluetooth on most; 10th gen has a light |
| Kindle Touch (4th gen) | 600×800 | scaled down | very old firmware and kernel; likely needs work |
| Kindles without a touchscreen | — | **no** | Folio needs touch |

## Known unknowns

* **Old kernels.** Folio is a Rust program; Rust's standard library officially needs Linux ≥ 3.2. Kindles
  before the Paperwhite 3 run older kernels. It may still work; it may not start.
* **Music** needs Bluetooth audio, which many older Kindles don't have. Without it the Music tab is only a library.
* **Frontlight** controls do nothing on models without a light.
* **Jailbreak and install steps differ by model and firmware.** `docs/INSTALL.md` describes what worked on the
  Paperwhite 4.
* **Reading quality on large panels:** book pages are rendered at 1072 px wide and then scaled. Rendering pages
  natively at the panel's resolution would make text crisp on the Scribe/PW5/Oasis; not done yet.

## Testing a model without owning it

folio-sim (a separate repo, not published yet) runs Folio against a simulated Kindle of any listed
size (its scaling and touch mapping are the same code path as on a device). It can't tell you whether a given
kernel or firmware will run Folio.
