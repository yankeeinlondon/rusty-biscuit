# Phase 1 consumer audit

Audit date: 2026-08-27
Baseline commit: `a114c1448234971cc4d399d69207250abc77d477`

## Dependency gate

The prerequisite `claudine/fixes/2026-08-12-ctx-launch-anchor` has not been
moved to `_completed`. Its implementation tasks and review findings are
complete, and this feature's explicit Phase 1 implementation request is the
owner's go-ahead to proceed with the audit and package-local work. It is not a
waiver of the prerequisite's deferred validation gates.

The prerequisite log records these environment results:

- macOS L1, L2, and lint passed; L2 focus sampling observed no terminal or
  browser focus change.
- WSL passed the complete affected matrix during review cycle 2, but a later
  cold snapshot run exhausted its filesystem before tests ran.
- Linux remains deferred because the build host repeatedly timed out before
  the exact current snapshot could execute.
- Native Windows remains deferred because no visible volume satisfies the
  repository's non-bypassable 50 GiB free-space preflight.
- Hosted CI remains deferred because the workflows accept only published Git
  refs and this work does not authorize committing or pushing.

Ruling: those Linux/native-Windows/hosted-CI gates do not prevent Phases 1-7
from proceeding, but Phase 8 cannot close until its cross-platform matrix and
the prerequisite's outstanding AC13/AC14 evidence are reconciled. WSL should
also be rerun against the final Phase 8 tree rather than relying on the older
review-cycle result.

## Baseline gates

The first `darkmatter/just test` invocation exposed a baseline feature-gating
regression: every ordinary L1 integration binary compiled
`cli/tests/common/level2.rs`, although `biscuit-test-harness` is optional and
enabled only by `terminal-tests`. The original failure was eight unresolved
`biscuit_test_harness` imports while compiling L1 binaries such as
`layout_flags`, `clean_json`, and `compose_basic`.

`darkmatter/cli/tests/common/mod.rs` now gates `pub mod level2` with
`#[cfg(feature = "terminal-tests")]`. The exact failing invocation was rerun
and passed. This keeps the L2 module reachable from all existing
`required-features = ["terminal-tests"]` test binaries without pulling the
terminal harness into L1.

| Area | Gate | Result |
|---|---|---|
| biscuit-file | `just test` | 794 nextest tests passed; 6 no-default-feature tests passed |
| biscuit-file | `just lint` | passed for library and CLI |
| darkmatter | `just test` | 7,533 passed; 50 skipped |
| darkmatter | `just lint` | passed for library, CLI, and DMLS |
| claudine | `just test` | 6,653 passed; 11 skipped across catalog-types, library, contract, CLI, and generator |
| claudine | `just lint` | 18 diagnostic guards passed; lifecycle guard and all five package Clippy gates passed |

These are macOS host results. No L2 test was run in this phase.

## GitNexus blast radius

The index is `rusty-biscuit`. Each requested symbol was queried upstream with
tests included. A not-found result is recorded as unknown and supplemented by
the source inventories below; it is never treated as zero impact.

| Symbol | Risk | Upstream summary |
|---|---:|---|
| `FileReferenceKind` | LOW | indexed enum; 0 graph edges (source consumers exist below) |
| `RootProvenance` | LOW | indexed enum; 0 graph edges (source consumers exist below) |
| `FileReferenceError` | UNKNOWN | not indexed by name; exhaustive source consumers recorded below |
| `candidate_plan` | LOW | indexed; 0 graph edges (source consumers exist below) |
| `resolve_in_context` | UNKNOWN | method not indexed by free name; source consumers recorded below |
| `complete_partial` | LOW | 3 direct test callers; no indexed process |
| `document_resolution_context` | CRITICAL | 10 direct, 23 depth-2, 66 depth-3; 99 total across 11 modules |
| `find_package_area_from` | CRITICAL | 5 direct, 17 depth-2, 21 depth-3; 43 total; two compose/transclusion processes |
| `find_git_root_from` | CRITICAL | 27 direct, 62 depth-2, 138 depth-3; 227 total; seven processes |
| `derive_source` | CRITICAL | 21 direct, 20 depth-2, 8 depth-3; 49 total; sequence execution affected |
| `prompt_magic_roots` | UNKNOWN | not indexed by name; source consumers recorded below |
| `build_child_env` | UNKNOWN | not indexed by name; source consumers and spawn census recorded below |

Direct-call review for the CRITICAL/HIGH surfaces:

- `document_resolution_context`: expression file resolution and missing-file
  shaping, absolute link resolution, reference graph options/transclusion,
  schema detection/resolution, and the transclusion resolver. Its two direct
  seam tests are also callers.
- `find_package_area_from`: both `ComposeOptions` expression-context builders,
  reference graph options/transclusion, and `package_area_for_reference`.
- `find_git_root_from`: Darkmatter CLI schema trigger/validate, DMLS trigger
  boundary, context options/catalog tests, expression path and skill helpers,
  file-links boundary discovery, link normalization/resolution, schema
  validation/clean/detect/format/resolve/rewrite, shell-policy store,
  transclusion, `document_resolution_context`, display abbreviation, and
  `claudine-gen::generator_schemas`.
- `derive_source`: system-prompt capture; wrapper detection; sequence graph,
  JIT, task, and proxy-loop entry; plus its invocation-context work-bound and
  topology tests.

The indexed process catalog was reviewed. Relevant named flows include
`Compose_preflight_approvals`, `Run_compose_pipeline`,
`Run_compose_pipeline_internal`, `Run_transclusion_phase`, `Run_subcommand`,
`Run_stage`, `repair_frontmatter`, and `execute_sequence`. The graph is weak
for Rust enum exhaustiveness, so the compiler/source inventory remains the
authority for the coordinated migration.

## Compiler-exhaustiveness and resolution consumers

The inventory was produced with exact Rust-token searches across
`biscuit-file`, `darkmatter`, and `claudine`. Darkmatter's unrelated Markdown
reference-graph `ReferenceKind` was excluded; “internal `ReferenceKind`” below
means biscuit-file's private file-reference enum.

### biscuit-file authorities and tests

- `biscuit-file/lib/src/file_reference/mod.rs`: public `FileReferenceKind`,
  private `ReferenceKind`, `RootProvenance`, constructors, conversions, and
  method surfaces.
- `biscuit-file/lib/src/file_reference/parse.rs`: `ReferenceKind` constructors
  and parse error creation.
- `biscuit-file/lib/src/file_reference/resolve.rs`: every internal-kind and
  public-kind match, `RootProvenance` construction, candidate planning,
  `completion_roots`, `complete_partial`, probing, and resolution errors.
- `biscuit-file/lib/src/file_reference/context.rs` and `error.rs`: context
  validation/discovery and the complete error enum.
- `biscuit-file/lib/src/lib.rs`: public re-exports and doctest surface.
- Grammar/resolution fixtures:
  `tests/reference_grammar.rs`, `tests/precedence_flip.rs`,
  `tests/detailed_resolution.rs`, `tests/resolution_context.rs`,
  `tests/completion_round_trip.rs`, and `tests/implicit_relative.rs`.

### external enum/error consumers

- Darkmatter file-reference-kind matches:
  `markdown/compose/remote.rs`, `markdown/compose/transclusion/resolver.rs`,
  `markdown/compose/util.rs`, and `markdown/schemas/resolve.rs`.
- Claudine file-reference-kind matches:
  `composition/sequence/source.rs` and `harness/error.rs`.
- Claudine root-provenance matches:
  `harness/error.rs` and `harness/resolve/tests.rs`.
- Darkmatter error wrapping/matching:
  `markdown/reference/errors.rs`, compose expression errors/functions/context,
  transclusion resolver/types/tests, schema errors/format/reference/resolve,
  and reference/transclusion error snapshots.
- Claudine error wrapping/matching:
  composition error/resolve/sequence, diagnostics discovery/snapshots,
  harness error/resolve and their exhaustive slug tests, plus diagnostic-detail
  conformance.

### candidate-plan, completion, and resolution callers

- `candidate_plan`: biscuit-file detailed/precedence tests; Darkmatter
  `compose/expression/resolve_ctx.rs` and `compose/link_resolve.rs`; Claudine's
  boundary lint asserts that the library does not bypass the higher-level seam.
- `completion_roots`: private to biscuit-file `resolve.rs`; all external
  completion flows enter through `complete_partial`.
- `complete_partial`: biscuit-file completion/implicit tests. Claudine CLI
  completion calls the `FileReference::complete` method surface through
  `claudine/cli/src/completion/scopes.rs`, so method-name-only searches are not
  sufficient for completion inventory.
- `resolve_in_context`: biscuit-file context/detailed/precedence/completion
  tests; Darkmatter expression, link, schema validation/resolution,
  transclusion, and document-context helpers; Claudine CLI sequence source;
  Claudine composition resolve/sequence and system-prompt resolve.

### known downstream surface checklist

The migration must restore all of these even where the source reaches the
resolver indirectly: Darkmatter library/CLI/DMLS schema resolution, rewrite,
detect, format, validation, and trigger discovery; transclusion; expression
path projection and git-root/skill functions; preflight and shell-policy
storage; TOC/link resolution and normalization; reference graphing; Claudine
library/CLI/generator sequence sources; harness diagnostics/resolution;
system prompts; and CLI composition completion.

Every external consumer of Darkmatter's public `find_git_root_from` is present:
`darkmatter/cli/src/commands/schema/{triggers,validate}.rs`,
`darkmatter/dmls/src/overlay/schema.rs`, and
`claudine/gen/src/inputs.rs` (with its pipeline fixture). The library's direct
consumers are listed in the GitNexus review above.

## Production process-spawn census for Phase 7

Scope is exactly `claudine/lib/src` and `claudine/cli/src`. Test modules,
test-only files, `clap::Command`, `claudine-contract`, and rendezvous crates are
excluded. Platform-specific production shell constructors are retained.

| Owner/site | Shape | Existing launch snapshot / Phase 7 route |
|---|---|---|
| `lib/model_catalog/provider_sources.rs::fetch_shell_command_models` | Tokio `Command`, inline `spawn` + `wait_with_output` | no launch snapshot; service boundary must receive the captured launch directory or an already-built contribution |
| `lib/dispatch/runner/bash.rs::execute_bash_action` | Tokio direct/interpreted commands, inline `output` | dispatch context has event/runtime context but no shared child-env contribution today; thread contribution into both arms |
| `lib/dispatch/runner/bash.rs::run_command_blocking` | Tokio `Command`, inline `output` | no launch snapshot parameter; extend this shared action helper rather than patch callers separately |
| `lib/harness/shell.rs::execute_approved_command` | Tokio `Command`, inline `spawn` | receives optional working directory, not immutable launch CWD; add launch contribution to approval/execution options |
| `lib/composition/lifecycle/executor.rs::SystemShellRunner` | helper-returned std `Command`, inline `status`; `cmd`/`sh` platform arms | trait currently receives only command text; thread the epoch/invocation launch contribution through execution context |
| `lib/composition/sequence/task/shell.rs::SystemTaskShell` | helper-returned std `Command`, inline `spawn`; `cmd`/`sh` platform arms | runner receives command/timeout/interrupt/live only; task execution already has source/invocation context upstream and must pass launch contribution without deriving it from source |
| `cli/commands/sequence.rs::expand_shell_source` | std `Command`, inline `output` | top-level command has invocation launch CWD; capture once and pass to the expansion closure/options |
| `cli/commands/providers.rs::{gen_command,run_gen_passthrough,run_agent_errors,render_mapping}` | helper-returned std `Command`, inline `status`/`output`; binary or Cargo fallback | top-level administrative command; apply one contribution inside `gen_command` so every terminal method inherits it |
| `cli/commands/init_wizard.rs::configure_tts` | std `Command`, inline `output` | interactive setup boundary knows its launch; pass contribution into the installer command |
| `cli/commands/config_tui/mod.rs::{query_say_voices,query_espeak_voices}` | std `Command`, inline `output` | config command boundary must capture/pass launch once; both query helpers need it |
| `cli/commands/wrap/exec/spawn/setup.rs::base_command` | helper-returned std `Command`; inherited/captured/semantic modes spawn it | already receives complete `EnvPlan.env` built from retained wrapper startup detection; `build_child_env` is the natural single insertion point |
| `cli/commands/wrap/exec/wiring/session.rs::run_kimi_wire_session` | separate std `Command`, inline `spawn`, Unix/Windows process-group arms | already receives the same complete env and child CWD; inherits `AGENT_CWD` once `build_child_env` contributes it, with an inventory assertion for this bypass of `base_command` |

Confirmed exclusions that otherwise look like production hits:

- `lib/linking/paths.rs` and `lib/system_prompt/context.rs` Git commands are
  inside test modules.
- Git commands in wrapper prompt/overlay/prep/shell-option/task-run files are
  test fixtures inside their local test modules.
- `cli/commands/wrap/exec/termination/windows.rs` command sites are Windows
  termination tests below its test boundary, not production descendants.
- `cli/provider_values.rs` uses Clap's `Command` only.

Phase 7 should commit a scanner/fixture covering all 12 rows, classify the two
helper-returned command families (`gen_command`, platform shell constructors),
and include the Kimi direct-spawn exception. The final inventory must be
regenerated after Phase 6 changes production source.

## Repository-first conflict fixtures to flip deliberately

Engine/Darkmatter fixtures:

- `biscuit-file/lib/tests/precedence_flip.rs`: candidate/root order and
  first-existing resolution matrices.
- `darkmatter/lib/src/markdown/compose/util.rs::seam_resolves_implicit_repository_first_and_explicit_source_only`.
- `darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs::implicit_reference_prefers_repository_root_over_base`.
- `darkmatter/lib/src/markdown/schemas/detect.rs::explicit_context_detection_uses_repository_first_candidates`.
- `darkmatter/lib/src/markdown/compose/link_resolve.rs::test_link_resolve_non_existent_implicit_is_repository_first`.
- `darkmatter/lib/src/markdown/schemas/resolve.rs` conflict fixtures around
  `bare_name_pins_to_schema_root_not_repository_root`; this configured-root
  pin remains a distinct contract and must not be accidentally flipped.
- `darkmatter/lib/src/markdown/compose/schema_validation.rs` repeated rewrite
  fixture and `markdown/schemas/tests/mod.rs`'s
  `resolve_ctx::document_relative_hit_wins_over_fallback_conflict` reference.
- `darkmatter/lib/src/markdown/compose/context/options.rs` and
  `expression/resolve_ctx.rs` comments/fixtures that explicitly exclude the
  legacy fallback directory from the repository-first plan.

Claudine fixtures:

- `lib/src/harness/resolve/tests.rs::implicit_reference_prefers_repository_root`.
- `lib/src/system_prompt/resolve/tests.rs::explicit_append_implicit_is_repository_first`.
- `lib/src/composition/lifecycle/control/tests.rs::resolve_proxy_target_prefers_repository_root_for_implicit`.
- `lib/src/composition/sequence/tests.rs::implicit_reference_is_repository_first`.
- `cli/tests/compose_cli.rs::compose_transclusion_resolves_repository_first_on_collision`.
- `cli/tests/level2_file_resolution_capture.rs::level2_implicit_reference_resolves_repository_first_in_tmux` (Phase 8 L2 update).
- Behavior-adjacent repository-first assertions/comments in
  composition lifecycle executor filesystem lookup, looping expression,
  sequence source, coordinator commit, harness resolve, system-prompt resolve,
  and schema diagnostics must be updated in the same behavior-changing phase.

## Phase 6 Claudine consumer closure

The Claudine migration now projects retained Sniff repository observations
through Darkmatter's repository-scope catalog for invocation, source, ambient
compatibility, and completion contexts. The consumer sweep confirmed:

- sequence and harness canonical routes derive per-document contexts while
  preserving the immutable launch context for caller-materialized values;
- system-prompt and overlay preparation use each source's captured resolution
  context and the shared launch prepared-context snapshot;
- composition completion uses the same projected intrinsic scopes and the same
  Claudine convention roots as runtime resolution;
- `claudine-gen` owns its command-boundary repository observation through
  Sniff, so it no longer keeps Darkmatter's display-oriented Git helper alive;
- no production Claudine file-reference match produces or consumes the removed
  `Package` kind or provenance. The remaining `!` tokens are the linking
  filter's negation syntax and schema-completion exclusion syntax, not
  file-reference grammar.

The conflict fixtures named above now assert source-first document references.
Their comments explicitly record that finalized-reference D3 supersedes
ctx-launch-anchor review-3 Finding 4. The paired real CLI regression retains
the other half of the ruling: caller `spec=spec.md` resolves from the launch
directory and remains absolute through direct/proxy composition and the
proxied success guard.

## Scope review verdict

The audit covers every owner named by the specification's Scope and Ownership
table, including the easy-to-miss Darkmatter CLI/DMLS and Claudine generator
consumers. The migration remains ordered `biscuit-file` → `darkmatter` →
`claudine`; the CRITICAL graph results make intermediate public-contract
handoff unsafe until downstream compilation is restored.

## Phase 2 downstream checkpoint

The Phase 2 grammar layer is green in `biscuit-file`; downstream migration is
intentionally deferred to Phases 4–6. The scoped downstream gates produced this
work list on 2026-08-27:

- `darkmatter/just test` compiled `darkmatter`, `darkmatter-cli`, and `dmls`.
  It stopped at
  `markdown::compose::expression::functions::tests::fn_filesystem::absolute_package_reference_uses_package_area_not_repository`:
  the fixture's authored `!shared.md` now returns the dedicated removed-sigil
  `InvalidSyntax` diagnostic. This fixture belongs to the Phase 4 grammar and
  scope migration.
- `claudine/just test` stopped during compilation with the complete current
  exhaustiveness list:
  - `claudine/lib/src/harness/error.rs:425` does not classify
    `FileReferenceError::{UnsupportedScheme, OutsideRepository,
    RepositoryEscape}`;
  - `claudine/lib/src/harness/error.rs:447` does not classify
    `FileReferenceKind::{RepositoryRoot, RepositoryScoped}`;
  - `claudine/lib/src/harness/error/tests.rs:334` does not include the three new
    error variants in its exhaustive catalog assertion.

No Darkmatter compiler error was produced at this checkpoint. Claudine's three
diagnostic/classification matches are the Phase 6 compiler-restoration list.

## Phase 3 downstream checkpoint

The Phase 3 biscuit-file engine is green. The remaining downstream migration
is confined to the previously assigned phases:

- Phase 4 updates Darkmatter's repository-scope projection, removes its
  repository-first expectations and `!` fixture, and routes its resolver sites
  through the catalog-backed context.
- Phase 5 materializes parameter references through biscuit-file's public
  containment check and adds `ctx.cwd` to Darkmatter's captured context.
- Phase 6 supplies Claudine's source-specific catalog projections, removes its
  duplicated magic-root registration, flips its repository-first fixtures, and
  completes the new file-reference error/kind classifications listed above.

No additional downstream owner surfaced while implementing the catalog,
containment, or completion paths. The public enum removals and new catalog API
therefore do not expand the Phase 2 consumer inventory.

## Phase 8 repository-search proof

The final proof was rerun on 2026-08-27 against production, tests, and current
documentation. Historical feature plans and reviews were excluded because they
intentionally quote the superseded grammar.

| Proof | Scope and result |
|---|---|
| Removed dependency | `rg cargo_metadata biscuit-file/{lib,cli,docs}`: zero matches. |
| Removed public vocabulary | `rg 'FileReferenceKind::Package\|RootProvenance::Package([^AR]\|$)\|MissingPackageContext'` across the three affected production/test/doc areas: zero matches. `PackageRoot` and `PackageArea` remain intentionally distinct. |
| Removed discovery helper | `rg find_package_area_from darkmatter/{lib,cli,dmls}`: zero matches. |
| Surviving Git-root helper | `find_git_root_from` has no external consumer. Its production calls are Darkmatter's display abbreviation and file-links boundary; the remaining catalog and shell-store hits are tests. No resolver uses it. |
| Catalog adapter ownership | `darkmatter::markdown::compose::context::repository_scope::repository_scope_catalog` is the sole `RepoInfo` → `RepositoryScopeCatalog` adapter. Claudine calls that public adapter from invocation/source resolution and completion; it never constructs the catalog from `RepoInfo`. |
| Removed prompt sigil | The shipped Markdown/YAML/JSON/TOML corpus search for a scalar file-shaped `!name.ext` reference returned zero matches. Remaining `!` tokens are expression/filter/glob negation or quoted historical research, not file-reference grammar. |

The system-prompt compatibility path was the last external consumer of
Darkmatter's Git-root helper. It now uses biscuit-file's repository-discovery
authority directly when no invocation/source snapshot is available; normal
request-scoped execution remains discovery-free.
