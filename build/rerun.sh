#!/bin/sh
# Restart the app on the Kindle: the boot supervisor relaunches kos from /var/local/kos/bin.
D=$(cd "$(dirname "$0")" && pwd)
"$D/ks" 'rm -f /tmp/kos.log; for p in $(pidof kos); do kill $p; done'
sleep 6
