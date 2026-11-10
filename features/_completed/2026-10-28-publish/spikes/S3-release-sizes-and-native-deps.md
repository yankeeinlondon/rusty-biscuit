---
spike: S3
spec: 2026-10-28-publish
question: What are the stripped, LTO sizes and dynamic-library dependencies of the 8 first-set CLIs, and what must a build host install?
answer: stripped + thin LTO, from 1 MB (bh) to 167 MB (claudine, Linux); claudine (both OSes) and md (Linux) are over 100 MB. cmake is no longer needed. A build host needs a C compiler, pkg-config, libasound2-dev (Linux, claudine), and libclang (macOS). Every binary except bh and bf links a system or Homebrew libpcre2-8 unless PCRE2_SYS_STATIC=1 is set
confidence: high (measured on both hosts at main a816f1447); the build times are from 16-core hosts, not hosted runners
date: 2026-11-07
---

## Answer

Each CLI was built cold and on its own, as `dist` with `precise-builds` would
build it, from a scratch clone of `main` at `a816f1447`. The profile was
temporary: `[profile.spike-dist] inherits = "release", strip = true, lto =
"thin"`. Each command was `cargo build --locked --profile spike-dist -p <pkg>
--bin <bin>`, with no wrapper and default features.

| CLI | Package | Linux x86_64 (MB) | macOS arm64 (MB) | Linux build (s) | macOS build (s) |
|---|---|---:|---:|---:|---:|
| `bh` | biscuit-hash-cli | 1.1 | 1.0 | 5 | 7 |
| `bf` | biscuit-file-cli | 6.9 | 5.9 | 15 | 21 |
| `hug` | tree-hugger-cli | 62.9 | 56.5 | 44 | 50 |
| `wt` | git-wt-cli | 63.0 | 46.8 | 142 | 172 |
| `sniff` | sniff-detection-cli | 89.2 | 65.4 | 167 | 208 |
| `md` | darkmatter-cli | **101.6** | 77.7 | 165 | 165 |
| `dmls` | darkmatter-lsp | 65.2 | 49.8 | 165 | 156 |
| `claudine` | claudine-cli | **166.7** | **136.4** | 216 | 217 |

MB here means 10^6 bytes. Exact byte counts are in Evidence.

**Over 100 MB:**

- `claudine` on both OSes.
- `md` on Linux, by 1.6 MB.

With `lto = "fat"` and `codegen-units = 1`, Linux `claudine` drops to 143.0 MB
(−14%) and `md` to 83.6 MB (−18%), but `claudine`'s build takes 655 s instead
of 216 s. Fat LTO alone does not bring `claudine` under 100 MB.

### Dynamic dependencies

| CLI | Linux (`ldd`, beyond libc/libm/libgcc_s/ld-linux) | macOS (`otool -L`, beyond system frameworks and `/usr/lib`) |
|---|---|---|
| `bh` | none | none |
| `bf` | none | none |
| `hug` | `libpcre2-8.so.0` | `/opt/homebrew/opt/pcre2/lib/libpcre2-8.0.dylib` |
| `wt` | none | `/opt/homebrew/opt/pcre2/lib/libpcre2-8.0.dylib` |
| `sniff` | `libpcre2-8.so.0` | `/opt/homebrew/opt/pcre2/lib/libpcre2-8.0.dylib` |
| `md` | `libpcre2-8.so.0` | `/opt/homebrew/opt/pcre2/lib/libpcre2-8.0.dylib` |
| `dmls` | `libpcre2-8.so.0` | `/opt/homebrew/opt/pcre2/lib/libpcre2-8.0.dylib` |
| `claudine` | `libasound.so.2`, `libpcre2-8.so.0` | `/opt/homebrew/opt/pcre2/lib/libpcre2-8.0.dylib` |

Other notes on the linked libraries:

- No binary links OpenSSL on either OS.
- The newest glibc symbol version any Linux binary requires is `GLIBC_2.39`;
  `bh` requires only `GLIBC_2.34`.
- On macOS, the system frameworks linked are CoreFoundation, Foundation,
  CoreText, IOKit, CoreAudio, Security, and libobjc. `claudine` adds
  UserNotifications, CoreLocation, and AudioToolbox. All of them ship with
  macOS.

### What a build host must install

| Requirement | Who needs it | Evidence |
|---|---|---|
| C compiler | all except `bh` and `bf` (`aws-lc-sys`, `ring`, `onig_sys`, bundled `libsqlite3-sys`, `pcre2-sys`, 17 tree-sitter grammars) | build-script list per package |
| **cmake: not needed** | none | a logging `cmake` shim first on `PATH` was called **0 times** on both hosts, for all 8 packages. `aws-lc-sys 0.40.0` builds with its `cc` builder |
| pkg-config | every package that pulls in `pcre2-sys` (all but `bh`/`bf`), and `claudine` on Linux | logging `pkg-config` shim: `libpcre2-8` from 6 packages, `alsa` from `claudine-cli` |
| `libasound2-dev` | `claudine` on Linux | `alsa-sys` ← `alsa` ← `cpal` ← `rodio` ← `playa` |
| libclang | macOS only, every package depending on `sniff` (`hug`, `wt`, `sniff`, `md`, `dmls`, `claudine`) | `bindgen` for `coreaudio-sys` ← `sniff` |
| OpenSSL dev headers: not needed | none | no `openssl-sys` build script and no `libssl` in `ldd` |

## Evidence

- **Hosts and time windows (UTC, 2026-11-07).**
  - `$BUILD_LINUX`: Debian 13, glibc 2.41, 16 vCPU, 16 GB. Builds ran
    15:53:30–16:08:51.
  - macOS host: arm64, 16 cores. Builds ran 15:55:47–16:12:27.
  - The extra fat-LTO run on the rig: 16:18:58–16:37:40.
  - Rust 1.98.1 from `rust-toolchain.toml`. `RUSTC_WRAPPER` and
    `RUSTC_WORKSPACE_WRAPPER` were empty, and the kache shims were removed
    from `PATH`.
  - Each package started from an empty `target/`. Crates were fetched once
    beforehand, so download time is not counted.
- **Linker on the rig.** The rig's `~/.cargo/config.toml` links
  x86_64-unknown-linux-gnu through `clang` with `-fuse-ld=mold`, so the Linux
  binaries are mold-linked. CI and `dist` would use the default linker. A
  small size difference is possible; it was not measured.
- **The flags took effect.** The final `rustc` call for `claudine` carried
  `-C opt-level=3 -C lto=thin -C strip=symbols`, and `file` reports every
  binary as `stripped`.
- **Exact bytes.**
  - Linux: bh 1,142,856; bf 6,855,192; hug 62,933,248; wt 62,957,776;
    sniff 89,176,704; md 101,646,848; dmls 65,187,200; claudine 166,655,784.
  - macOS: bh 979,888; bf 5,929,872; hug 56,505,936; wt 46,793,968;
    sniff 65,395,152; md 77,712,112; dmls 49,784,608; claudine 136,420,368.
- **Where the size is.** In Linux `claudine` (`size -A`), `.text` is
  88.2 MB, `.rodata` is 46.9 MB (`md`: 10.7 MB), `.rela.dyn` is 9.0 MB,
  `.data.rel.ro` is 8.2 MB, and `.eh_frame` is 8.1 MB. `claudine-cli`
  depends on `darkmatter-cli` as a library, so it contains all of `md` plus
  about 36 MB more read-only data.
- **Fat LTO** (`lto = "fat"`, `codegen-units = 1`, strip): `claudine` was
  143,012,976 bytes, built in 655 s from empty. `md` was 83,637,064 bytes in
  467 s, with the dependencies it shares with `claudine` already compiled.
- **`pcre2` path.** `pcre2-sys 0.2.10` ← `pcre2 0.2.11` ← `hyperpolyglot
  0.1.7` ← `sniff`. When `pkg-config` finds the library, `pcre2-sys` links it
  dynamically: Debian's `libpcre2-8.so.0`, or Homebrew's `pcre2 10.49` on this
  Mac.
  - **Fix verified on macOS.** Rebuilt `hug` from empty with
    `PCRE2_SYS_STATIC=1` (47 s): `otool -L` shows no pcre2 dylib, and the
    binary is 57,085,936 bytes, 0.6 MB larger.
- **Windows** was not built: it is outside this agent's hosts.

### Comparison with the Oct 3 figures (`2026-10-03-binary-releases`)

The Oct 3 spec recorded one size, "Unstripped sizes reach 120 MB
(`claudine`)", without the OS. Today `claudine` is **larger stripped with thin
LTO** (136 MB on macOS, 167 MB on Linux) than that unstripped figure. So it
has grown substantially since Oct 3. The binary itself is unchanged in kind:
the `darkmatter-cli` library dependency and the `.rodata` from embedded
content are the large parts. The Oct 3 build requirements change as follows:

| Oct 3 statement | Now |
|---|---|
| `md`, `dmls`, `sniff`, `wt` need cmake for `aws-lc-sys` | cmake is never invoked |
| `claudine` adds ALSA and OpenSSL | ALSA yes, OpenSSL gone (no `openssl-sys`, no `libssl`) |
| on macOS `sniff` pulls `coreaudio-sys`, needs libclang | still true, and `sniff` now reaches `hug`, `wt`, `md`, `dmls`, and `claudine` |
| `libasound` is `claudine`'s only dynamic system-library dependency | false: `libpcre2-8` too, and for 5 other CLIs |
| `bh` and `bf` need only a C compiler at most | true; neither runs a native build script |

## What this changes in the spec

- **Scope, "stripped, LTO release profile".** Strip plus thin LTO is not
  enough to keep `claudine` under 100 MB, so the spec's reason for the profile
  (PyPI's per-file limit) does not hold for the binary itself. S14a found the
  compressed wheel at about 76 MB, so PyPI is not blocked today. A GitHub
  Release asset has a 2 GiB limit, and Homebrew has none in this range.
  Choose:
  - thin LTO: build speed;
  - fat LTO with `codegen-units = 1`: about 15% smaller, and `claudine` takes
    3× as long to build;
  - reduce what `claudine` embeds or links.
- **Every release build needs `PCRE2_SYS_STATIC=1`.** Without it:
  - the macOS binaries depend on `/opt/homebrew/opt/pcre2/…` and fail to load
    on any Mac without Homebrew's pcre2 (Homebrew's formula would just declare
    the dependency, but the archives and installers have no way to);
  - the Linux binaries need `libpcre2-8` on the user's system, and the manylinux
    wheel has to carry a copy of it.

  Whether hosted macOS runners have Homebrew pcre2 installed (and so would
  produce the same bad link) was not determined.
- **System dependencies through dist's `dependencies` key.** Drop cmake. Keep
  `libasound2-dev` (apt) for `claudine`. pkg-config and a C compiler are
  already on hosted images; libclang is in Xcode's command-line tools.
- **The Linux installers and docs** should say that the binaries need glibc
  2.39 or newer when built on a glibc ≥ 2.39 host. The hosted `ubuntu-latest`
  label is moving to Ubuntu 26 between Oct 19 and Nov 19, 2026
  (actions/runner-images#14748), which raises that floor again. Pin the
  release runner's image or build in an older-glibc container if older
  distributions matter.

## Surprises

- **The macOS binaries are not portable as built**, because of the Homebrew
  pcre2 path. This was not known on Oct 3, and it affects 6 of the 8 CLIs
  before anything is published.
- **`md` crosses 100 MB on Linux** (101.6 MB), and `claudine` is 67% over.
  The binaries grew a lot in a month.
- These build times are on 16-core hosts. A hosted `ubuntu-latest` runner has
  4 vCPU, so expect about 3–4× longer per CLI there (an estimate, not
  measured).
