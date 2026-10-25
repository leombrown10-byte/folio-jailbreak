#!/bin/sh
# Cross-compile the Kindle OS app (static ARMv7 musl) inside Docker. Run from repo root.
set -e
mkdir -p build/dist
docker run --rm -v "$PWD":/w -w /w/app -v kos-cargo-reg:/root/.cargo/registry -v kos-target:/w/app/target messense/rust-musl-cross:armv7-musleabihf \
  sh -c 'cargo build --release --target armv7-unknown-linux-musleabihf 2>&1 | tail -80 && cp target/armv7-unknown-linux-musleabihf/release/kos /w/build/dist/kos && ls -la /w/build/dist/kos'
