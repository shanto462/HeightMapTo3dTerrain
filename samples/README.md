# Samples

Real-world heightmaps for trying and testing `hmterrain`.

| Folder | In git | Contents |
| --- | --- | --- |
| `heightmaps/` | yes | Sample inputs as 16-bit and 8-bit grayscale PNGs, plus `metadata.json` with the true height range and pixel size of each. |
| `scripts/` | yes | Scripts that download, convert and render the samples. |
| `output/` | no | Everything the scripts generate: meshes, renders, synthetic heightmaps. |

## Try it

```bash
cargo build --release
python3 samples/scripts/convert_all.py
f3d samples/output/meshes/st_helens.glb
```

`convert_all.py` writes a TIN `.glb`, a full grid `.glb`, a printable `.stl`
with a base, an `.obj`, and a 100k-triangle `.ply` for every sample. The meshes
use true scale: heights in meters and the real pixel size from `metadata.json`.

## Samples

| Name | Size | Height range (m) | Pixel size (m) |
| --- | --- | --- | --- |
| `pasadena` | 800×800, 8-bit | -200 to 200 (example) | 1 |
| `everest` | 768×768 | 2169 to 8741 | 67.5 |
| `grand_canyon` | 1024×512 | 677 to 2506 | 61.8 |
| `fuji` | 768×768 | 176 to 3754 | 31.2 |
| `st_helens` | 512×512 | 943 to 2537 | 13.2 |
| `yosemite` | 768×512 | 1046 to 3024 | 30.2 |
| `matterhorn` | 768×768 | 1701 to 4354 | 13.3 |
| `kilimanjaro` | 768×768 | 828 to 5883 | 76.3 |
| `crater_lake` | 768×768 | 1453 to 2715 | 28.0 |

## Scripts

All scripts write inside `samples/` and run with
[uv](https://docs.astral.sh/uv/), which installs the Python packages they need:

```bash
uv run --with numpy --with pillow python samples/scripts/fetch_terrarium.py    # re-download the real heightmaps
uv run --with numpy --with pillow python samples/scripts/generate_fbm.py       # 2049x2049 synthetic terrain in output/synthetic
uv run --with numpy --with matplotlib python samples/scripts/render_glb.py IN.glb OUT.png 2   # shaded render, 2x vertical exaggeration
uv run --with numpy --with matplotlib --with pillow python samples/scripts/build_gallery.py   # README gallery (needs f3d)
```

## Data sources

`pasadena` is the original sample of this project. The other heightmaps come
from the [Terrain Tiles](https://registry.opendata.aws/terrain-tiles/) open
dataset on AWS (Mapzen Terrarium encoding), which the
[Tangram Heightmapper](https://tangrams.github.io/heightmapper/) and the
[3D Map Generator](https://3d-map-generator.com/free-heightmap-generator/) also
use. The dataset combines SRTM, USGS 3DEP, ETOPO1 and other sources. See its
[attribution list](https://github.com/tilezen/joerd/blob/master/docs/attribution.md).
