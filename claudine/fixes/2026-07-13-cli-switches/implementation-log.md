---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-schema-enhancement/claudine/fixes/2026-07-13-cli-switches/spec.md"
plan: "claudine/fixes/2026-07-13-cli-switches/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
packages:
    - claudine
    - claudine-cli
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
