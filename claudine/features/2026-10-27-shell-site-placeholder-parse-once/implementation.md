---
kind: implementation-record
---
# Implementation record: shell-site placeholder parse-once

## Environment

- Revision: `ed223084c` plus uncommitted working-tree changes (Phase 4 documentation and records are uncommitted; earlier phases are committed).
- Host: macOS 27.2 (Darwin), single host. No `RUSTFLAGS` or `CARGO_PROFILE_*` overrides.
- Build: `cargo bench` release/bench profile; allocation target with `--features work-counters`, Criterion target without.

## Commands

- `cargo bench --color=never -p claudine --features work-counters --bench pipeline_work_alloc -- A6`
- `cargo bench --color=never -p claudine --bench pipeline_work -- A6_shell_preparation`
- Gates: `just test` and `just lint` in `darkmatter/` and `claudine/` (all passed; 12,994 and 11,789 tests).

## Results (100 steps, 10 holes)

| Measure | Original | Now |
| --- | --- | --- |
| Expression parses | 10,000 | 6,000 (steps x holes x 6) |
| Allocation bytes | 26,709,585 | 25,588,809 |
| Allocation count | not recorded | 195,225 |
| Peak bytes | not recorded | 1,957,479 |
| Criterion time | 8.2117 ms | [7.5387, 7.5963, 7.6730] ms (median 7.5963) |

10-step row: 600 parses, 20,940 allocs, 3,088,910 bytes, peak 297,852, 969 us (single sample).

## Reading the numbers

Count reduction is exact: 4,000 fewer parses (-40%), asserted by the scaling test. The measured effects are smaller: bytes fell 4.2% and the median time 7.5%, each from a single run, not repeated, so they are indicative only. The retained spanned syntax and the one-time erased `Expr` per occurrence add allocations that partly offset the saved parses. The specification's 6.4% estimate is not claimed as achieved.

## Departures and open items

- Departures are recorded in the implementation log under `decisions_for_review`.
- No validation failures remain. One test (`sequence_task_providers::empty_hint_setter_list_uses_the_missing_provider_gate`) ran slowly under full-suite load in Phase 3; unrelated and pre-existing.
