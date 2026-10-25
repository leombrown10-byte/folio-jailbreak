#!/bin/sh
# Cross-compile native helpers for the Kindle (static ARMv7 musl) inside Docker. Run from repo root.
set -e
M=build/mupdf-1.24.11-source
docker run --rm -v "$PWD":/w -w /w messense/rust-musl-cross:armv7-musleabihf sh -c "
  armv7-unknown-linux-musleabihf-gcc -Os -march=armv7-a -mfpu=neon -static -I$M/include native/kreader.c \
    -o build/dist/kreader $M/build/release/libmupdf.a $M/build/release/libmupdf-third.a -lm
  armv7-unknown-linux-musleabihf-strip build/dist/kreader"
ls -la build/dist
