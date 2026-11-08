---
spike: S14a
spec: 2026-11-06-publish-more-channels
question: Does a binary linking `libasound` pass manylinux auditing as a maturin `bin` wheel?
answer: not as built; `auditwheel repair` makes it pass by vendoring libasound and libpcre2-8, giving `manylinux_2_39_x86_64` on a glibc 2.41 build host. A lower tag needs a build on an older glibc, which was not tested
confidence: high for what was run; manylinux_2_28 is not determined
date: 2026-11-07
---

## Answer

The raw maturin `bindings = "bin"` wheel for `claudine` is **not** manylinux
compliant as built. It is only `linux_x86_64`, because it needs two shared
libraries that no manylinux policy allows: `libasound.so.2` and
`libpcre2-8.so.0`. glibc is not what blocks it.

`auditwheel repair` fixes that. It copies both libraries into the wheel
(`claudine_cli.libs/`), sets the binary's RPATH to `$ORIGIN/../claudine_cli.libs`,
and tags the wheel **`manylinux_2_39_x86_64`**. PyPI accepts that tag. The
repaired wheel installs with `uv tool install` and `claudine --version` runs.
The tag is 2_39 because the newest glibc symbols the binary uses are
`GLIBC_2.39` (libc) and `GLIBC_2.38` (libm), from building on the rig's glibc
2.41.

**`manylinux_2_28` was not reached.** maturin's `--compatibility
manylinux_2_28` refuses this binary because of too-new glibc symbols. To get a
lower tag, the binary has to be built against an older glibc, for example in a
`manylinux_2_28` container with `alsa-lib-devel` installed. That was not tried.

## Evidence

Build host: `$BUILD_LINUX` (Debian 13 trixie, glibc 2.41, 16 vCPU). Tools went
into a scratch venv made with a standalone `uv 0.9.5`, because the rig has no
`python3-venv`: maturin 1.16.0, auditwheel 6.8.2, patchelf 0.19.3. The source was a
scratch clone of `main` at `a816f1447`, with S3's temporary profile
`[profile.spike-dist] inherits = "release", strip = true, lto = "thin"`.

1. `maturin build -m claudine/cli/Cargo.toml -b bin --profile spike-dist
   --compatibility linux --auditwheel skip`: 223 s cold, run 16:10–16:14 UTC.
   - It produced `claudine_cli-0.1.0-py3-none-linux_x86_64.whl` at **76,043,623
     bytes**, below PyPI's 100 MB default.
   - The wheel contains `claudine_cli-0.1.0.data/scripts/claudine` at
     166,655,784 bytes (75.9 MB compressed), byte-identical to S3's Linux
     `claudine` (`cmp` matched).
   - It also contains a 1.3 MB CycloneDX SBOM.
   - Only the `claudine` binary is packaged. The `claudine-fake-*` test bins
     need `test-fixtures`, so they stay out.
2. `auditwheel show` on the raw wheel says it is "consistent with
   linux_x86_64". The versioned symbols limit it to `manylinux_2_39_x86_64`
   (libasound `ALSA_0.9*`, libc up to `GLIBC_2.39`, libm up to `GLIBC_2.38`,
   libgcc_s up to `GCC_4.2.0`).
3. `auditwheel -v repair` ran in 6 s.
   - In the policy dump, every policy from `manylinux_2_5` to `manylinux_2_39`
     lists `libasound.so.2` and `libpcre2-8.so.0` as libraries to graft.
   - It wrote `claudine_cli-0.1.0-py3-none-manylinux_2_39_x86_64.whl` at
     75,372,442 bytes, containing:
     - `claudine_cli.libs/libasound-0faa79c7.so.2.0.0` (1.24 MB)
     - `claudine_cli.libs/libpcre2-8-8ee551e7.so.0.14.0` (0.72 MB)
     - `claudine_cli.scripts/claudine`, the patched binary
     - `claudine_cli-0.1.0.data/scripts/claudine`, which auditwheel 6.x turns
       into a 200-byte Python launcher that `os.execv`s the patched binary
4. `uv tool install ./repaired/…manylinux_2_39_x86_64.whl` into a scratch
   `UV_TOOL_DIR` worked, and `claudine --version` printed `claudine 0.1.0`
   (exit 0). `ldd` shows the binary loading both vendored libraries from
   `claudine_cli.libs`.
5. maturin's own checks (warm, using the same target):
   - `--auditwheel check` failed: "Your library requires copying the above
     external libraries" (libasound, libpcre2-8).
   - `--compatibility manylinux_2_28 --auditwheel check` failed: "not
     manylinux_2_28 compliant because of the presence of too-recent versioned
     symbols: libc … GLIBC_2.29–2.39, libm … GLIBC_2.29–2.38".
   - `--auditwheel repair` produced
     `claudine_cli-0.1.0-py3-none-manylinux_2_39_x86_64.whl` (76,818,510
     bytes). That is the same result in one step, without a separate
     auditwheel.

## What this changes in the spec

- **"Spikes needed: ALSA and manylinux"**: settled. `uv` can ship `claudine`
  with a repaired wheel, which maturin's `--auditwheel repair` produces
  directly. The real constraints are these:
  - **The tag depends on the build host's glibc.** A wheel built on the rig
    gets `manylinux_2_39`. A build on hosted `ubuntu-latest` gets the tag of
    that image's glibc (2.39 on Ubuntu 24.04, newer on the Ubuntu 26 image
    the label is moving to). Users on older distributions then cannot install
    it. For a wider tag, build the Linux wheel in a `manylinux_2_28`
    container (maturin-action's default), with `alsa-lib-devel` and
    `pcre2-devel`, or with `PCRE2_SYS_STATIC=1` (see below). Not determined
    here.
  - **The wheel carries its own copy of `libasound`.** The library loads, but
    ALSA reads its configuration (`alsa.conf`) and plugins from paths compiled
    into that copy. Audio output through the vendored copy on a
    PipeWire/PulseAudio desktop was **not** tested; only `--version` ran.
    Test audio playback before promising it.
- **The PyPI size risk** (scope item 2): the wheel is about 75–77 MB, below
  the 100 MB default, even though the binary itself is 167 MB (see S3).
  Compression saves the PyPI channel. The margin is about 23 MB, and the binary
  grew a lot since Oct 3 (S3), so the limit can still be reached.
- **The PyPI launcher is a Python shim.** After repair, the installed
  `claudine` is a Python script that `execv`s the real binary, so each run
  first starts Python. Measured: not determined.

## Surprises

- **`libpcre2-8` is the second blocker, and it affects every channel, not just
  PyPI.** `pcre2-sys` (through `sniff` → `hyperpolyglot` → `pcre2`) links the
  system `libpcre2-8` dynamically whenever `pkg-config` finds it. With
  `PCRE2_SYS_STATIC=1` it links statically (verified on macOS in S3). That
  removes the second grafted library, and the macOS Homebrew-path dependency
  that S3 found.
- The rig has no `python3-venv` and no `uv`. A standalone uv in scratch was
  needed. The scratch venv and uv were deleted afterwards.
