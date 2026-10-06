"""Download Mapzen Terrarium tiles (AWS Open Data) and save real-world heightmaps.

Terrarium encodes elevation as (R * 256 + G + B / 256) - 32768 meters.
"""
import io, json, math, urllib.request
from pathlib import Path
import numpy as np
from PIL import Image

OUT = Path(__file__).resolve().parent.parent / "heightmaps"
URL = "https://s3.amazonaws.com/elevation-tiles-prod/terrarium/{z}/{x}/{y}.png"
PLACES = {
    # name: (lat, lon, zoom, tiles_x, tiles_y)
    "everest": (27.988, 86.925, 11, 3, 3),
    "grand_canyon": (36.10, -112.10, 11, 4, 2),
    "fuji": (35.361, 138.728, 12, 3, 3),
    "st_helens": (46.191, -122.195, 13, 2, 2),
    "yosemite": (37.745, -119.59, 12, 3, 2),
    "matterhorn": (45.976, 7.658, 13, 3, 3),
    "kilimanjaro": (-3.067, 37.355, 11, 3, 3),
    "crater_lake": (42.94, -122.105, 12, 3, 3),
}

def tile_xy(lat, lon, z):
    n = 2 ** z
    x = (lon + 180) / 360 * n
    y = (1 - math.asinh(math.tan(math.radians(lat))) / math.pi) / 2 * n
    return x, y

meta = {}
for name, (lat, lon, z, tx, ty) in PLACES.items():
    cx, cy = tile_xy(lat, lon, z)
    x0, y0 = int(cx - tx / 2 + 0.5), int(cy - ty / 2 + 0.5)
    rows = []
    for y in range(y0, y0 + ty):
        row = []
        for x in range(x0, x0 + tx):
            with urllib.request.urlopen(URL.format(z=z, x=x, y=y), timeout=30) as r:
                rgb = np.asarray(Image.open(io.BytesIO(r.read())).convert("RGB")).astype(np.float64)
            row.append(rgb[..., 0] * 256 + rgb[..., 1] + rgb[..., 2] / 256 - 32768)
        rows.append(np.hstack(row))
    elev = np.vstack(rows)
    lo, hi = float(elev.min()), float(elev.max())
    norm = (elev - lo) / (hi - lo)
    Image.fromarray(np.round(norm * 65535).astype(np.uint16)).save(OUT / f"{name}_16bit.png")
    Image.fromarray(np.round(norm * 255).astype(np.uint8)).save(OUT / f"{name}_8bit.png")
    meters_per_px = 156543.03392 * math.cos(math.radians(lat)) / 2 ** z
    meta[name] = {"file": f"{name}_16bit.png", "width": elev.shape[1], "height": elev.shape[0], "min_m": round(lo, 2),
                  "max_m": round(hi, 2), "meters_per_pixel": round(meters_per_px, 3)}
    print(name, meta[name])
existing = json.loads((OUT / "metadata.json").read_text()) if (OUT / "metadata.json").exists() else {}
existing.update(meta)
(OUT / "metadata.json").write_text(json.dumps(existing, indent=2))
