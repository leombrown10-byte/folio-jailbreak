# Third-party components

Folio is AGPL-3.0 because it links MuPDF. This file lists what else it uses. Binaries and data not in this
repository are fetched or built separately; check each project's license before redistributing them.

| Component | Use | License |
|---|---|---|
| [MuPDF](https://mupdf.com) | EPUB/PDF/CBZ/FB2/XPS rendering (`kreader`) | AGPL-3.0 (Artifex) |
| [libmobi](https://github.com/bfabiszewski/libmobi) | MOBI/AZW conversion | LGPL-3.0 |
| [DjVuLibre](https://djvu.sourceforge.net) | DjVu conversion | GPL-2.0+ |
| [FBInk](https://github.com/NiLuJe/FBInk) | e-ink drawing | GPL-3.0 |
| [fontdue](https://crates.io/crates/fontdue) | font rasterizer | MIT / Apache-2.0 |
| [symphonia](https://crates.io/crates/symphonia) | audio decoding | MPL-2.0 |
| [serde](https://serde.rs), serde_json, libc | | MIT / Apache-2.0 |
| [IBM Plex](https://github.com/IBM/plex) | UI and reading fonts (`build/fonts`, `app/assets`) | SIL OFL 1.1 |
| [WordNet 3.0](https://wordnet.princeton.edu) | dictionary data | Princeton WordNet license (permissive; keep the copyright notice) |
| Webster's 1913 | optional archaic dictionary | public domain |
| WinterBreak, KUAL, MRPI, USBNetwork | jailbreak and tooling (not included) | see their projects |

WordNet notice: WordNet Release 3.0 © 2006 by Princeton University. All rights reserved.

Amazon's software is **not** included and must never be committed (see `.gitignore`: `build/backup/`).
