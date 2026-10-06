# Synthetic fBm terrain, 2049x2049, 16-bit grayscale PNG.
from pathlib import Path

import numpy as np
from PIL import Image
OUT = Path(__file__).resolve().parent.parent / "output" / "synthetic"
OUT.mkdir(parents=True, exist_ok=True)
rng = np.random.default_rng(7)
n = 2049
acc = np.zeros((n, n))
amp, total = 1.0, 0.0
for octave in range(9):
    cells = 2 ** (octave + 2)
    grid = rng.random((cells + 1, cells + 1))
    xs = np.linspace(0, cells, n)
    i = np.minimum(xs.astype(int), cells - 1); f = xs - i
    f = f * f * (3 - 2 * f)
    a = grid[np.ix_(i, i)]; b = grid[np.ix_(i, i + 1)]; c = grid[np.ix_(i + 1, i)]; d = grid[np.ix_(i + 1, i + 1)]
    fx = f[None, :]; fy = f[:, None]
    acc += amp * ((a * (1 - fx) + b * fx) * (1 - fy) + (c * (1 - fx) + d * fx) * fy)
    total += amp; amp *= 0.5
acc = (acc / total) ** 2
acc = (acc - acc.min()) / (acc.max() - acc.min())
Image.fromarray((acc * 65535).astype(np.uint16)).save(OUT / "fbm16.png")
Image.fromarray((acc * 255).astype(np.uint8)).save(OUT / "fbm8.png")
