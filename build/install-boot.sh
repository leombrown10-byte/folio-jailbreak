#!/bin/sh
# Make Folio the Kindle's interface. Needs SSH (developer USB networking) and a prior build/deploy.sh.
# Reversible: Amazon's UI job files are only renamed to *.amazon (see build/restore-stock-ui.sh).
set -e
D=$(cd "$(dirname "$0")" && pwd)
KS="$D/ks"
"$KS" 'test -x /var/local/kos/bin/kos && test -x /var/local/kos/boot.sh' || { echo "run build/deploy.sh first"; exit 1; }
"$KS" 'mntroot rw
cd /etc/upstart
for j in lab126_gui framework framework_setup lab126_gui_monitor lab126_gui_setup kb pillow statusbar webreader kfxreader kfxview kppmainapp progressivedownloads whisperstore kindle contentpackd; do
	[ -f $j.conf ] && mv $j.conf $j.conf.amazon
done
cat > kos.conf <<EOC
description "Folio (replaces the stock UI)"
start on n_ready and langpicker_ready and ekart_ready
stop on ota-update
exec /var/local/kos/boot.sh
EOC
sync
mntroot ro
ls /etc/upstart | grep -E "kos|amazon"'
echo "installed. Restart the Kindle to start Folio."
