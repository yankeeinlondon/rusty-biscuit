# Measurements

## S2: JSONC parser throughput (Phase 1 spike)

The analysis and recommendation are in [spike-s2-jsonc.md](./spike-s2-jsonc.md).

### Environment

- **Host:** macOS 27.2 (build 26B5091g) from `sw_vers`; Apple M4 Max from
  `sysctl -n machdep.cpu.brand_string`; 128 GiB RAM; on AC power.
- **Toolchain:** `rustc 1.98.1 (48a229cea 2026-09-01)`, `aarch64-apple-darwin`,
  and `cargo 1.98.1`.
- **Harness:** a throwaway binary crate at `/tmp/jsonc-spike/bench`, outside
  the repository, built with `cargo build --release` using the default
  release profile.
- **Crate versions:** `jsonc-parser` 0.33.2 (`serde` feature), `json-five`
  0.3.1, `json-strip-comments` 3.1.2, `serde` 1.0.228, and `serde_json`
  1.0.149. The `serde` and `serde_json` versions are pinned to match the
  workspace `Cargo.lock`.
- **Request shape:** `parse_to_serde_value::<BunLock>` or its equivalent into a
  typed partial struct: `lockfileVersion: u32`, `workspaces` as a
  `BTreeMap<String, {name, version}>` through a duplicate-rejecting
  `Visitor`, and `packages: IgnoredAny`.

### Input

The generator is `gen_lock(workspaces, packages)` in the harness. Its output
follows Bun 1.3.3's layout (`lockfileVersion`, `configVersion`, `workspaces`,
and `packages` tuples with integrity strings). It adds a `//` comment before
every workspace, a `/* */` comment before every package, and trailing commas in
every object and array.

| Input | Workspaces | Packages | Bytes |
|-------|-----------:|---------:|------:|
| small | 201 (including root) | 5,000 | 1,710,222 (1.63 MiB) |
| large | 201 (including root) | 50,000 | 16,475,222 (15.71 MiB) |

### Method and cache state

- The input string is generated in memory before timing starts, so no file I/O
  is timed and the page cache does not affect the numbers.
- Each candidate gets one untimed warm-up parse, which also checks that it
  returned 201 workspaces, followed by 7 timed runs with `Instant`. The table
  reports the median.
- Peak allocation comes from a counting `#[global_allocator]`: the high-water
  mark of live bytes above the pre-call baseline, from the first timed run.
  Allocation count is the number of `alloc` and `realloc` calls in that run.
- The machine was otherwise idle, apart from normal desktop load. The runs were
  single-threaded and sequential.

### Results

| Candidate | Input | Median | Min | Max | Throughput | Peak alloc | Allocations |
|-----------|-------|-------:|----:|----:|-----------:|-----------:|------------:|
| `jsonc-parser` 0.33.2 (strict opts) | 1.63 MiB | 2.40 ms | 2.32 | 9.42 | 679 MiB/s | 0.03 MiB (0.02x) | 633 |
| `jsonc-parser` 0.33.2 (default opts) | 1.63 MiB | 2.40 ms | 2.30 | 2.44 | 681 MiB/s | 0.03 MiB (0.02x) | 633 |
| `json-five` 0.3.1 | 1.63 MiB | 8.17 ms | 8.01 | 9.11 | 200 MiB/s | 11.72 MiB (7.18x) | 389,529 |
| `json-strip-comments` 3.1.2 + `serde_json` | 1.63 MiB | 2.52 ms | 2.48 | 2.58 | 648 MiB/s | 1.66 MiB (1.02x) | 635 |
| `jsonc-parser` 0.33.2 (strict opts) | 15.71 MiB | 22.61 ms | 22.22 | 22.87 | 695 MiB/s | 0.03 MiB (0.00x) | 633 |
| `jsonc-parser` 0.33.2 (default opts) | 15.71 MiB | 22.39 ms | 22.32 | 23.05 | 702 MiB/s | 0.03 MiB (0.00x) | 633 |
| `json-five` 0.3.1 | 15.71 MiB | 78.30 ms | 76.09 | 81.64 | 201 MiB/s | 149.16 MiB (9.49x) | 3,674,536 |
| `json-strip-comments` 3.1.2 + `serde_json` | 15.71 MiB | 24.16 ms | 23.66 | 24.64 | 650 MiB/s | 15.74 MiB (1.00x) | 635 |

The 9.42 ms max for `jsonc-parser` (strict) on the small input is a single
outlier. An earlier session, which differed only in using `serde` 1.0.229 and `serde_json` 1.0.151, measured a 2.37 ms max, and its medians
were within 5% of these for every candidate.

### Observations

- `jsonc-parser`'s peak allocation and allocation count stay the same from
  5,000 to 50,000 packages. They depend only on the retained workspace records
  (201 keys plus names and versions), which confirms that `packages` is scanned
  without building a tree.
- `json-five`'s peak grows with the input (7-9.5x) because it materializes a
  full AST before serde runs.
- The strict and default `ParseOptions` perform the same, so the strict
  options cost nothing.

## S3: typed parser feasibility (Phase 1 spike)

The analysis is in [spike-s3-parsers.md](./spike-s3-parsers.md#parser-cost-finding-for-phase-2-and-phase-5).
These are feasibility numbers, not the Phase 5 measurement protocol.

### Environment

- **Host:** macOS 27.2; Apple M4 Max; on AC power.
- **Toolchain:** rustc 1.98.1 (48a229cea 2026-09-01).
- **Harness:** a throwaway binary crate at `/tmp/s3-spike`, outside the
  repository, built with `cargo build --release` (default release profile).
- **Crate versions:** `serde_yaml_ng` 0.10.0, `toml` 1.1.2, and `serde_json`
  1.0.149, which are the versions Sniff resolves through `biscuit-file`.

### Input

Generated in memory; each lockfile carries 200 workspace members plus 20,000
registry packages in that format's real layout (field names taken from the S1
fixtures).

| Format | Bytes |
|---|---:|
| npm `package-lock.json` v3 | 5.9 MiB |
| pnpm `pnpm-lock.yaml` 9.0 (`importers`, `packages`, `snapshots`) | 4.4 MiB |
| Yarn Berry `yarn.lock` (metadata version 10) | 5.7 MiB |
| `Cargo.lock` v4 | 4.3 MiB |
| `uv.lock` (version 1, revision 3, with `sdist` and `wheels`) | 8.3 MiB |

### Method and cache state

In-memory input, so neither file I/O nor the page cache is timed. One untimed
warm-up parse is followed by 7 timed runs; the table gives the median. Peak
allocation is the high-water mark of live bytes above the pre-call baseline,
measured with a counting `#[global_allocator]`. Runs were single-threaded.

### Results

| Parse | Median | Peak alloc |
|---|---:|---:|
| npm typed (`BTreeMap<String, {link, resolved, workspaces}>`) | 5.8 ms | 3.4 MiB |
| npm `serde_json::Value` | 10.0 ms | 43.2 MiB |
| pnpm typed (`lockfileVersion` + key-set visitor) | 82.2 ms | 74.9 MiB |
| pnpm `serde_yaml_ng::Value` | 103.7 ms | 111.9 MiB |
| Yarn typed (map visitor, `resolution` only) | 109.6 ms | 75.6 MiB |
| Yarn `serde_yaml_ng::Value` | 129.4 ms | 93.8 MiB |
| Cargo typed (`name`, `version`, `source`) | 18.8 ms | 74.3 MiB |
| Cargo `toml::Value` | 21.5 ms | 74.3 MiB |
| uv typed (`[manifest].members` + local sources) | 45.4 ms | 181.9 MiB |
| uv `toml::Value` | 55.1 ms | 181.9 MiB |

**Reading:** `serde_json` streams, so a typed partial struct is roughly 13x
smaller at peak. `serde_yaml_ng` buffers every event and `toml` builds its own
document tree, so for YAML and TOML a typed struct saves time (15–20%) and, for
YAML, some memory, but it cannot bring the peak near the input size.

## Phase 5: production parser measurements

These numbers cover the production parsers through the public detection API.
They follow the `2026-09-20-repo-perf` recording rules: full provenance,
warm-cache labeling, serialized runs, and counters snapshotted next to the
timings. Tests assert on counters, not on these timings.

### Environment

- **Host:** macOS 27.2 (build 26B5091g); Apple M4 Max, 16 logical CPUs,
  128 GiB RAM. Other agent sessions were running (load average 6–14 during
  the timed runs), and no build or test ran in this session while timing.
- **Toolchain:** `rustc 1.98.1 (48a229cea 2026-09-01)`, `aarch64-apple-darwin`.
- **Code under test:** `sniff` 0.1.0 at `54ebd4c68`, default features. Sibling
  Phase 5 tasks had uncommitted edits in the tree during the build, but only
  to `#[cfg(test)]` code and doc comments, so the measured code matches the
  commit.
- **Harness:** a throwaway binary crate at `/tmp/lockfile-measure`, outside the
  repository, with a path dependency on `sniff/lib` and its own `[workspace]`
  table. It was built with `cargo build --release` using the default release
  profile, and it started from the workspace `Cargo.lock`, so it resolves the
  same crate versions: `serde_json` 1.0.149, `jsonc-parser` 0.33.2,
  `serde_yaml_ng` 0.10.0, `toml` 1.1.2, and `serde` 1.0.228.
- **Request shape:** `detect_repo_with_request(root,
  &RepoRequest::structure().with_lockfile_provenance(true))` ("on") against
  the same call with `false` ("off"). Each call runs under a fresh
  `PerformanceCollector`. The on−off difference isolates the lockfile read
  and parse (plus membership comparison). The off run still probes lockfile
  presence.

### Input

Each fixture is generated into a canonicalized temporary directory: 200
workspace members under `packages/pkg-NNN` (`crates/` for Cargo), each with
its own manifest, and 20,000 registry packages. Every member has 5
dependencies, and every registry package has 2. The layouts copy the S1
real-tool fixtures.

| Format | Layout imitates | Bytes |
|---|---|---:|
| npm `package-lock.json` v3 (npm workspaces, `link: true` records) | npm 11.6.4 | 7,193,023 (6.86 MiB) |
| Bun `bun.lock` (lockfileVersion 1, a `//` comment per workspace, a `/* */` comment per package, trailing commas) | Bun 1.3.3 | 4,338,113 (4.14 MiB) |
| pnpm `pnpm-lock.yaml` 9.0 (`importers`, `packages`, `snapshots`) | pnpm 9.15.9 | 4,733,938 (4.51 MiB) |
| Yarn Berry `yarn.lock` (`__metadata.version` 10, `checksum`) | Yarn 4.18.1 | 6,743,093 (6.43 MiB) |
| `uv.lock` (version 1, revision 3, `[manifest]`, `sdist`, `wheels`) | uv 0.9.5 | 12,469,161 (11.89 MiB) |
| `Cargo.lock` v4 (`source`, `checksum`, `dependencies`) | Cargo 1.98.1 | 4,827,504 (4.60 MiB) |

### Method and cache state

- Each fixture gets one untimed warm-up of both "on" and "off", which also
  asserts the corroboration status. Then come 7 timed runs, alternating "on"
  and "off". Tables give the median, with min–max in parentheses.
- **Cache state:** the files are read from disk, so read time is included,
  but the page cache is warm after the warm-up (the files were also just
  written).
- **Allocation:** a counting `#[global_allocator]` tracks all threads. Peak is
  the high-water mark of live bytes above the pre-call baseline. Retained is
  the live bytes above the baseline after the call returns, first while the
  `RepoInfo` is still held and again after it is dropped. Allocations are
  `alloc` plus `realloc` calls. These values are the medians of the 7 runs,
  and they varied little from run to run.
- **Counters:** `filesystem.repo.lockfile_reads` and
  `filesystem.repo.lockfile_parses` matched in every run.
- **Reference:** a separate in-memory parse of the same bytes into the
  generic tree (`serde_json::Value`, `serde_yaml_ng::Value`, or `toml::Value`),
  with 1 warm-up and 7 runs. It has no file I/O and no detection work.

### Results

| Format | Status (on / off) | Reads on/off | Parses on/off | On (ms) | Off (ms) | Δ (ms) | Peak on | Peak off | Peak on ÷ input | Allocations on / off |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| npm | `match` / `not_requested` | 1 / 0 | 1 / 0 | 27.17 (25.77–29.27) | 21.28 (19.80–23.57) | 5.89 | 8.77 MiB | 0.71 MiB | 1.28x | 108,506 / 44,559 |
| Bun | `match` / `not_requested` | 1 / 0 | 1 / 0 | 26.52 (25.32–27.79) | 20.65 (19.93–21.29) | 5.87 | 4.59 MiB | 0.71 MiB | 1.11x | 46,434 / 44,539 |
| pnpm | `match` / `not_requested` | 1 / 0 | 1 / 0 | 82.43 (80.48–85.94) | 20.62 (20.16–23.88) | 61.80 | 56.41 MiB | 0.71 MiB | 12.5x | 1,494,181 / 44,592 |
| Yarn | `match` / `not_requested` | 1 / 0 | 1 / 0 | 106.29 (104.36–112.57) | 25.69 (25.34–27.18) | 80.60 | 62.77 MiB | 0.80 MiB | 9.8x | 1,920,952 / 56,698 |
| uv | `match` / `not_requested` | 1 / 0 | 1 / 0 | 88.09 (85.73–105.97) | 19.05 (18.07–20.39) | 69.05 | 492.24 MiB | 0.89 MiB | 41.4x | 228,999 / 42,812 |
| Cargo | `members_present` / `not_requested` | 1 / 0 | 1 / 0 | 47.90 (47.20–53.25) | 23.92 (23.30–24.64) | 23.97 | 163.74 MiB | 0.71 MiB | 35.6x | 200,396 / 55,529 |

Retained bytes with the `RepoInfo` still held were the same for "on" and
"off" in every format: 147.0 KiB (Yarn 155.4, uv 147.6, Cargo 145.4). After
the result was dropped, 0.6 KiB ("on") and 0.5 KiB ("off") remained. No
lockfile data survives the call beyond the observation itself.

Same-bytes generic reference (in memory):

| Format | Generic tree | Median | Peak |
|---|---|---:|---:|
| npm | `serde_json::Value` | 12.06 ms | 31.02 MiB |
| pnpm | `serde_yaml_ng::Value` | 87.20 ms | 112.21 MiB |
| Yarn | `serde_yaml_ng::Value` | 99.08 ms | 95.07 MiB |
| uv | `toml::Value` | 83.82 ms | 480.18 MiB |
| Cargo | `toml::Value` | 25.39 ms | 158.98 MiB |

TOML scaling check, with the same generator at 10,000 and 40,000 registry
packages: the uv peak was 249.10 MiB (6.0 MiB input) and 978.51 MiB (23.7 MiB
input). The Cargo peak was 82.53 MiB (2.3 MiB input) and 326.17 MiB (9.2 MiB
input). Peak grows linearly with input size.

### Observations

- **JSON and JSONC stream.** The npm and Bun peaks are the file string plus
  about 1–2 MiB, 3.5x smaller than the generic npm tree. The Bun Δ (5.9 ms for
  4.1 MiB) matches S2's `jsonc-parser` rate once the file read is added. The
  npm Δ (5.9 ms) is in line with S3's 5.8 ms typed parse on a slightly smaller
  file.
- **YAML matches S3.** The pnpm (61.8 ms, 56 MiB) and Yarn (80.6 ms, 63 MiB)
  numbers are within S3's range (82.2 ms and 74.9 MiB for pnpm, 109.6 ms and
  75.6 MiB for Yarn) and below the generic reference in both time and peak.
  `serde_yaml_ng` still buffers every event, so the peak stays at about
  10–12x the input.
- **TOML peaks are larger than S3 reported.** uv reaches 41x its input
  (S3: 22x) and Cargo reaches 36x (S3: 17x). This is not a production
  regression: on the same bytes, the production peak equals the generic
  `toml::Value` peak plus the file string. That is S3's finding: `toml` builds
  its own document tree, so a typed struct cannot shrink the peak. The larger
  ratio comes from this generator's denser rows (inline tables and long
  `sdist`/`wheels` URLs for uv, and `dependencies` arrays on every Cargo
  entry). A 20,000-package `uv.lock` in this shape costs about half a GiB of
  transient memory for about 70 ms, and all of it is freed before the call
  returns.
- The Cargo Δ (24.0 ms) includes building the dependency-version index that
  shares the same single parse. The counters show one read and one parse.

## Phase 5: passive corpus pass

The CLI was built with `cargo build --release -p sniff-cli` at `54ebd4c68`
and run as `sniff --base <repo> repo structure --json` on local checkouts,
which were not modified. This pass records statuses only and is not a test
dependency. Each repository took 0.02–1.08 s. For repositories where
`repo structure --json` prints `{}` (no monorepo), the `structure` object of
`sniff --base <repo> repo --json` was read instead (marked †). A
`standalone_lockfiles` value of "—" means the field was omitted as empty.

| Repository | Layer root | Authority | Status | Reason | Paths | Extra / missing | `standalone_lockfiles` |
|---|---|---|---|---|---|---|---|
| `wt/rusty-biscuit/fix-sniff` (this repository) | `.` | `cargo-workspace` | `members_missing` | `subset_only` | `Cargo.lock` | missing `darkmatter/dmls/zed-dmls` | — |
| | `.` | `pnpm-workspaces` | `match` | — | `pnpm-lock.yaml` | — | |
| `personal/homelab` | `.` | `pnpm-workspaces` | `mismatch` | — | `pnpm-lock.yaml` | extra `app/dns` | — |
| `personal/say` | `.` | `pnpm-workspaces` | `match` | — | `pnpm-lock.yaml` | — | — |
| `personal/ta` | `.` | `cargo-workspace` | `members_present` | `subset_only` | `Cargo.lock` | — | — |
| `typescript-go` | `.` | `npm-workspaces` | `mismatch` | — | `package-lock.json` | extra `_packages/api`, `_packages/ast` | — |
| `forks/motion-plus` | `.` | `yarn-workspaces` | `unverifiable` | `unsupported_version` | `yarn.lock` | — | — |
| | `.` | `npm-workspaces` | `absent` | — | — | — | |
| `forks/tablex` | `.` | `cargo-workspace` | `members_present` | `subset_only` | `Cargo.lock` | — | — |
| | `.` | `bun-workspaces` | `unverifiable` | `no_membership_data` | `bun.lockb` | — | |
| `forks/unocss` | `.` | `pnpm-workspaces` | `match` | — | `pnpm-lock.yaml` | — | — |
| `forks/ratatui` | `.` | `cargo-workspace` | `members_present` | `subset_only` | `Cargo.lock` | — | — |
| `forks/yazi` | `.` | `cargo-workspace` | `members_present` | `subset_only` | `Cargo.lock` | — | — |
| `ai/lemmy` | `.` | `npm-workspaces` | `mismatch` | — | `package-lock.json` | extra `apps/code-search-mcp` | — |
| | `apps/diffy-mcp` | `npm-workspaces` | `match` | — | `package-lock.json` | — | |
| `ai/llama.cpp` | `examples/llama.android` | `gradle-multi-project` | `absent` | — | — | — | poetry `poetry.lock` at root: `unverifiable`, `no_membership_data` |
| `forks/wallaby-v3-demo` † | no layers | | | | | | `[]` |
| `ai/open-webui` † | no layers | | | | | | `[]` |
| `forks/akaunting` † | no layers | | | | | | composer `composer.lock` at root: `unverifiable`, `no_membership_data` |

No repository produced `unreadable`. Findings behind the unexpected rows:

- **This repository, `members_missing`: a false result.** The root
  `Cargo.toml` lists `darkmatter/dmls/zed-dmls` under `[workspace].exclude`.
  The Cargo layer still includes that package in its packages (with
  `is_excluded: true`), and corroboration compares against that list without
  dropping excluded packages. Cargo correctly writes no lock entry for it. The
  expected status is `members_present`.
  **Fixed in Phase 5:** `lockfile::cargo::compare` now skips every path with
  an excluded seed. After the fix, this repository reports `members_present`,
  and the L1 regression test
  `lockfile_isolation::a_cargo_workspace_exclude_is_not_a_missing_member`
  pins the behavior.
- **`typescript-go`, `mismatch`: a discovery gap that corroboration exposed.**
  `package.json` declares `"./_packages/*"`. Manifest discovery expands a
  `./`-prefixed glob that contains a wildcard to nothing (reproduced in a
  scratch directory: `"./pk/*"` finds no members, while `"pk/*"` finds both),
  though `"./_extension"` without a wildcard works. The lockfile records both
  members, so the extras are real members that the manifest side missed. As
  a result, the repository also reports `is_monorepo: false`.
  This predates the feature and is recorded as the unscheduled fix
  `npm-dot-slash-workspace-patterns`.
- **`personal/homelab` and `ai/lemmy`, `mismatch`: stale lockfiles.** The
  extra paths (`app/dns`, `apps/code-search-mcp`) no longer exist on disk.
- **`forks/motion-plus`, `unsupported_version`: expected.** Its `yarn.lock`
  has `__metadata.version: 5` (Yarn 3.1.0), which is outside the accepted 6,
  8, and 10.
- **`repo structure --json` prints `{}` for non-monorepo checkouts,** so their
  `standalone_lockfiles` (for example akaunting's Composer lockfile) appear
  only in the `repo --json` aggregate.
