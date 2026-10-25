# Installing Folio

Written from what was done on one device (Kindle Paperwhite 4, firmware 5.17.1.0.3). Treat every step as
untested on anything else. You can brick a Kindle; keep a second way to read.

## 0. Before you start

* A jailbreakable firmware. Check <https://kindlemodding.org> for your model and firmware (do **not** update the firmware afterwards).
* A Mac/Linux computer with Docker (to build) and `ssh`.
* Back up the Kindle's contents.

## 1. Jailbreak and tools

Follow the community guide for your firmware (the author used WinterBreak), then install KUAL, MRPI and
USBNetwork (the "lite" variant is fine). Enable USB networking and confirm you can `ssh root@192.168.15.244`
(default password for the author's USBNetwork setup is in `build/askpass.sh`; change it if you care).

Folio only uses USB networking as a developer link; Wi-Fi stays off.

## 2. Build or download the binaries

* Easiest: take `kos` (and `kreader`, `fbink`, `mobitool`, `ddjvu`) from a GitHub Release.
* Or build: see [BUILDING.md](BUILDING.md).

Put them in `build/dist/` (and fonts are already in `build/fonts/`).

## 3. Push Folio to the Kindle

```sh
build/deploy.sh          # copies everything to /var/local/kos on the Kindle's system partition
```

Folio lives in `/var/local/kos`, not on the USB drive: the drive disappears while a computer is connected.

## 4. Make Folio the interface (reversible)

```sh
build/install-boot.sh    # renames Amazon's UI start-up jobs to *.amazon and installs /etc/upstart/kos.conf
```

Restart the Kindle. Folio's first screen is a five-circle touch setup. If it crashes three times in a row the
supervisor shows a recovery message and brings USB networking up so you can log in.

To go back: `build/restore-stock-ui.sh`.

## 5. Optional: remove Amazon's software (NOT reversible)

`build/remove-amazon.sh` deletes Amazon's UI, services and data and installs Folio's branding. After it the
stock UI cannot be restored without a firmware reinstall. It refuses to run unless you set
`FOLIO_I_UNDERSTAND_THIS_IS_PERMANENT=yes`. Back up first, for example
`ssh root@192.168.15.244 'tar cf - /opt/amazon /usr/java /etc/upstart' > amazon-backup.tar`
(keep that file private: it is Amazon's software and must never be committed or shared).

Known trap: deleting `/var/local/kindleforeink_weblabs.json` or other Amazon dynamic-config data can stop the
X server from starting (`FormatEPDC` empty). The script no longer deletes that file; if you hit it, the fix is to
make `/etc/upstart/makexconfig` fall back to `epdcFormat=Y8`.

## Troubleshooting

* Blank screen with the cable plugged in at boot: boot with the cable out, plug in after Home shows.
* `/var/log/messages` and `/var/log/Xorg.0.log` are the first places to look.
* Music clicks, freezes: see the notes in `app/src/music.rs` (the mixer only plays complete files, so tracks are decoded into segment files).
