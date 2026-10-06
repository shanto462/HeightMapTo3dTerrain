# Contributing

Thanks for your interest in improving HeightMapTo3dTerrain. Bug reports, feature ideas, and pull requests are all welcome.

By taking part in this project you agree to follow the [Code of Conduct](CODE_OF_CONDUCT.md). To report a security problem, follow [SECURITY.md](SECURITY.md) and do not open a public issue.

## Development setup

1. Install Rust with [rustup](https://rustup.rs/). The stable toolchain is enough. The minimum supported Rust version (MSRV) is the `rust-version` field in `Cargo.toml`.
2. Make sure you have the `rustfmt` and `clippy` components:

   ```sh
   rustup component add rustfmt clippy
   ```

3. Clone the repository and build it:

   ```sh
   git clone https://github.com/shanto462/HeightMapTo3dTerrain.git
   cd HeightMapTo3dTerrain
   cargo build
   ```

4. Run the CLI from source, for example:

   ```sh
   cargo run --release -- dem.png terrain.glb --max-height 250
   ```

## Local test files

Sample heightmaps and their scripts live in `samples/` and are committed. Put
generated meshes, renders and other output in `samples/output/`, which Git
ignores. Images used by the README live in `assets/`.

## Checks to run before you open a pull request

CI runs these exact commands. Please run them locally first:

```sh
cargo fmt --all --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --locked
```

CI also runs these, which you can run if you have the tools installed:

- `cargo deny check` (dependency licenses, advisories, and sources; needs [cargo-deny](https://github.com/EmbarkStudios/cargo-deny))
- `cargo llvm-cov` (test coverage; needs [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov))
- A build and test run on the MSRV toolchain

## Commit messages

This project uses [Conventional Commits](https://www.conventionalcommits.org/). Start each commit message, and the pull request title, with a type:

- `feat:` a new feature, for example `feat: add FBX export`
- `fix:` a bug fix, for example `fix: close the mesh at the image border`
- `perf:` a performance improvement
- `refactor:` a code change with no behavior change
- `test:` tests only
- `docs:` documentation only
- `ci:` CI workflows
- `deps:` dependency updates
- `chore:` anything else

Add `!` after the type for a breaking change, for example `feat!: rename --base to --base-thickness`.

## Pull request flow

1. Fork the repository and create a branch from `main`, for example `fix/border-holes`.
2. Make your change. Add or update tests for it.
3. If users will notice the change, add a line to `CHANGELOG.md` under `## [Unreleased]`.
4. Run the checks above.
5. Open a pull request to `main` and fill in the template.
6. Wait for review. The maintainer may ask for changes.

## Branch rules

- `main` is protected. All changes go through a pull request.
- A pull request can merge only when the **`CI result`** status check is green. This one check passes only when every CI job passes (format, clippy, tests on Linux, macOS and Windows, MSRV, docs, cargo-deny, and coverage).
- Keep pull requests small and focused. One topic per pull request is easier to review.

## Releases

Releases are made by the maintainer. The maintainer bumps the version in `Cargo.toml`, moves the `## [Unreleased]` notes in `CHANGELOG.md` to a new `## [x.y.z]` section, and pushes a `vx.y.z` tag. The release workflow then builds the binaries, attests them, and publishes the GitHub Release.
