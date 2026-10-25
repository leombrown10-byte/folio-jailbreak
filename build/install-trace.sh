#!/bin/sh
# Boot diagnostics: log which start-up events fire (to /var/local/kostrace.log, which survives reboots).
D=$(cd "$(dirname "$0")" && pwd)
"$D/ks" 'mntroot rw
cd /etc/upstart
for ev in "mounted_userstore" "started powerd" "started volumd" "x_setup_ready" "bootactions_ready" "blanket_loaded_langpicker" "langpicker_ready" "n_ready" "ekart_ready" "started usbnetlite"; do
	n=$(echo "$ev" | tr " " "_")
	printf "start on %s\ntask\nscript\n\techo \"\$(date +%%T) %s\" >> /var/local/kostrace.log\nend script\n" "$ev" "$ev" > kostrace_$n.conf
done
printf "start on started kos\ntask\nscript\n\techo \"\$(date +%%T) started kos\" >> /var/local/kostrace.log\nend script\n" > kostrace_kos.conf
echo "---- new boot ----" >> /var/local/kostrace.log
sync; mntroot ro; initctl reload-configuration; ls /etc/upstart | grep -c kostrace'
