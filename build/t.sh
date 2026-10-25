#!/bin/sh
# Send test commands: build/t.sh "tap 500 600" "swipe ..." ; then optional screenshot name via SHOT=name
D=$(cd "$(dirname "$0")" && pwd)
for c in "$@"; do "$D/ks" "echo '$c' > /tmp/kos.cmd"; sleep 1.5; done
