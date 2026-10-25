#!/bin/sh
# Push the app to the Kindle's persistent system partition over the USB network.
set -e
D=$(cd "$(dirname "$0")" && pwd)
KS="$D/ks"
R=/var/local/kos
push() { "$KS" "cat > $2.new && chmod +x $2.new && mv $2.new $2" < "$1"; }
"$KS" "mkdir -p $R/bin $R/fonts $R/data /mnt/us/books /mnt/us/music"
for f in kos kreader mobitool ddjvu fbink; do [ -f "$D/dist/$f" ] && push "$D/dist/$f" $R/bin/$f; done
for f in "$D"/fonts/*.ttf; do [ -f "$f" ] && "$KS" "cat > $R/fonts/$(basename "$f")" < "$f"; done
push "$D/boot.sh" $R/boot.sh
echo deployed
