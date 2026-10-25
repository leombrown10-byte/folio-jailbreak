# Building

Everything is cross-compiled for ARMv7 (hard-float, musl, static) inside Docker image
`messense/rust-musl-cross:armv7-musleabihf`. Nothing is built on the Kindle.

## The app (`kos`, Rust)

```sh
build/build-app.sh      # -> build/dist/kos
```

This is also what CI runs.

## Native helpers (not built by CI)

`kreader` (C, `native/kreader.c`) links a static MuPDF; `mobitool` (libmobi) and `ddjvu` (djvulibre) convert
MOBI/AZW and DjVu. `fbink` draws to the e-ink panel.

Notes from the original build (reconstructed from memory of what worked on the author's machine — expect to
adjust):

* MuPDF 1.24.11: built static in the same container with the TOFU font flags set (`-DTOFU`, `-DTOFU_CJK`, … to drop
  large font data), `HAVE_OBJCOPY=no`, no X11/GL; `IBM Plex` fonts were substituted for the built-in Nimbus fonts.
  Then `build/build-native.sh` links `native/kreader.c` against `libmupdf.a` and `libmupdf-third.a`.
* libmobi: autotools/CMake, static, only `mobitool` is used.
* djvulibre 3.5.28: static `ddjvu` only.
* FBInk: use the release binary for Kindle (<https://github.com/NiLuJe/FBInk>).

If you get these building cleanly, please contribute a `build/fetch-deps.sh`.

## Dictionary

```sh
# WordNet 3.0 (Princeton) -> dict.tsv
curl -L -o wn.tgz https://wordnetcode.princeton.edu/3.0/WordNet-3.0.tar.gz && tar xzf wn.tgz
python3 build/dict/wordnet.py WordNet-3.0/dict dict.tsv
```

`dict_old.tsv` (Webster's 1913, public domain) is optional: same `word<TAB>definition` format, sorted by
lowercase word. Without it, archaic words just have no entry.

Put both under `/mnt/us/kos-update/` on the drive, or copy to `/var/local/kos/data/`.

## Releases

Pushing a tag `vX.Y.Z` makes CI build `kos` and attach it to a GitHub Release.
