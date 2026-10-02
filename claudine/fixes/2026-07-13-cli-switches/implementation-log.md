---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-schema-enhancement/claudine/fixes/2026-07-13-cli-switches/spec.md"
plan: "claudine/fixes/2026-07-13-cli-switches/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
packages: []
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
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
