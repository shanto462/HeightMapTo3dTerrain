# Security Policy

## Supported versions

Only the latest 2.x release gets security fixes.

| Version | Supported |
| ------- | --------- |
| 2.x     | Yes       |
| < 2.0   | No        |

## Reporting a vulnerability

Please do not report security vulnerabilities in public issues, pull requests, or discussions.

Report them privately with GitHub private vulnerability reporting:

https://github.com/shanto462/HeightMapTo3dTerrain/security/advisories/new

Please include:

- The affected version (`hmterrain --version`).
- What the problem is and what an attacker could do with it.
- Steps to reproduce, for example the command you ran and a crafted input file.

## What to expect

- I will confirm that I received your report within 7 days.
- I will keep you updated while I investigate and work on a fix.
- When a fix is released, I will publish a GitHub security advisory and credit you, unless you ask me not to.

## Verifying release binaries

Every release includes a `SHA256SUMS` file and a signed build provenance attestation. You can check a downloaded archive with the GitHub CLI:

```sh
gh attestation verify hmterrain-<version>-<target>.tar.gz --repo shanto462/HeightMapTo3dTerrain
```
