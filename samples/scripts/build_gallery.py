"""Build the README gallery: convert each sample, render it, write thumbnails.

Run from anywhere after `cargo build --release`:
    uv run --with numpy --with matplotlib --with pillow python samples/scripts/build_gallery.py

Meshes and full-size renders go to samples/output/gallery. Thumbnails go to
assets/gallery, and the stats of every conversion to samples/output/gallery/stats.json.
"""
import json
import re
import subprocess
import sys
from concurrent.futures import ProcessPoolExecutor
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
from PIL import Image, ImageChops, ImageOps

SAMPLES = Path(__file__).resolve().parent.parent
REPO = SAMPLES.parent
HMTERRAIN = REPO / "target" / "release" / "hmterrain"
OUT = SAMPLES / "output" / "gallery"
THUMBS = REPO / "assets" / "gallery"
GALLERY = ["pasadena", "everest", "grand_canyon", "fuji", "yosemite", "matterhorn", "kilimanjaro", "crater_lake"]
ERRORS = [3, 10, 30]  # meters, for the accuracy-versus-size row (St. Helens)
STAT = re.compile(r"(\d+) vertices \(([\d.]+)% of pixels\), (\d+) triangles, max error ([\d.]+) in ([\d.]+)s")

sys.path.insert(0, str(Path(__file__).parent))
from render_glb import load_glb, render  # noqa: E402


def convert(src, dst, meta, extra=()):
    scale = ["--min-height", str(meta["min_m"]), "--max-height", str(meta["max_m"]),
             "--pixel-size", str(meta["meters_per_pixel"])]
    out = subprocess.run([HMTERRAIN, src, dst, *scale, *extra], check=True, capture_output=True, text=True)
    v, pct, t, err, secs = STAT.search(out.stderr).groups()
    return {"vertices": int(v), "percent": float(pct), "triangles": int(t), "max_error": float(err),
            "seconds": float(secs), "args": scale + list(extra)}


def trim(path, size):
    im = Image.open(path).convert("RGB")
    box = ImageChops.difference(im, Image.new("RGB", im.size, "white")).getbbox()
    return ImageOps.contain(im.crop(box) if box else im, size)


def wireframe(glb, png):
    pos, idx = load_glb(glb)
    fig = plt.figure(figsize=(4, 4), dpi=100)
    ax = fig.add_axes([0, 0, 1, 1])
    ax.triplot(pos[:, 0], -pos[:, 2], idx, linewidth=0.2, color="#333333")
    ax.set_aspect("equal")
    ax.set_axis_off()
    fig.savefig(png, facecolor="white")
    plt.close(fig)


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    THUMBS.mkdir(parents=True, exist_ok=True)
    meta = json.loads((SAMPLES / "heightmaps" / "metadata.json").read_text())
    stats = {}

    for name in GALLERY:
        src = SAMPLES / "heightmaps" / meta[name]["file"]
        stats[name] = convert(src, OUT / f"{name}.glb", meta[name])
        # 8-bit copy: converting the 16-bit PNG to "L" would clip it to white.
        preview = src.with_name(src.name.replace("_16bit", "_8bit"))
        ImageOps.contain(Image.open(preview).convert("L"), (320, 320)).save(
            THUMBS / f"{name}_input.jpg", quality=85, optimize=True)

    helens = meta["st_helens"]
    src = SAMPLES / "heightmaps" / helens["file"]
    for e in ERRORS:
        stats[f"st_helens_e{e}"] = convert(src, OUT / f"st_helens_e{e}.glb", helens, ["--max-error", str(e)])
    stats["st_helens_solid"] = convert(src, OUT / "st_helens_solid.glb", helens, ["--base", "300"])

    jobs = [(OUT / f"{n}.glb", OUT / f"{n}.png", 1.0 if n == "pasadena" else 2.0) for n in GALLERY]
    # f3d draws the flat walls of the solid more cleanly than matplotlib.
    subprocess.run(["f3d", OUT / "st_helens_solid.glb", f"--output={OUT / 'st_helens_solid.png'}",
                    "--resolution=960,640", "--up=+Y", "--camera-elevation-angle=30",
                    "--camera-azimuth-angle=-30", "--background-color=1,1,1", "--anti-aliasing",
                    "--no-config"], check=True)
    with ProcessPoolExecutor() as pool:
        list(pool.map(render, *zip(*jobs)))
        list(pool.map(wireframe, [OUT / f"st_helens_e{e}.glb" for e in ERRORS],
                      [OUT / f"st_helens_e{e}.png" for e in ERRORS]))

    for name in GALLERY + ["st_helens_solid"]:
        trim(OUT / f"{name}.png", (480, 320)).save(THUMBS / f"{name}_output.jpg", quality=88, optimize=True)
    for e in ERRORS:
        trim(OUT / f"st_helens_e{e}.png", (320, 320)).save(THUMBS / f"st_helens_e{e}.png", optimize=True)
    (OUT / "stats.json").write_text(json.dumps(stats, indent=2))
    for name, s in stats.items():
        print(f"{name:18} {s['vertices']:>8} v {s['percent']:>5}% {s['triangles']:>8} t  err {s['max_error']}  {s['seconds']}s")


if __name__ == "__main__":
    main()
