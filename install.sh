#!/bin/sh
# Folio installer for a jailbroken Kindle, run from your computer with the Kindle plugged in over USB.
#
#   ./install.sh            check the Kindle, fetch what is missing, install Folio (asks before each change)
#   ./install.sh check      only look: connection, model, firmware, free space, which files you have
#   ./install.sh restore    put Amazon's interface back (only if you never ran build/remove-amazon.sh)
#   ./install.sh -y ...     answer yes to every question
#
# Before this: jailbreak the Kindle and enable USB networking (README, "Install", steps 0-1).
set -e
cd "$(dirname "$0")"
KS=build/ks
DIST=build/dist
REPO=$(git remote get-url origin 2>/dev/null | sed -E 's#(git@github.com:|https://github.com/)##; s#\.git$##')
REPO=${REPO:-leombrown10-byte/folio-jailbreak}
REL="https://github.com/$REPO/releases/latest/download"
YES=0
MODE=install
for a in "$@"; do
	case "$a" in
		-y|--yes) YES=1 ;;
		check|install|restore) MODE=$a ;;
		-h|--help|help) sed -n '2,9p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
		*) echo "unknown option: $a (try ./install.sh help)"; exit 1 ;;
	esac
done

say()  { printf '\n\033[1m%s\033[0m\n' "$*"; }
ok()   { printf '  \033[32mok\033[0m   %s\n' "$*"; }
warn() { printf '  \033[33m!!\033[0m   %s\n' "$*"; }
bad()  { printf '  \033[31mno\033[0m   %s\n' "$*"; }
ask()  { [ "$YES" = 1 ] && return 0; printf '%s [y/N] ' "$1"; read -r r || r=n; [ "$r" = y ] || [ "$r" = Y ]; }
dev()  { KS_TIMEOUT=8 "$KS" "$@" 2>/dev/null || true; }

# ---------------------------------------------------------------- 1. the Kindle
check_device() {
	say "1. Is the Kindle reachable?"
	out=$(dev 'echo up')
	if [ "$out" != up ]; then
		bad "cannot log in to the Kindle at 192.168.15.244"
		cat <<'EOF'

     Things to check:
       - the Kindle is plugged in with a data cable (not a charge-only cable)
       - it is jailbroken and USB networking is on (README, Install, steps 0-1)
       - on the Kindle, USB networking is active: boot with the cable unplugged, plug in once it is on the Home screen
       - your computer shows a new network interface for the Kindle (192.168.15.x)
EOF
		return 1
	fi
	ok "connected"
	fw=$(dev 'cat /etc/prettyversion.txt' | head -1)
	kern=$(dev 'uname -r')
	free=$(dev "df -k /var/local | tail -1 | awk '{print int(\$4/1024)}'")
	ok "firmware: ${fw:-unknown}   kernel: ${kern:-unknown}   free space for Folio: ${free:-?} MB"
	case "$kern" in
		2.*|3.0*|3.1.*) warn "this kernel is older than Linux 3.2, which Rust programs officially need: Folio may not start" ;;
	esac
	[ "${free:-0}" -ge 60 ] 2>/dev/null || warn "less than 60 MB free in /var/local; Folio needs about 40 MB plus your data"
	if dev 'test -x /var/local/kos/bin/kos && echo y' | grep -q y; then ok "Folio is already installed (this will update it)"; fi
	if dev 'test -x /mnt/us/libkh/bin/fbink && echo y' | grep -q y; then HAVE_FBINK_ON_DEVICE=1; ok "jailbreak tools found (KUAL, FBInk)"; else HAVE_FBINK_ON_DEVICE=0; warn "no FBInk on the device (/mnt/us/libkh/bin/fbink): is KUAL installed?"; fi
	return 0
}

# ---------------------------------------------------------------- 2. the files
fetch_file() { # name
	f=$1
	[ -f "$DIST/$f" ] && { ok "$f present"; return 0; }
	if [ "$f" = fbink ]; then
		if [ "${HAVE_FBINK_ON_DEVICE:-0}" = 1 ] && ask "  copy FBInk from the Kindle (it ships with KUAL)?"; then
			dev 'cat /mnt/us/libkh/bin/fbink' > "$DIST/fbink"
			if [ -s "$DIST/fbink" ]; then chmod +x "$DIST/fbink"; ok "fbink copied from the Kindle"; return 0; fi
			rm -f "$DIST/fbink"
		fi
		bad "fbink missing"; return 1
	fi
	if ask "  download $f from the latest GitHub release?"; then
		if curl -fL --progress-bar -o "$DIST/$f.part" "$REL/$f" && [ -s "$DIST/$f.part" ]; then mv "$DIST/$f.part" "$DIST/$f"; chmod +x "$DIST/$f"; ok "$f downloaded"; return 0; fi
		rm -f "$DIST/$f.part"; warn "no release has $f yet"
	fi
	return 1
}

get_files() {
	say "2. Folio's files"
	mkdir -p "$DIST"
	missing=""
	for f in kos kreader mobitool ddjvu fbink; do fetch_file "$f" || missing="$missing $f"; done
	[ -z "$missing" ] && return 0
	warn "still missing:$missing"
	case "$missing" in *kos*)
		if command -v docker >/dev/null 2>&1 && ask "  build the app (kos) here with Docker? (a few minutes)"; then build/build-app.sh >/dev/null && ok "kos built"; fi ;;
	esac
	for f in kreader mobitool ddjvu; do
		[ -f "$DIST/$f" ] && continue
		if command -v docker >/dev/null 2>&1 && ask "  build $f from source with Docker? (10-20 minutes the first time)"; then build/fetch-deps.sh "$f" >/dev/null && ok "$f built"; fi
	done
	for f in kos kreader mobitool ddjvu fbink; do
		[ -f "$DIST/$f" ] || { bad "$f is still missing; see README, Building"; return 1; }
	done
}

# ---------------------------------------------------------------- 3. install
do_install() {
	check_device || exit 1
	get_files || exit 1
	say "3. Copy Folio to the Kindle"
	echo "   Files go to /var/local/kos on the Kindle's system storage. Nothing of Amazon's is touched yet."
	ask "   continue?" || { echo "stopped."; exit 0; }
	build/deploy.sh
	say "4. Make Folio the interface"
	echo "   This renames Amazon's UI start-up jobs to *.amazon and adds Folio's start-up job. It is reversible"
	echo "   (./install.sh restore). The Kindle keeps working as before until you restart it."
	ask "   continue?" || { echo "Folio is copied but not active. Run ./install.sh again to finish."; exit 0; }
	build/install-boot.sh
	say "Done."
	cat <<'EOF'

   Restart the Kindle (hold the power button, or: Settings > Restart in the Kindle's menu).
   Folio starts with five touch-setup circles: tap the middle of each one. Then a short how-to appears.

   Then plug the Kindle into your computer to copy books into "books" and music into "music".
   Problems? README, Install, Troubleshooting.
EOF
}

case "$MODE" in
	check) check_device && { say "Files"; for f in kos kreader mobitool ddjvu fbink; do [ -f "$DIST/$f" ] && ok "$f" || warn "$f not downloaded or built yet"; done; } ;;
	install) do_install ;;
	restore)
		check_device || exit 1
		say "Restore Amazon's interface"
		ask "   rename the .amazon start-up jobs back and remove Folio's job?" || exit 0
		build/restore-stock-ui.sh ;;
esac
