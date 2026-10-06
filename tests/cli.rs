//! End-to-end tests of the `hmterrain` binary.

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use image::{ImageBuffer, Luma};
use predicates::prelude::*;
use tempfile::TempDir;

/// Writes a 16-bit PNG with a smooth hill and returns its path.
fn hill_png(dir: &Path, size: u32) -> PathBuf {
    let path = dir.join("hill.png");
    let c = (size - 1) as f32 / 2.0;
    let image = ImageBuffer::from_fn(size, size, |x, y| {
        let d = ((x as f32 - c).powi(2) + (y as f32 - c).powi(2)) / (c * c);
        Luma([(65535.0 * (-3.0 * d).exp()) as u16])
    });
    image.save(&path).unwrap();
    path
}

fn hmterrain() -> Command {
    Command::cargo_bin("hmterrain").unwrap()
}

#[test]
fn converts_to_every_format() {
    let dir = TempDir::new().unwrap();
    let input = hill_png(dir.path(), 65);
    for ext in ["glb", "obj", "stl", "ply"] {
        let output = dir.path().join(format!("hill.{ext}"));
        hmterrain()
            .arg(&input)
            .arg(&output)
            .args(["--max-height", "50", "--max-error", "0.25"])
            .assert()
            .success()
            .stderr(predicate::str::contains("65x65 16-bit").and(predicate::str::contains(ext)));
        assert!(
            output.metadata().unwrap().len() > 100,
            "{ext} output is empty"
        );
    }
    let (doc, _, _) = gltf::import(dir.path().join("hill.glb")).unwrap();
    let bounds = doc
        .meshes()
        .next()
        .unwrap()
        .primitives()
        .next()
        .unwrap()
        .bounding_box();
    assert_eq!(bounds.min[0], -32.0);
    assert_eq!(bounds.max[0], 32.0);
    assert!(bounds.max[1] <= 50.0 && bounds.max[1] > 49.0);
}

#[test]
fn tin_is_smaller_than_grid() {
    let dir = TempDir::new().unwrap();
    let input = hill_png(dir.path(), 129);
    let size = |mode: &str| {
        let output = dir.path().join(format!("{mode}.stl"));
        hmterrain()
            .arg(&input)
            .arg(&output)
            .args(["--mode", mode, "-q"])
            .assert()
            .success()
            .stderr("");
        output.metadata().unwrap().len()
    };
    let (tin, grid) = (size("tin"), size("grid"));
    assert_eq!(grid, 84 + 50 * 2 * 128 * 128);
    assert!(tin < grid / 2, "tin {tin} bytes vs grid {grid} bytes");
}

#[test]
fn accepts_the_v1_command_line() {
    let dir = TempDir::new().unwrap();
    let input = hill_png(dir.path(), 17);
    let output = dir.path().join("out.obj");
    hmterrain()
        .arg(&input)
        .arg(&output)
        .args(["-200", "200", "--mode", "grid"])
        .assert()
        .success();
    let text = std::fs::read_to_string(&output).unwrap();
    let heights: Vec<f32> = text
        .lines()
        .filter_map(|l| l.strip_prefix("v "))
        .map(|l| l.split(' ').nth(1).unwrap().parse().unwrap())
        .collect();
    assert_eq!(heights.len(), 17 * 17);
    let lo = heights.iter().copied().fold(f32::INFINITY, f32::min);
    let hi = heights.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    assert_eq!((lo, hi), (-200.0, 200.0));
}

#[test]
fn reports_clear_errors() {
    let dir = TempDir::new().unwrap();
    let input = hill_png(dir.path(), 9);

    hmterrain()
        .arg(dir.path().join("missing.png"))
        .arg(dir.path().join("out.glb"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot read heightmap"));

    hmterrain()
        .arg(&input)
        .arg(dir.path().join("out.fbx"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot tell the format"));

    hmterrain()
        .arg(&input)
        .arg(dir.path().join("out.glb"))
        .args(["--mode", "grid", "--max-error", "1"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("need --mode tin"));

    hmterrain()
        .arg(&input)
        .arg(dir.path().join("out.glb"))
        .args(["--pixel-size", "-1"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("pixel size"));

    assert!(!dir.path().join("out.glb").exists());
    assert!(!dir.path().join("out.glb.partial").exists());
}

#[test]
fn format_flag_overrides_extension() {
    let dir = TempDir::new().unwrap();
    let input = hill_png(dir.path(), 9);
    let output = dir.path().join("mesh.bin");
    hmterrain()
        .arg(&input)
        .arg(&output)
        .args(["--format", "glb", "--base", "2"])
        .assert()
        .success();
    assert!(std::fs::read(&output).unwrap().starts_with(b"glTF"));
}

#[test]
fn prints_help_and_version() {
    hmterrain()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--max-error").and(predicate::str::contains("Examples:")));
    hmterrain()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}
