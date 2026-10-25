# Capture the Kindle's e-ink framebuffer over SSH and save it as a PNG. usage: python3 shot.py out.png
import subprocess, sys, zlib, struct, os
here = os.path.dirname(os.path.abspath(__file__))
W, H, STRIDE = 1072, 1448, 1088
raw = subprocess.run([os.path.join(here, "ks"), f"dd if=/dev/fb0 bs={STRIDE} count={H} 2>/dev/null"], capture_output=True).stdout
if len(raw) < STRIDE * H:
    sys.exit(f"short read: {len(raw)}")
rows = b"".join(b"\x00" + raw[y * STRIDE:y * STRIDE + W] for y in range(H))
def chunk(t, d):
    c = struct.pack(">I", len(d)) + t + d
    return c + struct.pack(">I", zlib.crc32(t + d) & 0xffffffff)
png = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", W, H, 8, 0, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(rows, 6)) + chunk(b"IEND", b"")
open(sys.argv[1], "wb").write(png)
print("saved", sys.argv[1])
