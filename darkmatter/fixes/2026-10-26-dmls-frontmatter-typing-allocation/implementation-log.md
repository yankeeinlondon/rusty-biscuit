---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-pipeline/darkmatter/fixes/2026-10-26-dmls-frontmatter-typing-allocation/spec.md"
plan: "darkmatter/fixes/2026-10-26-dmls-frontmatter-typing-allocation/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
    - darkmatter/dmls/tests/l1/frontmatter_response_contracts.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/fixtures/response_contracts/a3_completions.json
    - darkmatter/dmls/tests/fixtures/response_contracts/a3_diagnostics.json
    - darkmatter/dmls/tests/fixtures/response_contracts/a3_fault_interpolation_hovers.json
    - darkmatter/dmls/tests/fixtures/response_contracts/a3_fault_item_hovers.json
    - darkmatter/dmls/tests/fixtures/response_contracts/a3_fault_shape_hovers.json
    - darkmatter/dmls/tests/fixtures/response_contracts/a3_lifecycle_faults.json
    - darkmatter/dmls/tests/fixtures/response_contracts/a3_lifecycle_key_hovers.json
    - darkmatter/dmls/tests/fixtures/response_contracts/a3_lifecycle_value_hovers.json
    - darkmatter/dmls/tests/fixtures/response_contracts/a3_loop_hovers.json
    - darkmatter/dmls/tests/fixtures/response_contracts/a3_navigation.json
    - darkmatter/dmls/tests/fixtures/response_contracts/a3_plain_and_silent_hovers.json
    - darkmatter/dmls/tests/fixtures/response_contracts/a3_property_hovers.json
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
    - dmls
human_review: false
message_to_agent: |-
    Phase 1 pinned the editor responses on the frozen A3 document in
    darkmatter/dmls/tests/l1/frontmatter_response_contracts.rs, against JSON
    goldens in darkmatter/dmls/tests/fixtures/response_contracts/. These twelve
    tests must keep passing UNCHANGED through phases 2-4; never regenerate the
    goldens (DMLS_UPDATE_GOLDEN=1) to make a failure go away, because a diff
    there is exactly the behavior change this fix must not make. The pre-change
    A3 bench numbers are in the log and in baseline-a3.tsv (expression hover
    144,793,776 B, verb hover 154,998,355 B at a3x1-warm). Phase 2 replaces the
    meaning of SHAPE_MATERIALIZATIONS, which
    sequence_tests.rs::expression_pass_materializes_each_distinct_ancestor_shape_once
    still asserts. Note: another process on this host committed in-progress
    files of this worktree during phase 1; check git log before assuming what
    is committed.
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

### Response contracts

Existing assertions reused as-is (no change needed; they already pin behavior
later phases must keep):

- `dmls/src/providers/frontmatter/sequence_tests.rs`: typed descent through
  array indices, crossing modes (`union_arms_are_chosen_by_array_crossing`,
  `transparent_crossing_serves_key_only_authoring_ancestry`), discriminated
  arm selection inside items, shipped Claudine predicate typing, list-item
  hover/completion/navigation boundaries, and list-item expression
  diagnostics. `expression_pass_materializes_each_distinct_ancestor_shape_once`
  asserts the old `SHAPE_MATERIALIZATIONS` meaning that Phase 2 replaces.
- `dmls/tests/l1/lifecycle_event_views.rs` (per-event binding views in
  diagnostics, hover, completion) and `lifecycle_host_profile.rs` (host
  profile call diagnostics by line).
- `recursive_schema_editor.rs` and `union_item_editor.rs` (recursive
  definitions and union items through a real LSP session).

What was missing: an exact pin of every response on the large lifecycle
document the allocation fix targets, and of definition/document-link
output on it (the consumer split Phase 3 makes). Added
`dmls/tests/l1/frontmatter_response_contracts.rs` (declared in
`tests/l1/main.rs`), which drives the provider registry in-process exactly
as the A3 bench does (frozen document and frozen Claudine bundle, read
through `include_str!` inside each test), and compares portable JSON
against goldens in `dmls/tests/fixtures/response_contracts/`
(regenerate with `DMLS_UPDATE_GOLDEN=1` only for an intended change):

| Test | Pins |
| ---- | ---- |
| `a3_lifecycle_value_hovers_are_unchanged` | the bench's verb (`ensure_file`) and expression (`file_exists(`) hover positions, and a guard's expression value |
| `a3_lifecycle_key_hovers_are_unchanged` | a guard's `when` key and its `then` key (merged-arm hover) |
| `a3_loop_hovers_are_unchanged` | the `loop` block key, its `until` expression, and a `break` item's `reason` key |
| `a3_property_hovers_are_unchanged` | a schema-typed `file` key and its expression value |
| `a3_plain_and_silent_hovers_are_unchanged` | plain and empty-string values, and three positions that must stay silent (item marker, comment, body) |
| `a3_completions_are_unchanged` | 5 positions; every item's label, kind, detail, sort/filter text, insert text and edits, in order (documentation prose excluded) |
| `a3_diagnostics_are_unchanged` | every diagnostic's code, range, message, severity, source, and data; a second publish on the same context is identical |
| `a3_lifecycle_fault_diagnostics_and_completions_are_unchanged` | injected lifecycle faults' diagnostics: malformed guard, unknown verb (`anyOf`), undeclared read in a verb interpolation, `err` unavailable in `initialize`, guard without `then`, blank stack item, mistyped boolean; plus completions at a blank key inside a guard item (merged-arm keys), a blank stack item (verbs), and a blank top-level line |
| `a3_fault_item_hovers_are_unchanged` | hovers on the unknown verb, the undeclared read, and the `err` read |
| `a3_fault_shape_hovers_are_unchanged` | hovers on the guard without `then` and the mistyped boolean |
| `a3_fault_interpolation_hovers_are_unchanged` | hovers inside verb and message interpolations (the lifecycle fallback path) |
| `a3_navigation_is_unchanged` | definition at 7 positions (two file values with targets on disk, file-typed key and expression, verb value, schema type, body) and the full document-link list |

Hovers are split into tests of at most three positions (five for the cheap
silent ones). The first Windows cross-check showed a 15-hover test at 7.3 s
and a 7-hover test at 7.3 s, over the L1 budget, because each pre-fix hover
types the whole document. After the split, each golden's content was checked
to be identical to the one it came from.

Requirement → test mapping for this phase: baseline bytes → `baseline-a3.tsv`
plus the table above; exact hover text/ranges → the eight `*_hovers_are_unchanged` tests; completion order and edits →
`a3_completions_are_unchanged`,
`a3_lifecycle_fault_diagnostics_and_completions_are_unchanged`;
diagnostic codes/ranges/messages/severities → `a3_diagnostics_are_unchanged`,
`a3_lifecycle_fault_diagnostics_and_completions_are_unchanged`; definition targets and document links →
`a3_navigation_is_unchanged`. The Input Robustness Matrix does not apply: no
file-format or configuration reader changed.

### Design Decisions

- **Decision:** pin responses with JSON goldens over the frozen A3 inputs,
  driven through the public `ProviderRegistry` in-process.
  - **Alternatives considered:** per-field `assert_eq!` on a handful of
    positions (misses unlisted fields and ordering); an LSP-session
    subprocess test (much slower, adds nothing for an in-process ownership
    change); goldens over the live `claudine/schemas/` (would drift with
    unrelated Claudine schema edits).
  - **Why:** the spec demands *exact* hover text, completion order/edits,
    and diagnostic fields; a golden catches any field drift, and the frozen
    bench inputs are the same document whose allocation is being fixed.
  - **Downsides:** an intended response change elsewhere (for example new
    hover prose) requires regenerating the goldens; completion
    documentation prose is deliberately left out to limit that churn.
- **Decision:** keep the Claudine bundle as a module-level `const` of
  `include_str!`s, but put the A3 document and each golden's
  `include_str!` inside its `#[test]`.
  - **Alternatives considered:** repeat the nine-file bundle in every test.
  - **Why:** the bundle is frozen and rarely edited, so the wider CI cell a
    module-level literal implies is cheap; the document and goldens stay
    narrowed to their tests.
  - **Downsides:** an edit to the frozen bundle schedules the whole `dmls::l1`
    binary instead of these twelve tests.

### Notes

- During this phase another process on the host committed this worktree's
  in-progress files (`a7dcd6bed` with the plan, log, and baseline; `6594bcae5`
  with the first version of the test file). This session made no commits.
  The remaining edits (test registration, goldens, the added fault test,
  this log) are uncommitted.
- One command of this session accidentally ran in the main checkout
  (`/Volumes/coding/personal/rusty-biscuit`) and created empty golden
  placeholders plus a `mod` line there; both were reverted immediately and
  that checkout's `git status` is clean again.

### Gates

- Scope check: `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: [dmls, test-toolkit, darkmatter]`, `change_class: package`
  (the same on the final tree), so both gates ran.
- `just lint` (darkmatter/): pass on the final tree; `cargo clippy -p dmls --all-targets -- -D warnings`: pass.
- `just test` (darkmatter/), final tree after the hover split: **12,545
  passed, 10 skipped, 0 failed** in 79 s, with no test at 5 s or longer
  (the earlier run, before the split, was 12,538 passed).
- `just check-tier-coverage darkmatter`: 0 stranded.
- `just cross-check dmls --os windows` (build-win-native), attempt 1:
  1,445 passed, but two contract tests took 7.3 s (over the 5 s budget); the
  hover tests were split (see above). Attempt 2: **1,452 passed, 9 skipped**,
  every `frontmatter_response_contracts` test at 2.2–3.8 s, so the golden
  path/URI normalization holds on native Windows.
- Dependent packages: this phase changed only `dmls` test files and test
  fixtures. No dependent package compiles or reads them, so there were no
  dependent tests to run.
- No skills or docs changed: Phase 1 changes no behavior. The darkmatter
  skill is updated in Phase 5 with the new cache ownership.
- I mistakenly started `just lint` once from the repository root, which lints
  the whole monorepo. It was stopped after it began checking other packages,
  and the darkmatter-area lint was run instead.
