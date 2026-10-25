#!/bin/sh
# Put Amazon's interface back. Only works if build/remove-amazon.sh was NOT run (that script deletes the files
# this one needs). Needs SSH (developer USB networking).
set -e
D=$(cd "$(dirname "$0")" && pwd)
KS="$D/ks"
"$KS" 'mntroot rw
cd /etc/upstart
rm -f kos.conf
for f in *.conf.amazon; do [ -f "$f" ] && mv "$f" "${f%.amazon}"; done
sync
mntroot ro
ls /etc/upstart | grep -c amazon || true'
echo "stock UI jobs restored. Restart the Kindle."
