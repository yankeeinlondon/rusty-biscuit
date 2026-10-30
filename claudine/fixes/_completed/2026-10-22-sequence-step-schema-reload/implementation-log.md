# Implementation log: 2026-10-22-sequence-step-schema-reload

## 2026-10-22: implementation

`mask_body_events` now takes the composed source. It inserts `[]` only for
the events that source's authored frontmatter declares, or that its
overrides already carry. All three call sites pass their source: the step
in `iterate.rs`, and the two body-only task paths in `task_run.rs`.
Nothing else supplies lifecycle stacks: there is no frontmatter
inheritance, and `DefaultLifecycleEmitter` only renders. So the authored
map plus the overrides is the complete set.

## Measurements (A3)

The workload is `claudine sequence --goose --dry-run --yolo` on a
three-item sequence. It ran on the debug binary, with each
`load_effective_schema` call instrumented temporarily (the
instrumentation was removed). Same host and load (about 9.5):

| Build | Total | Cold loads |
| --- | --- | --- |
| masks every event (before) | 1.95–2.42 s | 2 (0.88–1.17 s, 0.66–0.67 s) |
| masks present events (after) | 1.18–1.59 s | 1 (0.88–1.15 s) |

All three bodies compose in both builds.

## Verification

- A1: `container::tests::only_present_events_are_masked`, which fails
  under the old mask-everything behavior.
- A2, A4: `cd claudine && just test` passed 10,626. That includes
  `body_only_steps_run_root_success_once`.
- The full `claudine-cli` L1 suite passed 4,721 on the idle Linux host
  (`just cross-check`), with no test over 5 s.
- `cargo clippy -p claudine-cli --all-targets -D warnings`: clean.
