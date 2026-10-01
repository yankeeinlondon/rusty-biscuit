---
spec: "/Volumes/coding/wt/rusty-biscuit/fix-magic-globs/darkmatter/fixes/2026-09-30-file-refs-use-magic/spec.md"
plan: "darkmatter/fixes/2026-09-30-file-refs-use-magic/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
    - darkmatter/lib/src/markdown/compose/preflight/collect.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/preflight_repository_sigils.rs
    - darkmatter/cli/tests/l1/compose_transclusion.rs
docs_updated_during_phase_1:
    - darkmatter/docs/inline/preflight-checks.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/compose.md
packages:
    - darkmatter
    - darkmatter-cli
    - biscuit-file
    - sniff
source_files_during_phase_2:
    - biscuit-file/lib/src/file_reference/context.rs
    - biscuit-file/lib/src/file_reference/mod.rs
    - biscuit-file/lib/src/file_reference/resolve.rs
    - biscuit-file/lib/src/lib.rs
    - darkmatter/cli/src/commands/compose.rs
    - darkmatter/cli/src/commands/schema/triggers.rs
    - darkmatter/cli/src/commands/schema/validate.rs
    - darkmatter/lib/benches/compose_pipeline.rs
    - darkmatter/lib/benches/compose_schema_transclusion.rs
    - darkmatter/lib/benches/phase6_interpolation.rs
    - darkmatter/lib/benches/reference_graph.rs
    - darkmatter/lib/src/markdown/compose/cache/hashing.rs
    - darkmatter/lib/src/markdown/compose/context/capture/agent.rs
    - darkmatter/lib/src/markdown/compose/context/capture/mod.rs
    - darkmatter/lib/src/markdown/compose/context/checked.rs
    - darkmatter/lib/src/markdown/compose/context/current.rs
    - darkmatter/lib/src/markdown/compose/context/mod.rs
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/context/request.rs
    - darkmatter/lib/src/markdown/compose/context/runtime.rs
    - darkmatter/lib/src/markdown/compose/expression/lint.rs
    - darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/assignment.rs
    - darkmatter/lib/src/markdown/compose/interpolation/fatality_characterization.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
    - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
    - darkmatter/lib/src/markdown/compose/preflight/collect.rs
    - darkmatter/lib/src/markdown/compose/preflight/lifecycle.rs
    - darkmatter/lib/src/markdown/compose/preflight/mod.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs
    - darkmatter/lib/src/markdown/compose/tests/frontmatter.rs
    - darkmatter/lib/src/markdown/compose/tests/icmp.rs
    - darkmatter/lib/src/markdown/compose/tests/identity.rs
    - darkmatter/lib/src/markdown/compose/tests/lazy_roots.rs
    - darkmatter/lib/src/markdown/compose/tests/provider_network.rs
    - darkmatter/lib/src/markdown/compose/tests/rendering.rs
    - darkmatter/lib/src/markdown/compose/tests/schema.rs
    - darkmatter/lib/src/markdown/compose/tests/shell.rs
    - darkmatter/lib/src/markdown/compose/tests/transclusion.rs
    - darkmatter/lib/src/markdown/compose/toc_linking/mod.rs
    - darkmatter/lib/src/markdown/compose/transclusion/resolver.rs
    - darkmatter/lib/src/markdown/compose/type_tests.rs
    - darkmatter/lib/src/markdown/compose/util.rs
    - darkmatter/lib/src/markdown/reference/file_tree/mod.rs
    - darkmatter/lib/src/markdown/reference/graph.rs
    - darkmatter/lib/src/markdown/reference/mod.rs
    - darkmatter/lib/src/markdown/reference/types.rs
    - darkmatter/lib/src/markdown/reference/validate.rs
    - darkmatter/lib/src/markdown/schemas/clean.rs
    - darkmatter/lib/src/markdown/schemas/format.rs
    - darkmatter/lib/tests/l1/ambient_ctx_capture.rs
    - darkmatter/lib/tests/l1/array_rendering_json.rs
    - darkmatter/lib/tests/l1/backslash_escape_spans.rs
    - darkmatter/lib/tests/l1/compose_diagnostic_identity.rs
    - darkmatter/lib/tests/l1/compose_expression_failure_contract.rs
    - darkmatter/lib/tests/l1/compose_phase6.rs
    - darkmatter/lib/tests/l1/compose_reuse_phase5.rs
    - darkmatter/lib/tests/l1/context_functions.rs
    - darkmatter/lib/tests/l1/dasherized_identifier_compose.rs
    - darkmatter/lib/tests/l1/data_origin.rs
    - darkmatter/lib/tests/l1/declined_path_transclusion.rs
    - darkmatter/lib/tests/l1/disclosure_transclusion_integration.rs
    - darkmatter/lib/tests/l1/empty_package_area.rs
    - darkmatter/lib/tests/l1/expression_regression.rs
    - darkmatter/lib/tests/l1/feature_review_incident.rs
    - darkmatter/lib/tests/l1/file_tree_roots.rs
    - darkmatter/lib/tests/l1/find_files_and_try_frontmatter.rs
    - darkmatter/lib/tests/l1/frontmatter_surface_projection.rs
    - darkmatter/lib/tests/l1/git_context_integration.rs
    - darkmatter/lib/tests/l1/interpolation_literal_pipeline.rs
    - darkmatter/lib/tests/l1/link_interpolation_integration.rs
    - darkmatter/lib/tests/l1/literal_token.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/meta_schema_phase4.rs
    - darkmatter/lib/tests/l1/missing_ctx_capture.rs
    - darkmatter/lib/tests/l1/nested_composition.rs
    - darkmatter/lib/tests/l1/persistent_cache_disabled.rs
    - darkmatter/lib/tests/l1/predict_conflicts.rs
    - darkmatter/lib/tests/l1/preflight_child_state_parity.rs
    - darkmatter/lib/tests/l1/preflight_repository_sigils.rs
    - darkmatter/lib/tests/l1/reference_integration.rs
    - darkmatter/lib/tests/l1/request_context_builder.rs
    - darkmatter/lib/tests/l1/request_context_epoch.rs
    - darkmatter/lib/tests/l1/schemas_literal_expression.rs
    - darkmatter/lib/tests/l1/set_overlay_integration.rs
    - darkmatter/lib/tests/l1/shell_block_integration.rs
    - darkmatter/lib/tests/l1/shell_expansion_coordinates.rs
    - darkmatter/lib/tests/l1/shell_probe_preflight.rs
    - darkmatter/lib/tests/l1/shell_result_values.rs
    - darkmatter/lib/tests/l1/suggest_constraint_phase3.rs
    - darkmatter/lib/tests/l1/ternary_integration.rs
    - darkmatter/lib/tests/l1/transcluded_shell_failure.rs
    - darkmatter/lib/tests/l1/unknown_identifier_warning.rs
    - darkmatter/lib/tests/l1/url_root_identity.rs
    - darkmatter/lib/tests/level2/level2_render_tree_terminal/support/mod.rs
    - darkmatter/lib/tests/request_support/mod.rs
    - sniff/lib/src/filesystem/repo/glob.rs
    - sniff/lib/src/filesystem/repo/manifest_index.rs
    - sniff/lib/src/filesystem/repo/mod.rs
docs_updated_during_phase_2:
    - darkmatter/lib/README.md
    - darkmatter/docs/topics/magic-paths.md
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/inline/text-replacement.md
    - darkmatter/docs/inline/preflight-checks.md
    - biscuit-file/docs/topics/file-references.md
    - sniff/docs/sniff-library-architecture.md
docs_created_during_phase_2:
    - darkmatter/docs/topics/compose-requests.md
skills_files_updated_during_phase_2:
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/compose.md
    - .claude/skills/biscuit-file/references/file-references.md
    - .claude/skills/sniff/remote-and-repository.md
---

# Implementation Log for 2026-09-30-file-refs-use-magic (8 phases)

## Phase 1

### Wave 1

#### Reproduce Incident 1

From `claudine/` with the installed `md` (`md 0.1.0`, `~/.cargo/bin/md`):

```text
$ md compose docs/use-claudine/SKILL.md
⤫ TransclusionError: file reference failure
┃
┃ `^` repository reference requires a repository containing reference CWD
┃ `/Volumes/coding/wt/rusty-biscuit/fix-magic-globs/claudine/docs/use-
┃ claudine`
┃
┃ Check sigil usage: `@` magic, `&` repository root, `^` repository-scoped.
```

#### Spike S1: builder-failure inputs (recorded as R11 in the plan)

Ran with a throwaway crate (`/tmp/s1-spike`, path dependencies on
`biscuit-file` and `sniff`; nothing added to the repository).

- (a) `FileResolutionContext::from_snapshot("relative/dir", ..)` fails
  `validate()` with `RelativeContextDirectory { anchor: RequestDirectory }`.
- (b) A request directory inside a repository plus an opening reference
  through `~` or `{{VAR}}` that resolves outside it fails
  `validate()` after `for_source_reference` with
  `RepositoryRootNotContainingSource`. A request directory in `VAULT` with
  the repository root elsewhere fails the same way. With no repository the
  `~` anchor becomes the tree and validation passes.
- (c) A syntactically corrupt `.git/config` (`[core` unterminated) makes
  `GitRepo::discover` and `find_git_root` return `Err`. Rewriting the config
  as valid repairs it. Every other broken state tried (a `.git` file naming
  a missing or empty gitdir, a garbage `.git` file, an empty `.git`
  directory, a missing `HEAD`, an unreadable `.git` on Unix) is reported as
  **no repository**, not an error. A malformed `HEAD`, format version 99, and
  an unknown extension all discover successfully.
- Phase 4 consequence: DMLS must see a `.git/config` change to drop the
  failed entry (see `message_to_agent` in the spec).

#### Spike S2: matrix and guard topology (recorded under R6 in the plan)

Ran with temporary edits to `claudine/cli` (`Cargo.toml`
`source-inputs`, `tests/l1/s2_spike.rs`) and `darkmatter/dmls`
(`tests/l1/s2_spike.rs`), all reverted afterward.

- A `#[path]`-included copy of `darkmatter/cli/tests/common/source_scan.rs`
  compiled and ran in both `dmls::l1` and `claudine-cli::l1`. The darkmatter
  `l1` binary already does this (`semantic_results_never_persist.rs`).
- With only the `#[path]` include, the planner **refused** the declaration:
  `RealWorkspaceTestInputTests.test_every_declared_source_input_is_read_by_its_declarer`
  failed with "claudine-cli declares darkmatter/cli/tests/common/source_scan.rs
  but no L1 test of it names the path". The index walks `#[path]` modules but
  does not count them as references.
- Adding `include_str!("<same path>")` makes the test pass. At module level
  the planned cell is `claudine-cli ubuntu-latest L1 (binary_id(claudine-cli::l1))`.
  Inside the test function it narrows to
  `(binary_id(claudine-cli::l1) & test(=s2_spike::s2_spike_path_include_compiles))`.
  The R6 alternative: each declaring test spells the shared file with an
  in-function `include_str!` next to the `#[path]` include.
- `claudine-cli::l1` holds both completion and composition tests. Several
  `compose_*` modules there are `#[cfg(unix)]`.

### Wave 2

#### Regression tests (written first, confirmed red before the fix)

- `darkmatter/cli/tests/l1/compose_transclusion.rs::test_compose_repository_sigils_from_a_nested_document`.
  This is a git repository built with `CliProcessFixture::initialize_repository_at`.
  `md` is launched from `repo/pkg` (`ambient_context`) with the relative
  argument `docs/guide/doc.md`. The document holds `::file &amp-target.md`,
  `::file ^caret-target.md`, and `::file ^pkg/docs/cli/index.md`, which is
  the shape of the original report. The test asserts exit 0, all three
  bodies in stdout, no leaked `::file`, and no "requires a repository
  containing reference CWD" message. Before the fix it failed with exactly
  that message for `&`.
- `darkmatter/lib/tests/l1/preflight_repository_sigils.rs` uses the same
  layout through `gix::init`. Each test runs two request shapes that carry
  no file-resolution context: `ComposeOptions::for_document(launch_dir, ..)`,
  which is what `md` builds (the source is inside the request repository),
  and `ComposeOptions::new()` (the source is outside the process's
  repository). The tests:
  - `compose_preflight_resolves_repository_sigils_from_a_nested_document`
    checks the approval set holds each target's command and the graph edges
    resolve to the three canonical target paths, in order;
  - `collect_shell_commands_resolves_repository_sigils_from_a_nested_document`
    checks the entries match the targets' commands, in order;
  - `compose_preflight_approvals_resolves_repository_sigils_from_a_nested_document`
    checks the pre-approved set and the discovered count;
  - `compose_preflight_reports_a_missing_repository_target_as_not_found`
    is the negative case: a missing `&` target still fails, names the
    target, and is no longer the missing-repository precondition.

  All four failed before the fix with "`&` repository reference requires a
  repository containing reference CWD".

#### Preflight fix

- `ComposeOptions::prepare_root(&mut self, &Markdown)` (`pub(crate)`,
  `compose/context/options.rs`) runs, in order, `extend_context_for`,
  `establish_repository_observation`, and `ensure_file_resolution_context`.
  `prepared_root(&self, ..)` is the borrowing form, which clones.
- `run_compose_pipeline` calls `prepare_root` in place of the three inline
  calls.
- Pre-flight calls `prepared_root` at the top of `collect_effects` and
  `collect_frontmatter_shell_commands`. **Departure from the plan's wording:**
  the plan lists `compose_preflight`, `compose_preflight_approvals`, and the
  public `collect_*` entries. Every one of those reaches the walk through
  `collect_effects` (`compose_preflight_approvals` goes through
  `compose_preflight`; `collect_shell_commands` goes through
  `collect_shell_commands_with_graph`), except
  `collect_frontmatter_shell_commands`, which is prepared separately. So two
  call sites cover all five entries, plus the pipeline's
  `validate_pre_approved` and `nested.rs`'s use of it.
- The ambient repository observation lives behind an `Arc<OnceLock>` that
  clones share, so preparing a clone in pre-flight does not add a second
  discovery when the pipeline later runs with the caller's options.
- Doc drift fixed: `establish_repository_observation`'s doc said the "root
  pipeline entry" calls it. It now names `prepare_root`.

#### Docs and skill

- `darkmatter/docs/inline/preflight-checks.md`: collection resolves
  transclusion targets as composition does, so `&` and `^` reach the same
  file from any launch directory.
- `.claude/skills/darkmatter/SKILL.md`: pre-flight is a root entry and
  prepares through `prepare_root`.
- `.claude/skills/darkmatter/compose.md`: the "fixed by the root pipeline
  entry" line now names `prepare_root` and both of its callers.

#### Incident check after the fix

Running `target/debug/md compose docs/use-claudine/SKILL.md` from `claudine/`
with the worktree build exits 0 and prints 88 lines with no unresolved
`::file`.

#### Gates

- `just test` (darkmatter): 8749 passed, 12 skipped, exit 0. The four new
  library tests and the new CLI test ran in it.
- `just lint` (darkmatter): exit 0.
- `just test` (claudine, a downstream caller of `compose_preflight`): 8072
  passed, 9 skipped, exit 0.
- No pre-existing failures seen.
- No cross-OS run in this phase. The change adds no path comparison and no
  `#[cfg]` code, and the tests compare canonicalized paths on both sides.
  Windows evidence is gathered in Phase 8 under the plan's Definition of
  Success.
- Not done, per the phase prompt: Commit 1 and `git verify-commit`. The plan's
  Checkpoint asks for them; they are left to the separate commit process.

#### Requirement-to-test mapping

| Requirement | Test |
|---|---|
| `md compose` resolves `&` from a nested directory | `compose_transclusion::test_compose_repository_sigils_from_a_nested_document` |
| `md compose` resolves `^` (bare and `^pkg/...`, the incident shape) | same test |
| `compose_preflight` collects `&`/`^` targets with no context (both request shapes) | `preflight_repository_sigils::compose_preflight_resolves_repository_sigils_from_a_nested_document` |
| `collect_shell_commands` likewise | `preflight_repository_sigils::collect_shell_commands_resolves_repository_sigils_from_a_nested_document` |
| `compose_preflight_approvals` likewise | `preflight_repository_sigils::compose_preflight_approvals_resolves_repository_sigils_from_a_nested_document` |
| A missing `&` target still fails, as not-found, not as the repository precondition | `preflight_repository_sigils::compose_preflight_reports_a_missing_repository_target_as_not_found` |

The Input Robustness Matrix does not apply (R15): no file-format reader was
added or changed.

## Phase 2

### Wave 3

#### biscuit-file accessor (subagent)

- `FileReferenceError::resolution_failure()` (`biscuit-file/lib/src/file_reference/resolve.rs`)
  delegates to the private `classify_error`, which stays the one place the
  classification is decided.
- **Departure from the plan:** the plan asks for one test per
  `ResolutionFailure` variant. `classify_error` never returns `NoMatch` (a
  no-match is a result, not an error), so there is no input for it. Four
  variant tests plus `resolution_failure_never_no_match` were added instead:
  `resolution_failure_invalid_reference`, `resolution_failure_missing_context`,
  `resolution_failure_io`, `resolution_failure_unsupported_remote`.
- Also made public (not in the plan): `biscuit_file::capture_env()`, the
  environment reader `FileResolutionContext::new` uses. `RequestSnapshot::from_process()`
  needs it so R2's "same helpers" holds (non-UTF-8 variables skipped,
  `USERPROFILE` via `home_dir()`). Documented in
  `biscuit-file/docs/topics/file-references.md` and the `biscuit-file` skill.
- `just test` (1016 passed) and `just lint` in `biscuit-file/`: exit 0.

#### sniff manifest list (subagent)

- `sniff::filesystem::repo::PACKAGE_MANIFEST_FILE_NAMES` replaces the private
  `MANIFEST_FILES` in `glob.rs`; the membership check in `manifest_index.rs`
  (~691) uses it.
- **Departure:** the two `manifest_index.rs` sites at ~459 and ~501 map each
  name to a `ManifestKind` with a `match` on literals and cannot be written
  over a slice. They were left, and
  `manifest_index::tests::manifest_index_recognizes_every_package_manifest_file_name`
  keeps the ~501 copy in step with the constant (the ~459 walker copy is not
  covered). `glob::tests::package_manifest_file_names_are_pinned` pins the
  contents and order.
- `just test` (3121 passed, 32 skipped) and `just lint` in `sniff/`: exit 0.

#### `RequestSnapshot` and `ContextBuildError`

New module `darkmatter/lib/src/markdown/compose/context/request.rs`,
re-exported from `darkmatter::markdown::compose`:

- `RequestSnapshot::new(dir)` (no home, empty environment), `from_process()`
  (returns `io::Result`; the current-directory read can fail),
  `with_home`, `with_env`, `with_magic_root`, `with_magic_root_tier`
  (Claudine's `~/.claudine` roots need `MagicPathTier::User`; R2 named only
  the inferred form), `with_opening_reference`, `at_request_dir` (keeps home,
  environment, and roots; drops the opening reference, which was resolved for
  the old directory), and accessors.
- `ContextBuildError::{Discovery, Invalid}` with `request_dir()` and
  `resolution_failure()`. **Ruling made here:** a discovery failure classifies
  as `MissingContext`, the class biscuit-file gives its own `Git` errors, so
  the two never disagree.

### Wave 4

#### Builder

`build_resolution_context(&RequestSnapshot)` follows R3: `from_snapshot`,
discovery, scope catalog (which also syncs the launch `@` scope), the
snapshot's extra roots, `for_source_reference` for an opening reference,
`validate()`, then one `tracing::debug!` with `request_dir`, `base_dir`, and
`base_dir_origin`.

- Discovery runs through the same `Repo`-group capture `ctx.repo*` uses
  (`capture_runtime_context_for_groups`), so the observation can be shared.
  With only that group requested, the capture's sole `git` diagnostic is the
  discovery failure, which becomes `ContextBuildError::Discovery`. A relative
  request directory skips discovery (validation rejects it with a typed error
  instead of discovering from the process directory).
- A topology-walk failure under a found repository still degrades to "no
  packages", as before. Only the Git discovery itself is an error.
- `capture_file_resolution_context` and its helper are deleted, with the
  re-exports. Its library callers (`schemas/clean.rs`, `util.rs`
  `source_link_context`'s fallback, tests) and the CLI's
  `schema validate` / `schema triggers` now call the builder.
- Magic paths: `ComposeOptions::magic_paths`, `with_magic_path`, and
  `TransclusionOptions::magic_paths` are deleted (R3). The cache and graph
  identities already encode the context's tier-aware registrations, so their
  explicit `magic_paths` fields were dropped and the identity tests now vary
  the context's roots. `ResolutionContext::magic_paths` (expression side) and
  `document_resolution_context`'s parameter are kept for the no-context
  fallback, which Phase 5 removes; every caller now passes `&[]`.

#### `ComposeRequest`

- `ComposeRequest { options, context }`. `prepare(options, &snapshot)` and
  `with_context(options, context)` (which validates) both return
  `Result<_, ContextBuildError>`. `options()`, `context()`, and
  `map_options()` (for settings decided after preparation, such as
  pre-flight's approval set) are public; `root_options()` (options with the
  context attached) is crate-private.
- `prepare` keeps "one discovery per request" (the `lazy_roots` tests pin
  it): a `ComposeOptions::new()` context that captured only date/time is
  re-anchored on the request directory (`ComposeContext::with_anchor`); the
  request's existing observation is reused when it contains the request
  directory; otherwise the builder's discovery is installed as the request's
  observation (`CurrentAuthority::adopt_ambient_repository`) when the request
  observes its own repository from that directory.
- `ensure_file_resolution_context`, `prepare_root`, and `prepared_root` are
  deleted. `establish_repository_observation` became the module-private
  (`pub(super)`) `establish_request_repository`, called only by
  `ComposeOptions::for_document` and `ComposeRequest`.
- `ComposeOptions::with_file_resolution_context` is now `pub(crate)`: outside
  the crate the only way to attach a context is `ComposeRequest`. The internal
  `Option` field stays until Phase 5, which already lists
  `file_resolution_context()` and `source_file_resolution_context`.
  **Departure from R4's wording** ("`ComposeOptions` loses its `Option`
  context field"): the field is now internal transport between a request and
  its derived child options; Phase 5 makes it required.
- Public entry points on `&ComposeRequest`: `Markdown::compose_with`,
  `compose_preflight`, `compose_preflight_approvals`, `collect_shell_commands`,
  `collect_shell_commands_with_graph`, `collect_frontmatter_shell_commands`,
  `transclusions_with_options`, `ReferenceGraphOptions::with_compose`,
  `shell_expansion::execute_directive`, and `execute_resolved_shell_values`.
  `Markdown::compose()` and `compose_mut()` are deleted.
- Not moved, recorded as departures: `normalize_links` is `pub` inside the
  crate-private `link_normalization` module and `link_resolve` is
  `#[cfg(test)]`; neither is reachable from outside the crate, and both run on
  derived child options inside the pipeline. `validate_pre_approved` is
  `pub(crate)` and also runs on child options (`nested.rs`); it now calls the
  internal `collect_effects` / `frontmatter_shell_commands`.
- Internal inline passes that compose over derived options (pre-flight's
  inline pass, reference-graph content preparation) use the new crate-private
  `Markdown::compose_with_options`, and `ReferenceGraphOptions::from_compose_options`
  replaces their `with_compose` calls. `run_compose_pipeline(&ComposeRequest)`
  delegates to `run_root_pipeline(options)`.
- Added `ComposeOptions::source()` (public getter) so test helpers outside the
  crate can choose a request directory.

#### `ctx.*` from the request

- `ComposeRequest` aligns the `ComposeContext` environment with the request
  context's (`align_context_environment`), so `{{ env.X }}`, `ctx.agent`, and
  `ctx.model` read the snapshot. Extension never overwrites the environment
  (`extend_missing_with` keeps `inner.env`), so later group captures keep it.
- `ResolutionContext::agent()` falls back to the request context's `AGENT`
  before the process.
- Home on the request path already came from the context
  (`options.rs` `expression_resolution_context`), so no change was needed
  there; the `dirs::home_dir()` fallbacks now run only without a context.
- **Residue left for Phase 5** (its "anything left in `runtime.rs`" item):
  `ComposeContext::capture()` / `capture_minimal()` still read the current
  directory and environment, and `capture/mod.rs`'s capture still calls
  `std::env::vars()`. On a request path the environment is replaced and a
  minimal context re-anchored, so neither reaches `ctx.*`; non-request users
  (DMLS validation, `local_expression_resolution_context` without a context)
  still depend on them.
- **Not changed, recorded:** `conditions.rs:379` (`env.*`) belongs to the
  standalone `evaluate_condition_against(data, work_dir)` shortcut API, which
  has no request. It is not on the compose path. Phase 5/6 must classify it.
- `capture/agent.rs`: the two `populate_agent` tests now pass an environment
  map instead of mutating the process environment. The third
  (`capture_runtime_context_includes_agent_group`) still mutates it under
  `serial_test`.

#### Migrating darkmatter's tests

- About 900 call sites in `darkmatter/lib` were moved with a balanced-paren
  rewrite onto two helpers that reproduce the request directory the deleted
  fallback chose (a file source's directory, else the context's absolute
  anchor, else the process directory), with process home and the options'
  own context environment:
  - unit tests: `crate::markdown::compose::test_request(options)`
    (`context/request.rs` `test_support`), which keeps a context the options
    already hold through `with_context`;
  - integration tests: `crate::request_support::{request, request_at}`
    (`lib/tests/request_support/mod.rs`, declared in `l1/main.rs`).
- Tests whose subject is the context moved to `ComposeRequest::with_context`
  (`file_tree_roots`, `link_interpolation_integration`,
  `reference_integration::reference_options`, `unknown_identifier_warning`).
- Behavior surfaced by the migration, now asserted rather than masked: a
  reference graph whose request directory is in this repository rejects a
  document in a temp directory (`RepositoryRootNotContainingSource`) instead
  of silently re-anchoring on the document. Graph tests now anchor the
  request at their fixture (`request_at`).
- `schema_validation::undecided_root_union_resolves_a_caller_file_from_the_launch_area`
  now prepares its request at the launch directory, which is what it tests.
- `transclusion::resolver::resolves_magic_path_prepended` no longer changes
  the process directory; it builds its context from a snapshot root.
- `test_compose_mut_modifies_in_place` was deleted with `compose_mut`.
- Benches and the level2 terminal support were moved to `ComposeRequest`.
- `lib/tests/l1/declined_path_transclusion.rs` is `#[cfg(windows)]`; macOS
  never compiled it. The Windows cross-check caught it.

#### New tests

| Requirement | Test |
|---|---|
| `new()` reads nothing from the process | `context::request::tests::a_new_snapshot_reads_nothing_from_the_process` |
| `at_request_dir` keeps home, env, roots; drops the opening | `context::request::tests::at_request_dir_keeps_home_environment_and_roots_but_not_the_opening` |
| One `debug` event naming request dir and `base_dir` origin (scoped `tracing_test` subscriber) | `context::request::tests::a_successful_build_emits_one_debug_event` |
| A failed build emits none | `context::request::tests::a_failed_build_emits_no_debug_event` |
| AC 6 (a): relative request directory rejected, by builder and `prepare` | `request_context_builder::a_relative_request_directory_is_rejected` |
| AC 6 (b): opening reference through `~` and `{{VAR}}` outside the repository rejected; control builds | `request_context_builder::an_opening_reference_outside_the_request_repository_is_rejected` |
| (b) negative control: no repository, same opening builds | `request_context_builder::an_opening_reference_builds_when_the_request_has_no_repository` |
| R11 (c): corrupt `.git/config` is `Discovery`, `MissingContext`; repair builds | `request_context_builder::a_corrupt_git_config_is_a_discovery_error_until_repaired` |
| Magic-path finding: a snapshot root resolves `@` in the context and through `compose_with`; without it, nothing | `request_context_builder::a_snapshot_magic_root_resolves_an_at_reference` |
| `{{ env.X }}`, `ctx.agent`, and a `{{X}}` file reference read the snapshot while the process lacks `X` | `request_context_builder::an_expression_and_a_file_reference_read_the_same_snapshot_environment` |
| A transclusion's `{{X}}` target reads the snapshot | `request_context_builder::a_transcluded_variable_reference_reads_the_snapshot_environment` |
| `with_context` validates | `request_context_builder::with_context_rejects_an_invalid_context` |
| One discovery per request survives preparation | existing `compose::tests::lazy_roots::ambient_repository::*` (unchanged, passing) |
| Incident 1 stays fixed | existing `preflight_repository_sigils::*` (now through `request_support`) and `compose_transclusion::test_compose_repository_sigils_from_a_nested_document` |

The Input Robustness Matrix does not apply (R15).

#### `md` CLI (compile bridge only; Phase 3 Track A owns the redesign)

- `md compose`: with `--set`, the request is `with_context` over the
  document context it already derived; without `--set` it is `prepare` at the
  document's directory (the launch directory for stdin) with
  `RequestSnapshot::from_process()`. That reproduces the deleted fallback.
  Validation, pre-flight, and compose share the request, and pre-flight's
  approvals are applied with `map_options`.
- `schema validate` / `schema triggers` find their trigger boundary through
  the builder. In `schema validate` a build failure is reported as the file's
  `ParseError` outcome (no better outcome exists yet).
- `from_process()` is called inside `compose.rs`, not once in `main`; Phase 3
  moves it.

#### Docs and skills

- New `darkmatter/docs/topics/compose-requests.md` (snapshot, builder steps,
  failure table, one environment per request, Mermaid flow).
- `darkmatter/docs/topics/magic-paths.md` rewritten for snapshot roots; the
  Claudine example is marked **planned** (Claudine still builds its own
  context until Phase 3).
- Stale `compose_with(options)` / `.compose()` examples fixed in
  `darkmatter/lib/README.md`, `docs/topics/schemas/definition.md`,
  `docs/inline/text-replacement.md`, `docs/inline/preflight-checks.md`.
- `darkmatter` skill (`SKILL.md`, `compose.md`) describes `ComposeRequest`,
  the test helpers, and `compose_with_options`; `prepare_root` is gone.

#### Gates

- `just test` (darkmatter: lib, cli, dmls): 8757 passed, 12 skipped, exit 0.
- `just lint` (darkmatter): exit 0.
- `cargo test --doc -p darkmatter`: 189 passed, 1 failed. The failure,
  `compose::expression::evaluate` (calls an undefined `evaluate_expr`), is
  pre-existing; the file was not touched. Four stale doctests this phase broke
  were fixed. Doctests are not part of `just test`.
- `just cross-check darkmatter --os windows` (filtered to the new and changed
  tests): first run failed to compile `declined_path_transclusion.rs`
  (Windows-only), then three new tests failed on verbatim `\\?\` spellings
  from `canonicalize`. Fixed with `biscuit_file::canonicalize_simplified` and
  portable `{{VAR}}` values; final run 12/12 passed. Linux: 18/18 passed.
  (The `just` wrapper mangles `-E '…(…)…'`; plain substring filters through
  `./scripts/cross-check.sh` work.)
- Not done: the full darkmatter suite on Windows/Linux (Phase 8).

#### Downstream compile errors (Phase 3's input)

messenger, messenger-cli, and claudine-gen still compile. `claudine` does
not, so `claudine-cli` is not checked yet. Per R16, Phase 2 cannot stand
alone, so Phases 2 and 3 land as one commit. `cargo check -p claudine
--all-targets` errors (rustc stops at these; more will follow in
`claudine-cli`):

- lib: `composition/lifecycle/executor.rs:337,344`, `composition/mod.rs:213`,
  `composition/preflight.rs:82,139`,
  `composition/prepare.rs:341,567,678,883,929`,
  `system_prompt/prepare.rs:173,198`. All are
  `with_file_resolution_context` (now crate-private) or an entry point that
  now takes `&ComposeRequest`.
- lib tests: `composition/preflight/tests.rs:324,384,431`,
  `composition/runtime_state/tests.rs:251`, `composition/sequence/tests.rs:761`,
  `invocation_context/tests.rs:306` (`capture_file_resolution_context`).

#### Re-verification (2026-10-01)

A second Phase 2 run found every Phase 2 task already checked and the work
present in the tree, so nothing was re-implemented. Gates re-run on macOS:
`just test` in `darkmatter/` gave 8757 passed and 12 skipped, and `just lint`
exited 0. The plan, log, and spec frontmatter were already complete, so they
were left unchanged.
