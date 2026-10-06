# Changelog

All notable changes to this project are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [2.0.0] - 2026-10-07

Complete rewrite in Rust. The command is now `hmterrain`.

### Added

- Adaptive meshes (TIN) by greedy Delaunay insertion, the new default. Every
  pixel stays within `--max-error` of the mesh, and flat areas need far fewer
  triangles. `--max-triangles` and `--max-vertices` set a size budget instead.
- Output formats: binary glTF (`.glb`), binary STL (`.stl`) and binary PLY
  (`.ply`), next to OBJ.
- `--base THICKNESS` closes the terrain into a watertight solid for 3D printing.
- 16-bit and floating-point heightmaps, plus TIFF, JPEG, BMP, TGA and PNM input.
- Non-square heightmaps.
- Texture coordinates, so the source image or a satellite photo can be draped
  over the terrain.
- `--pixel-size`, `--invert`, `--no-normalize`, `--no-center`, `--format`,
  `--threads` and `--quiet`.
- A Rust library (`heightmap_terrain`) with the same features.
- CI on Linux, macOS and Windows, release binaries for seven targets with
  build provenance, CodeQL, OpenSSF Scorecard, cargo-deny and Dependabot.

### Changed

- Heights are given with `--min-height` and `--max-height`. The v1 form
  `hmterrain in.png out.obj -200 200` still works.
- Grid cells are split along the diagonal with the closer heights, so diagonal
  ridges and valleys stay sharp.
- The mesh is centered on X and Z only. The heights you ask for are the heights
  you get.
- Output is written to a temporary file first, so a failed run never leaves a
  truncated mesh.
- The output file is no longer opened in another program after conversion.

### Fixed

- Height normalization divided by `max - min + 1`, so terrain used only part of
  the requested height range.
- Face indices in OBJ files were 0-based, which shifted every triangle by one
  vertex.
- Each grid cell was covered by four overlapping triangles with mixed winding,
  and a two-pixel border was missing.
- Normals were deduplicated as text and never referenced by faces, so viewers
  ignored them.
- Numbers used the system locale, which wrote commas as decimal separators on
  some systems and broke the OBJ file.
- Only the green channel was read, and 16-bit images lost precision.

## [1.0.1] - 2024-10-13

Last release of the original .NET implementation (.NET 8).

## [1.0.0] - 2021-06-29

First release of the .NET implementation.

[Unreleased]: https://github.com/shanto462/HeightMapTo3dTerrain/compare/v2.0.0...HEAD
[2.0.0]: https://github.com/shanto462/HeightMapTo3dTerrain/compare/1.0.1...v2.0.0
[1.0.1]: https://github.com/shanto462/HeightMapTo3dTerrain/releases/tag/1.0.1
[1.0.0]: https://github.com/shanto462/HeightMapTo3dTerrain/releases/tag/1.0.0
