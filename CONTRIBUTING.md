# Contributing

Thanks for helping. This started on one Paperwhite 4, so the most valuable contributions are:

1. **Reports from other models/firmware**: what works, what doesn't (screen size, touch device, framebuffer format, audio).
2. **Reproducible native builds** (`build/fetch-deps.sh` for MuPDF, libmobi, djvulibre, FBInk).
3. Bug fixes and features from the "Known limits" list in the README.

## Ground rules

* Never commit Amazon firmware, backups or extracted Amazon files. Never commit credentials.
* Anything that deletes or overwrites data on a device must be explicit, reversible when possible, and loudly documented.
* Say what you tested on (model, firmware). Untested code paths should say so.
* Keep the UI responsive: no blocking calls on the event thread (`sys::sh` has a 3 s timeout for a reason).
* Run `build/build-app.sh` before opening a PR; CI does the same.

## Bug reports

Include: Kindle model, firmware, what you did, what you expected, and the tail of `/var/log/messages` and
`/var/local/kos/data/slow.log` if you have SSH.
