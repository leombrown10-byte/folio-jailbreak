#!/bin/sh
# Launch Kindle OS: stop Amazon's UI, run kos, always bring Amazon's UI back afterwards.
# --boot: autostart path. A crash/hang during start-up leaves .starting behind; the next boot then
# falls back to the stock UI instead of looping.
cd /mnt/us/kos || exit 1
export PATH=/mnt/us/kos/bin:$PATH
export HOME=/mnt/us/kos/data
if [ "$1" = "--boot" ]; then
	if [ -f .starting ]; then
		rm -f .starting
		exit 0
	fi
	touch .starting
fi
restore() { rm -f /mnt/us/kos/.starting; start lab126_gui >/dev/null 2>&1; }
trap restore EXIT INT TERM
lipc-set-prop com.lab126.powerd preventScreenSaver 1 2>/dev/null
stop lab126_gui >/dev/null 2>&1
sleep 1
./bin/kos >> /tmp/kos.log 2>&1
echo "kos exited: $?" >> /tmp/kos.log
