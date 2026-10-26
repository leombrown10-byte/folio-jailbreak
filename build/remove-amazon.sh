#!/bin/sh
# Remove Amazon's remaining software and data from the Kindle and brand it Folio. Needs SSH (developer USB networking).
# Keeps what Folio depends on: the Bluetooth/audio/power/USB/screen services and the jailbreak plumbing.
set -e
# POINT OF NO RETURN. This deletes Amazon's software from the device; there is no way back to the stock UI
# except a factory firmware restore. Take a backup first (see the Install section of the README) and confirm explicitly:
[ "$FOLIO_I_UNDERSTAND_THIS_IS_PERMANENT" = "yes" ] || { echo "refusing: set FOLIO_I_UNDERSTAND_THIS_IS_PERMANENT=yes to run"; exit 1; }
D=$(cd "$(dirname "$0")" && pwd)
KS="$D/ks"
# branding images + splash job
"$KS" 'mkdir -p /var/local/kos/brand'
for f in "$D"/brand/*.png; do "$KS" "cat > /var/local/kos/brand/$(basename "$f")" < "$f"; done
"$KS" 'mntroot rw
set +e
# 1. Amazon-only services: cloud, content, telemetry, OTA, app manager
for j in acedropboxd appmgrd archive todo otaupd otav3 minervad minerva_service minervaproxy stackdumpd iohwlogs; do
	stop $j >/dev/null 2>&1
	rm -f /etc/upstart/$j.conf
done
for b in acedropboxd appmgrd archive todo otaupd otav3 minervad minerva_service minervaproxy stackdumpd; do rm -f /usr/bin/$b; done
# 2. Folio screens replace Amazon artwork
for n in splash reboot repair update_initial update_success update_failure hibernation critbatt; do
	cp /var/local/kos/brand/$n.png /opt/amazon/low_level_screens/$n.png
done
# 3. early boot splash
cat > /etc/upstart/folio-splash.conf <<EOC
description "Folio boot splash"
start on filesystems_var_local
task
script
	/var/local/kos/bin/fbink -c -g file=/var/local/kos/brand/splash.png,w=1072,h=1448 -W GC16 -f >/dev/null 2>&1 || true
end script
EOC
# 4. name
echo folio > /etc/hostname
sync
mntroot ro
# 5. Amazon data on the system data partition
cd /var/local
rm -rf appreg.db cc.db wsync.db ksdk.* adunits grok.db dcm.db odot_outbox odot_outbox_discard WishList.db KWISClient.db kpp kmc mesquite bcache java printklogs
# 6. Amazon data on the user storage
cd /mnt/us
rm -rf voice audible screenshots .active_content_sandbox documents/dictionaries documents/Downloads
rm -rf system/* 2>/dev/null
df -k / /var/local /mnt/us | tail -3'
echo done
