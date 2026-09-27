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
