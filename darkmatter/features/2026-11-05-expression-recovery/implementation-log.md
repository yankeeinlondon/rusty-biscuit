---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-05-expression-recovery/spec.md"
plan: "darkmatter/features/2026-11-05-expression-recovery/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
packages:
    - darkmatter-lsp
    - darkmatter
    - claudine
    - claudine-cli
source_files_during_phase_1:
    - darkmatter/dmls/tests/l1/validation_timing_log.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
    - .claude/skills/darkmatter/dmls.md
source_files_during_phase_2:
    - darkmatter/lib/src/markdown/compose/expression/error.rs
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/data/error.rs
    - darkmatter/lib/src/markdown/compose/expression/data/contract.rs
    - darkmatter/lib/src/markdown/compose/expression/data/load.rs
    - darkmatter/lib/src/markdown/compose/expression/data/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/data/provenance_tests.rs
    - darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/fs.rs
    - darkmatter/lib/src/markdown/compose/nested.rs
    - darkmatter/lib/src/markdown/compose/remote_fetch.rs
    - darkmatter/lib/src/markdown/compose/remote_fetch/error.rs
    - darkmatter/lib/src/markdown/compose/cache/remote_cache.rs
    - darkmatter/lib/src/markdown/compose/report_diagnostics.rs
    - darkmatter/lib/src/markdown/errors/blocks.rs
    - darkmatter/lib/src/markdown/schemas/slots/host.rs
    - darkmatter/lib/tests/l1/load_data_diagnostics.rs
    - claudine/lib/src/composition/error/render/mod.rs
    - claudine/lib/src/composition/error/tests.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/darkmatter/expressions.md
    - .claude/skills/darkmatter/remote-cache.md
    - .claude/skills/darkmatter/requests.md
source_files_during_phase_3:
    - darkmatter/schemas/partials/functions.yaml
    - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/bind.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/facts.rs
    - darkmatter/lib/src/markdown/compose/expression/absence.rs
    - darkmatter/lib/src/markdown/compose/expression/call_check.rs
    - darkmatter/lib/src/lifecycle/executor.rs
    - darkmatter/lib/src/lifecycle/executor/tests.rs
    - darkmatter/lib/tests/l1/load_data_passive.rs
    - darkmatter/lib/tests/l1/design_time_call_check.rs
    - claudine/schemas/action.yaml
    - claudine/schemas/lifecycle.yaml
    - claudine/cli/tests/l1/snapshots/l1__context_command__context_expressions_function_order_snapshot.snap
    - darkmatter/lib/tests/l1/function_binding_registry.rs
    - darkmatter/schemas/partials/lifecycle.yaml
    - darkmatter/schemas/partials/md-lifecycle.yaml
    - darkmatter/lib/tests/l1/lifecycle_schema_generation.rs
    - darkmatter/dmls/tests/fixtures/response_contracts/a3_completions.json
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/embedded-darkmatter-base.json
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/darkmatter-schemas-darkmatter-schema-body.json
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/darkmatter-schemas-partials-fm.json
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-baseline-inline.json
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-baseline-root-union.digest
    - darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-baseline-schema-file.digest
docs_updated_during_phase_3:
    - darkmatter/docs/topics/darkmatter-expressions.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/expressions.md
    - .claude/skills/os/macos.md
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
    - |-
      How a contract import that cannot be read is graded (Phase 2, rules E4
      and E5). The plan said the new contract-import failure lives inside the
      data-failure variant, which is always fatal. Before this change an
      import failure was a plain file-reference failure, which is fatal for a
      missing or malformed reference but not for a remote reference in a run
      with remote reads off. I kept the old grading exactly: an import is
      fatal or not by the same file-reference rule as before, so unguarded
      behavior does not change. The alternative (always fatal) differs only
      for a remote URL used as an import, which never resolves anyway; if the
      reviewer prefers it, one match arm in `is_authoring_fatal` changes.
    - |-
      Claudine's error codes do not look through callback wrappers (Phase 2,
      rule E6). The plan said to use the new "file-reference diagnostic" view
      at Claudine's three render sites. That view looks through callback
      wrappers (a failure inside `map(...)`), but those sites never did: a
      file failure inside a callback has always been
      `composition.expression_invalid`, not
      `composition.invalid_file_reference`. To keep codes byte-identical I
      added a small helper that uses the view only when the failure is not
      wrapped in a callback. The alternative gives callback-wrapped file
      failures the file-reference code and detail, a visible change to
      Claudine output; it is a one-line change in the helper if wanted.
    - |-
      Two remote start-up failures have no test (Phase 2, rule E7). The plan
      asked for tests that the remote runtime's own start-up failures (its
      async runtime or HTTP client failing to build) are typed
      "infrastructure". Those failures cannot be caused from a test without
      a new test-only hook in production code, so they are set by explicit
      constructor calls and checked by reading. A test does cover a client
      build failure as reported by the fetch library. Adding a test-only seam
      to force them would need a small production change.
    - |-
      Recovery does not count as handling a missing value (Phase 3). The
      plan declares both `try` parameters as "any value or null", so that a
      `null` result counts as a success. But Darkmatter's warning rules treat
      any parameter that accepts null as one that handles a missing variable.
      That would have hidden the "undeclared property" warning for something
      like `try(titel, "x")`. The spec says `try` must not hide those
      warnings. I kept the plan's parameter types and added one exception in
      the warning rules: an operand of `try` never counts as handling a
      missing value. The alternative was to declare the parameters without
      null, but then `try(null, 7)` would fail to bind, which breaks the
      plan's own example. Undoing this means deleting `is_recovery` and its
      call in `absence.rs`, plus one test.
    - |-
      The generated function table was regenerated in Phase 3, not Phase 5.
      The plan leaves generated pages to the docs phase. But adding `try` to
      the function catalog makes an existing test fail until the generated
      table in `docs/topics/darkmatter-expressions.md` matches. Phase 3's
      checks must pass, so I ran `just regen-expr-doc` now. It added one row,
      which uses the catalog description as written. Phase 5 can still
      reword the description and regenerate.
message_to_agent: |-
  Phase 3 (lazy recovery and passive types) is done. try(expr, fallback) is
  registered (functions.yaml under Logical; FunctionBinding::lazy("try");
  LazyKind::{All, Any, Recover} replaced ShortCircuit) and evaluates in
  expression/mod.rs::evaluate_recovery and lifecycle/executor.rs::call_recovery.
  call_check.rs::try_type does the typing with the [] exception.
  What Phase 4 should know:
  (1) absence.rs::call_argument excludes try operands from "handles absence"
  (otherwise its unknown|null params would hide undeclared-property warnings).
  Phase 4's undeclared_property_warning_inside_try_remains should pass as is.
  (2) The generated table row in docs/topics/darkmatter-expressions.md was
  regenerated (the catalog test requires it). Rerun just regen-expr-doc after
  any functions.yaml description change.
  (3) design_time_call_check.rs is behind #[cfg(feature =
  "effects-instrumentation")]; run tests with
  --features effects-instrumentation,work-counters (just test does).
  (4) In lifecycle executor tests, a relative load_data needs the fixture's
  source_path inside its temp dir; the default "doc.md" gives an empty cwd,
  whose miss is RelativeContextDirectory (correctly not recoverable). See the
  action_call helper in lifecycle/executor/tests.rs.
  (5) Tests in the expression mod.rs: TestLookup now has an optional
  ResolutionContext (context_lookup(dir)) for real read failures.
  (6) Adding try to the catalog rippled into generated and pinned artifacts,
  all updated: darkmatter/schemas/partials/lifecycle.yaml (regenerated; try is
  now a lifecycle verb), md-lifecycle.yaml's hand-kept md-unknown-action
  pattern, six recursive_schema_golden goldens, a DMLS completions golden,
  claudine/schemas/{action,lifecycle}.yaml (hand-kept verb lists), and a
  claudine-cli insta snapshot. Phase 5's description edits to functions.yaml
  need just regen-expr-doc and just regen-lifecycle-schema again (descriptions
  flow into lifecycle.yaml), then the golden blesses
  (DARKMATTER_BLESS_RECURSIVE_GOLDEN=1, DMLS_UPDATE_GOLDEN=1) if they move.
  (7) CDPATH on this host: never "cd darkmatter" relatively from inside
  darkmatter/; it lands in the main checkout. Use absolute paths. This
  worktree's darkmatter just test runs 15951 tests.
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

## Phase 2

Phase 2 makes every failure that `try` will judge typed, and leaves unguarded
behavior byte-identical. `try` itself is not registered yet (Phase 3).

Paths below are relative to `darkmatter/lib/src/markdown/` unless stated
otherwise. `X/` means `compose/expression/`.

### What changed

**Wave 3: error model and classifier.**

- `X/error.rs`: new public `RemoteReadKind { Transport, Policy,
  InvalidReference, Infrastructure }` (re-exported from `expression`);
  `ExpressionError::RemoteRead` gained `kind`; new
  `ExpressionError::NestedData(Arc<DataError>)` (displayed and graded like
  `Data`); new `is_recoverable_read_failure()` (exhaustive match, no `_` arm),
  `data_failure()`, and `file_reference_diagnostic()`. The file-reference
  fatality rule moved into a shared helper (`file_reference_is_fatal`) so a
  contract import is graded exactly as it was when it surfaced as a
  `FileReference` error.
- `X/data/error.rs`: new `DataError::ContractImport(FileReferenceDiagnostic)`
  (same `Display` as the former file-reference message, `code()` and
  `written_contract()` are `None`, never a deferral). The
  `From<Arc<DataError>>` conversion still flattens `FileReference` and keeps
  `ContractImport` inside `Data`.
- `X/data/contract.rs::import_failure` builds `ContractImport`.
- `compose/nested.rs` re-raises a child's data failure as `NestedData`, and its
  chain walk uses `data_failure()` (so a grandchild's `NestedData` is found).
- `schemas/slots/host.rs::failure_code` uses `data_failure()` (the fourth E5
  site found in Phase 1).

**Wave 4: remote producers.**

- New `compose/remote_fetch/error.rs`: `RemoteFetchFailure { kind, message }`,
  `Display` = message, `From<FetchError>` and `From<RemoteReadError>` with the
  E7 mapping, and `invalid_utf8` for the three response UTF-8 sites.
- `compose/cache/remote_cache.rs::fetch_with_cache` returns
  `Result<_, RemoteFetchFailure>`.
- `compose/remote_fetch.rs`: `FetchSlot::Failed(RemoteFetchFailure)`; runtime
  and client start-up failures are `Infrastructure`; a policy pre-check
  failure keeps its `RemoteReadError` kind; new crate-private
  `get_content_typed`; public `get_content` is now the string adapter
  (`failure.message`), so its strings are unchanged.
- `X/resolve_ctx.rs`: `RemoteTextError::Failed(RemoteFetchFailure)`; an
  unparsable URL is `InvalidReference`; the "no slot after
  `register_nested`" arm is `Infrastructure`; the policy-derived `Denied` arm
  still runs first.
- `X/functions/fs.rs::remote_text_error` fills `RemoteRead.kind`. Because Wave 4
  ran right after Wave 3 in the same session, the temporary
  `kind: Infrastructure` placeholder the plan allowed was never written.

**Wave 4: diagnostic projection.**

- `compose/report_diagnostics.rs::with_expression_cause` reads data failures
  through `data_failure()` (so `NestedData` projects like `Data`), and
  `data_records` projects `ContractImport` with the file-reference fields.
- `errors/blocks.rs::interpolation_block` renders through
  `file_reference_diagnostic()`, so an import still gets the "invalid file
  path" block.
- `claudine/lib/src/composition/error/render/mod.rs`: a new
  `interpolation_file_reference` helper is used at the three sites
  (`compose_failed_code`, and the two `detail()` arms). It does not look
  through a callback wrapper, because those sites never did; looking through
  would change a callback failure's code.

**Comment drift fixed in the same change:** `DataError::FileReference` no
longer says "or a contract import"; `prepare_contract`'s and
`import_failure`'s docs name `ContractImport`; `load_data_fn`'s `## Errors`
names `RemoteRead` and `ContractImport`; `is_authoring_fatal`'s doc covers
`NestedData` and the import grading; `FetchSlot::Failed`, `fetch_with_cache`,
`get_content`, `RemoteTextError::Failed`, and `remote_text_error` docs describe
the typed failure. No `docs/` page changed: no user-visible behavior changed.

### Requirement-to-test mapping (Phase 2)

Boundary per row: F1 is an in-crate unit test of the typed value; F2 is the
public compose API in-process (L1); the Claudine row is Claudine's public
error projection over Darkmatter's types (crate boundary, L1).

| Behavior | Test(s) | Rung |
|---|---|---|
| E3 classifier: every `ExpressionError` and `DataError` variant, each also inside two callbacks | `X/error.rs` `tests::recovery::every_variant_is_classified_and_callbacks_delegate` | F1 |
| E3: every `FileReferenceError` cause under `NotFound` (only `Io` recovers) | `tests::recovery::only_an_io_cause_keeps_a_not_found_miss_recoverable` | F1 |
| E4: import false where the same source miss is true; same text and grading | `tests::recovery::a_contract_import_is_not_recovered_where_the_same_source_miss_is` | F1 |
| E5: `NestedData(Malformed)` false, `Data(Malformed)` true; same display; accessor | `tests::recovery::a_nested_data_failure_reads_and_grades_like_a_direct_one_but_is_not_recovered` | F1 |
| E6 view covers sources and imports only | `tests::recovery::the_file_reference_view_covers_sources_and_imports_only` | F1 |
| `RemoteRead` display and grading unchanged by `kind` | `tests::recovery::remote_read_display_is_unchanged_by_its_kind` | F1 |
| E4 `ContractImport`: display = former message, `code()` None, never a deferral (remote kind too) | `X/data/error.rs` `a_contract_import_reads_like_a_file_reference_but_has_its_own_identity`; `X/data/contract.rs` `a_missing_import_is_a_contract_import_failure` (renamed from `…_stays_a_file_reference_failure`; its assertion changed with the constructor) | F1 |
| Typed provenance survives memo reuse | `X/data/provenance_tests.rs`: `a_memoized_contract_import_failure_stays_a_contract_import`, `a_memoized_malformed_source_stays_malformed`, `a_memoized_missing_source_stays_a_clean_miss` | F1 (through `call_function` and the real `DataRuntime`) |
| E7 mapping: each `FetchError`, policy/invalid URL errors, UTF-8 | `compose/remote_fetch/error.rs` tests (4) | F1 |
| E8: a 500 and a non-UTF-8 body are `Transport`, a blocked redirect is `Policy`, the string adapter is byte-identical | `X/resolve_ctx.rs` `fetch_remote_text_keeps_the_failed_fetch_kind` (wiremock, loopback) | F2 |
| E8: unparsable URL is `InvalidReference`; denied host stays `Denied` | `fetch_remote_text_types_an_unparsable_url_as_an_invalid_reference`; existing `fetch_remote_text_on_demand_enforces_policy` unchanged | F1 |
| `get_content` strings unchanged | every existing test in `remote_fetch.rs` passes unmodified | F2 |
| Projection: missing import record unchanged (code, message, source, reference fields, empty callback path) | `lib/tests/l1/load_data_diagnostics.rs` `a_missing_contract_import_keeps_its_file_reference_record` | F2 |
| Projection: unreadable import (`io`, real read refused by the read guard) | `an_unreadable_contract_import_keeps_its_file_reference_record` | F2 |
| Projection: import inside a callback keeps `map[0]` | `a_missing_contract_import_in_a_callback_keeps_its_item_position` | F2 |
| Projection: malformed data inside `as_markdown` (now `NestedData`) equals the direct record | `a_malformed_file_inside_nested_composition_reports_its_data_code_at_the_call`; existing `a_violation_inside_nested_composition_reports_its_data_code_at_the_call` unchanged | F2 |
| Claudine: import renders `composition.invalid_file_reference`, same `detail` and status block | `claudine/lib/src/composition/error/tests.rs` `a_contract_import_failure_renders_as_an_invalid_file_reference` | F2 |

**Before/after proof for the import records.** The two import projection tests
were run a second time with `import_failure` temporarily switched back to
`DataError::FileReference`, which is exactly the old path. They passed both
ways, so the asserted records are the ones composition produced before this
change. The switch was reverted at once.

The Input Robustness Matrix does not apply (E20): no loader or configuration
reader changed.

### Departures

- Waves 3 and 4 ran in one session, without subagents. The edits share one
  error type, so splitting them would have cost more than it saved. The
  placeholder `kind: Infrastructure` was never written.
- `ContractImport`'s fatality follows the file-reference rule instead of
  "always fatal like `Data`" (see `decisions_for_review`).
- Claudine's helper does not look through callbacks (see `decisions_for_review`).
- The runtime and client start-up `Infrastructure` branches have no direct test
  (no seam; see `decisions_for_review`).
- The only existing assertion that changed is the renamed
  `contract.rs` unit test, whose subject is the constructor that changed.

### Gates

| Gate | Result |
|---|---|
| `darkmatter/` `just test` (run 1) | failed: 1 failure, `compose::tests::lookup_parity::the_inventory_lists_every_lookup_implementation`. My new classifier test had defined its own `EvaluationLookup`, which the inventory guard rejects. Fixed by calling `functions::call_function` instead |
| `darkmatter/` `just test` (run 2) | **pass**: 15918 run, 15918 passed (10 slow, all existing CLI/DMLS tests under full-run load), 10 skipped |
| `darkmatter/` `just lint` | **pass** (also after the fix) |
| `just check-tier-coverage darkmatter` | **pass**: 0 stranded |
| `claudine/` `just test-library` | **pass**: 6501 run, 6501 passed |
| `claudine/` `just lint` | **pass** |
| Dependents | Of the other packages that depend on `darkmatter`, only `claudine-cli`'s `tests/l1/error_guards.rs` names the changed types: `cargo nextest run -p claudine-cli --test l1 -E 'test(/error_guards/)'` passed (9/9). The rest (research, messenger, the biscuit/sniff/playa CLIs, claudine-gen) do not use `ExpressionError`, `DataError`, or `RemoteRead`. |

Scope calculator (`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`):
`packages: [claudine, darkmatter, test-toolkit, darkmatter-cli]`,
`change_class: package`. The gates were therefore required, and all of them ran.

Cross-OS check: not run. The phase changes typed errors and their projection,
with no path comparison, process, console, or OS-specific code. The new tests
use temp directories, loopback wiremock, and a file-name read guard, all of
which behave the same on every OS.

Phase 2 is complete: implementation complete for this phase, all gates green.

## Phase 3

Phase 3 registers `try(expr, fallback)` and makes it evaluate on both
runtime paths and type correctly in passive analysis. No surface wiring or
consumer tests yet (Phase 4).

Paths below are relative to `darkmatter/lib/src/markdown/` unless stated
otherwise. `X/` means `compose/expression/`.

### What changed

**Wave 5: runtime recovery.**

- `darkmatter/schemas/partials/functions.yaml`: `try` under `# Logical`,
  after `or`, with the E2 declaration (`parameters(expr: unknown | null,
  fallback: unknown | null)`, `returns(unknown | null)`, `category(Logical)`,
  the two examples). Not `fallible`, no aliases.
- `X/functions/mod.rs`: `ShortCircuit::{OnFalsy, OnTruthy}` replaced by
  `LazyKind::{All, Any, Recover}`. `lazy_kind` maps by canonical name with no
  default arm (`debug_assert!` and `None` for an unmapped lazy name, so it is
  never run as another function's rule). `LAZY_BINDINGS` gains
  `FunctionBinding::lazy("try")` (`SlotClass::Pure`, no read fact, E17).
  `lazy_function` returns `(registration, LazyKind)`. The direct value-call
  rejection in `call_entry` is unchanged and now also covers `try`.
- `X/call_binding/bind.rs`: new `FunctionRegistration::check_lazy_return`
  (the single overload's `check_return`; a lazy call has no `BoundCall`).
- `X/mod.rs`: `evaluate_function` dispatches on `LazyKind`; new
  `evaluate_recovery` (E10: arity first; protected operand once; on success
  bind at index 0 and check the return; on a recoverable failure evaluate,
  bind at index 1, and check the fallback; any other failure, and any
  fallback failure, escapes unchanged). Both operands use the call's own
  absence scope (no facts accumulate); the same lookup, observer, and budget.
  The `and`/`or` loop is unchanged apart from the enum names.
- `lifecycle/executor.rs` (`darkmatter/lib/src/lifecycle/`): the operand path
  dispatches `LazyKind::Recover` to a new `call_recovery` (E11). Only
  `LifecycleExprError::Evaluate(e)` with `e.is_recoverable_read_failure()`
  selects the fallback; `Compose`, `Context`, and `Prose` propagate. The
  result is the selected value.
- `X/scope/facts.rs`: `LazyKind::Recover` returns the incoming scope (E14).
- `X/absence.rs`: `call_argument` never marks a `try` operand as handling
  absence (new `is_recovery`). See the decision below: without this, the
  `unknown | null` parameters would have suppressed undeclared-property
  warnings inside `try`, which the spec forbids.

**Wave 5: passive types.**

- `X/call_check.rs`: new `try_type`, dispatched from `expression_type`'s
  `FunctionCall` arm after `load_data_type`, identified through
  `inspect_registration` (any spelling). Two operands: the union of both
  operand types in the incoming scope. An authored `[]` (through `Paren`)
  adds nothing when some arm of the other operand's shape is accepted by
  `simplified_schema::completion::alternative_accepts_in(arm, [], scope.definitions())`,
  or the other shape is `Unknown` (which absorbs the union anyway). The
  `expression_type` table doc gained the `try` row.

**Generated docs.** `just regen-expr-doc` added the `try` row to
`darkmatter/docs/topics/darkmatter-expressions.md` (no per-function page is
generated for lazy functions, as for `and`/`or`). The plan schedules
generated pages for Phase 5, but the catalog test
`narrative_doc_function_table_matches_catalog` fails until the table matches
the catalog, so this phase's gate required it (see Departures).

**Catalog-derived artifacts.** Adding a catalog function changes every
artifact generated from or pinned to the catalog. Each was regenerated or
updated, not hand-edited, except where noted:

- `darkmatter/schemas/partials/lifecycle.yaml`: `just regen-lifecycle-schema`
  added the `try-short` (`tuple([unknown | null, unknown | null])`) and
  `try-long` (`{expr, fallback}`, optionally `no_error`) verb types and the
  `try` entries in the bundles. So `try` is now a lifecycle expression verb,
  like `and`/`or`; E11's operand path is what runs it.
- `darkmatter/schemas/partials/md-lifecycle.yaml`: `md-unknown-action`'s
  negative-lookahead pattern lists every verb md provides; it is maintained
  by hand. `try` was inserted between `trim` and `try_frontmatter`. Without
  it, an authored `try:` would also pass through md's open arm untyped
  (`lifecycle_schema_parity::the_open_arm_rejects_every_md_verb_and_admits_every_unprovided_verb`).
- The six `lib/tests/fixtures/recursive_schema_golden/golden/` files that
  embed the baseline (`embedded-darkmatter-base.json`,
  `darkmatter-schemas-darkmatter-schema-body.json`,
  `darkmatter-schemas-partials-fm.json`, `effective-baseline-inline.json`,
  and the two `effective-baseline-*.digest` files): reblessed with
  `DARKMATTER_BLESS_RECURSIVE_GOLDEN=1`. Each JSON diff is the one pattern
  line above gaining `|try|`, and each digest grows by exactly 4 bytes.
- `darkmatter/dmls/tests/fixtures/response_contracts/a3_completions.json`:
  reblessed with `DMLS_UPDATE_GOLDEN=1`; the only change is a new
  `try(expr, fallback)` function completion item (18 added lines), so DMLS
  already offers `try` from the catalog.
- `lib/tests/l1/lifecycle_schema_generation.rs`: the pinned verb count went
  from 130 to 131.
- `lib/tests/l1/function_binding_registry.rs`: a `try` row, and the row
  macro now strips a raw-identifier `r#` (`try` is a Rust keyword).

**Claudine (dependent package).** Claudine's lifecycle schemas list every
Darkmatter verb by hand, and its tests pin them to the catalog:

- `claudine/schemas/action.yaml` and `claudine/schemas/lifecycle.yaml`: a
  `try` entry (long form `try-short`/`try-long@darkmatter:lifecycle` in the
  `expression` category, `try-short` in each bundle) before every
  `try_frontmatter` entry, six in all. `just regen-lifecycle-artifact` left
  `schemas/globals.lifecycle.json` unchanged.
- `claudine/cli/tests/l1/snapshots/l1__context_command__context_expressions_function_order_snapshot.snap`:
  one added line, `try(expr, fallback)` (the `assertion_line` metadata insta
  writes for a new snapshot was removed to match the other snapshots).

**Skill.** `.claude/skills/darkmatter/expressions.md`: the lazy-function
paragraph now covers `LazyKind`, `evaluate_recovery`/`call_recovery`, the
no-guard and no-absence rules, and `try_type`; the classifier bullet no
longer says the form is planned.

### Requirement-to-test mapping (Phase 3)

Boundary per row: F1 is an in-crate unit test through the real evaluator,
registry, or executor; F2 is the crate's public API from `tests/l1` (passive
analysis, or `Markdown::compose_with_warnings` in-process).

| Behavior | Test(s) | Rung |
|---|---|---|
| E10: all six falsy values and a non-falsy value are successes; fallback never entered | `X/mod.rs` `tests::recovery::fallback_not_evaluated_after_success` | F1 |
| E10: protected operand evaluated once | `tests::recovery::protected_runs_once` | F1 |
| E10: fallback runs once after a real `load_data` miss; the miss stays the loader's | `tests::recovery::fallback_runs_once_after_recoverable_failure` | F1 |
| E1/E10: nonrecoverable failures (handler error, arity, arithmetic, unknown function) escape unchanged, fallback never entered | `tests::recovery::nonrecoverable_failure_skips_fallback` | F1 |
| Fallback failure is its own (handler error; a second miss names its own file) | `tests::recovery::fallback_error_escapes_its_own_try` | F1 |
| Nested `try` catches only a recoverable fallback failure | `tests::recovery::enclosing_try_catches_recoverable_fallback_failure`, `…_rethrows_nonrecoverable_fallback_failure` | F1 |
| Budget shared, not reset (2 visits fail, 3 succeed, the fallback alone fits in 2) | `tests::recovery::try_shares_the_iteration_budget` | F1 |
| Case-insensitive dispatch | `tests::recovery::try_lookup_is_case_insensitive` | F1 |
| Arity checked before anything is evaluated (0, 1, 3 operands) | `tests::recovery::wrong_arity_fails_before_evaluation` | F1 |
| E9: `and → All`, `or → Any`, `try → Recover`; the lazy set is exactly these three | `X/functions/mod.rs` `registration_tests::every_lazy_function_maps_to_its_own_kind`; existing `evaluation_modes_pair_with_their_handler_kinds` (the `lazy_function(...).is_some()` invariant) | F1 |
| Direct value call of `try` rejected, not recoverable | `registration_tests::direct_value_call_of_try_is_rejected` | F1 |
| E17: `try` is `Pure`, reads nothing, has no aliases | `registration_tests::try_is_pure_and_reads_nothing_of_its_own` | F1 |
| E11: the operand path returns the selected value (array, falsy `0`, fallback), not a Boolean | `lifecycle/executor/tests.rs` `try_action_returns_the_selected_value_not_a_boolean` | F1 (real `StackExecutor`) |
| E11: a template fallback is interpolated | `try_template_operand_fallback_is_interpolated` | F1 |
| E11: an unused template operand is never interpolated (control proves interpolating it fails) | `unused_template_operand_is_not_interpolated` | F1 |
| E11: a template operand's `Compose` failure (a miss inside the template) propagates | `template_operand_compose_failure_is_not_recovered` | F1 |
| E13: union of both branches, either order | `lib/tests/l1/load_data_passive.rs` `try_typing::try_type_unions_both_branches` | F2 |
| E13: `null` survives | `try_typing::try_type_keeps_null` | F2 |
| E13: a different fallback type stays in the union | `try_typing::try_type_with_different_fallback_type` | F2 |
| E13: unknown contract stays unknown | `try_typing::try_type_unknown_contract_stays_unknown` | F2 |
| E13 exception: rows kept with `[]` (both orders, `([])`, `TRY`), callback item members kept; control that a ternary erases them | `try_typing::empty_fallback_keeps_typed_array_items` | F2 |
| E13: `min(1)` list, fixed tuple, object keep the ordinary union | `try_typing::empty_fallback_kept_for_min_one_array`, `…_for_fixed_tuple`, `…_for_object` | F2 |
| E13: an all-optional tuple drops `[]` | `try_typing::empty_admitting_tuple_drops_empty_fallback` | F2 |
| E13: recursive reference provable with its definitions, unprovable without them | `try_typing::empty_fallback_kept_for_unprovable_recursive_reference` | F2 |
| T4: three-argument `load_data` | `try_typing::try_type_over_three_argument_load_data` | F2 |
| E14: no guard facts (control: a plain guard does narrow) | `try_typing::try_adds_no_guard_facts` | F2 |
| E14/spec: `try` operands do not handle absence (control: `is_null(x)` does) | `try_typing::try_operands_do_not_handle_absence` | F2 |
| Precheck sees a bad call in an unreachable fallback and in the protected operand, before any handler or read | `lib/tests/l1/design_time_call_check.rs` `walk::invalid_call_in_unreachable_fallback_rejects`, `walk::invalid_call_in_protected_operand_rejects` (passive check with handler counters, plus a composed document whose earlier `load_data` never reads) | F2 |
| Catalog entry accepted, examples evaluate | existing `function_catalog_parity` and catalog tests, unmodified | F2 |
| Binding-registry matrix: `try` has a row (accepted = a real miss recovered; rejected = arity; converted impossible: the selected operand is kept as is, `"4"` stays text) and enters no handler | `lib/tests/l1/function_binding_registry.rs` `rows::r#try` (the row macro now accepts a raw identifier, since `try` is a Rust keyword) | F2 |

The Input Robustness Matrix does not apply (E20): no loader or configuration
reader changed. The catalog entry is read by the existing catalog parser,
whose shipped-artifact corpus test (`function_catalog_parity`) covers it.

### Departures

- Waves were run in this session without subagents: both tasks share the
  `LazyKind` name and are small.
- Generated table regenerated in Phase 3, not Phase 5 (the catalog test
  requires it). Phase 5 still owns the topic prose and any generated page.
- `absence.rs` was edited, though no Phase 3 task owned it (decision below).
- **Two gate runs went to the wrong checkout.** On this host `CDPATH`
  includes `/Users/ken/coding/personal`, so a relative `cd darkmatter` run
  from inside `darkmatter/` (where no `darkmatter/darkmatter` exists)
  silently changes to the main checkout
  (`/Volumes/coding/personal/rusty-biscuit/darkmatter`). One `just test` run
  (13051 tests, all passed) and one scoped run there tested the main
  checkout, not this worktree. They were discarded, every later command uses
  an absolute `cd`, and `git status` of the main checkout was confirmed
  clean afterward. The tell is the test count: this worktree has 15951 L1
  tests; the main checkout has 13051.
- The `executor/tests.rs` helper sets `source_path` inside the fixture's
  temp dir: with the fixture's default relative `doc.md`, a relative
  `load_data` fails as `RelativeContextDirectory`, which is correctly not
  recoverable and would not exercise the fallback.

### Gates

| Gate | Result |
|---|---|
| `darkmatter/` `just test` (run 1, worktree) | failed: 1, `function_binding_registry::every_registered_function_has_a_row` (no `try` row). Fixed |
| (run 2) | discarded: ran in the main checkout through `CDPATH` (see Departures) |
| `darkmatter/` `just test` (run 3) | failed: 5 lifecycle schema generation and parity tests. Fixed by `just regen-lifecycle-schema`, the md open-arm pattern, and the pinned count |
| (run 4) | failed: 3 `recursive_schema_golden` entries. Reblessed (6 golden files, pattern-only diffs) |
| (run 5) | failed: 1, `frontmatter_response_contracts::a3_completions_are_unchanged`. Reblessed (one new completion item) |
| `darkmatter/` `just test` (run 6) | **pass**: 15951 run, 15951 passed, 10 skipped |
| `darkmatter/` `just lint` | **pass** |
| `just check-tier-coverage darkmatter` | **pass**: 0 stranded |
| `cargo check -p darkmatter --examples` | **pass** |
| Claudine, scoped (`claudine` with `work-counters`, `claudine-cli` with `test-fixtures`; tests matching lifecycle, schema, verb, catalog, golden, function, completion, parity, expression, condition, compos, context, error) | first run: 3 catalog-pinned failures (fixed above) and `lifecycle_message_drain::stalled_deliveries_share_one_drain_budget_and_are_reported_as_unknown`, which failed only because that first run omitted `claudine-cli/test-fixtures` (its drain-budget seam), and passes with it. Final run: **pass**, 6090 of 6090 |
| `simplified-schema` corpus tests (they `include_str!` the changed Darkmatter YAML) | **pass**: 18 of 18 |

No test failed for load: every failure above was deterministic and reproduced
alone.

Scope calculator (`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`):
`packages: [claudine, darkmatter, test-toolkit, claudine-cli, darkmatter-cli, darkmatter-lsp, simplified-schema]`,
`change_class: package` (the working tree also holds Phase 2's uncommitted
changes). The gates were required and all ran.

Cross-OS check: not run. The phase adds evaluator, typing, and schema
behavior with no path comparison, process, console, or OS-specific code. The
new tests use temp directories and in-process evaluation only.

Phase 3 is complete: implementation complete for this phase, all gates green.
