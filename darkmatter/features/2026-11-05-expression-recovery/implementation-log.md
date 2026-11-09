---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-05-expression-recovery/spec.md"
plan: "darkmatter/features/2026-11-05-expression-recovery/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
packages:
    - darkmatter-lsp
source_files_during_phase_1:
    - darkmatter/dmls/tests/l1/validation_timing_log.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
    - .claude/skills/darkmatter/dmls.md
decisions_for_review:
    - |-
      How the tests make an "unreadable source file" (rule E22). The plan
      said to use a directory named `data.yaml`, and to fall back to a unit
      test that injects a failure into the data memo if the directory did
      not work. It did not work: Darkmatter resolves a directory as "no
      matching file", which is indistinguishable from a missing file, so it
      never reaches the read-error path. I chose a third option: the public
      read guard (`simplified_schema::resolve::buffers::guarded`). A guard
      that refuses the resolved path makes the real read fail with an I/O
      error (permission denied) through the normal load path, on every OS,
      with no file-permission tricks. The plan's injected-failure unit test
      is still the fallback. The alternatives were the injected-failure test
      (works, but skips the real read site) and `chmod 000` (does not work on
      Windows or as root). If the reviewer prefers the injected-failure test,
      only the Phase 4 "unreadable" cases change.
    - |-
      A fourth place that interprets data failures (rule E5). The plan
      listed three places that must recognize both a direct data failure and
      one passed up from nested composition. A fourth exists: the schema-slot
      failure code (`markdown/schemas/slots/host.rs:132`). Nested composition
      is forbidden inside a slot, so the nested form cannot reach that place
      today. I still decided it uses the same accessor as the others, so
      every reader spells "a data failure" one way. The alternative is to
      leave it matching only the direct form, which behaves the same today
      but would silently diverge if slots ever allowed nesting. Reversing
      this is a one-line change in Phase 2.
    - |-
      A failing DMLS test was fixed although this feature does not touch it.
      `validation_timing_log::the_editor_setting_overrides_the_file_threshold`
      failed in every full `just test` run (the phase's own gate) and passed
      when run alone. Under load, one editor-settings change makes the
      language server validate the open document twice: once immediately,
      and once more after the trigger registry is prepared again. The test
      expected exactly one log event per change and read the stale second
      one. I changed only the test: after each settings change it waits for
      the server to settle and checks that every new event has the right
      level. It now passes 192 of 192 runs under heavy load (it failed 46 of
      96 before). The alternative is a product change, so that a settings
      change which affects only the log threshold does not re-validate twice.
      That saves a little work but was out of scope here. If the reviewer
      wants that product change, the test helper can stay as it is.
message_to_agent: |-
  Phase 1 (verification) is done. Read the implementation log's "Rebase check",
  "Citation corrections", and "Revised rules" before coding: many line numbers in
  the plan have moved (for example LAZY_BINDINGS is now functions/mod.rs:270,
  lazy_function :934, the lifecycle executor is darkmatter/lib/src/lifecycle/executor.rs,
  not markdown/lifecycle). Key facts for Phase 2:
  (1) The slot gate's refusal is ExpressionError::SlotReadRefused (error.rs:333);
  E3 classifies it false with no extra arm. (2) E5 has a fourth data_failure()
  site: markdown/schemas/slots/host.rs:132. (3) Contract imports still become
  DataError::FileReference in data/contract.rs:284 (import_failure). (4)
  fetch_remote_text is at resolve_ctx.rs:436-455; its "Ok(None) after
  register_nested" arm (:449) is the Infrastructure case. (5) The plan's
  frontmatter `packages` key was overwritten for this phase with the packages
  Phase 1 touched (darkmatter-lsp). The feature as a whole touches darkmatter,
  darkmatter-cli, darkmatter-lsp, and claudine. (6) The worktree still holds
  uncommitted tabular-input-options review edits (bind.rs, call_check.rs, the
  load_data_options tests, and docs) that are not this feature's. Do not revert
  them.
---

# Implementation Log for 2026-11-05-expression-recovery (5 phases)

## Phase 1

Phase 1 is verification only. No library source changed. One test file
(a pre-existing load-sensitive DMLS test; see Baseline) and one skill page
changed.

Both prerequisite features have landed. `2026-11-05-dynamic-schema-types`
(implementation log line 2555) and `2026-11-05-tabular-input-options`
(implementation log line 924) both say "Implementation complete, ready for
review". The working tree still holds uncommitted review edits for
tabular-input-options (`call_binding/bind.rs`, `call_check.rs`, the
`load_data_options*` tests, and docs). The baseline below was measured over
that tree.

Paths below are relative to `darkmatter/lib/src/` unless stated otherwise.
`X/` means `markdown/compose/expression/`.

### Rebase check

**dynamic-schema-types**

- **D1 holds.** The observation log lives in the memo entry:
  `X/data/runtime.rs:32` (the hash is on the raw entry, so a hit at any layer
  records the same observation) and `:737-806` (`prepare_contract` replays a
  contract's observations and slot dependencies on a hit). A failed read is
  observed with `hash: None` (`X/data/load.rs:116-119`, the `unread` closure,
  and `:194`). The not-found probe is recorded by `record_probed_candidates`
  (`X/data/load.rs:98`). A missing import is a `MissingProbe` (`runtime.rs:868`).
- **D2 differs in naming only.** `SlotClass` is a required fact on every
  `FunctionBinding` (`X/functions/mod.rs:97`, `:154`; the registry refuses a
  binding without one at `:375`). `FunctionBinding::lazy` gives `SlotClass::Pure`
  (`:207-209`). There is no `slot_class(&SpannedExpr)` walk in `call_check.rs`.
  The walk is `markdown/schemas/slots/admission.rs::slot_class_report` (`:63`),
  and its `FunctionCall` arm visits every argument (`:125-128`). There is no
  `REMOTE_READ_FUNCTIONS` list. `markdown/compose/remote.rs::fetches_url`
  (`:436`) reads the registration's `read_fact`, so nothing is hand-kept.
  Revised E17: see "Revised rules".
- **D3 holds.** There is no `slot_gate()` method. `ResolutionContext` has a
  `slot_gate` field, checked by `authorize_slot_read` (`X/resolve_ctx.rs:248-262`).
  `load_data` calls it after resolution and before `record`, the remote fetch,
  and every memo layer (`X/data/load.rs:105-113`). A second check,
  `authorize_slot_observations` (`X/resolve_ctx.rs:309`), runs in `call_entry`
  over every observation the call made, before its result (success or failure)
  is returned (`X/functions/mod.rs:1074-1083`). The refusal is
  **`ExpressionError::SlotReadRefused { target, refusal: GateRefusal }`**
  (`X/error.rs:333`). It is none of `FileReference`, `Data`, or `RemoteRead`, so
  E3 needs no extra arm. It falls into the exhaustive `false` list.
- **D4 holds.** `SessionMode::ReferenceCheck` (`markdown/schemas/slots/session.rs:69`)
  turns a slot failure that `needs_grant` into an unresolved template, which
  the reference check defers (`session.rs:413-418`).
- **D5 holds.** `DataRuntime::memo_generation` (`X/data/runtime.rs:694`).
  `invalidate_memo` (`:687`) starts a new generation. A pinned slot snapshot
  keeps its generation and inputs (`session.rs:112-117`, `Pin`).
- **D6 holds.** `markdown/schemas/slots/codes.rs:9-41` has all five spellings
  in `SLOT_DIAGNOSTIC_CODES` (15 codes in total).
- **D7 holds.** Admission walks every argument (D2). Pre-flight target
  collection uses `visit_call_nodes` over every call node in both `?:`
  branches and every `||` operand (`markdown/schemas/slots/grants/targets.rs:1-11`,
  `:75-100`).
- **D8 holds.** The slot evaluator uses the shared `evaluate` with a fresh
  `IterationBudget` per slot (`markdown/schemas/slots/evaluator.rs:178-193`).
  `host.rs::failure_code` (`:128-135`) keeps the `ExpressionError::Data` cause
  and maps it to its `dm.data.*` code.
- **D9 holds.** `[]` yields `dm.schema.slot_empty`
  (`markdown/schemas/slots/tests.rs:426`; `simplified-schema/lib/src/errors.rs:258`).
  Pinned resolutions keep their outcome and inputs (`session.rs:112-117`).
- **D10 holds.** `X/data/load.rs:172-181`: `buffers::read` failure →
  `DataError::FileReference` with `FileRefFailure::NotFound` and
  `FileReferenceError::Io { path, source }`.
- **D11 holds.** `ExpressionError::RemoteRead` is at `X/error.rs:454`,
  `SchemaFileRead` at `:470`, and `Other` at `:481`. Contract imports still
  become `DataError::FileReference`, at `X/data/contract.rs:282-287`
  (`import_failure`, line `:284`).
- **Q21 holds.** A slot read of an absent target inside the repository is
  granted automatically as a probe
  (`markdown/schemas/slots/grants/auto.rs:121-127`; `grants/gate.rs:227-233`
  keeps it a content read once it exists). Two conditions apply: the session
  has a repository root, and the defining schema is inside it (`auto.rs:115-120`).
  For a target outside the repository, `authorize_slot_observations` refuses
  the `MissingProbe` before `try` sees the `NotFound`. `try` therefore receives
  `SlotReadRefused`, which propagates as E18 requires.

**tabular-input-options**

- **T1 holds.** `X/data/error.rs:35` (`INVALID_OPTIONS_CODE`), `:152`
  (`InvalidOptions`), `:244` (`code()`), and `:39` (listed in
  `DATA_DIAGNOSTIC_CODES`, tested at `:329-332`).
- **T2 holds.** There are three layers (`X/data/runtime.rs:452`, `:472`, `:496`)
  through one `single_flight` per key and generation (`:393-438`). A deferral
  (`RemoteNotEnabled`) is removed from the memo and never kept (`:426-436`).
- **T3 holds.** A raw local failure is `DataError::FileReference(NotFound, Io)`
  (D10). UTF-8 is decoded in the source layer, and its failure is
  `DataError::Malformed` (`X/data/parse.rs:65-72`; called at `load.rs:189`).
- **T4 holds.** `load_data_type` (`X/call_check.rs:274-293`) matches
  `[file, contract] | [file, contract, _]`.
- **T5 holds.** `X/data/load.rs:81-161`. The order is: options
  (`DataOptions::from_bound`, `:85`), contract (`:90`), resolution (`:92-104`),
  slot gate, record, format (`unsupported_format` only when no `format` option
  is given, `:127-130`), applicability (`:132`), row shape (`:141-151`), remote
  fetch and host policy (`:155-163`), and then the raw read, parse, and bind.
- **T6 holds.** `require_positional_rows` raises `RowShapeConflict` for a
  headerless object-row contract (`load.rs:145-147`).
- **T7 holds.** `DataRuntimeStats` (`X/data/runtime.rs:193-205`).
- **T8 holds.** `ctx.fetch_remote_text` (which enforces policy) runs before
  `load_raw` (`load.rs:155-163`).

### Citation corrections (E3–E14)

Where nothing is listed, the citation still holds.

| Rule | Plan cites | Now |
|---|---|---|
| E2 | `functions/mod.rs:174` `LAZY_BINDINGS` | `X/functions/mod.rs:270` |
| E2 | `functions.yaml:559-565` (`or`) | holds (`darkmatter/schemas/partials/functions.yaml:559-565`) |
| E3 | `is_authoring_fatal` `:644` | `X/error.rs:656` |
| E3 | `FileRefFailure::classify` `:80-96` | `X/error.rs:80-94` |
| E4 | `contract.rs:244-249` | `X/data/contract.rs:282-287` |
| E4 | `From<Arc<DataError>>` `data/error.rs:216-225` | `X/data/error.rs:265-274` |
| E5 | `nested.rs:303-312`, raise `:311` | `markdown/compose/nested.rs:326-335`, raise `:334` |
| E5 | `nested.rs:375` | `nested.rs:392-398` (`data_failure`) |
| E5 | `report_diagnostics.rs:348` | `markdown/compose/report_diagnostics.rs:383` |
| E5 | `error.rs:656` (Data arm) | `X/error.rs:668` |
| E5 | — (missing) | also `markdown/schemas/slots/host.rs:132` (`failure_code`) |
| E6 | `report_diagnostics.rs:341-350` | `:376-390` (`with_expression_cause`) |
| E6 | `data_records` `:394-399` | `:462` |
| E6 | `blocks.rs:320` | holds (`markdown/errors/blocks.rs:320`) |
| E6 | Claudine `render/mod.rs:243`, `:523`, `:542` | hold |
| E7 | `remote_fetch.rs:44`, `:584`, `:610`, `:666` | `:44`; `:592`; `:618`; `get_content` `:674` |
| E7 | `remote_cache.rs:403-410`, `:416`, `:481`, `:523` | hold |
| E7 | `resolve_ctx.rs:319`, `:328` | `X/resolve_ctx.rs:440`, `:449` |
| E8 | `resolve_ctx.rs:315-334`, `:339-344`, `:331` | `:436-455`, `:459-465`, `:452` |
| E8 | `functions/fs.rs:82-93` | holds |
| E9 | `functions/mod.rs:167-180` | `X/functions/mod.rs:261-277` (catch-all `_ => OnFalsy` at `:275`) |
| E9 | `lazy_function` `:784` | `:934` |
| E9 | `expression/mod.rs:916`, `:976-1003` | `X/mod.rs:919-920`, `:979` onward |
| E9 | `scope/facts.rs:20`, `:46-51` | hold |
| E9 | `lifecycle/executor.rs:1338-1348` | `darkmatter/lib/src/lifecycle/executor.rs:1336-1360` (not under `markdown/`) |
| E9 | `call_entry` `functions/mod.rs:877-885` | `X/functions/mod.rs:1030-1046` |
| E10 | `bind.rs:315`, `:526`, `:562` | `X/call_binding/bind.rs:318`, `:529`, `:565` |
| E10 | `signature.rs:190`, `:211` | hold |
| E11 | `executor.rs:465` | holds |
| E11 | `lifecycle/error.rs:629-650` | `darkmatter/lib/src/lifecycle/error.rs:632-649` |
| E11 | Claudine `executor.rs:20`, `:453-462` | hold |
| E12 | `data/error.rs:184` (`is_deferral`) | `X/data/error.rs:229` |
| E12 | `data/runtime.rs:299-305` | `X/data/runtime.rs:426-436` |
| E12 | `reference/graph.rs:1154` | holds |
| E13 | `call_check.rs:178-180`, `:222`, `:236` | `X/call_check.rs:215-216`, `:259`, `:274` |
| E13 | `completion.rs:252` | `simplified-schema/lib/src/completion.rs:257` |
| E13 | `scope/shape.rs:296`, `:108`; `scope/mod.rs:207` | hold |
| E14 | `absence.rs:153-161` | holds |

### Revised rules

- **E5 (revised).** Use `ExpressionError::data_failure()` at four projection
  sites, not three. The fourth is `markdown/schemas/slots/host.rs:132`
  (`failure_code`). `as_markdown` is `SlotClass::Rejected`, so `NestedData`
  cannot reach a slot today. Using the accessor there anyway keeps one
  spelling of "a data failure" and costs nothing.
- **E17 (revised evidence; behavior unchanged).** The slot-class walk is
  `markdown/schemas/slots/admission.rs::slot_class_report`. Its
  `FunctionCall` arm already visits every argument, so registering
  `FunctionBinding::lazy("try")` (which is `SlotClass::Pure`, with no read
  fact) is enough and the walk needs no change. One consequence to know:
  pre-flight's `grants/targets.rs::is_pure` (`:158-178`) treats a call whose
  class is `Pure` and whose arguments are all pure as foldable. A
  `try(<pure>, <pure>)` call is then evaluated during pre-flight, which reads
  nothing. A `try` holding a read-side call is not pure and is collected
  through `visit_call_nodes` as before.
- **E22 (revised approach).** The directory fixture does not work (see
  "Scratch fixture" below). The unreadable source is made with
  `simplified_schema::resolve::buffers::guarded`, a public seam
  (`simplified-schema/lib/src/resolve/buffers.rs:104-145`). A guard that
  refuses the resolved path makes `buffers::read` fail with
  `io::ErrorKind::PermissionDenied` ("the host refused the read") at
  `load.rs:172`. That is a real `DataError::FileReference(NotFound, Io)`,
  reached through the normal load path on every OS. The guard is a
  thread-local stack, so the test must evaluate on the calling thread (an
  in-process `evaluate` or a single-document body interpolation). If it does
  not reach the read, use the plan's fallback: a unit test beside
  `X/data/runtime.rs` that seeds the raw layer.

### Scratch fixture (E22)

In a temp directory (not in the repo), `data.yaml` was created as a
directory, and `{{ as_json(load_data("data.yaml", "string[]")) }}` was
composed with `target/debug/md compose doc.md --output json`. The result is
`outcome: failure`, `reason: "file not found"`, and `failure: "no-match"`. A
genuinely missing `data.yaml` gives the same output. Resolution refuses a
directory before any read, so the call fails with a clean miss
(`FileReference(NotFound)` with `source: None`) and never reaches the
`Io`-cause path. The fixture would have tested the clean-miss row twice.

### Claudine touch points

`rg "ShortCircuit|lazy_function|ExpressionError::(FileReference|Data|RemoteRead)" claudine`
matches the following source sites:

- `claudine/lib/src/composition/error/render/mod.rs:243`, `:523`, `:542` (E6
  renderer)
- `claudine/lib/src/composition/error/tests.rs:1481` (a test that constructs
  `ExpressionError::FileReference`)

Every other match is in closed feature documents and a spike under
`claudine/features/_completed/`, none of them compiled. Claudine is therefore
touched only by E6 and by tests (E11). Confirmed.

### Baseline

The baseline was measured on macOS, over the tree described at the top of
this phase.

| Gate | Result |
|---|---|
| `darkmatter/` `just test` (runs 1 and 2) | **failed**: 15745/15746 passed, 1 failed (`darkmatter-lsp::l1 validation_timing_log::the_editor_setting_overrides_the_file_threshold`), 152 left unrun by fail-fast, 10 skipped |
| `darkmatter/` `just test` (run 3, after the test fix below) | **pass**: 15898 run, 15898 passed (14 slow), 10 skipped |
| `darkmatter/` `just lint` | **pass** (zed-dmls compiles for wasm32-wasip2) |
| `claudine/` `just test-library` | **pass**: 6500 run, 6500 passed (13 slow), 0 skipped |
| `claudine/` `just lint` | **pass** |
| `just check-tier-coverage darkmatter` | **pass**: 0 stranded |

**The one pre-existing failure.** The failing test is a DMLS test that this
feature's files do not touch. It passed when run alone, with the area's
unified features (`just test validation_timing_log`), and in the DMLS package
suite. It failed in both full-area runs, near the end of the run when load
peaks. Running it 96 times at 48-way parallelism reproduced it in 46 of 96
runs. With a widened assertion message, the cause showed: after the `-1`
settings change, the newest event was a `DEBUG` validation logged under the
*previous* threshold (`3_600_000`).

`router.rs::update_config` (`dmls/src/router.rs:1307`) drops the cached
document contexts on every configuration change. The document is then
refreshed immediately, its trigger registry is prepared again, and installing
that registry refreshes it once more. Under load both refreshes validate, so
one change can log two events. The test assumed exactly one event per change
and read the stale extra one at the next step.

The fix is in the test, not the product. `settled_events`
(`dmls/tests/l1/validation_timing_log.rs`) is used after each settings
change. It calls `flush_server()` (every message handled and every registry
job finished), drains the document's buffered diagnostics, and asserts that
at least one new event was logged and that every new event has the level the
threshold now selects. The document-open step still uses the exact
one-event check. After the fix, 192 runs under the same load all passed.
DMLS doing a second, redundant validation per configuration change is a
small efficiency issue. It is recorded under `decisions_for_review` and left
alone.

### Departures

- E22 uses the public read-guard seam instead of a directory or the
  runtime-seeding unit test. The runtime-seeding unit test stays as the
  fallback.
- E5 adds a fourth projection site (`slots/host.rs:132`).

### Decisions applied

E1–E24 as written, with the revisions above to E5, E17 (evidence only), and
E22.

### Scope calculator and gates

`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
reports `packages: [darkmatter, darkmatter-cli, darkmatter-lsp, test-toolkit]`
and `change_class: package`. The working tree also holds the uncommitted
tabular-input-options review edits, so not every one of these is from this
phase. Either way the gates were not skippable, and all of them ran (see
Baseline): `darkmatter/` `just test` and `just lint` pass, `claudine/`
`just test-library` and `just lint` pass, and `just check-tier-coverage
darkmatter` shows 0 stranded.

No dependent-package run was needed: the only source change is a DMLS test
function, which no other package compiles. No cross-OS check was run: the
change is a test-synchronization fix with no path, process, or platform code.

### Requirement-to-test mapping (Phase 1)

Phase 1 adds no behavior, so no feature test was added. The one test change
is a regression fix for the load race. The existing test
`validation_timing_log::the_editor_setting_overrides_the_file_threshold` is
the regression test: before the fix it failed 46 of 96 runs under 48-way
parallel load, and after the fix it passed 192 of 192. Its tier stays L1
(F2: an in-process LSP session through the public router).

Phase 1 is complete: every assumption has a verdict, every difference has a
revised rule, and the baseline is recorded and green.
