#!/usr/bin/env python3
"""Generate PWA icons from Caddie.svg.

The brand glyph is near-white on transparent and bleeds off every edge, which
makes it invisible in browser tab strips and clips it under the iOS and Android
icon masks. Every output here bakes in an opaque background and insets the glyph.

Rasterising uses qlmanage (macOS QuickLook), so this is a macOS-only build step.
Run it after changing Caddie.svg and commit the results.
"""

import re
import shutil
import struct
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SOURCE = ROOT / "Caddie.svg"
PUBLIC = ROOT / "apps" / "pwa" / "public"

BG = "#121416"
CANVAS = 512

# Fraction of the canvas the glyph height occupies, and corner radius.
# Every variant is full-bleed: qlmanage flattens transparency onto white, so a
# rounded background would render with white corner wedges.
SQUARE = (0.62, 0)  # iOS and Android apply their own mask
MASKABLE = (0.50, 0)  # inside the Android 80% safe-zone circle


def glyph_paths():
    svg = SOURCE.read_text()
    paths = re.findall(r'<path\s+d="([^"]+)"\s+fill="([^"]+)"', svg)
    if not paths:
        sys.exit(f"no <path> elements found in {SOURCE}")
    box = re.search(r'viewBox="0 0 ([\d.]+) ([\d.]+)"', svg)
    if not box:
        sys.exit(f"no viewBox found in {SOURCE}")
    return paths, float(box.group(1)), float(box.group(2))


def build_svg(paths, width, height, frac, radius):
    scale = (CANVAS * frac) / height
    w, h = width * scale, height * scale
    x, y = (CANVAS - w) / 2, (CANVAS - h) / 2
    if radius:
        bg = f'<rect width="{CANVAS}" height="{CANVAS}" rx="{radius}" ry="{radius}" fill="{BG}"/>'
    else:
        bg = f'<rect width="{CANVAS}" height="{CANVAS}" fill="{BG}"/>'
    body = "".join(f'<path d="{d}" fill="{fill}"/>' for d, fill in paths)
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{CANVAS}" height="{CANVAS}" '
        f'viewBox="0 0 {CANVAS} {CANVAS}">'
        f"{bg}"
        f'<g transform="translate({x:.3f} {y:.3f}) scale({scale:.6f})">'
        f"{body}"
        f"</g></svg>\n"
    )


def rasterise(svg_path, out_path, size, workdir):
    subprocess.run(
        ["qlmanage", "-t", "-s", str(CANVAS), "-o", str(workdir), str(svg_path)],
        check=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    rendered = workdir / f"{svg_path.name}.png"
    if not rendered.exists():
        sys.exit(f"qlmanage produced nothing for {svg_path}")
    shutil.move(rendered, out_path)
    if size != CANVAS:
        subprocess.run(
            ["sips", "-z", str(size), str(size), str(out_path)],
            check=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )


def write_ico(png_paths, out_path):
    blobs = [p.read_bytes() for p in png_paths]
    offset = 6 + 16 * len(blobs)
    header = struct.pack("<HHH", 0, 1, len(blobs))
    entries = b""
    for png, size in zip(blobs, ICO_SIZES):
        entries += struct.pack(
            "<BBBBHHII", size % 256, size % 256, 0, 0, 1, 32, len(png), offset
        )
        offset += len(png)
    out_path.write_bytes(header + entries + b"".join(blobs))


ICO_SIZES = [16, 32, 48]

OUTPUTS = [
    ("icon-32.png", SQUARE, 32),
    ("icon-192.png", SQUARE, 192),
    ("icon-512.png", SQUARE, 512),
    ("icon-maskable-192.png", MASKABLE, 192),
    ("icon-maskable-512.png", MASKABLE, 512),
    ("apple-touch-icon.png", SQUARE, 180),
]


def main():
    if not shutil.which("qlmanage") or not shutil.which("sips"):
        sys.exit("qlmanage and sips are required (macOS only)")
    paths, width, height = glyph_paths()
    PUBLIC.mkdir(parents=True, exist_ok=True)

    (PUBLIC / "icon.svg").write_text(build_svg(paths, width, height, *SQUARE))

    with tempfile.TemporaryDirectory() as tmp:
        work = Path(tmp)
        sources = {}
        for variant, (frac, radius) in (("square", SQUARE), ("maskable", MASKABLE)):
            svg_path = work / f"{variant}.svg"
            svg_path.write_text(build_svg(paths, width, height, frac, radius))
            sources[(frac, radius)] = svg_path

        for name, variant, size in OUTPUTS:
            rasterise(sources[variant], PUBLIC / name, size, work)
            print(f"wrote {name} ({size}px)")

        ico_parts = []
        for size in ICO_SIZES:
            part = work / f"ico-{size}.png"
            rasterise(sources[SQUARE], part, size, work)
            ico_parts.append(part)
        write_ico(ico_parts, PUBLIC / "favicon.ico")
        print(f"wrote favicon.ico ({'/'.join(str(s) for s in ICO_SIZES)})")

    print("wrote icon.svg")


if __name__ == "__main__":
    main()
