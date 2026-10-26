#!/bin/sh
# Build Folio's native helpers for the Kindle (static ARMv7 musl) in Docker and put them in build/dist:
#   kreader  (MuPDF + native/kreader.c)   mobitool (libmobi)   ddjvu (djvulibre)
# Downloads sources into build/deps (git-ignored). Needs Docker, curl, git, tar. Takes ~10-20 minutes the first time.
# Usage: build/fetch-deps.sh [kreader|mobitool|ddjvu]   (default: all)
set -e
cd "$(dirname "$0")/.."
ROOT=$PWD
W=build/deps
IMG=messense/rust-musl-cross:armv7-musleabihf
T=armv7-unknown-linux-musleabihf
MUPDF=mupdf-1.24.11-source
DJVU=djvulibre-3.5.28
LIBMOBI_SHA=906274205c11944b628da1c553b255acb1af7c55
WHAT=${1:-all}
mkdir -p "$W" build/dist
dk() { docker run --rm -v "$ROOT":/w -w "/w/$1" "$IMG" sh -c "$2"; }

want() { [ "$WHAT" = all ] || [ "$WHAT" = "$1" ]; }

if want kreader; then
	echo "== MuPDF"
	if [ ! -d "$W/$MUPDF" ]; then
		curl -fsSL -o "$W/mupdf.tgz" "https://mupdf.com/downloads/archive/$MUPDF.tar.gz"
		tar -xzf "$W/mupdf.tgz" -C "$W"
	fi
	# IBM Plex replaces the built-in fonts so Folio looks the same as its own UI; the generated font sources are
	# rebuilt from the .cff files (the files hold TrueType data; FreeType reads either)
	F=../../fonts
	( cd "$W/$MUPDF"
	for s in Regular Bold Italic BoldItalic; do
		cp "$F/IBMPlexSerif-$s.ttf" "resources/fonts/urw/NimbusRoman-$s.cff"
		cp "$F/IBMPlexSans-$s.ttf" "resources/fonts/urw/NimbusSans-$s.cff"
	done
	cp "$F/IBMPlexSerif-Regular.ttf" resources/fonts/sil/CharisSIL.cff
	cp "$F/IBMPlexSerif-Bold.ttf" resources/fonts/sil/CharisSIL-Bold.cff
	cp "$F/IBMPlexSerif-Italic.ttf" resources/fonts/sil/CharisSIL-Italic.cff
	cp "$F/IBMPlexSerif-BoldItalic.ttf" resources/fonts/sil/CharisSIL-BoldItalic.cff
	rm -f generated/resources/fonts/urw/NimbusRoman*.c generated/resources/fonts/urw/NimbusSans*.c generated/resources/fonts/sil/*.c
	rm -f build/release/source/fitz/noto.o )
	dk "$W/$MUPDF" "make -j8 build=release HAVE_X11=no HAVE_GLUT=no HAVE_CURL=no HAVE_OBJCOPY=no USE_SYSTEM_LIBS=no \
		CC=$T-gcc CXX=$T-g++ AR=$T-ar \
		XCFLAGS='-Os -march=armv7-a -mfpu=neon -DTOFU_CJK -DTOFU_NOTO -DTOFU_SYMBOL -DTOFU_EMOJI -DTOFU_SIL' \
		LDFLAGS=-static build/release/mutool 2>&1 | tail -3"
	M="$W/$MUPDF" build/build-native.sh
fi

if want mobitool; then
	echo "== libmobi"
	if [ ! -d "$W/libmobi" ]; then
		git init -q "$W/libmobi"
		git -C "$W/libmobi" fetch -q --depth 1 https://github.com/bfabiszewski/libmobi.git "$LIBMOBI_SHA"
		git -C "$W/libmobi" checkout -q FETCH_HEAD
	fi
	dk "$W/libmobi" "rm -rf b && mkdir b && cd b && cmake .. -DCMAKE_SYSTEM_NAME=Linux -DCMAKE_C_COMPILER=$T-gcc \
		-DBUILD_SHARED_LIBS=OFF -DUSE_ZLIB=OFF -DUSE_LIBXML2=OFF -DUSE_ENCRYPTION=OFF -DCMAKE_EXE_LINKER_FLAGS=-static \
		-DCMAKE_BUILD_TYPE=Release 2>&1 | tail -2 && make -j8 mobitool 2>&1 | tail -2 && $T-strip -o /w/build/dist/mobitool tools/mobitool"
fi

if want ddjvu; then
	echo "== djvulibre"
	if [ ! -d "$W/$DJVU" ]; then
		curl -fsSL -L -o "$W/djvulibre.tgz" "https://downloads.sourceforge.net/project/djvu/DjVuLibre/3.5.28/$DJVU.tar.gz"
		tar -xzf "$W/djvulibre.tgz" -C "$W"
	fi
	dk "$W/$DJVU" "./configure --host=$T CC=$T-gcc CXX=$T-g++ --disable-desktopfiles --disable-xmltools --without-jpeg \
		--disable-shared --enable-static LDFLAGS=-static CXXFLAGS=-Os CFLAGS=-Os 2>&1 | tail -2 \
		&& make -C libdjvu -j8 2>&1 | tail -2 && make -C tools ddjvu LDFLAGS='-all-static -static' 2>&1 | tail -2 \
		&& $T-strip -o /w/build/dist/ddjvu tools/ddjvu"
fi

echo "== done"
ls -la build/dist
