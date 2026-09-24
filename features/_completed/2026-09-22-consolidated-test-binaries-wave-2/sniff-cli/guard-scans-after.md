# `spawn_site_guard` scan set, before and after (`sniff-cli`)

The guard (`sniff/cli/tests/l1/spawn_site_guard.rs`) walks every `.rs` file under
`sniff/cli/tests/` and skips its own path, `common/`, and any file whose name starts
with `level2_` or `real_`. This lists what it scans, by path under `tests/`, computed
with that exact rule over the base tree (`8255ee228`) and the moved tree. R19 changes
the self-exclusion key from `spawn_site_guard.rs` to `l1/spawn_site_guard.rs`.

## Before (`8255ee228`, key `spawn_site_guard.rs`): 6 files

- `cli.rs`
- `cli_process_fixture.rs`
- `install_interview_cli.rs`
- `install_plan.rs`
- `snapshots.rs`
- `tty.rs`

## After (key `l1/spawn_site_guard.rs`): 9 files

- `l1/cli.rs`
- `l1/cli_process_fixture.rs`
- `l1/install_interview_cli.rs`
- `l1/install_plan.rs`
- `l1/main.rs`
- `l1/snapshots.rs`
- `l1/test_layout.rs`
- `l1/tty.rs`
- `level2/main.rs`

## Difference by path

- Former files, compared without their new `<target>/` directory: identical (6 = 6).
- New files scanned: `l1/main.rs`, `l1/test_layout.rs`, `level2/main.rs`. The two roots hold only `mod` lines, and the layout gate spawns nothing, so they add no spawn site and no PATH escape.
- `level2/level2_*.rs` stay excluded by file name, and `common/` by prefix, exactly as before.

## With the old key left in place: 10 files

The only extra file is `l1/spawn_site_guard.rs`, the guard itself. It was run once that
way (key temporarily reverted, then restored): both guard tests still passed with the same
totals, because the detector's sanitizer blanks the string-literal fixtures in the guard's
own tests, and the guard has no PATH escape. So the old key fails **silently**, not loudly:
nothing today would have caught it. The repair keeps the guard's stated rule (it never scans
itself), which a future detector fixture outside a string would depend on.

## Guard output, before and after

| | Spawn sites | PATH-escape sites |
|---|---:|---:|
| Base (`sniff-cli::spawn_site_guard`) | 0 | 4: `cli.rs:17`, `cli_process_fixture.rs:177`, `cli_process_fixture.rs:182`, `install_interview_cli.rs:15` |
| Moved (`sniff-cli::l1 spawn_site_guard::`) | 0 | 4: the same four lines, each path now under `l1/` |
| Moved, old key | 0 | 4: the same |

Every row passed both `l1_*` guard tests (`--no-capture` output, macOS).
