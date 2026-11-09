# Phase 2 build-target evidence

Local evidence for the "Build target jobs" task, produced on 2026-11-09 by
`scripts/distribution/build.sh` and `scripts/distribution/inspect_archive.py` with the
settings in `.github/distribution/build-images.json`. Only text is retained
here; archives and binaries are not checked in.

Inputs: a staging tree prepared by `distribution.py stage` for
`biscuit-hash-cli-v0.1.0` (`bh`) and `darkmatter-cli-v0.1.0` (`md`, brew
channel) with fixture selections (commit `1111…` is a fixture value, so
`SOURCE.txt` carries a placeholder commit), pinned dist 0.34.0, Rust 1.98.1
from the staged `rust-toolchain.toml`. `THIRD-PARTY-NOTICES.md` in these
archives is a placeholder; the real notices come from the license task.

## Results

| Directory | Target | How | Inspection | Smoke |
| --- | --- | --- | --- | --- |
| `bh-aarch64-apple-darwin/` | aarch64-apple-darwin | native, this host | pass: minos 15.0, `/usr/lib/libiconv.2.dylib`, `/usr/lib/libSystem.B.dylib`, no rpath | `bh --version` → `bh 0.1.0`, `--help` exit 0 |
| `md-aarch64-apple-darwin/` | aarch64-apple-darwin | native, this host | pass: minos 15.0, only `/usr/lib/` and `/System/Library/Frameworks/` (Foundation, CoreText, IOKit, CoreAudio, CoreFoundation, Security), no rpath | `md --version` → `md 0.1.0`, `--help` exit 0 |
| `bh-aarch64-unknown-linux-gnu/` | aarch64-unknown-linux-gnu | pinned container, `linux/arm64` | pass: highest `GLIBC_2.34`; NEEDED `libc.so.6`, `ld-linux-aarch64.so.1`, `libgcc_s.so.1`; no RPATH/RUNPATH | clean runtime image, no network, no build tools: `--version`/`--help` exit 0; `ldd` all resolved and allowed |
| `md-aarch64-unknown-linux-gnu/` | aarch64-unknown-linux-gnu | pinned container, `linux/arm64` | pass: highest `GLIBC_2.35` (at the floor); NEEDED `libgcc_s.so.1`, `libm.so.6`, `libc.so.6`, `ld-linux-aarch64.so.1`; no PCRE2, no RPATH | same as bh |
| `bh-x86_64-apple-darwin-cross/` | x86_64-apple-darwin | **cross-built on arm64, not native evidence** | pass: x86_64 Mach-O, minos 15.0, same two system dylibs | under Rosetta only (`smoke-rosetta-…`), not native evidence |

Checkpoint "actual local `bh` and `md` archives exclude every fixture": each
`inspect-*.json` lists the archive `entries` and `executables`; both `md`
archives contain exactly `md`, `LICENSE`, `README.md`, `SOURCE.txt`, and
`THIRD-PARTY-NOTICES.md` under one `darkmatter-cli-<target>/` root, and `md`
is the only executable (no darkmatter test-helper binaries). Every
`dist-build-*.log` shows the single selection line
`building <target> target, … using cargo profile dist --package=<package>)`
and no `--workspace`, `--all-features`, or `--features`.

The `bh` logs show no `Compiling` lines: an earlier attempt in the same
staging `target/` had already compiled identical inputs (that attempt
stopped on a defect in the script's own post-build checks, since fixed), so
cargo reused the artifacts. The `md` logs are full compiles.

## Files per directory

- `build-env-<target>.json`: the record `distribution.py provenance`
  consumes (`target`, `rustc`, `image`). On this host `image_os`/
  `image_version` are `local` and `xcode_build` is `unavailable` (Command Line
  Tools only; `command_line_tools` and `sdk_version` are recorded instead).
- `dist-build-<target>.log`: dist's stderr, including the cargo output.
- `dist-manifest-<target>.json`: `dist build --print=linkage
  --output-format=json` output (also copied to
  `target/distrib/<target>-dist-manifest.json` for the global step).
- `<archive>.tar.xz.sha256`: dist's checksum sidecar.
- `inspect-<package>-<target>.json`: the inspection report.
- macOS: `otool-load-commands-…` (`otool -l`) and `otool-libraries-…`
  (`otool -L`).
- Linux: `readelf-…` (`readelf --wide --dynamic --program-headers
  --version-info`), `ldd-…` (from the runtime image), `ldd-check-…json`.
- `smoke-…txt`: host/OS, build tools found on `PATH` (empty in the Linux
  runtime image), and the `--version`/`--help` output with exit codes.

## Commands

From the repository root, with `/tmp/dist-stage` the staged tree and
`/tmp/dist-stage-linux` an `rsync -a --exclude /target/` copy of it:

```sh
# macOS, native (also inspects and smoke-tests)
DIST_BIN=/tmp/dist-bin/dist scripts/distribution/build.sh build \
  /tmp/dist-stage biscuit-hash-cli-v0.1.0 aarch64-apple-darwin bh /tmp/dist-out/aarch64-apple-darwin
DIST_BIN=/tmp/dist-bin/dist scripts/distribution/build.sh build \
  /tmp/dist-stage darkmatter-cli-v0.1.0 aarch64-apple-darwin md /tmp/dist-out/md-aarch64-apple-darwin

# Linux, in ubuntu:22.04@sha256:076e7bf8…5e11 (linux/arm64): build + readelf inspection
DIST_CONTAINER_CACHE=/tmp/dist-container-cache scripts/distribution/build.sh container \
  /tmp/dist-stage-linux biscuit-hash-cli-v0.1.0 aarch64-unknown-linux-gnu bh /tmp/dist-out/aarch64-unknown-linux-gnu
# then smoke + ldd in a clean container of the same digest (no build tools, --network none)
scripts/distribution/build.sh smoke-container \
  /tmp/dist-out/aarch64-unknown-linux-gnu/biscuit-hash-cli-aarch64-unknown-linux-gnu.tar.xz \
  biscuit-hash-cli aarch64-unknown-linux-gnu bh /tmp/dist-out/aarch64-unknown-linux-gnu
# (md: the same two commands with darkmatter-cli-v0.1.0 / darkmatter-cli / md;
#  the md container build took 7 minutes)

# x86_64 macOS cross-build (local evidence only)
DIST_ALLOW_CROSS=1 DIST_BIN=/tmp/dist-bin/dist scripts/distribution/build.sh build \
  /tmp/dist-stage biscuit-hash-cli-v0.1.0 x86_64-apple-darwin bh /tmp/dist-out/x86_64-apple-darwin
```

## Pending

- **x86_64-apple-darwin native:** pending a `macos-15-intel` runner; the
  cross-built archive above exercises the x86_64 Mach-O inspection only.
- **x86_64-unknown-linux-gnu:** pending CI infrastructure (`ubuntu-22.04`
  runner); not attempted locally because an emulated amd64 build is too slow.
- **Hosted-runner provenance:** `ImageOS`/`ImageVersion` and the Xcode build
  number are recorded only on GitHub-hosted runners.

## Integration with real notices (`integration-real-notices/`)

After the license task finished, the staged tree received the real cargo-about
0.8.4 notices for `biscuit-hash-cli` and `darkmatter-cli`, and both were rebuilt
for `aarch64-apple-darwin` with `build.sh build`. Each archive's
`THIRD-PARTY-NOTICES.md` is byte-identical to `licenses.sh` output, inspection
passes (`inspect-*.json`), and smoke passes (`smoke-*.txt`). The `md` global
step (`dist build --artifacts=global`, brew channel) produced
`darkmatter-cli-installer.sh` (with `check_glibc "2" "35"`), `darkmatter-cli.rb`,
and `sha256.sum`. `installers.py harden` applied its preflight
(`harden-md.json`) and `installers.py check-sums` verified the archive against
both dist checksum records; the installer and formula are not in dist's
`sha256.sum` (`checksums-md.json`).
