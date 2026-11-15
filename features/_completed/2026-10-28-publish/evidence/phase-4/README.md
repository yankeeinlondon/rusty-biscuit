# Phase 4 runtime verification evidence

Local evidence for the "Verify clean installs" task, produced on 2026-11-09
by `scripts/distribution/verify.sh` (which runs `verify_binstall.py
fixture-check` and `verify_runtime.py`) over freshly built `biscuit-hash-cli`
(`bh`) outputs. Only text is kept here. Archives, binaries, and the fixture
release are not checked in. Nothing was published, and `docs/deployment.yaml`
was not changed.

## Host

- macOS 27.2 on an Apple M1 Max (arm64), with Command Line Tools and Homebrew
  installed. Every macOS result from this host is therefore
  `native-build-host` evidence, never `native-clean-runtime`.
- Docker engine (ArcBox, Linux 6.18, `aarch64`). `linux/arm64` containers
  therefore run natively. No x86_64 Linux host, x86_64 macOS host, clean
  macOS 15 VM, or WSL2 host was available.

## Inputs (`build/`)

The inputs were the same staged trees as Phase 2 (`/tmp/dist-stage`, plus its
Linux copy, which received the real cargo-about notices first), built with
pinned dist 0.34.0 (`/tmp/dist-bin/dist`) and Rust 1.98.1. The steps mirror
the release workflow's Package job.

```sh
DIST_BIN=/tmp/dist-bin/dist scripts/distribution/build.sh build \
  /tmp/dist-stage biscuit-hash-cli-v0.1.0 aarch64-apple-darwin bh /tmp/phase4-build/aarch64-apple-darwin
DIST_CONTAINER_CACHE=/tmp/dist-container-cache scripts/distribution/build.sh container \
  /tmp/dist-stage-linux biscuit-hash-cli-v0.1.0 aarch64-unknown-linux-gnu bh /tmp/phase4-build/aarch64-unknown-linux-gnu
# both targets' manifests, archives, and .sha256 files copied into /tmp/dist-stage/target/distrib, then:
(cd /tmp/dist-stage && /tmp/dist-bin/dist build --tag=biscuit-hash-cli-v0.1.0 --artifacts=global --output-format=json)
scripts/distribution/installers.py harden  /tmp/phase4-assets/biscuit-hash-cli-installer.sh --output /tmp/phase4-assets/biscuit-hash-cli-installer.sh
scripts/distribution/installers.py check-sums /tmp/phase4-assets
```

- Both archive inspections pass (`inspect-*.json`).
- `check-sums` verified both archives against `sha256.sum` and their
  `.sha256` files (`checksums-bh.json`).
- The hardened installer embeds the SHA-256 of both archives.
  `installer-vs-fixture.diff` compares it with the committed
  `fixtures/biscuit-hash-cli-installer.sh`.
- The Linux build log has no `Compiling` lines: only the notices file changed
  since Phase 2, so cargo reused the compiled artifacts.

## Verification run (`verify/`)

```sh
PYTHON=/tmp/dist-venv/bin/python scripts/distribution/verify.sh /tmp/phase4-verify --artifacts /tmp/phase4-assets
```

Exit 0. The result is **incomplete**: 6 passed, 0 failed, 10 pending, and 2
disabled. `verify/README.md` and `verify/report.json` are the combined
report. `verify/runtime/<target>/<route>.txt` are the transcripts: OS/CPU,
`ldd --version`, the downloader package versions, listings of the install
root, HOME, and TMPDIR after each run, and every command with its output and
exit status. `verify/runtime/runtime-report.json` also records
`DYLD_PRINT_LIBRARIES` loads (macOS) and parsed `ldd` rows (Linux).

| Target | Route | Evidence class | Result |
| --- | --- | --- | --- |
| aarch64-apple-darwin | archive | native-build-host | pass: exactly `bh` plus the four notice files; `bh 0.1.0`; `--help` exit 0; dyld loaded only `/usr/lib/` and `/System/Library/` |
| aarch64-apple-darwin | installer | native-build-host | pass: install root holds only `bh`; HOME untouched; dist's own SHA-256 check ran (`/sbin/sha256sum`) |
| aarch64-apple-darwin | clean-runtime | pending-infrastructure | no clean macOS 15 VM |
| x86_64-apple-darwin | archive, installer | pending-infrastructure | no native x86_64 macOS host; Rosetta is emulation and was not used |
| aarch64-unknown-linux-gnu | glibc-refusal | native-clean-runtime | pass: Ubuntu 20.04 (glibc 2.31) refused with `requires glibc 2.35 or newer; found glibc 2.31`, exit 1; install root, HOME, and TMPDIR stayed empty |
| aarch64-unknown-linux-gnu | archive | native-clean-runtime | pass: pinned 22.04 image, no packages added, `--network none`; `ldd`: `libc.so.6`, `ld-linux-aarch64.so.1`, `libgcc_s.so.1`, all resolved and allowed |
| aarch64-unknown-linux-gnu | installer | native-clean-runtime | pass: pinned 22.04 image plus `curl` and `xz-utils`, `--network none`, archive read from `file:///artifacts`; root holds only `bh`; no profile written |
| aarch64-unknown-linux-gnu | wsl2-archive, wsl2-installer | pending-infrastructure | no WSL2 host (WSL2 is never counted as native Windows) |
| x86_64-unknown-linux-gnu | glibc-refusal, archive, installer | pending-infrastructure | no native x86_64 Linux host; an emulated amd64 container is not runtime evidence |
| x86_64-unknown-linux-gnu | wsl2-archive, wsl2-installer | pending-infrastructure | no WSL2 host |
| * | channel:brew, channel:binstall | disabled | `docs/deployment.yaml` does not enable either channel for `biscuit-hash-cli` |
| * | binstall fixture-check | native-build-host | pass: cargo-binstall 1.26.2, downloaded because the local copy is 1.25.3, and checked against the SHA-256 pinned in `setup-dist/action.yml` |

Archive sizes (release information only; there is no size threshold):
`biscuit-hash-cli-aarch64-apple-darwin.tar.xz` 344,728 bytes, SHA-256
`44bf4b3f…f0cb`; `biscuit-hash-cli-aarch64-unknown-linux-gnu.tar.xz` 350,416
bytes, SHA-256 `17a55aba…d4d4`. Each digest is the one in dist's `.sha256`
file, verified with `shasum -a 256 -c`.

## Notes

- dist writes each `.sha256` file with a trailing blank line. `shasum -c`
  therefore warns `1 line is improperly formatted` but still verifies the
  file (exit 0). The warning is kept in `shasum_check`.
- A downloader is required before the preflight runs: dist's installer calls
  `downloader --check` first. A machine with neither `curl` nor `wget`
  therefore gets "need curl or wget" instead of the glibc refusal. Either way
  nothing is written. For this reason the 20.04 refusal image also has `curl`
  and `xz-utils`.
- The two derived images, `biscuit-verify-runtime:*`, are built locally from
  the pinned digests with network access. Everything that runs in them has
  none. The apt package versions are recorded in each transcript.
- Ubuntu 20.04 is pinned in `verify_runtime.py` (`OLD_GLIBC_IMAGE`, index
  `sha256:8feb4d8c…8214`, arm64 `sha256:722ea796…f7`). It is not in
  `build-images.json` because nothing is built on it.

## Pending

- **Clean macOS 15 runtime (both CPUs):** no VM without developer tools or
  Homebrew is provisioned. Run `verify.sh` on such a VM. `verify_runtime.py`
  classifies a host with neither as `native-clean-runtime`.
- **x86_64-apple-darwin native:** needs a `macos-15-intel` runner or an Intel
  Mac with the x86_64 archive.
- **x86_64-unknown-linux-gnu:** needs an x86_64 Docker engine (for example an
  `ubuntu-22.04` runner) and the x86_64 archive.
- **WSL2 (both Linux artifacts):** run `verify.sh` inside WSL2. The
  `wsl2-archive` and `wsl2-installer` routes then run natively and are
  labeled `wsl2`.
- **Homebrew and binstall channels:** disabled by policy. Once a package is
  approved with a channel enabled, the channel is verified only against a
  live Release, so the channel stays `pending-infrastructure` until then.
