#!/bin/sh
# Kindle OS boot supervisor. Started by the upstart job /etc/upstart/kos.conf (it replaced Amazon's UI job).
# Lives on the persistent system partition (/var/local), NOT on the user storage: that storage disappears
# whenever a computer is connected. kos exit codes: 0 = restart the app, 42 = reboot, 43 = power off.
cd /var/local/kos || exit 1
export PATH=/var/local/kos/bin:$PATH
export HOME=/var/local/kos/data
LOG=/var/local/kos/data/boot.log
echo "$(date +%T) boot.sh start" >> /var/local/kostrace.log
# Amazon's UI used to load the "usb" module of blanket (the service that handles USB drive mode). Do it ourselves.
( . /etc/upstart/functions; . /etc/upstart/blanket_functions; i=0
  while [ $i -lt 40 ]; do f_blanket_load_module usb >> /var/local/kostrace.log 2>&1 && break; sleep 2; i=$((i+1)); done
  echo "$(date +%T) blanket usb loaded" >> /var/local/kostrace.log ) &
# One-time: install the early Folio boot-splash job (it starts as soon as the system data partition is mounted).
if [ ! -f /var/local/kos/.splash-job-v2 ]; then
	if mntroot rw >/dev/null 2>&1; then
		cat > /etc/upstart/folio-splash.conf <<'EOC'
description "Folio boot splash"
start on started filesystems_var_local
task
script
	/var/local/kos/bin/fbink -c -g file=/var/local/kos/brand/splash.png,w=1072,h=1448 -W GC16 -f >/dev/null 2>&1 || true
end script
EOC
		sync
		mntroot ro >/dev/null 2>&1
		touch /var/local/kos/.splash-job-v2
		echo "$(date +%T) splash job installed" >> /var/local/kostrace.log
	fi
fi
crashes=0
while [ ! -f /var/local/kos/RECOVERY ]; do
	t0=$(date +%s)
	./bin/kos >> /tmp/kos.log 2>&1
	rc=$?
	echo "$(date) kos exit $rc" >> "$LOG"
	[ $rc -eq 42 ] && { sync; reboot; exit 0; }
	[ $rc -eq 43 ] && { sync; poweroff; exit 0; }
	if [ $(( $(date +%s) - t0 )) -lt 20 ]; then crashes=$((crashes + 1)); else crashes=0; fi
	[ $crashes -ge 3 ] && break
	sleep 1
done
# Recovery: the app keeps crashing. Say so and bring up USB networking so it can be repaired.
fbink -c >/dev/null 2>&1
fbink -y 6 -S 3 "Kindle OS recovery" >/dev/null 2>&1
fbink -y 10 "The app failed to start. Connect USB: ssh root@192.168.15.244 (password: kindle)." >/dev/null 2>&1
/usr/local/bin/usbnetwork.sh >/dev/null 2>&1
sleep 100000000
