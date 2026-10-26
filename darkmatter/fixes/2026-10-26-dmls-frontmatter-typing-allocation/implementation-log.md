---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-pipeline/darkmatter/fixes/2026-10-26-dmls-frontmatter-typing-allocation/spec.md"
plan: "darkmatter/fixes/2026-10-26-dmls-frontmatter-typing-allocation/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
---

# Implementation Log for 2026-10-26-dmls-frontmatter-typing-allocation (5 phases)

## Phase 1

### Baseline A3 measurement

Taken before any source change, at commit `8aac9b7e64898cd7802405c35a4848d29905e2fa`
(branch `feat/pipeline`, working tree clean apart from fix-directory Markdown).

- **Command** (repository root): `cargo bench -p dmls --bench pipeline_work_a3 --color=never`
- **Build settings:** the default `bench` profile (inherits `release`; the root
  `Cargo.toml` overrides only `dev`, `test`, and `profiling`); no `RUSTFLAGS`
  or `CARGO_*` environment overrides; no feature flags; no
  `A3_PROVIDER_BREAKDOWN`. Toolchain `rustc 1.98.1 (48a229cea 2026-09-01)`,
  `cargo 1.98.1 (797e8a9bc 2026-08-05)`.
- **Host** (via `sniff os` / `sniff hardware`): `shazam.home`, macOS 27.2
  (Darwin 27.2.0), Apple M4 Max, 16 cores, arm64, 128 GB.
- **Input identity** (SHA-256, frozen bundle last changed in `79403b467`):
  - `darkmatter/lib/benches/fixtures/pipeline-work/implement-plan.md`
    `37ed7aa0785d27d507bb147c2972e61389f00c7e8cb72b6b3ecf9d9c3e4495de`
  - `darkmatter/lib/benches/fixtures/pipeline-work/globals.lifecycle.json`
    `3e42c2c9d66e25496880051a7a07c175feab9f4a078880c3f18174135ff395b1`
  - `darkmatter/lib/benches/fixtures/claudine-schemas/`: action `2a250d9d…`,
    claudine `9817beb7…`, globals `4a5df458…`, inline-compose `ff7d9165…`,
    lifecycle `3f3a4b61…`, linking `414a73e6…`, review `e2c5c417…`,
    sequence-types `b21f920e…`, sequence `606ca7a7…`
- **Protocol:** unchanged bench — `a3x1` and `a3x10`, each warm and cold, one
  priming loop, one counted loop (the bytes below), then `TIMED_RUNS = 5`
  timed loops for the medians. Phase order within a loop:
  **overlay → diagnostics → completion → verb hover → expression hover**, so
  diagnostics runs before both hovers.
- **Raw output:** `baseline-a3.tsv` beside this log (the bench's full TSV:
  work counters, allocations, bytes, peak live bytes, median ns). Wall time
  of the whole run: 14.8 s.

| Case | Expression hover bytes | Verb hover bytes | Diagnostics bytes |
| ---- | ---------------------: | ---------------: | ----------------: |
| a3x1-warm  | 144,793,776 | 154,998,355 |   525,453,337 |
| a3x1-cold  | 144,793,776 | 163,339,067 |   928,335,132 |
| a3x10-warm | 966,693,644 | 976,898,223 |   970,158,217 |
| a3x10-cold | 966,693,644 | 985,238,935 | 1,373,040,012 |

Supporting values from the same run: a3x10 hover peak live bytes 417,936,523;
completion 17.9 MB at every size (already size-independent); the a3x1-warm
expression hover allocated 749,516 times with a 24.3 ms median.
