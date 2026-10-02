---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-schema-enhancement/claudine/fixes/2026-07-13-cli-switches/spec.md"
plan: "claudine/fixes/2026-07-13-cli-switches/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
packages:
    - claudine
    - claudine-cli
    - claudine-gen
    - claudine-catalog-types
    - darkmatter
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - claudine/lib/src/composition/provider_tail.rs
    - claudine/lib/src/composition/mod.rs
    - claudine/lib/src/composition/types.rs
    - claudine/lib/src/composition/coordinator/invocation.rs
    - claudine/cli/src/main.rs
    - claudine/cli/src/argv/mod.rs
    - claudine/cli/src/argv/partition.rs
    - claudine/cli/src/commands/compose/mod.rs
    - claudine/cli/src/commands/compose/prep.rs
    - claudine/cli/src/commands/compose/prep/tests.rs
    - claudine/cli/src/commands/sequence.rs
    - claudine/cli/src/commands/wrap/mod.rs
    - claudine/cli/src/commands/wrap/flags.rs
    - claudine/cli/src/commands/wrap/flags/tests.rs
    - claudine/cli/src/commands/wrap/profile/resolve.rs
    - claudine/cli/src/commands/wrap/profile/tests/positional.rs
    - claudine/cli/src/commands/wrap/provider_tail_report.rs
    - claudine/cli/src/commands/wrap/provider_tail_report/tests.rs
    - claudine/cli/src/commands/wrap/composition/mod.rs
    - claudine/cli/src/commands/wrap/composition/pipeline.rs
    - claudine/cli/src/commands/wrap/composition/dry_run.rs
    - claudine/cli/src/commands/wrap/composition/provider_args.rs
    - claudine/cli/src/commands/wrap/sequence/iterate.rs
    - claudine/cli/src/commands/wrap/sequence/task_run.rs
    - claudine/cli/src/output/mod.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/cli/tests/l1/provider_tail_notice.rs
    - claudine/cli/tests/l1/snapshots/l1__wrap_basics__wrapper_reports_removed_sensitive_env_names.snap
docs_updated_during_phase_2:
    - claudine/docs/topics/argv-normalization.md
    - claudine/docs/topics/cli-pre-parsing.md
    - claudine/docs/topics/composition.md
    - claudine/docs/providers/dispatch-inventory.json
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/cli-reference.md
source_files_during_phase_3:
    - claudine/lib/src/secrets/mod.rs
    - claudine/lib/src/secrets/tests.rs
    - claudine/lib/src/signals/bespoke.rs
    - claudine/lib/src/signals/mod.rs
    - claudine/cli/src/commands/wrap/resume.rs
    - claudine/cli/src/commands/wrap/mod.rs
    - claudine/cli/src/commands/wrap/profile/pi.rs
    - claudine/cli/src/commands/wrap/launch_plan.rs
    - claudine/cli/src/commands/wrap/launch_plan/tests.rs
    - claudine/cli/src/commands/wrap/composition/pipeline.rs
    - claudine/cli/src/commands/wrap/provider_tail_report.rs
    - claudine/cli/src/commands/wrap/env/mod.rs
    - claudine/cli/src/commands/wrap/env/sanitize.rs
    - claudine/cli/src/commands/wrap/env/tests.rs
    - claudine/cli/src/commands/wrap/exec/mod.rs
    - claudine/cli/src/commands/wrap/exec/spawn/semantic.rs
    - claudine/cli/src/commands/wrap/exec/spawn/captured.rs
    - claudine/cli/src/commands/wrap/exec/spawn/inherited.rs
    - claudine/cli/src/commands/wrap/exec/wiring/session.rs
    - claudine/cli/src/commands/wrap/wrapper_exec.rs
    - claudine/cli/src/commands/wrap/wrapper_stages.rs
    - claudine/cli/src/commands/wrap/harness_orch/attempt.rs
    - claudine/cli/src/commands/wrap/harness_orch/launch.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/target_launch.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/target_launch/tests.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/active_state_wiring.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/retry_resume.rs
    - claudine/cli/src/commands/wrap/harness_orch/session_key.rs
    - claudine/cli/src/commands/wrap/harness_orch/session_key/tests.rs
    - claudine/cli/src/commands/wrap/harness_orch/types.rs
    - claudine/cli/src/output/mod.rs
    - claudine/cli/src/output/native_exit.rs
    - claudine/cli/src/output/error_report.rs
    - claudine/cli/src/output/error_report/tests.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/cli/tests/l1/provider_tail_launch.rs
docs_updated_during_phase_3:
    - claudine/docs/topics/composition.md
    - claudine/docs/topics/argv-normalization.md
    - claudine/docs/topics/cli-pre-parsing.md
    - claudine/docs/providers/dispatch-inventory.json
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/cli-reference.md
source_files_during_phase_4:
    - claudine/catalog-types/src/cli_switch.rs
    - claudine/catalog-types/src/lib.rs
    - claudine/lib/src/provider/cli_switch.rs
    - claudine/lib/src/provider/mod.rs
    - claudine/lib/src/provider/tests.rs
    - claudine/lib/src/provider/antigravity/data.rs
    - claudine/lib/src/provider/claude/data.rs
    - claudine/lib/src/provider/codex/data.rs
    - claudine/lib/src/provider/gemini/data.rs
    - claudine/lib/src/provider/goose/data.rs
    - claudine/lib/src/provider/kilo/data.rs
    - claudine/lib/src/provider/kimi/data.rs
    - claudine/lib/src/provider/opencode/data.rs
    - claudine/lib/src/provider/pi/data.rs
    - claudine/lib/src/provider/qwen/data.rs
    - claudine/gen/src/emit/cli_switches.rs
    - claudine/gen/src/emit/mod.rs
    - claudine/gen/src/errors.rs
    - claudine/gen/src/generate.rs
    - claudine/gen/src/generate/coerce/cli_switches.rs
    - claudine/gen/src/generate/coerce/mod.rs
    - claudine/gen/src/inputs.rs
    - claudine/gen/src/registry.rs
    - claudine/gen/src/registry/tests.rs
    - claudine/gen/src/schema_compat.rs
    - claudine/gen/src/vocabulary/tests.rs
    - claudine/gen/tests/l1/cli_switches.rs
    - claudine/gen/tests/l1/main.rs
    - claudine/gen/tests/l1/pipeline.rs
    - claudine/gen/tests/l1/registry_coverage.rs
    - claudine/gen/tests/fixtures/agent-cli-r2/codex.md
    - claudine/gen/tests/fixtures/generated-artifact-baseline.json
docs_updated_during_phase_4:
    - claudine/docs/topics/provider-metadata.md
    - claudine/docs/research/agent-cli/_schema.yaml
    - claudine/docs/providers/catalog.json
docs_created_during_phase_4:
    - claudine/docs/research/agent-cli/_types.yaml
    - claudine/docs/research/agent-cli/_schema.r1.yaml
skills_files_updated_during_phase_4:
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/research-contracts.md
source_files_during_phase_5:
    - claudine/catalog-types/src/cli_switch.rs
    - claudine/lib/src/provider/cli_switch.rs
    - claudine/lib/src/provider/cli_switch/tests.rs
    - claudine/lib/src/provider/mod.rs
    - claudine/lib/src/provider/antigravity/data.rs
    - claudine/lib/src/provider/claude/data.rs
    - claudine/lib/src/provider/codex/data.rs
    - claudine/lib/src/provider/gemini/data.rs
    - claudine/lib/src/provider/goose/data.rs
    - claudine/lib/src/provider/kilo/data.rs
    - claudine/lib/src/provider/kimi/data.rs
    - claudine/lib/src/provider/opencode/data.rs
    - claudine/lib/src/provider/pi/data.rs
    - claudine/lib/src/provider/qwen/data.rs
    - claudine/gen/src/main.rs
    - claudine/gen/src/report.rs
    - claudine/gen/tests/l1/cli_switches.rs
    - claudine/gen/tests/l1/generate_ux.rs
    - claudine/gen/tests/l1/pipeline.rs
    - claudine/gen/tests/fixtures/agent-cli-r1/codex.md
    - claudine/gen/tests/fixtures/agent-cli-r1/_schema.r1.yaml
    - claudine/gen/tests/fixtures/agent-cli-r2/codex.md
    - claudine/gen/tests/fixtures/generated-artifact-baseline.json
    - claudine/cli/src/commands/wrap/provider_tail_report.rs
    - claudine/cli/src/commands/wrap/provider_tail_report/tests.rs
    - claudine/cli/src/commands/wrap/composition/pipeline.rs
    - claudine/cli/src/commands/wrap/mod.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/cli/tests/l1/provider_tail_notice.rs
    - claudine/cli/tests/l1/switch_catalog_guard.rs
    - claudine/docs/research/agent-cli/_relations.py
    - claudine/justfile
docs_updated_during_phase_5:
    - claudine/docs/research/agent-cli/_schema.yaml
    - claudine/docs/research/agent-cli/_types.yaml
    - claudine/docs/research/agent-cli/_fleet.md
    - claudine/docs/research/agent-cli/antigravity.md
    - claudine/docs/research/agent-cli/claude.md
    - claudine/docs/research/agent-cli/codex.md
    - claudine/docs/research/agent-cli/gemini.md
    - claudine/docs/research/agent-cli/goose.md
    - claudine/docs/research/agent-cli/kilo.md
    - claudine/docs/research/agent-cli/kimi.md
    - claudine/docs/research/agent-cli/opencode.md
    - claudine/docs/research/agent-cli/pi.md
    - claudine/docs/research/agent-cli/qwen.md
    - claudine/docs/providers/catalog.json
    - claudine/docs/providers/dispatch-inventory.json
    - claudine/docs/topics/provider-metadata.md
    - claudine/docs/topics/argv-normalization.md
    - claudine/docs/topics/composition.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/research-contracts.md
    - .claude/skills/claudine/cli-reference.md
source_files_during_phase_6:
    - claudine/lib/src/composition/ownership.rs
    - claudine/lib/src/composition/ownership/tests.rs
    - claudine/lib/src/composition/mod.rs
    - claudine/lib/src/composition/provider_tail.rs
    - claudine/lib/src/composition/schema/mod.rs
    - claudine/lib/src/composition/types.rs
    - claudine/lib/src/provider/cli_switch.rs
    - claudine/lib/src/provider/mod.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - claudine/cli/src/main.rs
    - claudine/cli/src/argv/mod.rs
    - claudine/cli/src/argv/partition.rs
    - claudine/cli/src/argv/partition/tests.rs
    - claudine/cli/src/argv/rule4_help_hoist.rs
    - claudine/cli/src/commands/compose/mod.rs
    - claudine/cli/src/commands/compose/ownership.rs
    - claudine/cli/src/commands/compose/prep.rs
    - claudine/cli/src/commands/compose/prep/tests.rs
    - claudine/cli/src/commands/compose/setters.rs
    - claudine/cli/src/commands/compose/tests.rs
    - claudine/cli/src/commands/sequence.rs
    - claudine/cli/src/commands/wrap/provider_tail_report.rs
    - claudine/cli/src/commands/wrap/composition/pipeline.rs
    - claudine/cli/src/commands/wrap/harness_orch/launch.rs
    - claudine/cli/src/commands/wrap/sequence/mod.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/cli/tests/l1/provider_tail_ownership.rs
    - claudine/cli/tests/l1/level1_ownership_prompt_pty.rs
    - claudine/cli/tests/l1/provider_tail_launch.rs
    - claudine/cli/tests/l1/provider_tail_notice.rs
    - claudine/cli/tests/l1/wrap_compose_validation.rs
    - claudine/cli/tests/l1/effective_diagnostic_render.rs
    - claudine/cli/tests/level2/level2_typed_error_render_capture.rs
docs_updated_during_phase_6:
    - claudine/docs/topics/argv-normalization.md
    - claudine/docs/topics/cli-pre-parsing.md
    - claudine/docs/topics/composition.md
    - claudine/docs/topics/frontmatter-properties.md
    - claudine/docs/providers/dispatch-inventory.json
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/cli-reference.md
    - .claude/skills/claudine/timeline.md
---

# Implementation Log for 2026-07-13-cli-switches (7 phases)

## Phase 1

Phase 1 is investigation only: no source, doc, or skill file changed. This log
and the plan's checkboxes and frontmatter are the only edits.

Paths in the plan written as `harness_orch/...` and `wrap/...` live under
`claudine/cli/src/commands/wrap/`.

### Rulings

No author was available during this non-interactive run, so each ruling below
takes the plan's default unless a spike showed the default could not hold.
Rulings 10–14 are new: the spikes found them.

1. **R9 lands after R8 data exists.** Accepted. Phases 2–3 (reporting, resume,
   binary coverage) ship against the current partition.
2. **All-`unknown` metadata is safe.** Accepted. Unknown is never "none"
   (spec rule 5), so R9 can land before every provider is researched, provided
   the generator gives every compiled provider entries or an explicit unknown
   gap (Phase 4 enforces this).
3. **The shared ownership function lives in the `claudine` lib.** Accepted,
   with one refinement from Spike A. Types and pure ownership logic go in
   `claudine::composition`; the CLI passes the owned-flag surface from
   `OwnedFlags::for_composition` in as data, because the lib has no clap
   surface. **Refinement:** `partition_composition_tail` runs in
   `cli/src/main.rs:275`, before clap and before any file is resolved.
   Type-aware ownership needs the file's frontmatter, so it cannot replace
   that call in place. Phase 6 keeps the pre-clap partition for what needs no
   file (Claudine-owned flags and values, the first `--`, ordering errors,
   file identification). It then classifies the remaining tokens in a second
   pass inside the composition command, after `resolve_composition_source`.
   Help and version never reach that second pass, which keeps "help opens no
   file" true.
4. **`argv` in a setter and in `--set` share one validator.** Accepted.
5. **Number grammar.** Accepted: leading `+` accepted; `.5` and `5.`
   accepted; `0x..`, `NaN`, `inf`/`infinity`, and `1_000` rejected.
6. **Variadic minimum.** Accepted: `variadic_min` in the catalog; a missing
   minimum is unknown, counts as 1 for ownership, and never fails the
   resolved-provider check.
7. **No wider measurement.** Accepted. No benchmarks are added. Spike A found
   the early read costs nothing new (see below).
8. **The ambiguity-prompt answer is not stored.** Accepted: it applies to one
   invocation and is never cached on disk.
9. **Windows non-UTF-8 fixtures.** Accepted: use platform-gated fixtures.
   Unix builds an invalid byte with `OsStrExt::from_bytes`; Windows builds an
   unpaired surrogate with `OsStringExt::from_wide(&[0xD800])`.
10. **New: the schema property-name seam needs a public Darkmatter API.**
    `EffectiveSchema::declares_top_level_property`
    (`darkmatter/lib/src/markdown/schemas/mod.rs`) already walks `properties`,
    `patternProperties`, every root-union arm, `allOf`, `if`/`then`/`else`, and
    local `$ref`, which is exactly what spec rule 2 needs. It is `pub(crate)`.
    Default for Phase 6: make it `pub` in Darkmatter (a one-line change in a
    second package) instead of copying the walk into Claudine. Phase 6 must add
    `darkmatter` to `packages`.
11. **New: schema names come from the document `$schema` only.** Claudine
    loads with `DarkmatterSchemas::new()` and no baseline or trigger discovery
    (`lib/src/composition/schema/mod.rs::load_effective_schema_in_context`).
    Ownership uses that same loader, so Darkmatter baseline properties are not
    "schema parameters" for rule 2. A document with no `$schema` has no
    parameter names; setters keep rule 3 behavior. That case differs from
    "names cannot be established" (a read failure), as the Phase 6 matrix
    requires.
12. **New: the `agent-cli` contract has no `schema_revision`.** The plan says
    to "increment `schema_revision`", but `docs/research/agent-cli/_schema.yaml`
    has none; only `reasoning-level` and `steering` carry one. Default for
    Phase 4: treat today's unversioned contract as revision 1 and introduce
    `schema_revision: literal(2; required)` with the new switch fields.
    **Sequencing consequence:** Phase 4 lands before Phase 5 re-researches the
    fleet, so every committed `agent-cli/*.md` is still at the old contract.
    `claudine-gen check` and `just test` must stay green between those phases.
    Default: the generator projects a document without revision 2 as an
    explicit whole-provider unknown gap instead of failing. Phase 5 removes
    those gaps by re-researching. Phase 4 must not hand-edit existing research
    documents.
13. **New: switch scope gets a new field name.** Existing records carry
    `scope: ["global", "config"]`, which mixes a global marker with
    category labels (`config`, `model_selection`). Default for Phase 4: put the
    normalized invocation scope in a new property (for example
    `invocation_scope`), and leave the legacy `scope` labels as descriptive
    data. Do not reinterpret them as command paths.
14. **New: the composition path never builds an `AgentErrorReport` today.**
    Only the direct wrapper does (`cli/src/commands/wrap/mod.rs:252`,
    `from_exit_code_with_source`, stderr only). The harness path
    (`harness_orch/attempt.rs`) routes `stderr_text` into attempt
    classification. Phase 3's "one builder, both paths" therefore adds the
    composition call site; it does not reroute one.

### Spike findings

**Spike A: early frontmatter read seam.** Answered: no new extraction is
needed.

- `claudine::composition::resolve_composition_source_in_context(file_ref,
  &FileResolutionContext)` (`lib/src/composition/resolve.rs:174`) resolves
  the reference through `biscuit_file::FileReference` and loads the Markdown
  with typed frontmatter. It runs no templates, shell, lifecycle actions,
  provider discovery, or network. The CLI wrapper
  `cli/src/commands/compose/prep.rs::resolve_composition_source` adds
  bare-name autocomplete recovery. Ownership must reuse the already-resolved
  `ResolvedCompositionSource` from `run_composition_inner`
  (`prep.rs:206`), not resolve a second time.
- Literal `agent` hints: `parse_selection_hints_from_frontmatter`
  (`lib/src/composition/hints.rs:26`) is already called on the raw
  frontmatter at `prep.rs:581` and in `commands/wrap/sequence/mod.rs:284`.
  Templated values fall through untouched, so an unresolved `agent` narrows
  nothing, as the spec requires.
- Source-relative `$schema`:
  `composition::schema::load_effective_schema_in_context(source, fallback,
  Some(ctx))` (`lib/src/composition/schema/mod.rs:419`) returns
  `Option<EffectiveSchema>` from the document `$schema`. Darkmatter rejects
  remote `http(s)` `$schema` values up front (`SchemaError::RemoteUnsupported`),
  so this read never touches the network. Property names: see ruling 10.
  `load_effective_schema_in_context` is `pub(super)`, so Phase 6 needs a
  narrow public lib entry for the names.
- Cost: `InvocationContext::capture()` and source resolution already run
  before preparation in every composition command, so ownership adds no new
  read. Sequence uses the same pair.

**Spike B: structured-stream capture bounds.** Answered: bounded tails of
both streams can be had without keeping whole streams, but two paths need
plumbing.

- Streaming path (`commands/wrap/exec/spawn/semantic.rs`): stdout is kept in a
  bounded ring of `claudine::signals::EXIT_STDOUT_TAIL_LINES` (10) lines,
  but only fed to the signal hub's exit payload; it is not returned on the
  summary. Stderr is collected into `captured` with **no bound** (only when
  `suppress_stderr_on_success` or a stderr bridge is active) and returned as
  `summary.stderr_text`.
- Captured path (`commands/wrap/exec/spawn/captured.rs`,
  `run_child_capture`): both streams become full `String`s, bounded only by
  `claudine::runaway::CaptureVolumeCap` (the runaway volume cap).
- Interactive and passthrough launches inherit stdio, so nothing is captured.
  The typed input must then carry absent tails, and the classifier returns
  `None`.
- Reusable bound: `claudine::signals::exit_source_payload` already tails both
  streams with `EXIT_STDOUT_TAIL_LINES` / `EXIT_STDERR_TAIL_LINES` (10 each)
  through a private `tail_lines` helper in `lib/src/signals/bespoke.rs`.
  Default for Phase 3: build the typed native-exit input with those same
  constants (expose or move `tail_lines`), and return the streaming stdout
  ring on the summary instead of dropping it.

**Spike C: SimplifiedSchema and an `argv` `string[]`.** Answered: yes for
both.

- Checked with the installed `md schema validate` on temp fixtures:
  `argv: string[]` accepts `[alpha, beta]`; `argv: number[]` rejects
  `["alpha"]` (`argv/0 "alpha" is not of type "number"`); a scalar `argv:
  alpha` is rejected (`is not of type "array"`). Validation coerces scalars,
  so `argv: [1, 2]` also passes `string[]`. That does not matter here,
  because positionals are always strings.
- Non-persistence: `inline-compose` writes only the agent's on-disk delta plus
  the closure stamps. Transient `--set`, positional setters, sequence state,
  and `proxy.with` inputs are never written to the source (documented in
  `docs/topics/composition.md` under the completion verdict, and guarded by
  `cli/tests/l1/inline_completion_lifecycle.rs` near lines 863 and 938).
  Applying `argv` through the existing caller-overlay path inherits this.
  Phase 6 should still add an `argv`-specific non-persistence assertion.

**Spike D: generator extension point.** Answered: `cli_switches` is not read
anywhere today.

- No `cli_switches` reference exists in `claudine/gen`, `claudine/catalog-types`,
  or `claudine/lib`. It is research-only data in
  `docs/research/agent-cli/_schema.yaml:35`.
- Extension point: `gen/src/registry.rs` declares each projected field as an
  `entry(name, DeclaredSource::Research { topic, path }, &[SchemaExpectation],
  Coercion::…, description)`. `config_paths` (`registry.rs:447`, topic
  `agent-cli`) is the closest precedent. Phase 4 adds a `cli_switches`
  entry, a coercion under `gen/src/generate/coerce/`, an emitter, and a new
  vocabulary module in `catalog-types/src/` (alongside `resume_support.rs`,
  `offering.rs`, and others). `gen/src/schema_compat.rs` checks the
  expectations against the sidecar.
- Data quality now: Codex has `flag: --config`, `scope: ["global", "config"]`.
  The `-c` alias exists only in the prose table, so aliases need Phase 5
  research (see ruling 13 for scope).

### Baseline (from `claudine/`, macOS)

| Gate | Result |
| --- | --- |
| `just test` | 8075 passed, 9 skipped, 0 failed (exit 0, 66 s) |
| `just lint` | clean (exit 0) |
| `cargo run -p claudine-gen -- check` | clean (exit 0): catalog, signals, stream vocabulary, steering, Darkmatter name table, and families all match. One pre-existing warning: the `unchained-ai` models-catalog artifact is 87 days old (max 30). That predates this work and is not a drift failure. |

No pre-existing failures.

### Guards

| Guard | Location | Protects |
| --- | --- | --- |
| `wrap_direct_argv.rs` | `cli/tests/l1/` (`mod` at `main.rs:181`) | Exact child argv of direct wrappers (`opencode`, `goose`). Both tests are `#[cfg(unix)]`. R5 must leave these unchanged. |
| `argv_normalization.rs` | `cli/tests/l1/` (`main.rs:15`), 14 tests | The pre-clap normalization (provider booleans, `--help` hoisting) reaches the binary entrypoint without disturbing other commands. |
| `test_placement.rs` | `cli/tests/l1/` (`main.rs:164`), 17 tests | Inline `#[cfg(test)]` line budget; focus-stealing APIs only in `level3_*`; every `cli/tests/**/*.rs` compiled by a declared target (`autotests = false`). |
| `dispatch_inventory.rs` | `cli/tests/l1/` (`main.rs:63`), 12 tests | No new per-variant `match Provider` dispatch in `lib/src` or `cli/src`; regenerates and drift-checks `docs/providers/dispatch-inventory.json`. Phase 5–6 switch lookups must use generated data, not `match`. |
| `spawn_site_guard.rs` (Phase 7) | `cli/tests/l1/` (`main.rs:162`), 19 tests | Every L1 `claudine` spawn goes through `CliProcessFixture`; the allowlist is empty. |

### Test mapping for this phase

Phase 1 changes no behavior, so it adds no test. The baseline above is the
safety net for later phases.

## Phase 2

Phase 2 replaced the two parallel tail fields with one typed descriptor,
moved the forwarding notice to command-scoped state shared by composition and
the direct wrappers, fixed its wording, redacted the remaining argv display
surfaces, and refused non-UTF-8 tail tokens. The child argv is unchanged on
both launch paths.

### What landed

- **`claudine::composition::ProviderTail`** (new
  `lib/src/composition/provider_tail.rs`, re-exported from `composition`).
  Ordered tokens plus `boundary: Option<usize>` (`None` no `--`, `0` fully
  explicit, `len` authored empty suffix). Built with
  `ProviderTail::new(implicit, opaque: Option<Vec<String>>)`, so an
  out-of-range boundary cannot be constructed. Accessors: `launch_args()`
  (unredacted, for launching only), `boundary()`, `implicit_args()`,
  `opaque_args()`, `assignments()`, `is_empty()`. `Default` is the empty
  tail. `Debug` is hand-written and prints counts only.
  - `SwitchAssignment { switch, values: Range<usize> }` is the slot the plan
    asked for; it is always empty until Phase 6 fills it.
  - **Departure:** the plan said to add the descriptor to `types.rs`. It got
    its own module because `types.rs` is already ~1,050 lines and the
    descriptor brings its own tests; `types.rs` only holds the fields.
- **`ProviderTailNotices`** (same module): `Arc<Mutex<HashSet<key>>>` keyed
  `(provider, tokens, boundary)`. `claim()` locks, inserts, and releases
  before returning, so rendering never happens under the lock. Its `Debug`
  prints only the claim count.
  - **Departure (location):** ruling 3 put shared types in the lib. The
    registry lives there too, because the pipeline only sees the lib's
    `CompositionExecutionRequest`. Threading a CLI-only value through every
    `execute_composition_*` entry point (loop, staged, sequence, proxy) would
    have touched far more call sites than one request field.
- **Request plumbing:** `CompositionExecutionRequest.provider_args` and
  `provider_args_explicit` became `provider_tail` and
  `provider_tail_notices`. `SharedComposeArgs` carries the same two as
  `#[arg(skip)]` fields. The notice registry is created by clap's parse, so it
  lives as long as the top-level command. The three request builders
  (`compose/prep.rs`, `sequence/task_run.rs`, `sequence/iterate.rs`) clone
  both, so every attempt, retry, step, and parallel task shares one registry.
  The unused `InvocationInputsDraft.provider_args`/`provider_args_explicit`
  became one `provider_tail` field. The `commands/sequence.rs` and
  `compose/prep/tests.rs` helpers use `Default::default()`.
  `LaunchPlanInputs::provider_args_tail` stays `Vec<String>`, fed from
  `launch_args()`; the seeding order is unchanged.
- **CLI conversion in one place:** `argv::ProviderArgs` is gone.
  `partition_composition_tail` returns `ProviderTail` directly and
  `main.rs::inject_provider_tail` stores it.
- **Partition boundary:** the partition now records where an authored `--`
  split the tail (it used to discard the implicit prefix's identity once it
  saw `--`). Only the first `--` is consumed, as before.
- **Non-UTF-8 refusal (R6):** `to_string_lossy` is gone from the partition.
  A non-UTF-8 implicit or opaque tail token is
  `PartitionError::NonUtf8ProviderArgument { position }`, where position
  counts forwarded tokens from 1; the bytes are never echoed. A non-UTF-8
  token outside the tail is still left for clap. Direct wrappers already got a
  clap error, and a binary test now proves both paths refuse.
- **Notice module:** `cli/src/commands/wrap/composition/provider_args.rs`
  (and its process-wide `static ANNOUNCED`) is deleted. The replacement is
  `cli/src/commands/wrap/provider_tail_report.rs`: `announce(provider, tail,
  notices, silent, quiet, term)` plus `switch_names_for_display(tail)`, the
  single value-free name renderer. Wording:
  - implicit: `Forwarding provider arguments to Codex: -c`
  - explicit: `Forwarding an opaque argument tail to Codex (passed after --).`
  - mixed: `Forwarding provider arguments to Codex: -c, followed by an opaque
    argument tail (passed after --).`
  - an implicit tail with no switch-shaped token (direct-wrapper operands
    only): `Forwarding provider arguments to Codex.`
  - "not recognized by Claudine" is gone. A short token with attached text
    (`-csecret`, `-yq`) renders as `a short switch with attached text (not
    shown)`. Names are stripped of control characters and Prose-escaped
    (`Prose::escape_text` passes ESC sequences through, so it is not enough
    on its own).
- **Direct wrapper (R5):** `ExtractedWrapperFlags` gained `dash_boundary`
  (the consumed `--` position after flag removal), and
  `extract_prompt_source_from_passthrough` now also returns the indices it
  removed, so the boundary is shifted correctly when the prompt sat before it.
  `flags::passthrough_provider_tail` builds the descriptor from the
  post-extraction passthrough, for reporting only. The wrapper announces it
  right after the preflight preamble with its own registry.
  `wrap_direct_argv.rs` is untouched and green.
- **Redaction:** both `yolo applied to provider argv` debug traces (direct
  wrapper and composition pipeline) and the direct-wrapper `--dry-run`
  `Command:` line now pass through `redact_sensitive_args`. The composition
  dry-run "Provider args" row and `AGENT_PARAMS` already did; they now read
  from the descriptor.

### Findings worth knowing

- **The stderr trace formatter drops these fields.** `telemetry.rs`'s
  `RelativePathEventFormat` prints only selected fields (message, event,
  tool name/detail), so `child_args` never reached stderr even unredacted.
  A mutation check confirmed the binary test cannot see the trace field. The
  call-site redaction protects any subscriber that does print fields; the
  binary assertion guards the stderr surface as a whole. The dry-run
  redaction assertion did fail under mutation, as did notice dedup.
- **Direct wrappers now print the notice.** `claudine codex -- --version`
  prints the opaque-tail notice, which changed one snapshot
  (`l1__wrap_basics__wrapper_reports_removed_sensitive_env_names.snap`, one
  added line). This is the R5 behavior, not a regression.
- **Dispatch inventory regenerated.** The 11 `Provider::…` literals in the
  new notice unit tests are `reference`-class, `exempt_candidate` entries;
  the rest of the diff is line shifts in edited files. Regenerated with
  `CLAUDINE_UPDATE_INVENTORY=1`. No new dispatch.
- **Not yet hidden by any redaction:** `-csecret` style values in
  `AGENT_PARAMS` and the dry-run row. `redact_sensitive_args` masks known
  secret flags and credential-shaped values (`sk-…`), not arbitrary attached
  short values. The notice never echoes them. Phase 3 owns the binary
  secrets matrix (criterion 9) and should decide whether that is enough.
- `correlated_with_forwarded_tail` still takes `(switch_names, explicit:
  bool)` and keeps its "without recognizing" wording; Phase 3 rebuilds it and
  should take `&ProviderTail` and `switch_names_for_display`.
- **Doc drift fixed:** `docs/topics/argv-normalization.md` ended with
  "Reference: features `2026-04-17-cli-pre-processing` and
  `2026-07-13-cli-switches`". Docs must not name a feature or fix, so that
  line was replaced with a pointer to the test file. The same kind of
  reference remains in `cli-pre-parsing.md` (lines 18 and 362) and
  `composition.md`; those predate this work and were left for Phase 7's docs
  pass.

### Test mapping

| Behavior | Test (tier) |
| --- | --- |
| Descriptor shapes: implicit, explicit (`0`), empty suffix (`len`), mixed | `lib composition::provider_tail::tests::boundary_records_every_shape`, `default_is_empty_with_no_boundary` (L1 unit) |
| Redacted `Debug` (`sk-secret`, `-csecret`, `--token=` absent) | `provider_tail::tests::debug_never_prints_tokens` (L1 unit) |
| Distinct-pair dedup; clone shares; boundary in key | `notices_claim_each_distinct_pair_once`, `boundary_is_part_of_the_notice_key` (L1 unit) |
| Two commands in one process do not leak | `separate_registries_do_not_share_claims` (L1 unit) |
| Concurrent claims: exactly one winner | `concurrent_claims_admit_exactly_one_winner` (L1 unit) |
| Partition keeps boundary; only first `--` consumed; empty suffix | `argv::partition::tests::separator_after_implicit_prefix_keeps_the_boundary`, `authored_empty_suffix_is_preserved`, `only_the_first_separator_is_consumed` (L1 unit) |
| Non-UTF-8 refused by position (Unix bytes / Windows unpaired surrogate) | `partition::tests::non_utf8_*` (L1 unit, platform-gated fixture); `provider_tail_notice::composition_refuses_a_non_utf8_tail_token_without_launching` (L1 binary) |
| Direct wrappers refuse non-UTF-8 too | `provider_tail_notice::direct_wrapper_also_refuses_non_utf8_passthrough` (L1 binary) |
| Notice wording: implicit, explicit, mixed, empty suffix, no recognition claim, attached short, escaping, control chars | `wrap::provider_tail_report::tests::*` (L1 unit) |
| Notice through the binary; child gets tokens unchanged; secrets not echoed | `provider_tail_notice::compose_implicit_tail_notice_names_switches_and_forwards_tokens_unchanged`, `compose_explicit_and_mixed_tails_report_the_opaque_suffix` (L1 binary) |
| `--quiet`/`--silent` suppress | `provider_tail_notice::quiet_and_silent_suppress_the_notice_but_not_forwarding` (L1 binary) |
| Sequence steps + parallel tasks: one notice, tail once per launch | `provider_tail_notice::sequence_steps_and_parallel_tasks_share_one_notice_per_pair` (L1 binary) |
| Direct wrapper notice, quiet, prompt-only silence, consumed `--` boundary | `provider_tail_notice::direct_wrapper_announces_the_shared_notice`, `direct_wrapper_reports_the_suffix_after_its_consumed_separator_as_opaque` (L1 binary); `flags::tests::dash_boundary_counts_the_flags_removed_before_it`, `passthrough_provider_tail_splits_at_the_boundary` (L1 unit) |
| Dry-run row, direct dry-run command line, debug traces redacted | `provider_tail_notice::dry_run_and_debug_traces_redact_tail_secrets` (L1 binary) |
| Direct-wrapper child argv unchanged | `wrap_direct_argv.rs` (unchanged, green) |

The new binary file is declared in `cli/tests/l1/main.rs`; no test name
carries a tier marker, so `just test` runs all of them.

### Gates

| Gate | Result |
| --- | --- |
| `just test` (claudine/) | 8108 passed, 9 skipped, 0 failed (baseline 8075) |
| `just lint` (claudine/) | clean |
| Windows (native, `build-win-native` via `scripts/cross-check.sh claudine-cli --os windows provider_tail partition flags:: positional`) | 163 passed |

The Windows run proves the unpaired-surrogate fixtures compile and the
non-UTF-8 refusal holds there for both launch paths. It first flagged an
unused `SECRET` constant on Windows (used only by unix-gated tests); that
constant is now `#[cfg(unix)]`. Two notes for later phases:

- `just cross-check` re-splits a nextest filterset containing parentheses
  (`syntax error near unexpected token '('`), both in the root recipe and in
  the remote `_test` recipe. Pass plain name filters, or call
  `scripts/cross-check.sh` directly with them.
- `scripts/cross-check.sh` left every change in the working tree **staged**
  after it finished, despite its header saying the index is untouched. HEAD
  did not move. The index was reset with `git reset` (working tree kept), so
  nothing is staged. Check `git diff --cached` after a cross-check run.
- Pre-existing Windows warning, not from this work: unused constant
  `GENERATED_MARKER` in `cli/tests/l1/sequence_initialize_include_preflight.rs`.

Linux, WSL2, and macOS-CI were not run separately: the change has no
Linux-specific code, and every unix-gated test passed on macOS.

No pre-existing failures were found.

## Phase 3

Phase 3 made a lifecycle `resume` re-send the forwarded tail exactly once,
gave both launch paths one typed native-exit input and one report builder that
correlates an argument rejection with the tail, and added compiled-binary
coverage of every launch shape (R1, R2, R7).

### What landed

- **Resume (R1).** `resume::assemble_resume_args(entrypoint, base_args, tail)`
  replaces `append_resume_passthrough_args`. The resume argv is the resume
  entrypoint, then the tail once, then the allowlisted transport/safety flags.
  The allowlist reads Claudine's injections only: the base argv with the tail's
  first contiguous run removed. That works because every launch plan seeds its
  argv with the tail and later stages only prepend the entrypoint
  (`insert(0, …)`) or append. The invariant is documented on
  `LaunchPlanInputs::provider_tail`. If the run is ever not found, a
  `tracing::warn!` fires and the whole base argv is read, so no injection is
  lost. The allowlist's de-duplication now checks only the resume entrypoint
  and flags already carried. It no longer checks the tail, so a user `--json`
  never suppresses Claudine's copy of a flag the fresh launch also had.
  - `LaunchPlanInputs.provider_args_tail: Vec<String>` became
    `provider_tail: ProviderTail`. `RebuiltLaunchIdentity` and `AttemptLaunch`
    carry it, and `build_harness_launch` takes it.
  - `session_key.rs` module docs now name `assemble_resume_args` and say the
    tail is in both canonical argvs, so it cannot make a resume look
    incompatible.
- **Typed native-exit input (R2).** New `cli/src/output/native_exit.rs`:
  `NativeExit { exit_code, termination, stdout, stderr, shown_headline }`.
  Each tail is a `StreamTail { text, shown }` bounded by the lib's
  `EXIT_*_TAIL_LINES` (10). `claudine::signals::tail_lines` is now public for
  this. `Debug` prints lengths only. Evidence per path:
  - semantic (structured) spawn: a new stderr ring sits beside the existing
    stdout ring, returned as `ProcessResult::stream_tails`. stderr counts as
    shown (streamed live, or echoed when a suppressed run fails); stdout does
    not.
  - captured spawn (composition non-structured): both whole streams, bounded.
    stdout counts as shown when it produced a final message.
  - inherited spawn, Kimi wire, and the direct-wrapper document harness: exit
    code and termination only, so the failure stays unclassified (the spec
    allows this: "unavailable evidence leaves the failure unclassified").
- **Classifier (R2).** `classify_native_exit` reads both tails, stderr first.
  Precedence: interruption (termination, or exit 130/143) → timeout →
  missing binary (`LaunchFailed`, or exit 127 plus "not found"/"no such file")
  → auth → API (`API Error:`) → model → argument rejected → missing argument →
  `None`. Signatures were tightened:
  - dropped `invalid argument` (matched `invalid argument: api key`), the
    bare `required argument`, and the bare `authentication`. The last one
    turned `--authentication-mode` into an auth failure.
  - auth is now an explicit phrase list.
  - the named switch is taken from the matched line only.
  Every signature has a positive and a near-miss row.
- **One builder (R2).** `AgentErrorReport::for_native_exit(provider, exit,
  tail, model_source) -> NativeExitReport { report, correlated }`. It replaced
  `from_exit_code`, `from_exit_code_with_source`, `correlated_with_forwarded_tail`,
  and `classify_exit`, which removed both stale `#[allow(dead_code)]`
  allowances. It correlates only with a non-empty tail, a failed exit, and
  `ArgumentRejected` whose named switch (if any) is a tail token (bare,
  `--name=…`, or a short switch with attached text). Wording: "{P} rejected
  its arguments. This was likely caused by the forwarded arguments: {names |
  opaque}." No recognition claim. Provider text is masked by
  `provider_tail_report::tail_redactor`, which combines the shared
  recognizer with the values `redact_sensitive_args` masks in the tail. That
  uses the new lib API `Redactor::with_known_values`. Control characters are
  stripped and markup escaped. `attributes_to_tail` exposes the decision
  for a path that echoes output itself.
  - direct wrapper: `run_execution_stage` returns a `NativeExit`;
    `WrapperOutcome::AgentExited` carries it (boxed, for clippy's
    large-variant lint) plus the passthrough tail, and always renders the
    builder's report on non-zero exit (unchanged behavior for generic causes).
  - composition: rendered once, only when correlated, in
    `classify_attempt_phase` after `drive_terminal_recovery` returns
    `Completed`. A retried or resumed failure therefore never renders one.
    Other causes keep the existing failure reporting.
  - captured path: when the attempt will be correlated, the raw stderr echo
    is skipped. Every other captured stderr echo, and `AttemptOutcome::stderr_text`
    (which feeds the failure headline and `err.msg`), is masked against the
    tail when the tail is non-empty.
- **Redaction gap closed (criterion 9).** `redact_sensitive_args` now masks a
  credential-shaped value attached to a short switch (`-csk-…` becomes
  `-c****`). New `sensitive_arg_values` returns the original masked values for
  the echo redactor.

### Departures and decisions

- **No duplicate excerpt, via the failure headline.** The composition path
  already prints a per-attempt failure headline
  (`report_unhandled_failure`) built from the last stderr line. Left alone, a
  correlated report's excerpt repeated that line. `NativeExit::shown_headline`
  records the headline when it was shown (`show_checks`), and the builder drops
  an excerpt the headline contains, pointing at it instead. Under `--quiet`
  or `--silent` the headline is not shown, so the report carries the excerpt.
- **Plain `-csecret` stays visible in dry-run and `AGENT_PARAMS`.** The plan
  listed `-csecret` among the secrets. The spec's R7 list is `--api-key sk-…`
  and `--token=…`, and the spec says redaction "covers recognized secrets".
  Without switch metadata, an attached value cannot be told apart from a
  switch cluster (`-yq`). Masking every attached short token would hide real
  argv from the audit row. So only a credential-shaped attached value is
  masked. The notice never echoes any attached text, and the binary test
  uses `-csk-proj-…`. Phase 5 metadata can widen this.
- **Multi-provider sequence test uses a proxy.** Sequence steps share the
  sequence document's `agent` (`sequence/resolve.rs`), so a step's own `agent:`
  is ignored at launch. Step `b` reaches Claude by an `initialize` proxy to a
  document naming `agent: claude`. That is the supported way a sequence
  launches different providers.
- **The direct-wrapper document harness stays unclassified.** A direct
  passthrough that runs through the harness loop records an empty tail in its
  launch plan (`LaunchPlanInputs::recorded_only`). The loop therefore never
  correlates there, and the wrapper's after-loop report has an exit code but
  no evidence. Its resume behavior is unchanged: the spec keeps direct-wrapper
  parsing as it is. Child argv for direct wrappers is unchanged
  (`wrap_direct_argv.rs` green).
- **Plan/spec.** Criteria 1, 2, 7, 8, 9, 10, 11, 15, and 28 are marked Done in
  the spec table. Criterion 25 is split: the boundary and resume parts are
  Done, and the ownership checks wait for R9.

### Unexpected commit during this phase

While this phase was in progress, commits `778fea8bb` ("fix(claudine): replace
provider tail fields with typed ProviderTail…") and `6456ef433` ("planning:
record Phase 2 close") were made in this worktree by a process outside this
session. `778fea8bb` captured the working tree mid-phase, so it holds most of
Phase 3's source and tests under a Phase 2 message. After it, this phase also
changed `cli/src/commands/wrap/mod.rs` and `cli/src/output/error_report.rs`
(clippy fixes), the three topic pages, the two skill files, `plan.md`,
`spec.md`, and this log. Those are uncommitted. This session made no commit;
the commit history needs the author's review.

### Test mapping

| Behavior | Test (tier) |
| --- | --- |
| Resume argv: entrypoint, tail once, injections only; repeated switches; user `--json` neither dropped nor doubled; injection matching a tail flag still carried; tail not found | `commands::wrap::resume::tests::*` (6, L1 unit) |
| Resume through the binary (Codex `exec resume thread-7`, `--add-dir a --add-dir a --json` once) | `provider_tail_launch::a_resume_carries_the_tail_exactly_once` (L1 binary) |
| Retry / proxy target / each multi-provider sequence step carry the tail once; one notice per provider | `provider_tail_launch::a_retry_…`, `a_proxy_target_…`, `each_step_of_a_multi_provider_sequence_…` (L1 binary) |
| Exact headline argv, setter-shaped value not applied (`compose`, `inline-compose`, `sequence`); explicit `--` consumed | `provider_tail_launch::compose_forwards_…`, `inline_compose_…`, `sequence_forwards_…`, `compose_consumes_the_separator_…` (L1 binary) |
| Secrets (`--api-key sk-…`, `--token=…`, `-csk-…`) reach the child; absent from notice, debug, dry run, `AGENT_PARAMS`, correlated report (echo masked) | `provider_tail_launch::secrets_reach_the_child_and_no_display_surface` (L1 binary) |
| Classifier precedence, positive + near-miss per signature, stdout read, matched-line switch, bounded/redacted `Debug` | `output::error_report::tests::termination_decides_…`, `every_signature_has_a_positive_and_a_near_miss`, `auth_outranks_…`, `stdout_is_read_…`, `the_switch_comes_from_the_matched_line_only`, `evidence_is_bounded_…` (L1 unit) |
| Builder: correlated once; injected switch generic; operand-only explicit; attached/`=` forms; no misattribution (timeout, interrupt, auth, API, ambiguous, empty tail); masking + escaping; shown line or headline not repeated; generic and per-cause remediation kept | `output::error_report::tests::*` (L1 unit) |
| Correlation through the binary: captured stderr (once, quiet keeps excerpt, exit 2 preserved), structured stdout, streamed stderr (not repeated), operand-only explicit, and no correlation for injected/auth/API/ambiguous/interrupted/timeout | `provider_tail_launch::a_captured_stderr_…`, `a_structured_stdout_…`, `a_streamed_stderr_…`, `an_operand_only_explicit_…`, `other_failures_are_not_attributed_to_the_tail` (L1 binary) |
| `-csk-…` masked; masked values recovered for echo masking | `commands::wrap::env::tests::redact_sensitive_args_masks_a_credential_attached_to_a_short_switch`, `sensitive_arg_values_returns_each_masked_original_value` (L1 unit) |
| `Redactor::with_known_values` | `claudine secrets::tests::caller_known_values_are_masked_like_learned_ones` (L1 unit) |

`provider_tail_launch.rs` is `#![cfg(unix)]` (shell stubs), declared in
`cli/tests/l1/main.rs`, with no tier marker in any path segment, so `just test`
runs it. Mutation reasoning: before this phase, the resume test fails (old
allowlist dropped `--add-dir`), and every correlation test fails (nothing
called the builder).

The Input Robustness Matrix does not apply: the classifier reads free-form
provider prose, not a file format or configuration.

### Gates

| Gate | Result |
| --- | --- |
| `just test` (claudine/) | 8132 passed, 9 skipped. The first run had one LEAK-FAIL in the untouched lib test `composition::sequence::task::tests::shell_tasks::an_early_wait_error_still_reaps_the_whole_tree` (44 s under load). It passes in isolation, and the full rerun was green. |
| `just test-l2` (claudine/) | 277 + 3 passed |
| `just lint` (claudine/) | clean, after eliding two lifetimes and boxing `NativeExit` in `WrapperOutcome` |
| Windows native (`scripts/cross-check.sh claudine-cli --os windows error_report resume:: env::tests provider_tail launch_plan session_key`) | 126 passed; nothing left staged |
| Dispatch inventory | regenerated: 1733 → 1750 sites, all new entries `reference`-class (test fixtures in `error_report/tests.rs`; the rest are line shifts); `conditional` unchanged at 60 |

Linux and WSL2 were not run separately: nothing here is Linux-specific, and the
Unix-gated binary tests passed on macOS.

## Phase 4

Phase 4 added the revision-2 switch-metadata contract for the `agent-cli`
research topic, the shared vocabulary in `claudine-catalog-types`, and the
`claudine-gen` projection into a new `ProviderInfo::cli_switches` field (R8,
part 1). No research document was edited (ruling 12), so every compiled
provider generates an explicit `Unknown` gap until Phase 5 re-researches it.

### What landed

- **Contract.** `docs/research/agent-cli/_types.yaml` (new) defines
  `cli_switch`, `switch_scope`, `switch_value_type`
  (`none|string|number|variadic|unknown`), and `switch_attachment`
  (`space|equals|short_attached`), each property described (contract lint:
  35 properties, 0 problems). Fields: `flag`, `aliases`, `value` (placeholder
  only), `value_type`, `value_optional` (scalar only), `variadic_min`
  (integer ≥ 1 or `unknown`, variadic only), `attachment`, `invocation_scope`
  (`applies_to: global|command` plus an exact `command` path; `[]` is the
  root), the legacy `scope` labels (descriptive only, ruling 13), `default`,
  `description`, `example`, `notes`, `evidence_ids`, and `gap`.
  `_schema.yaml` now declares `schema_revision: literal(2; required)`,
  `versions_examined`, `evidence`, the typed `cli_switches`, and
  `cli_switches_gap` (required when the inventory is empty).
- **Revision-aware loading (ruling 12).** The previous contract is frozen,
  byte for byte, as `_schema.r1.yaml`. `inputs::validate_frontmatter` checks a
  document's `schema_revision` (absent means 1) against the `literal` its
  sidecar pins, and validates a mismatched document against
  `_schema.r<revision>.yaml` through Darkmatter's
  `effective_for_with_override`. A missing frozen contract fails with
  `GenError::ResearchRevisionUnsupported`. Legacy documents keep feeding
  `config_paths` unchanged.
- **Vocabulary.** `catalog-types/src/cli_switch.rs`: `CliSwitchCatalog
  { Researched(&[CliSwitch]) | Unknown { gap } }`, `CliSwitch`,
  `SwitchValue { None | String { optional } | Number { optional } |
  Variadic { min } | Unknown }`, `VariadicMin { AtLeast(u32) | Unknown }`,
  `SwitchAttachment`, `SwitchScope { Global | Command(&[&str]) }`. The serde
  form is the catalog shape. `SwitchValue::VARIANTS` and
  `SwitchAttachment::VARIANTS` are the research vocabularies the coercion
  checks against. The lib re-exports them from `provider::cli_switch` and
  `claudine::provider`.
- **Generator.** A new last registry entry `cli_switches` (research,
  `agent-cli/cli_switches`, `Coercion::CliSwitchRecords`). The coercion
  (`gen/src/generate/coerce/cli_switches.rs`) reads the topic frontmatter **as
  authored** (new `ProviderInputs::research_authored`), because Darkmatter's
  coercion turns `7` into `"7"` and treats `null` as absent. It distinguishes
  absent, null, and present for every load-bearing field. It rejects spellings
  two records share at a command path both accept (global meets every path;
  command entries meet only at an identical path), and it sorts records by
  canonical spelling and then by scope. The emitter (`gen/src/emit/cli_switches.rs`)
  turns the catalog shape into a `CliSwitchCatalog` literal and rejects any
  other shape, because overrides reach it directly.
- **Schema gate.** `schema_compat::load_sidecar_schema` inlines a top-level
  `name@./_types.yaml` object import so the `RecordArray` expectation checks a
  named record type. The expectation lists every field the coercion reads.
- **Regenerated.** All ten `data.rs` files (each `CliSwitchCatalog::Unknown`
  with the revision gap), `catalog.json`, both field-list guards (45 → 46), the
  registry source counts (research 11 → 12), and the generated-artifact byte
  baseline (11 changed pins). The `Researched` form was compiled once: Codex
  was generated from the revision-2 fixture into the real tree, `cargo check -p
  claudine` came back clean, and the committed files were restored
  (`claudine-gen check` clean afterwards).

### Departures and decisions

- **`cli_switches` is the last `ProviderInfo` field**, not next to
  `non_interactive_conflicting_flags`. Emission orders are numbered, so
  appending changes no other field's order.
- **Attachment is treated as load-bearing.** The plan's matrix lists five
  columns. `attachment` decides whether `-cfoo` or `--config=x` is one token
  (spec rule 6), so it gets a column too.
- **`variadic_min` accepts `unknown`.** Ruling 6 allows an unknown minimum.
  The plan's matrix makes an absent minimum an error. The contract satisfies
  both: absent or `null` fails, and an unknown minimum is written explicitly as
  `unknown` with a `gap`.
- **Matrix outcomes where the plan left a cell open.** Absent `aliases` means
  none (documented in the contract and on the topic page). An empty
  `invocation_scope` list is an error. A `command: []` path is the root and is
  valid. A global entry beside a command entry is an error, because global
  already covers every path.
- **The rest of the agent-cli contract is not narrowed.** The description
  lint still reports 25 problems on older properties (no description, quoted
  `{ … }` objects): `binaries`, `config_paths`, `subcommands`, and others.
  Revision 2 has no documents yet, so Phase 5 can still narrow them inside
  revision 2 before the fleet runs (see `message_to_agent`).
- **Fleet prompt untouched.** `_fleet.md` still describes revision 1. Updating
  it, piloting Codex, and re-researching are Phase 5. Until then, running
  `just research agent-cli` would write revision-1 documents that the
  generator turns into gaps.
- **CI visibility.** The fixture's new contract-file copies are spelled
  `manifest_dir!().join("../docs/…")`. Running `scripts/ci/test_inputs.py`'s
  `scan` confirms `_types.yaml`, `_schema.r1.yaml`, the shared `_types.yaml`,
  and the fixture map to `binary_id(claudine-gen::l1)`. The committed
  `agent-cli/_schema.yaml` and per-provider documents are still copied through
  `format!` paths in `pipeline.rs`/`generate_ux.rs` and through `area()` in the
  drift tests, so the index does not see them. That gap predates this phase
  and applies to every research topic.

### Input robustness matrix

One table-driven test (`cli_switches::every_matrix_cell_has_its_defined_outcome`,
63 cells) walks the revision-2 Codex fixture through the real pipeline. The
control row is `cli_switches::control_row_types_dash_c_as_a_string`. "Generator"
means the switch coercion itself refuses the shape (Darkmatter's shape check
accepts it), asserted by an error message naming the field. "Error" means any
gate refuses it.

| Shape | `value_type` | `value_optional` | `variadic_min` | `aliases` | `attachment` | invocation scope |
| --- | --- | --- | --- | --- | --- | --- |
| absent | error | generator (string/number) | generator (variadic) | valid, none (control row) | error | error |
| explicit null | error | generator (string, and on a none switch) | generator (variadic, and on a none switch) | generator | error | error; `command: null` generator |
| wrong type, whole | error (`[string]`, `3`) | error (`"no"`) | error (`"two"`, `1.5`) | error (`"-c"`) | error (`space`) | error (`global`) |
| wrong type, one element | n/a | n/a | n/a | error (`["-c", 123]`) | error (`[space, glued]`) | generator (`[exec, 7]`) |
| wrong type, every element | n/a | n/a | n/a | error (`[123]`) | error (`[glued]`) | generator (`[7]`) |
| empty | error (`""`) | n/a | error (`0`) | valid (`[]`) | generator (string); generator if set on none | generator (`[]` list); `command: []` = root, valid |
| duplicate key | error | error | error | error | n/a | error |
| duplicate value | n/a | n/a | n/a | generator (repeated spelling, alias = flag, alias claimed by another record where both apply) | generator (`[space, space]`) | generator (repeated entry; global beside a command) |
| not allowed here | error (not a member) | generator (on a none switch) | generator (on a string) | n/a | generator (`short_attached` without a short spelling) | generator (`command` on a global entry) |
| unknown | n/a | n/a | generator without `gap`; valid with one | n/a | n/a | n/a |
| trailing/invalid content | error (whole document) | | | | | |

`description` absent, empty, and whitespace-only are each refused (the
generator also trims before its own non-empty check).

Document-level cells: `schema_revision: null` and `schema_revision: 3` fail
(the second with no frozen contract to read); a document without revision 2
generates the gap (`a_revision_one_document_generates_an_explicit_gap`, which
also proves a missing `_schema.r1.yaml` is `ResearchRevisionUnsupported`);
`cli_switches: []` needs `cli_switches_gap`, and the gap beside records fails
(`an_empty_inventory_needs_a_stated_gap`).

Code smells grepped in the new reader and emitter: no `#[serde(default)]`, no
`unwrap_or_default()`, no `.ok()`, no `filter_map`. The only `Option` is the
output type's `gap` (absent and null are decided before it) and
`optional_text`'s return, which follows the three-way `Field` match.

Mutation check: making the coercion accept `null` for `value_optional`,
`variadic_min`, and `aliases` fails three cells. A first version of the test
accepted any coercion error and missed this. Each generator cell now names the
field its error must mention, and the null-aliases cell moved to `--oss`, where
reading `null` as empty would otherwise pass.

### Test mapping

| Behavior | Test (tier) |
| --- | --- |
| `-c` → `--config`, `string`, all three attachment forms, global; variadic min; root scope; absent aliases = none; `unknown` with gap; emitted Rust literal | `claudine-gen::l1 cli_switches::control_row_types_dash_c_as_a_string` (L1) |
| Every matrix cell above | `cli_switches::every_matrix_cell_has_its_defined_outcome` (L1) |
| Ordering determinism (catalog value and `data.rs` bytes) | `cli_switches::output_order_does_not_depend_on_document_order` (L1) |
| Same spelling at disjoint paths is valid; at a shared path conflicts | `cli_switches::a_spelling_may_repeat_at_disjoint_command_paths` (L1) |
| Empty inventory needs `cli_switches_gap`; gap beside records fails | `cli_switches::an_empty_inventory_needs_a_stated_gap` (L1) |
| Revision-1 document → explicit gap; no frozen contract → `ResearchRevisionUnsupported` | `cli_switches::a_revision_one_document_generates_an_explicit_gap` (L1) |
| Gate refuses a contract that drops a field the coercion reads | `cli_switches::the_gate_requires_every_field_the_coercion_reads` (L1) |
| Every compiled provider has records or a gap (real area) | `cli_switches::every_compiled_provider_has_switches_or_a_gap` (L1) |
| Drift of regenerated output | existing `drift::*` (L1) and `claudine-gen check` |
| Vocabulary names and serde catalog shape | `claudine-catalog-types cli_switch::tests::*` (2, L1) |
| Field lists | `registry_coverage::*`, `registry::tests::registry_matches_matrix_source_counts`, lib `provider::tests::serialized_field_list_matches_catalog` (L1) |

All new gen tests are in `gen/tests/l1/cli_switches.rs`, declared in
`tests/l1/main.rs`, with no tier marker in any path segment. The fixture is
read through `include_str!`.

### Gates

| Gate | Result |
| --- | --- |
| `cargo run -p claudine-gen -- check` | clean: 10 providers, catalog.json, signals, vocabulary, steering, agentic CLIs, families |
| `claudine-gen` + `claudine-catalog-types` nextest | 234 passed |
| `just test` (claudine/) | 8142 passed, 9 skipped, 0 failed |
| `just lint` (claudine/) | clean; one pre-existing linker warning (`__eh_frame section too large`) |
| Windows native (`just cross-check claudine-gen --os windows`) | 202 passed (includes every new `cli_switches` test and the `manifest_dir!().join("../…")` fixture copies) |

Linux and WSL2 were not run separately. The new code is pure data handling,
and its only path operations are `with_file_name` and manifest-relative joins,
which the Windows run covered. CI's Linux leg runs the same L1 binary.

## Phase 5

Phase 5 narrowed the rest of the `agent-cli` contract inside revision 2,
rewrote the fleet prompt with three gates, piloted Codex, re-researched all
ten providers, regenerated the catalog, and added the switch lookup and the
catalog-driven forwarding explanations (R8, part 2).

### What landed

- **Contract narrowed inside revision 2.** No document carried revision 2
  yet, so the remaining quoted-object properties became named types in
  `agent-cli/_types.yaml` (`binary_record`, `install_method`, `subcommand`,
  `config_path`, `env_var`, `introspection_command`), and the header gained
  `provider`, `contract_checked`, `reasoning_effort`, and the shared
  `research_agent`/`research_model`/`research_effort` enums. Field names are
  unchanged, so the `config_paths` coercion and its `RecordArray` expectation
  needed no edit. Contract lint: 25 problems → 0 (87 properties). A
  `subcommand.name` is a command path with words separated by one space
  (`exec resume`); the grammar refuses a literal space in a pattern, and
  `\x20` works.
- **Relations gate** (`agent-cli/_relations.py`, new). Researcher pairing and
  no self-research, provider matches file, unique evidence ids, every cited id
  exists, a typed switch cites evidence that is not inference alone, every
  evidence version is in `versions_examined`, subcommand paths are normalized
  and unique, each command scope names a listed subcommand, exactly one binary
  record per OS, and the body sections. It then runs
  `claudine-gen --area <claudine> validate <slug>` (see below) so the
  generator's own switch rules judge the document; nothing is mirrored in
  Python.
- **`claudine-gen validate <slug>`** (new subcommand). Generates one provider
  from the working tree and exits non-zero only when the generator refuses an
  input, printing the reason on stdout; drift from the committed `data.rs` is
  not a failure. The `research` recipe now builds `claudine-gen` before a
  fleet starts.
- **Fleet prompt** (`agent-cli/_fleet.md`) copied from the reasoning-level
  pattern: rotation, `only=`, `covering=`, `skip_research`, revision-aware
  refresh (`schema_revision == 2 && contract_checked` within 14 days), shape
  then relations gates in `success`, `contract_checked` stamp, cap-aware
  retry with the rejected findings shown to the researcher. Scope rule: every
  switch at the root and at every `non_interactive` path (including resume) is
  required; management-only paths may be omitted, which is safe because an
  unlisted switch reads as unrecognized. Attachment forms need parser source
  or a test whose result shows the value was read (`--help` proves nothing).
- **Pilot (Codex)** by Claude/Sonnet (the rotation assigns Codex to Claude):
  passed all three gates first time. 50 switches at Codex 0.159.3, 19
  evidence entries (source, local help, disposable tests). `-c` = `--config`,
  `string`, global, `space`/`equals`/`short_attached`. It found `--image`
  variadic at the root and `exec` but scalar at `exec resume`/`exec fork`.
- **Fleet.** All ten providers researched and confirmed on 2026-10-01, every
  one on its first attempt (no retries, no usage caps), about 75 minutes.

  | Provider | Researcher | Versions | Switches | unknown |
  | --- | --- | --- | --- | --- |
  | antigravity | opencode / glm-5.3 | 1.2.13, 1.2.14 | 33 | 0 |
  | claude | opencode / glm-5.3 | 2.1.287 | 129 | 0 |
  | codex | claude / sonnet | 0.159.3 | 50 | 0 |
  | gemini | codex / gpt-6.1-sol | 0.61.0, 0.62.0 | 152 | 0 |
  | goose | opencode / glm-5.3 | 1.52.0 | 78 | 0 |
  | kilo | codex / gpt-6.1-sol | 7.3.45, 7.8.3 | 175 | 1 |
  | kimi | claude / sonnet | 2.1.1 | 29 | 0 |
  | opencode | codex / gpt-6.1-sol | 1.18.33 | 156 | 2 |
  | pi | claude / sonnet | 0.87.1, 1.0.0 | 66 | 0 |
  | qwen | opencode / glm-5.3 | 0.24.7, 0.19.8 | 170 | 0 |

  No roster entry carries `skip_research: true`, so no compiled provider
  needed a stand-alone gap.
- **Regenerated** every `data.rs` and `catalog.json`; `claudine-gen check`
  clean; 11 byte-baseline pins re-blessed (xxh64 seed 0, computed with a
  throwaway pure-Python xxh64 that first reproduced the four unchanged pins).
- **Lookup API** (`lib/src/provider/cli_switch.rs`, re-exported from
  `claudine::provider`): `lookup_switch(provider, path, spelling) ->
  SwitchLookup { Known | NotInCatalog | CatalogGap { gap } }`,
  `SwitchLookup::value()` (anything unestablished is `SwitchValue::Unknown`),
  `match_switch_token(provider, path, token) -> Option<SwitchToken>` (exact
  spelling, then `--name=value` only for `equals`, then `-xvalue` only for
  `short_attached`), and `lookup_candidates(...) -> CandidateSwitch` keeping
  every `(provider, lookup)` arm with `agreed_value()`. Scope logic is
  `SwitchScope::applies_at` / `CliSwitch::{applies_at, spellings, accepts}` in
  `claudine-catalog-types`.
- **Message enrichment** (`cli/src/commands/wrap/provider_tail_report.rs`).
  `announce` now takes a `SwitchContext { provider, command_path }`, built by
  `SwitchContext::for_launch(profile, non_interactive)` from what the
  profile's own `apply_entrypoint` puts in front of an empty argv. After the
  unchanged INFO line, an `UnorderedList` gives one sentence per distinct
  implicit switch: `-c is Codex's --config switch (override one configuration
  value for this run); forwarding to Codex.`, or `--frobnicate: Claudine's
  compiled Codex switch catalog has no established type for it at its `exec`
  command; Claudine forwards it anyway.` A researched short-attached value is
  split off in the notice (`-csecret` → `-c`). Explicit tails stay opaque.
- **Guard.** `cli/tests/l1/switch_catalog_guard.rs` fails on a hand-written
  switch table in `lib/src` or `cli/src` (a `CliSwitch {` literal, a
  `[CliSwitch` array, or `CliSwitchCatalog::Researched(&`) outside generated
  `data.rs`, and checks that a generated `data.rs` still carries one.
- **Frozen revision-1 contract retired** from the docs tree (every document is
  revision 2). It and the last revision-1 Codex document (from `6a29f4f09`)
  moved to `gen/tests/fixtures/agent-cli-r1/`, so the revision mechanism and
  `ResearchRevisionUnsupported` stay tested; `pipeline.rs` no longer copies it.

### Departures and decisions

- **Contract narrowed inside revision 2** rather than a later revision 3, as
  the Phase 4 handoff suggested: one fleet run covered both.
- **The generator gate lives in the relations script, not the lifecycle.**
  The first prompt draft ran `cargo run -p claudine-gen` from `success`; the
  dry run refused it (`'cargo' is a dangerous command`), and an interpolated
  executable path is refused too. `python3` is a literal executable, so the
  script calls the pre-built binary. Recorded as a trap in
  `research-contracts.md`.
- **Entrypoint for messages comes from the profile, not the catalog.** Codex's
  `entrypoints` list puts `review` first for interactive mode, but its profile
  launches the root command; reading the catalog would have looked switches
  up at the wrong path.
- **Explanations are a list under the existing line**, not a replacement:
  the Phase 2 wording, keys, and dedup are unchanged, and each item carries
  the spec's sentence. The description is cut at its first `. ` or `; ` so it
  reads inside parentheses (Codex's is one long sentence).
- **The correlated error report still uses the metadata-free summary.** Its
  builder does not know the launch's command path; Phase 6 can pass a
  `SwitchContext` through when per-spawn checks land.
- **Guard needles are constructions only.** A first version also matched
  `SwitchValue::String` etc. and failed on the lookup's own `match` arm;
  Phase 6 ownership must match on values, so patterns are allowed.
- **Re-research also refreshed `config_paths`** (same field, new research):
  for example Gemini dropped credential/state files and gained `.env`
  locations, and Kilo gained its `kilo.json`/`opencode.json` config files. All
  L1 tests pass with the new paths; no consumer test pinned the old lists.
- **Dispatch inventory regenerated**: 1750 → 1762 sites, all `reference`
  class (`Provider::Codex` in tests); `conditional` stays 60.
- **Commits outside this session.** As in Phase 3, a process outside this
  session committed this phase's work while it was in progress (`70e6584f0`
  through `7cca26546`). History was not rewritten; the working tree is
  authoritative, and the remaining edits are uncommitted.

### Test mapping

| Behavior | Test (tier) |
| --- | --- |
| Narrowed contract refuses each probed shape (model `default`, effort, provider case, version, URL, unknown keys, `os: all`) | probed by hand with `md schema validate` (log above); the gen matrix test still walks every switch cell on the updated fixture |
| `validate` accepts drifted inputs, refuses a conflicting alias with the reason on stdout, writes nothing | `claudine-gen::l1 generate_ux::validate_accepts_drifted_inputs_and_reports_a_refusal_on_stdout` (L1) |
| Revision-1 document → gap; missing frozen contract → `ResearchRevisionUnsupported` (fixtures) | `cli_switches::a_revision_one_document_generates_an_explicit_gap` (L1) |
| Scope applies at global or identical path; spellings; attachment | `claudine-catalog-types cli_switch::tests::a_scope_applies_at_global_or_its_identical_path` (L1) |
| Exact spelling/alias lookup, no prefix/case/attached match | `claudine provider::cli_switch::tests::a_spelling_or_alias_finds_its_record_exactly` (L1) |
| Command-path scoping incl. resume path | `…::a_record_applies_only_at_its_command_paths` (fixed data), `…::codex_resume_entrypoint_has_its_own_answers` (real catalog) (L1) |
| Gap is not empty; unknown never none | `…::a_catalog_gap_establishes_nothing` (L1) |
| Attached forms split only where researched | `…::a_token_splits_only_in_researched_attachment_forms` (L1) |
| Candidate arms kept; agreement only on equal values | `…::candidates_keep_each_answer_and_agree_only_on_equal_values` (L1) |
| Criterion 14: Codex `-c` = `--config`, `string`, enriched message | `claudine-cli provider_tail_report::tests::codex_dash_c_is_explained_from_the_compiled_catalog` (L1); drift rejected by `drift::committed_data_matches_regenerated_inputs` and `claudine-gen check` |
| Launch entrypoint is the profile's | `…::the_switch_context_is_the_launch_entrypoint` (L1) |
| `-csecret` → `-c` only with research | `…::a_researched_attached_value_is_split_off_its_switch` (L1) |
| Unrecognized wording, no rejection claim, root phrasing | `…::an_unrecognized_switch_is_forwarded_without_a_rejection_claim` (L1) |
| Explicit tail never explained | `…::an_explicit_tail_is_never_explained` (L1) |
| End to end through the binary (compose and direct wrapper, quiet) | `claudine-cli::l1 provider_tail_notice::each_forwarded_switch_is_explained_from_the_compiled_catalog` (L1, unix) |
| No hand-written switch table | `claudine-cli::l1 switch_catalog_guard::switch_metadata_is_never_written_by_hand` (L1) |

New test files are declared: `switch_catalog_guard` in `cli/tests/l1/main.rs`;
`lib/src/provider/cli_switch/tests.rs` via `#[cfg(test)] mod tests;`. No path
segment carries a tier marker. Fixtures are read with `include_str!`.

### Gates

| Gate | Result |
| --- | --- |
| `just research agent-cli only=codex` (pilot) | confirmed, first attempt |
| `just research agent-cli` | ✓ every provider researched and confirmed |
| `cargo run -p claudine-gen -- check` | clean (10 providers and every artifact) |
| `claudine-gen` + `claudine-catalog-types` nextest | 236 passed / 33 passed |
| `just test` (claudine/) | 8157 passed, 9 skipped, 0 failed |
| `just lint` (claudine/) | clean; only the pre-existing `__eh_frame` linker warning |
| Windows native (`just cross-check claudine-gen --os windows`) | 203 passed (every gen L1 test, including `validate` and the revision-1 fixtures) |

Linux and WSL2 were not run separately, and `claudine-cli` was not
cross-checked: the new CLI code is string handling over static data with no
path or platform branch, and the new end-to-end test is `#[cfg(unix)]` like
its siblings (the recording fake provider is a shell script). The fleet
itself ran on macOS only; `_relations.py` appends `.exe` to the generator
path on Windows but was not exercised there. CI covers the Linux and macOS
L1 legs.

## Phase 6

Phase 6 replaced the "first unowned switch starts the tail" partition with
type-aware ownership (R9): a pure ownership function in the library, the CLI
half that reads the authored snapshot and the candidate providers, `argv`
positionals, the ambiguity prompt and error, and the resolved-provider check at
ownership, at preflight, and before every spawn.

### What landed

- **Pure core** (`lib/src/composition/ownership.rs`, re-exported from
  `claudine::composition`). `own_arguments(&ArgumentsAfterFile,
  &SchemaParameters, &[OwnershipCandidate]) -> Result<OwnedArguments,
  OwnershipError>` applies rules 1–9 left to right. Each forwarded switch opens
  a value run with one `Arm` per candidate (value type, attached, accepts a
  separate value), read through `match_switch_token` at that candidate's
  command path; a bare word is taken when every arm takes it, left as a
  positional when none does, and is `Ambiguous` otherwise. Rule 3 (`key=value`
  as a first value of a string/variadic switch in space form, for at least one
  arm) keeps its explicit precedence and is never an ambiguity. The result is
  Claudine's setter/positional tokens in order and a `ProviderTail` carrying one
  `SwitchAssignment` per implicit switch (`ProviderTail::with_assignments`,
  crate-private). `check_launch_tail(tail, provider, path)` is the one
  resolved-provider check (missing value, extra value, too few values, attached
  only); ownership fails with it only when every candidate fails. Both read a
  `SwitchSource`, so the tests can supply a fixed catalog
  (`provider::match_token_in` is now `pub(crate)` and re-exported for tests).
- **`ArgumentsAfterFile` / `CallerArgument`**: the partition's output. A removed
  Claudine option leaves `CallerArgument::ClaudineOption`, which closes any open
  value run, so `--codex -c phase=2 x=y` (with `phase` declared) fails instead
  of attaching `x=y`.
- **Authored snapshot** (`composition::authored_schema_parameters`): the literal
  `$schema` through the composer's own `load_effective_schema_in_context`
  (source-relative, no baseline/trigger layers, no templates or shell, remote
  refused by Darkmatter). Names come from Darkmatter
  `EffectiveSchema::declares_top_level_property`, now `pub` (ruling 10;
  `darkmatter` joins `packages`). A string or union-arm `$schema` holding `{{`
  or `$(` is `SchemaParameters::Unestablished`; a contested setter is then
  `ContestedSetter`. A load failure is an error.
- **Partition** (`cli/src/argv/partition.rs`): returns `(claudine argv,
  ArgumentsAfterFile)`. Claudine options (anywhere before `--`), setters before
  the file, and the file go to clap; every other token after the file is held.
  Non-UTF-8 after the file is `PartitionError::NonUtf8Argument { position }`
  (position counted after the file). Tests moved to `partition/tests.rs`.
- **CLI half** (`cli/src/commands/compose/ownership.rs::own_caller_arguments`),
  called by `compose`/`inline-compose` (`prep.rs`) and `sequence`
  (`sequence.rs`) right after the file resolves and before setters merge.
  Candidates: `--provider`, else literal `agent` (a templated entry keeps every
  provider), else every provider; each at
  `SwitchContext::for_launch(profile, !interactive)` with interactivity from
  `-i`/`--no-interactive` and the literal `interactive` hint. The schema is read
  only when a setter-shaped token exists; with no token at all nothing is read.
  On `Ambiguous` with more than one candidate and
  `resolve_interactive_options(silent).allowed()`, a `ChooseOne` prompt (same
  inline viewport as the provider picker) asks which agent the arguments are
  for; the answer narrows the candidates for ownership only.
- **`argv`** (`setters.rs`): `parse_composition_positionals` collects every
  non-setter after the file as `positionals` (the multiple-file error is gone);
  `merge_set_overrides(set, shorthand, positionals)` inserts `argv` as a string
  array only when there is at least one positional, and one validator
  (`reject_reserved_key`) refuses `argv` from a setter or a `--set` key. Sequence
  steps and proxy targets receive it through the existing caller overlay;
  `inline-compose` never persists it.
- **Resolved-provider check sites**: `SwitchContext::check` (new) in
  `composition/pipeline.rs` before the notice (the command's preflight, before
  `start`); in `wrap/sequence/mod.rs` for every review-eligible step whose target
  is resolved before step 1 (skipped under `--dry-run`); and in
  `harness_orch/launch.rs::build_harness_launch` before every spawn, with a
  resume checked at `SwitchContext::for_resume` (the resume entrypoint's words,
  minus the session id). A failure is a plain error before the spawn and never
  reaches `AgentErrorReport::for_native_exit`.
- **`looks_like_setter`** now delegates to `composition::setter_key`; its doc
  says shape decides ownership only before the file.

### Departures and decisions

- **An unresearched `--name=value` takes nothing more.** Rule 6 read literally
  would make any unmatched token unrecognized and let rule 5 give it the next
  bare word; a long switch with `=` already holds its value, so it does not.
  Unmatched short tokens (`-cfoo` for Claude) still take the next bare word, per
  rule 5.
- **The mismatch error shows the token redacted.** The spec asks the error to
  name the token. It is shown after a secret-named switch as `****`, with
  recognized secrets masked and control characters escaped. The ambiguity error
  never echoes the word.
- **Schema names are read lazily** (only when a setter-shaped token is present),
  so a document whose `$schema` cannot load still fails where it always did when
  nothing needs classifying.
- **`--exclude` does not shrink the candidate set**; the spec names only the CLI
  provider, `agent`, and every provider.
- **One prompt answer per invocation.** It narrows the candidates to the chosen
  provider for every remaining switch.
- **Ownership errors are plain `std::error::Error` values** (like
  `PartitionError`) carried as `eyre::Report::new`, not new
  `CompositionError` variants or catalog codes. `error_guards` requires
  `Report::new` rather than `eyre!` interpolation.
- **`lookup_candidates`/`CandidateSwitch` (Phase 5) are unused by ownership.**
  Ownership needs each arm's attached form and space acceptance, not just the
  value type, so it matches every candidate with `match_switch_token`. The pair
  is now used only by its own tests; a reviewer may want it removed.
- **Existing tests updated because behavior changed on purpose**:
  `wrap_compose_validation::compose_multiple_file_candidates_errors` became
  `compose_second_bare_word_is_a_positional_not_a_file`; the L1 and L2
  "unstructured error" controls now use an `argv=x` setter (a second bare word
  is no longer an error); the non-UTF-8 notice test asserts the new position
  wording; `each_step_of_a_multi_provider_sequence_carries_the_tail_once` now
  forwards `--add-dir x` (read the same way by Codex and Claude) because
  `-c x=y` handed to Claude now fails before the spawn, which the new
  `a_proxy_to_a_provider_that_types_the_tail_differently_fails_before_its_spawn`
  asserts.
- **Dispatch inventory regenerated**: conditional 60 → 62. Both new conditional
  sites are in `lib/src/provider/cli_switch/tests.rs`, a test file that was
  already modified before this phase began; the ownership tests pass explicit
  command paths rather than matching on `Provider`.
- **Docs updated now, not in Phase 7**: `argv-normalization.md` (partition
  rewritten, new "Type-aware ownership" section with a flow diagram),
  `composition.md` (positional arguments and `argv`, provider forwarding),
  `cli-pre-parsing.md`, and `frontmatter-properties.md` (`argv` row), because the
  docs tree must not describe the retired rule. Completion docs are untouched:
  completion still uses its old scan until Phase 7.

### Input robustness (token reader)

| Shape | Switch value | Type metadata | Schema parameter names | `argv` key |
| --- | --- | --- | --- | --- |
| absent | missing value error (`a_dash_value_must_be_attached`, `every_candidate_must_be_wrong…`, binary `…interrupting_a_value_run…`) | unknown, takes a bare word (`unknown_is_never_none`) | no `$schema` → rule 3 (`no_schema_declared_names_and_unestablished_names_are_three_outcomes`) | authored left alone (`bare_words_become_argv…`) |
| explicit null | n/a | researched `unknown` record reads as unrecognized (`unknown_is_never_none`, opencode `--get-yargs-completions`) | n/a (names only) | `argv=null` refused (`argv_is_reserved…`, `positionals_reject_an_argv_setter`) |
| wrong type, whole | `--max-turns abc` not taken → missing value (`a_number_switch_takes_only_a_number`) | n/a | unreadable/templated schema → error (`an_unreadable_schema…`, `a_templated_schema…`) | `argv=[1]` refused (`argv_is_reserved…`) |
| wrong type, one element | variadic run ends at a setter (`a_variadic_switch_takes_a_contiguous_run…`) | n/a | name in one union arm counts (`schema_names_come_from_a_source_relative_union`) | n/a |
| empty | `""` and `--config=` kept (`an_empty_value_is_a_value`) | n/a | empty schema ≠ no schema ≠ unestablished (three-outcomes test) | no positionals ≠ positionals (`merge_sets_argv_from_positionals_only_when_there_are_some`) |
| duplicate | repeated scalar = two assignments (`a_repeated_scalar_switch_is_two_assignments`) | duplicate alias rejected at generation (Phase 4) | — | duplicates kept in order (`bare_words_no_switch_takes_are_positionals_in_order`, `positionals_after_the_file_are_collected_in_order`) |
| trailing/invalid | non-UTF-8 refused by position (partition tests, `composition_refuses_a_non_utf8_tail_token…`); only first `--` consumed (`only_the_first_separator_is_consumed`) | n/a | nested raw-schema property is not a top-level name (`a_raw_json_schema_contributes_its_top_level_names`) | after `--` it is provider data (`argv_cannot_be_set_by_name`) |

Control row: `a_string_switch_takes_one_setter_and_the_next_setter_is_claudines`
(lib) and `a_string_switch_takes_one_setter_and_a_schema_parameter_stays_claudines`
(binary), `--codex -c model_reasoning_effort=low phase=2`.

Grep smells checked: no `#[serde(default)]`, `filter_map(.. as_str())`,
`unwrap_or_default()`, or `.ok()` on a load-bearing parse in the new code. The
one `.ok()` (`parse_selection_hints_from_frontmatter(..).ok()` in
`candidates`) is deliberate: an invalid `agent` keeps every provider a
candidate and ordinary preparation reports the diagnostic, as the spec
requires.

### Test mapping

| Behavior (criterion) | Test (tier) |
| --- | --- |
| Rules 1–9, union readings, attached forms, numbers, empty values, repeated switches, opaque suffix, resume-path answers, secrets in errors, redacted `Debug`, variadic minimum and unknown minimum (fixed catalog) (4, 16–18, 21, 22, 24–26) | `claudine composition::ownership::tests::*` — 28 tests (L1) |
| Partition: what reaches clap vs ownership, markers, boundary, ordering errors, non-UTF-8 by position, partition + ownership exact tokens (13, 15, 25) | `claudine-cli argv::partition::tests::*` (L1) |
| `argv` collection, reserved key from setter and `--set`, strings not JSON5 (23) | `claudine-cli commands::compose::tests::{positionals_after_the_file_are_collected_in_order, positionals_reject_an_argv_setter, merge_sets_argv_from_positionals_only_when_there_are_some, merge_rejects_argv_in_set_json}` (L1) |
| Spec's three example commands through the binary (16, 17, 18) | `provider_tail_ownership::{a_string_switch_takes_one_setter…, with_no_provider_named…, a_variadic_switch_takes_a_run…}` (L1, unix) |
| Resolved-provider check at preflight, sequence before step 1, resume entrypoint, proxy before spawn (21) | `provider_tail_ownership::{a_resolved_provider_that_takes_no_value…, a_sequence_fails_before_step_one…, a_resume_is_checked_at_the_resume_entrypoint}`, `provider_tail_launch::a_proxy_to_a_provider_that_types_the_tail_differently…` (L1, unix) |
| Ambiguity error and candidate narrowing; snapshot `agent=codex`; templated `agent` (19, 20, 29) | `provider_tail_ownership::{a_disagreement_without_a_terminal…, a_caller_agent_setter_does_not_narrow…, a_templated_agent_keeps_every_provider…}` (L1, unix) |
| Ambiguity prompt; answer decides ownership only (20) | `level1_ownership_prompt_pty::{…choosing_codex_forwards_the_word…, …choosing_claude_decides_ownership_only}` (L1 PTY, unix) |
| Source-relative union schema, raw JSON Schema top-level names, unreadable and templated schema, `--set $schema` does not change ownership (29) | `provider_tail_ownership::{schema_names_come_from_a_source_relative_union, a_raw_json_schema_contributes_its_top_level_names, an_unreadable_schema…, a_templated_schema…, a_caller_schema_override_does_not_change_ownership}` (L1, unix) |
| `argv` order, override, `--` excluded, reserved, through sequence steps and a proxy target, never persisted by inline (23) | `provider_tail_ownership::{bare_words_become_argv…, argv_cannot_be_set_by_name, argv_reaches_every_sequence_step_and_a_proxy_target, inline_compose_never_persists_argv}`, `wrap_compose_validation::compose_second_bare_word_is_a_positional_not_a_file` (L1) |
| Help opens no file (27) | `provider_tail_ownership::help_opens_no_composition_file` (L1, unix) |

New files are declared: `provider_tail_ownership` and
`level1_ownership_prompt_pty` (both `#[cfg(unix)]`) in `cli/tests/l1/main.rs`;
`partition/tests.rs` and `ownership/tests.rs` via `#[cfg(test)] mod tests;`.
No path segment carries a tier marker (`level1_` is not one).
`just check-tier-coverage claudine`: 0 stranded.

### Gates

| Gate | Result |
| --- | --- |
| `just test` (claudine/) | 8211 passed, 9 skipped, 0 failed |
| `just test-l2` (claudine/) | 277 passed (+3 gen); tmux sessions in the background |
| `just lint` (claudine/) | clean (only the pre-existing `__eh_frame` linker warning) |
| `cargo clippy -p darkmatter --lib` | clean |
| `cargo run -p claudine-gen -- check` | clean |
| `just check-tier-coverage claudine` | 0 stranded |
| Windows native (`just cross-check claudine --os windows`) | 4610 passed, 7 failed; all 32 `composition::ownership` tests pass |

The seven Windows failures are path-spelling and home-resolution failures in
code this phase did not change, so none is attributed to it:
`coordinator::tests::commit_records_only_handoffs` ×2 (a `\\?\` verbatim
prefix on one side of a path comparison), `resolve::tests::{resolve_missing_file,
detailed_resolution_preserves_non_no_match_typed_errors}`,
`schema::tests::shipped_implement_plan_prepares_with_unset_optional_commit_message`
(`/` vs `\` in a rendered `git add` path), `invocation_context::tests::an_external_prompt_opened_through_home_takes_home_as_its_tree_root`
(`~` resolves to the real profile), and
`sequence::task::tests::side_effect_tasks::a_failed_nested_mapping_set_projects_one_path_and_commits_nothing`.
Their only overlap with this phase is that `schema/mod.rs` gained a new
function. CI's push-to-`main` Windows leg is where they are fixed forward. The
CLI half was not cross-checked: it is string handling with no path or platform
branch, and its new binary tests are `#[cfg(unix)]` like their siblings; the
partition's non-UTF-8 test has a Windows (`from_wide`) fixture that CI's
Windows leg compiles.
