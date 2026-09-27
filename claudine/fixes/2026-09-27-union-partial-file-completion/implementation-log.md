---
spec: /Volumes/coding/wt/rusty-biscuit/feat-schema-enhancement/claudine/fixes/2026-09-27-union-partial-file-completion/spec.md
plan: claudine/fixes/2026-09-27-union-partial-file-completion/plan.md
implemented_by: claude/opus
started_phase: 1
source_files_during_phase_1:
    - claudine/cli/tests/l1/level1_provided_partial_file_pty.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
    - claudine-cli
---

# Implementation Log for 2026-09-27-union-partial-file-completion (5 phases)

## Phase 1

Phase 1 covered the rulings, the R7.1 reproduction, and spikes S1–S4. The
spikes ran as four parallel read-only agents. Their findings are in the plan
under "Spike findings", and each claim the rulings depend on was checked
against the code.

### Reproduction (R7.1)

- Added `union_partial_with_templated_file_sibling_reaches_chooser` to
  `claudine/cli/tests/l1/level1_provided_partial_file_pty.rs`, plus two local
  helpers: `plan_with_union_templated_sibling` and
  `stage_recording_goose_stub`. The file is already declared in the `l1`
  binary. The test name carries no tier marker, so it runs in L1.
- It is **red on purpose**, as the plan directs. It fails with no chooser, the
  `no existing file matched reference` error, and the whole 11-line
  frontmatter block with no highlight. That shows C1 and C5 together. The full
  transcript is in the plan's "Reproduction record".
- The test uses `--goose` (the file's shared `compose_command`), not
  `--claude`. The stub records argv, and stdin when stdin is not a TTY. A
  manual run with the debug binary confirmed that the resolved spec path
  reaches the stub's argv (`run -t "<prompt>"`), so the final assertion checks
  something real.
- The body uses `{{spec}}`. `{{doc}}` in a body renders Darkmatter's reserved
  `doc` namespace (the whole document), not the frontmatter property called
  `doc`. That behavior is documented in Darkmatter (`docs/inline/interpolation.md`).

### Rulings (N1–N9)

All nine are confirmed under `yolo: true`. Amendments are recorded inline in
the plan:

- **N3:** the sequence-item indentation trap. `  - spec:` is at indent 2, but
  its sibling `doc:` is at indent 4.
- **N7:** confirmed by S2. The caller records are already reachable at
  `translate_schema_failure`.
- **N8:** amended. No sizing helper or cap exists today; every call site
  hard-codes its rows. `HeightSpec::Cells(n)` reserves `n` rows and is
  clamped only to the terminal height. I first wrote that it shrinks to fit
  its content, checked `HeightSpec::resolve`, and corrected the amendment. The
  new helper sizes by option count plus chrome, capped at the existing 8.
- **N9:** confirmed by S1.

### Key spike conclusions

- **S1:** every non-success outcome of the supplied-file pass already errors
  before provider selection. The C2 gap is caused only by C1: an empty
  pending list for a union with no selected arm.
- **Open item (S1):** `Ctrl-C` on the single-candidate confirm dialog is
  probably ignored (`read_confirm_key`). Phase 3 should check it.
- **S2:** the late error is built in `translate_schema_failure`
  (`schema/translate.rs:75`). `classify_unresolved_file_reference` is not on
  the late path.
- **S3:** the inline `run_standalone` never enters the alternate screen. The
  provider picker has **no title text**, so R7.3 must assert on provider
  option labels or on the raw-mode entry.
- **S4:** there are no insta snapshots. About 30 tests assert on excerpt
  content, and about 7 of them will change. The two Level 2 inline-mismatch
  tests use YAML anchors, which N3 treats as unsafe, so Phase 4 must decide
  how they behave.

### Gates

- `just lint` (`claudine/`): green.
- `just test --no-fail-fast` (`claudine/`): 7370 tests run, 7369 passed, 1
  failed (the intentional R7.1 red), 9 skipped.
- Plain `just test` stops at the first failure: 4866 of 7370 tests run, with
  the same single failure.
- No cross-OS runs. The only change is a `#![cfg(unix)]` PTY test in an
  existing file.

### Requirement-to-test mapping

| Requirement | Test |
|---|---|
| R7.1 (reproduce C1) | `union_partial_with_templated_file_sibling_reaches_chooser`. Red now; it asserts the fixed behavior: the chooser, the stub launch, the path in the prompt, and no NoMatch error. |
