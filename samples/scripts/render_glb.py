"""Render a .glb terrain written by hmterrain as a shaded, color-mapped image.

    uv run --with numpy --with matplotlib python samples/scripts/render_glb.py IN.glb OUT.png [EXAGGERATION]

EXAGGERATION stretches heights for display only (default 1).
"""
import json
import struct
import sys

import matplotlib
import numpy as np

matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.colors import LightSource


def load_glb(path):
    """Returns (positions, triangle indices) of the first primitive."""
    data = open(path, "rb").read()
    json_len = struct.unpack_from("<I", data, 12)[0]
    doc = json.loads(data[20:20 + json_len])
    binary = data[20 + json_len + 8:]

    def accessor(i, width):
        a = doc["accessors"][i]
        view = doc["bufferViews"][a["bufferView"]]
        dtype = "<u4" if a["componentType"] == 5125 else "<f4"
        return np.frombuffer(binary, dtype, a["count"] * width, view["byteOffset"]).reshape(-1, width)

    prim = doc["meshes"][0]["primitives"][0]
    return accessor(prim["attributes"]["POSITION"], 3), accessor(prim["indices"], 1).reshape(-1, 3)


def render(glb, png, exaggeration=1.0):
    pos, idx = load_glb(glb)
    # glTF is Y-up with image rows along +Z; plot with Z up and north at the back.
    x, y, z = pos[:, 0], -pos[:, 2], pos[:, 1]
    fig = plt.figure(figsize=(7.2, 4.8), dpi=100)
    ax = fig.add_subplot(projection="3d")
    ax.plot_trisurf(x, y, z, triangles=idx, cmap="terrain", linewidth=0, antialiased=False,
                    shade=True, lightsource=LightSource(300, 40))
    ax.set_box_aspect((np.ptp(x), np.ptp(y), np.ptp(z) * exaggeration))
    ax.view_init(elev=35, azim=-65)
    ax.set_axis_off()
    ax.set_position([0, 0, 1, 1])
    fig.savefig(png, facecolor="white")
    plt.close(fig)


if __name__ == "__main__":
    render(sys.argv[1], sys.argv[2], float(sys.argv[3]) if len(sys.argv) > 3 else 1.0)
