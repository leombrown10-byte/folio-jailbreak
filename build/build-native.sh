#!/bin/sh
# Link kreader against the built MuPDF (static ARMv7 musl) inside Docker. Run from repo root; normally via build/fetch-deps.sh.
set -e
M=${M:-build/deps/mupdf-1.24.11-source}
docker run --rm -v "$PWD":/w -w /w messense/rust-musl-cross:armv7-musleabihf sh -c "
  armv7-unknown-linux-musleabihf-gcc -Os -march=armv7-a -mfpu=neon -static -I$M/include native/kreader.c \
    -o build/dist/kreader $M/build/release/libmupdf.a $M/build/release/libmupdf-third.a -lm
  armv7-unknown-linux-musleabihf-strip build/dist/kreader"
ls -la build/dist
