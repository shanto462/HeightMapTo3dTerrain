"""Convert every sample heightmap with hmterrain into samples/output/meshes.

Run from anywhere after `cargo build --release`:
    python3 samples/scripts/convert_all.py
"""
import json
import subprocess
from pathlib import Path

SAMPLES = Path(__file__).resolve().parent.parent
HMTERRAIN = SAMPLES.parent / "target" / "release" / "hmterrain"
OUT = SAMPLES / "output" / "meshes"
OUT.mkdir(parents=True, exist_ok=True)

for name, d in json.loads((SAMPLES / "heightmaps" / "metadata.json").read_text()).items():
    src = SAMPLES / "heightmaps" / d["file"]
    scale = ["--min-height", str(d["min_m"]), "--max-height", str(d["max_m"]),
             "--pixel-size", str(d["meters_per_pixel"])]
    runs = {
        f"{name}.glb": [],
        f"{name}_grid.glb": ["--mode", "grid"],
        f"{name}_print.stl": ["--base", str(round((d["max_m"] - d["min_m"]) * 0.1, 1))],
        f"{name}.obj": [],
        f"{name}.ply": ["--max-triangles", "100000"],
    }
    for output, extra in runs.items():
        subprocess.run([HMTERRAIN, src, OUT / output, *scale, *extra], check=True)
