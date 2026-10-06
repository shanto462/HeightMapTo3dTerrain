## Summary

<!-- What does this pull request change, and why? Link the issue if there is one, for example "Closes #12". -->

## Type of change

- [ ] Bug fix
- [ ] New feature
- [ ] Performance improvement
- [ ] Refactor (no behavior change)
- [ ] Documentation
- [ ] CI, build, or dependencies

## How I tested it

<!-- For example: "cargo test --locked", plus a manual run like "hmterrain dem.png terrain.glb --max-height 250". -->

## Checklist

- [ ] The PR title follows [Conventional Commits](https://www.conventionalcommits.org/) (for example `feat: add FBX export`).
- [ ] `cargo fmt --all --check` passes.
- [ ] `cargo clippy --all-targets --locked -- -D warnings` passes.
- [ ] `cargo test --locked` passes.
- [ ] `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --locked` passes.
- [ ] I added or updated tests for the change.
- [ ] I updated `CHANGELOG.md` under `## [Unreleased]` if users will notice the change.
