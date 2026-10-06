//! Command-line interface for `heightmap_terrain`.

#![cfg_attr(test, allow(clippy::float_cmp))]

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use clap::{Parser, ValueEnum};
use heightmap_terrain::export::{self, Format};
use heightmap_terrain::{
    HeightMapping, Heightmap, Mesh, MeshOptions, TinOptions, triangulate_grid, triangulate_tin,
};

/// Convert a grayscale heightmap into a 3D terrain mesh.
///
/// By default the mesh is adaptive: flat areas get large triangles and
/// detailed areas get small ones, while every pixel stays within 0.1% of the
/// height range (or half a gray level for 8-bit images, if that is larger).
/// Use `--max-error` to trade accuracy for size, or `--mode grid` to keep one
/// vertex per pixel.
#[derive(Debug, Parser)]
#[allow(clippy::struct_excessive_bools)] // Each flag is an independent switch.
#[command(
    name = "hmterrain",
    version,
    about,
    long_about,
    allow_negative_numbers = true
)]
#[command(after_help = "Examples:\n  \
    hmterrain dem.png terrain.glb --max-height 250\n  \
    hmterrain dem.tif terrain.stl --max-height 40 --pixel-size 0.2 --base 3\n  \
    hmterrain dem.png terrain.obj --min-height -200 --max-height 200 --max-error 1.5")]
struct Cli {
    /// Heightmap image: PNG, TIFF, JPEG, BMP, TGA or PNM; 8-bit, 16-bit or float.
    input: PathBuf,

    /// Output mesh. The format comes from the extension: .glb, .obj, .stl or .ply.
    output: PathBuf,

    /// Legacy form of --min-height, kept for v1 command lines.
    #[arg(hide = true, conflicts_with = "min_height")]
    legacy_min: Option<f32>,

    /// Legacy form of --max-height, kept for v1 command lines.
    #[arg(hide = true, conflicts_with = "max_height", requires = "legacy_min")]
    legacy_max: Option<f32>,

    /// Output format; overrides the file extension.
    #[arg(long, short = 'f', value_enum)]
    format: Option<FormatArg>,

    /// Height of the lowest pixel.
    #[arg(long, value_name = "Z", default_value_t = 0.0)]
    min_height: f32,

    /// Height of the highest pixel.
    #[arg(long, value_name = "Z", default_value_t = 100.0)]
    max_height: f32,

    /// Map black to --min-height and white to --max-height, instead of
    /// stretching the darkest and brightest pixel to them.
    #[arg(long)]
    no_normalize: bool,

    /// Treat bright pixels as low and dark pixels as high.
    #[arg(long)]
    invert: bool,

    /// Horizontal distance between two pixels.
    #[arg(long, value_name = "SIZE", default_value_t = 1.0)]
    pixel_size: f32,

    /// Triangulation strategy.
    #[arg(long, value_enum, default_value_t = Mode::Tin)]
    mode: Mode,

    /// Largest allowed vertical error in height units [default: 0.1% of the height
    /// range, or half a gray level if larger].
    #[arg(long, short = 'e', value_name = "Z")]
    max_error: Option<f32>,

    /// Stop refining before the mesh exceeds this many triangles.
    #[arg(long, value_name = "N")]
    max_triangles: Option<usize>,

    /// Stop refining before the mesh exceeds this many vertices.
    #[arg(long, value_name = "N")]
    max_vertices: Option<usize>,

    /// Close the mesh into a solid with walls and a flat bottom this far below
    /// the lowest point (for 3D printing).
    #[arg(long, value_name = "THICKNESS")]
    base: Option<f32>,

    /// Do not write vertex normals.
    #[arg(long)]
    no_normals: bool,

    /// Do not write texture coordinates.
    #[arg(long)]
    no_uvs: bool,

    /// Keep the top-left pixel at the origin instead of centering the mesh.
    #[arg(long)]
    no_center: bool,

    /// Worker threads [default: all cores].
    #[arg(long, short = 'j', value_name = "N")]
    threads: Option<usize>,

    /// Print nothing except errors.
    #[arg(long, short = 'q')]
    quiet: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Mode {
    /// Adaptive Delaunay mesh within an error limit.
    Tin,
    /// One vertex per pixel.
    Grid,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum FormatArg {
    Glb,
    Obj,
    Stl,
    Ply,
}

impl From<FormatArg> for Format {
    fn from(f: FormatArg) -> Self {
        match f {
            FormatArg::Glb => Format::Glb,
            FormatArg::Obj => Format::Obj,
            FormatArg::Stl => Format::Stl,
            FormatArg::Ply => Format::Ply,
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: &Cli) -> Result<()> {
    let format = match cli.format {
        Some(f) => f.into(),
        None => Format::from_path(&cli.output).with_context(|| {
            format!(
                "cannot tell the format of {}; use a .glb, .obj, .stl or .ply extension, or --format",
                cli.output.display()
            )
        })?,
    };
    if let Some(threads) = cli.threads {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global()
            .context("cannot start the thread pool")?;
    }

    let start = Instant::now();
    let mapping = HeightMapping {
        min: cli.legacy_min.unwrap_or(cli.min_height),
        max: cli.legacy_max.unwrap_or(cli.max_height),
        normalize: !cli.no_normalize,
        invert: cli.invert,
    };
    let map = Heightmap::load(&cli.input, &mapping)?;

    let (triangulation, error) = match cli.mode {
        Mode::Grid => {
            if cli.max_error.is_some() || cli.max_triangles.is_some() || cli.max_vertices.is_some()
            {
                bail!("--max-error, --max-triangles and --max-vertices need --mode tin");
            }
            (triangulate_grid(&map)?, 0.0)
        }
        Mode::Tin => {
            let options = TinOptions {
                max_error: cli.max_error.unwrap_or_else(|| default_error(&map)),
                max_triangles: cli.max_triangles,
                max_vertices: cli.max_vertices,
            };
            let (triangulation, stats) = triangulate_tin(&map, &options)?;
            (triangulation, stats.max_error)
        }
    };

    let options = MeshOptions {
        pixel_size: cli.pixel_size,
        center: !cli.no_center,
        normals: !cli.no_normals,
        uvs: !cli.no_uvs,
        base: cli.base,
    };
    let mesh = Mesh::build(&triangulation, &map, &options)?;
    drop(triangulation);
    export::save(&mesh, format, &cli.output)
        .with_context(|| format!("cannot write {}", cli.output.display()))?;

    if !cli.quiet {
        let pixels = u64::from(map.width()) * u64::from(map.height());
        let depth = map
            .bit_depth()
            .map_or_else(|| "float".to_owned(), |b| format!("{b}-bit"));
        eprintln!(
            "{}: {}x{} {depth} -> {} {}: {} vertices ({:.1}% of pixels), {} triangles, max error {error:.4} in {:.2}s",
            cli.input.display(),
            map.width(),
            map.height(),
            format.name(),
            cli.output.display(),
            mesh.vertex_count(),
            100.0 * mesh.vertex_count() as f64 / pixels as f64,
            mesh.triangle_count(),
            start.elapsed().as_secs_f64(),
        );
    }
    Ok(())
}

/// A thousandth of the height range is invisible at normal viewing distance.
/// Going below half a gray level would only reproduce quantization noise.
fn default_error(map: &Heightmap) -> f32 {
    let (lo, hi) = map.height_range();
    let relative = (hi - lo) * 1e-3;
    map.quantization_step()
        .map_or(relative, |step| relative.max(step / 2.0))
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn legacy_positional_heights() {
        let cli = Cli::try_parse_from(["hmterrain", "a.png", "b.obj", "-200", "200"]).unwrap();
        assert_eq!(
            (cli.legacy_min, cli.legacy_max),
            (Some(-200.0), Some(200.0))
        );
        let cli =
            Cli::try_parse_from(["hmterrain", "a.png", "b.obj", "--min-height", "-5"]).unwrap();
        assert_eq!(cli.min_height, -5.0);
        assert!(
            Cli::try_parse_from(["hmterrain", "a.png", "b.obj", "-1", "--min-height", "2"])
                .is_err()
        );
    }
}
