---
spec: "/Volumes/coding/wt/rusty-biscuit/fix-magic-globs/biscuit-file/features/2026-09-30-glob-reference/spec.md"
plan: "biscuit-file/features/2026-09-30-glob-reference/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
packages:
    - biscuit-file
    - darkmatter
    - darkmatter-cli
    - dmls
    - claudine
    - claudine-cli
source_files_during_phase_1:
    - biscuit-file/lib/Cargo.toml
    - Cargo.lock
docs_updated_during_phase_1:
    - biscuit-file/docs/dependencies.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - biscuit-file/lib/src/file_reference/glob/mod.rs
    - biscuit-file/lib/src/file_reference/glob/error.rs
    - biscuit-file/lib/src/file_reference/glob/parse.rs
    - biscuit-file/lib/src/file_reference/glob/roots.rs
    - biscuit-file/lib/src/file_reference/glob/list.rs
    - biscuit-file/lib/src/file_reference/glob/matches.rs
    - biscuit-file/lib/src/file_reference/mod.rs
    - biscuit-file/lib/src/file_reference/resolve.rs
    - biscuit-file/lib/src/lib.rs
    - biscuit-file/lib/tests/l1/main.rs
    - biscuit-file/lib/tests/l1/glob_reference/mod.rs
    - biscuit-file/lib/tests/l1/glob_reference/grammar.rs
    - biscuit-file/lib/tests/l1/glob_reference/order.rs
    - biscuit-file/lib/tests/l1/glob_reference/boundary.rs
    - biscuit-file/lib/tests/l1/glob_reference/literal.rs
    - biscuit-file/lib/tests/l1/glob_reference/unfiltered.rs
docs_updated_during_phase_2:
    - biscuit-file/docs/topics/file-references.md
    - biscuit-file/docs/dependencies.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/biscuit-file/SKILL.md
    - .claude/skills/biscuit-file/references/file-references.md
source_files_during_phase_3:
    - darkmatter/lib/src/markdown/schemas/file_match.rs
    - darkmatter/lib/src/markdown/schemas/validate.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/format.rs
    - darkmatter/lib/src/markdown/schemas/simplified/grammar.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/error.rs
    - darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs
    - darkmatter/lib/src/markdown/compose/file_links/discovery.rs
    - darkmatter/lib/src/markdown/compose/file_links/types.rs
    - darkmatter/lib/src/markdown/compose/file_links/mod.rs
    - darkmatter/lib/src/markdown/compose/file_links/parser.rs
    - darkmatter/lib/src/markdown/compose/transclusion/engine.rs
    - darkmatter/lib/src/markdown/compose/glob_listing.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
    - darkmatter/lib/src/markdown/compose/context/report.rs
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/compose/tests/transclusion.rs
    - darkmatter/lib/src/markdown/types.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/glob_consumers.rs
    - darkmatter/lib/tests/l1/find_files_and_try_frontmatter.rs
    - darkmatter/lib/tests/l1/context_construction_guard.rs
    - darkmatter/lib/tests/l1/schemas_grammar_proptest.rs
    - darkmatter/lib/tests/level2/level2_render_tree_terminal/support/mod.rs
    - biscuit-file/lib/src/file_reference/glob/mod.rs
    - biscuit-file/lib/src/file_reference/resolve.rs
    - biscuit-file/lib/tests/l1/glob_reference/literal.rs
    - claudine/cli/src/completion/schema_completion/candidates.rs
    - claudine/cli/src/completion/schema_completion/tests.rs
    - claudine/lib/src/composition/prepare.rs
    - claudine/lib/src/composition/schema/mod.rs
    - claudine/lib/src/composition/schema/classify.rs
    - claudine/lib/src/composition/schema/translate.rs
    - claudine/lib/src/composition/schema/supplied.rs
    - claudine/lib/src/composition/schema/tests.rs
docs_updated_during_phase_3:
    - darkmatter/docs/inline/file-links.md
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/schemas/expression-functions.yaml
    - darkmatter/docs/topics/darkmatter-expressions.md
    - biscuit-file/docs/topics/file-references.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/compose.md
    - .claude/skills/darkmatter/schema.md
    - .claude/skills/biscuit-file/SKILL.md
    - .claude/skills/os/windows.md
source_files_during_phase_4:
    - darkmatter/lib/src/markdown/schemas/roots.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/clean.rs
    - darkmatter/lib/src/markdown/schemas/about.rs
    - darkmatter/lib/src/markdown/schemas/triggers/mod.rs
    - darkmatter/lib/src/markdown/schemas/triggers/discovery.rs
    - darkmatter/lib/src/markdown/schemas/triggers/grammar.rs
    - darkmatter/lib/src/markdown/schemas/triggers/matcher.rs
    - darkmatter/lib/src/markdown/schemas/triggers/assemble.rs
    - darkmatter/lib/src/markdown/schemas/triggers/lint.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/schema_roots.rs
    - darkmatter/lib/tests/l1/meta_schema_phase4.rs
    - darkmatter/lib/tests/l1/schemas_literal_expression.rs
    - darkmatter/cli/src/commands/schema/triggers.rs
    - darkmatter/cli/src/commands/schema/validate.rs
    - darkmatter/cli/src/commands/clean/frontmatter_repair.rs
    - darkmatter/cli/tests/l1/schema_triggers.rs
    - darkmatter/cli/tests/common/protected_env.rs
    - darkmatter/dmls/src/context.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/overlay/schema.rs
    - darkmatter/dmls/tests/l1/lsp_session.rs
docs_updated_during_phase_4:
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
    - darkmatter/docs/topics/schemas/authoring-schemas.md
    - darkmatter/docs/topics/schemas/schema-activation.md
    - darkmatter/example-docs/schemas/external.md
    - claudine/docs/rollout-strategy.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/schema.md
    - .claude/skills/os/windows.md
source_files_during_phase_5:
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
    - darkmatter/lib/src/markdown/schemas/file_match.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/schema_roots_parity.rs
    - biscuit-file/lib/src/file_reference/glob/mod.rs
    - biscuit-file/lib/src/file_reference/glob/list.rs
    - biscuit-file/lib/src/file_reference/glob/matches.rs
    - biscuit-file/lib/tests/l1/glob_reference/boundary.rs
    - claudine/cli/src/completion/schema_completion/candidates.rs
    - claudine/cli/src/completion/schema_completion/tests.rs
    - claudine/cli/src/commands/schema_interactive/mod.rs
    - claudine/cli/tests/l1/compose_schema_cli.rs
    - claudine/cli/tests/l1/level1_review_router_partial_pty.rs
    - claudine/cli/tests/common/review_router.rs
    - claudine/cli/tests/level2/level2_provided_partial_file_capture.rs
    - claudine/cli/tests/level2/level2_windows_provided_partial_file_capture.rs
docs_updated_during_phase_5:
    - biscuit-file/docs/topics/file-references.md
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
    - claudine/docs/topics/completions/shell-completions.md
    - claudine/docs/topics/completions/auto-complete.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/darkmatter/dmls.md
    - .claude/skills/darkmatter/schema.md
    - .claude/skills/biscuit-file/SKILL.md
    - .claude/skills/claudine/SKILL.md
source_files_during_phase_6:
    - darkmatter/lib/tests/l1/glob_implementation_guard.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/Cargo.toml
docs_updated_during_phase_6: []
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/os/build-hosts.md
source_files_during_phase_7:
    - darkmatter/lib/src/markdown/errors/blocks.rs
    - darkmatter/lib/tests/common/entry_point_parity/mod.rs
    - darkmatter/lib/tests/l1/entry_point_parity.rs
    - darkmatter/cli/tests/l1/entry_point_parity.rs
    - darkmatter/dmls/tests/l1/entry_point_parity.rs
    - claudine/cli/tests/l1/entry_point_parity.rs
    - claudine/cli/src/completion/schema_completion/mod.rs
    - claudine/cli/src/completion/schema_completion/parity_tests.rs
docs_updated_during_phase_7:
    - darkmatter/docs/errors/file-reference-failures.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
    - .claude/skills/darkmatter/SKILL.md
---

# Implementation Log for 2026-09-30-glob-reference (8 phases)

Standing sections (later phases append to them): [Spikes](#spikes),
[Changed Outputs](#changed-outputs), [Departures from Spec](#departures-from-spec),
[Windows Evidence](#windows-evidence), [Handed-off Reads](#handed-off-reads).
Phase entries follow, then the [baseline inventory](#appendix-baseline-inventory).

## Spikes

### Spike 1: `globset` semantic probe

`globset 0.4.18` (the workspace's locked version), throwaway crate in
`/tmp/gr-spike1` (not committed), macOS. "lit-sep" is
`GlobBuilder::new(p).literal_separator(true)` with case-sensitive matching;
"default" is `Glob::new(p)`, which `FileMatchGlobs::compile` and the trigger
matcher use today.

| Question | Pattern → path | lit-sep | default |
|---|---|---|---|
| `*` vs `/` | `*.md` → `docs/a.md`; `docs/*.md` → `docs/a/b.md` | no; no | **yes; yes** |
| `**/` matches zero directories | `**/intro.md` → `intro.md`; `docs/**/x.md` → `docs/x.md` | yes; yes | yes; yes |
| `[id]` is a class | `[id].md` → `[id].md` / `i.md` / `d.md` | no / yes / yes | same |
| `{a,b}` alternation | `{a,b}.md` → `a.md`, `b.md` / `c.md` | yes / no | same |
| escaping a literal `[id].md` | `\[id\].md` and `globset::escape("[id].md")` = `[[]id[]].md` → `[id].md` / `i.md` | yes / no (both) | same |
| leading `!` | `!x.md` → `!x.md` / `x.md` | literal `!` (yes / no) | same |

Extra observations, all from the same run:

- Matching is case-sensitive by default (`*.MD` does not match `a.md`).
- `*` and `**` match dot-files and dot-directories (`*.md` → `.x.md`,
  `**` → `.hidden/x.md`). There is no hidden-file rule.
- `?` crosses `/` with the defaults but not with `literal_separator(true)`.
- **Windows trap.** `globset`'s `backslash_escape` defaults to `true` on Unix
  and `false` on Windows, so `\[id\].md` means different things per OS.
  `globset::escape` emits character classes (`[[]`, `[]]`, `[*]`, `[?]`),
  which read the same on every OS. Consequences for Phase 2:
  `GlobReference::escape` delegates to `globset::escape`, and the builder sets
  `backslash_escape` explicitly so a pattern never depends on the host OS.

Outcome for R4 and the `%` rewrite: `**/x` matches a top-level `x`, so the
`%` → `**/<payload>` rewrite needs no zero-depth alternative, and
criterion 15 (`{pkg}/intro.md` matched by `^**/intro.md`) holds as written.
R4 (`literal_separator(true)`, case-sensitive) stands. It **differs** from
`FileMatchGlobs::compile` and `triggers::matcher::compile_globs` today, which
use the defaults, so `*` crosses `/` there now. `find_files()` and
`::file-links` already use `literal_separator(true)`. No existing test
asserts that `*` crosses `/` in `match()` or `$path` (searched below), so
R4's "existing-test results decide" clause does not overturn it. One
shipped artifact changes; see [Changed Outputs](#changed-outputs).

### Spike 2: unfiltered-walk cost

Release build of the same throwaway crate: `walkdir` from the repository
root of this worktree, `follow_links(false)`, no filters, matching
`**/*.md` (`literal_separator(true)`) on every file. macOS dev host, one
sample per configuration (two runs where shown).

| Configuration | Entries visited | `**/*.md` matches | Wall clock |
|---|---|---|---|
| warm `target/` present (cold, then warm cache) | 376,673 (355,233 under `target/`) | 5,272 | 1.14 s, then 0.62 s |
| `target/` absent (simulated by pruning `target` entries) | 21,440 | 5,272 | 79 ms, then 62 ms |

Below the plan's ~2 s threshold, so R7 needs no extra `list_files` note.
`target/` makes up 94% of the entries and about 90% of the time. The
`find_files()` docs should still say that a `&**/…` walk from the repository
root visits build output, because the spec forbids content filters (Phase 3
docs). No other OS was measured (R12).

## Changed Outputs

Recorded as they are found; Phase 8 consolidates them.

- **`file(match(...))` and `$path` `*` stop crossing `/` (R4).** Shipped
  prompt `prompts/_reviews/review-implementation.md` (and its DMLS corpus copy
  `darkmatter/dmls/tests/fixtures/mapping_only_corpus/_reviews__review-implementation.md`)
  declares `template: file(match(prompts/*.md,.claudine/prompts/*.md))`.
  Today it also offers `prompts/_add/…`, `prompts/_reviews/…`, and the other
  nested partials. After Phase 2 it offers only top-level prompts. This is
  criterion 10's intended behavior, not a regression. No other shipped
  `match()` or `$path` pattern has a single `*` after a `/`.
- **All-negation lists (R6).** Today the two consumers disagree:
  `FileMatchGlobs::is_match` admits every path no negative rejects, while
  `triggers::matcher::path_matches` admits nothing. R6 replaces both with
  `GlobReferenceError::NoPositivePattern`. No existing test, shipped schema,
  or shipped prompt uses an all-negation list (searched `match("!…`, `$path`
  lists, and every `"!` literal under `darkmatter/` and `claudine/`). The
  only all-negative-like case is Claudine's unrelated `linking::filter`, which
  is not a glob reference. So the ruling stands and departs from no
  existing behavior that a test pins.

- **`%` recursive search is local-first (Phase 2, Decision 15).** It is now
  `take_first` of `**/<payload>`: the shallowest match under the first root
  that has one, instead of the lexical winner across all roots. No existing
  test expectation changed (all 1017 Phase 1 tests passed unmodified); the
  new behavior is pinned by `glob_reference::literal::recursive_references_are_local_first`.
  Shipped doc changed: the "Recursive Search" section of
  `biscuit-file/docs/topics/file-references.md`.
- **`%` and file symlinks (Phase 2).** The old walk counted only regular
  files, so a file symlink was never a `%` match. Now an in-tree file symlink
  can match; in a bare/`./`/`../` search one whose target leaves the tree is
  skipped; under `%&`/`%^` a match whose target leaves the repository still
  fails with `RepositoryEscape` (the post-match containment check is kept).
- **`%` with an absolute payload (Phase 2).** `%/a/b/x.md` now searches
  `/a/b/**/x.md`. The old walk required the parent to end with `a/b`, which
  in practice only matched `/a/b/x.md` itself.

- **`find_files()` merges every root, native order (Phase 3, criterion 10).**
  A bare pattern lists the document folder's matches, then the repository
  root's; `^` lists package, area, then repository. The "first existing
  candidate only" rule is gone: `find_files('near/*.md')` with `docs/near` a
  file now returns the repository root's `near/a.md` (was `[]`). Test
  `find_files_searches_the_first_existing_candidate_only` was rewritten as
  `find_files_merges_the_document_folder_and_the_repository_root`; its `Io`
  pin for a symlink-loop root is kept. "Sorted" became "shallowest first" in
  one row comment; no expected list changed in
  `find_files_reports_every_match_of_a_glob_reference`.
- **`find_files()` error wording (Phase 3).** Failures are
  `ExpressionError::GlobReference` with `GlobReferenceError`'s message:
  `` `&area/[` is not a valid glob ``, `` cannot use the `%` recursive modifier ``,
  `cannot use a remote URL` (were `invalid glob`, `not a `%` recursive
  reference`, `cannot search`). An invalid glob is now authoring-fatal like
  every other glob-reference failure (it was the non-fatal `Other`).
  `find_files_input_matrix` fragments updated.
- **`find_files()` and out-of-tree file symlinks (Phase 3, criterion 25).**
  Omitted and reported as one `dm.glob.skipped_symlink` warning per link
  (previously listed).
- **`::file-links` boundary from the context (Phase 3, criterion 14).** The
  process directory no longer bounds a document outside a repository, and an
  absolute glob outside the repository is listed (it was silently dropped).
  A relative glob that leaves the repository is a typed
  `FileLinksError::GlobReference { RelativeTreeEscape }` (strict) or a
  skipped directive with an `InvalidReference` warning (permissive); it was
  silently dropped. The repository icon follows the context's repository
  root, not a `.git` probe. Tests rewritten: discovery unit tests
  `glob_excludes_outside_cwd_boundary` →
  `without_a_repository_the_context_tree_root_bounds_the_glob`,
  `parent_escape_glob_is_filtered_by_boundary` →
  `a_relative_glob_cannot_leave_the_repository` +
  `an_absolute_glob_may_name_a_directory_outside_the_repository`; compose
  unit test `out_of_bound_path_is_ignored` →
  `a_relative_glob_leaving_the_repository_is_a_typed_error`;
  `rich_fixture_renders_full_presentation_contract` now supplies a context
  with a repository root (a bare `.git` directory is not a repository to the
  prepared context). `FileLinksError::InvalidGlob` was replaced by
  `FileLinksError::GlobReference`.
- **`::file-links` bare globs merge the repository root (Phase 3,
  criterion 10).** A bare `*.md` in a nested document also lists the
  repository root's top-level `*.md`; the tree is rooted at the common
  ancestor, which can now be the repository root.
- **`match()` definition errors (Phase 3, criterion 8).** A `match()`
  pattern that is not a glob reference fails schema parsing
  (`SchemaError::Grammar`, naming property and pattern). The grammar
  proptest generator (`schemas_grammar_proptest::glob_pattern`) now generates
  only valid glob references (it produced `!` alone, now invalid).
- **`match()` judgment (Phase 3).** The fixed anchor list (launch area,
  request directory, document directory, repository root) and the
  process-directory fallback are gone. A frontmatter value is judged from the
  document's folder and a caller-supplied value from its origin (launch)
  context, each by nearest root. In a single schema `match()` still only
  suggests, so no single-schema output changes. Every existing root-union
  test passes unchanged.

- **Windows: a reference naming `*` or `?` is a miss, not `Io` (Phase 3).**
  `FileReference::new("docs/*.md")` on Windows was `ResolutionFailure::Io`
  (the OS rejects the name); it is now `NoMatch`, as on Unix, and carries the
  literal-miss hint.

- **Schema roots replace the ancestor walk (Phase 4, Decision 24).** Trigger
  discovery and bare-name `$schema` lookup search the five roots (package,
  package area, `base_dir()`, `SCHEMAS_DIR`, `~/schemas`) of the document's
  context. In-between folders (`{package}/docs/schemas/`) are no longer
  searched; `SCHEMAS_DIR` and `~/schemas` now are. Unit tests that relied on
  `pkg/schemas` being an ancestor now declare `pkg` as the package
  (`discovery::tests::scan_for`, `schemas::phase4_trigger_assembly::doc_context`);
  `ancestor_walk_*` unit tests were replaced by
  `symlinked_tree_schemas_root_is_not_searched`,
  `scan_skips_in_between_schemas_folders`, and
  `scan_reads_another_repository_s_roots_never`.
- **`$path` is a glob reference (Phase 4).** `*` no longer crosses `/` (R4;
  the old matcher used `globset`'s default, which let `prompts/*.md` match
  `prompts/a/b.md`), a bare pattern is read from the trigger's folder rather
  than the discovery boundary, and `@`, `%`, `vault:`, `{{VAR}}` (anywhere),
  an invalid glob, and an all-`!` list are definition errors naming the
  pattern. No shipped `$path` trigger exists, so no shipped output changes.
- **`md schema triggers` output (Phase 4).** The `Boundary:` line and the
  walked `Schema roots:` list are gone; it prints the five roots in search
  order, marking absent folders, a duplicate of an earlier root, an unset or
  invalid `SCHEMAS_DIR`, and "no package / package area". Reviewed once on
  this repository (`md schema triggers claudine/README.md`):

  ```text
  Document: /Volumes/coding/wt/rusty-biscuit/fix-magic-globs/claudine/README.md
  Schema roots (search order):
  1. package root: none (the document is not in a package)
  2. package-area root: /Volumes/coding/wt/rusty-biscuit/fix-magic-
     globs/claudine/schemas
  3. file tree root: /Volumes/coding/wt/rusty-biscuit/fix-magic-globs/schemas
  4. SCHEMAS_DIR: unset
  5. home: /Users/ken/schemas (absent)
  Shadowed envelopes:
  - none
  Triggers:
  - none
  ```
- **`md schema about` `$path` row (Phase 4).** Its description now names the
  glob-reference prefixes and the refused ones instead of "boundary-relative
  path".
- **Shipped example (Phase 4, Decision 28).**
  `darkmatter/example-docs/schemas/external.md` names `$schema: ./external.yaml`.
- **DMLS outside a repository (Phase 4).** A workspace that is not a Git
  repository gives each document a context rooted at its own folder, so its
  triggers now come from that folder's `schemas/` (and `SCHEMAS_DIR`,
  `~/schemas`) rather than from the workspace folder's. The LSP test
  `trigger_payload_failure_retains_effective_schema_and_diagnoses_envelope`
  relied on the old reach (the open envelope in `schemas/` saw itself) and
  now runs in a repository; the two overlay unit tests whose fixture context
  had no repository now use one (`test_support::resolution_in_repository`).

- **`match()` completion reaches every root (Phase 5, criteria 3, 5, 17).**
  Claudine's TAB and ENTER walks used to walk the launch directory only and
  sort candidates by text. They now walk `roots()` (a bare pattern: the
  launch directory, then the repository root) in native order, and TAB
  values outside the launch directory are spelled `../…`, `&…`, `~/…`, or
  absolute. Existing tests changed:
  `schema_completion::tests::property_value_match_pattern_anchors_on_cwd_not_repo_root`
  asserted that a repository-root `docs/top.md` is **not** offered from
  `claudine/`; criterion 5 reverses that, so it became
  `property_value_bare_match_offers_the_launch_folder_then_the_repository_root`
  (offers `docs/area.md` then `../docs/top.md`; its fixture now runs
  `git init`, since a bare `.git` directory is not a repository to a
  prepared context and `../` was rendered absolute).
- **Shipped review router (Phase 5).** `prompts/review.md` declares
  `spec: file(required;eager;match(**/*spec*.md))`. Launched from a package
  with `spec=<partial>`, it now also offers repository-root specs that
  contain the partial, after the launch area's (a chooser where there used
  to be a single confirmation). The PTY fixture
  `claudine/cli/tests/common/review_router.rs` planted a repository-root
  "decoy" matching the partial `fixes/2026-09-10-local` to prove a
  launch-area-only walk (from `2026-09-10-no-interactive-completion`); that
  premise contradicts Decision 19 / criterion 5, so the decoy moved to
  `fixes/2026-09-10-remote-decoy/` (reached by the walk, filtered by the
  partial), the L2 capture assertions follow the new name, and the
  companion `level1_review_router_partial_repo_root_launch_widens_candidates_to_the_chooser`
  became `level1_review_router_partial_offers_a_repository_root_spec_after_the_launch_area`
  (package launch, a root spec matching the partial is second in the
  chooser and its identity survives the proxy). An author who wants the old
  launch-area-only reach writes `./**/*spec*.md`.
- **Chooser order (Phase 5).** The ENTER missing-property chooser and the
  provided-partial chooser no longer re-sort `match()` candidates by path;
  they list them in native order. The bare-`file` default glob keeps its
  sorted order.
- **`find_files()` failure row (Phase 7).** A `find_files()` whose glob
  reference fails (a tree escape, `&`/`^` outside a repository, a malformed
  prefix) now ends its `MarkdownError: interpolation failed` block with
  `failure: <class>`, in `md` and in `claudine compose`/`inline-compose`/
  `sequence`. Before, the block had no row, although
  `MarkdownError::resolution_failure()` already returned the class and
  `::file-links` with the same glob printed it. Found by the parity matrix's
  `md` cell `MdCompose × FindFiles × TreeEscape`.

## Departures from Spec

- **R2: optional first-root list mode not built (deliberate omission).** The
  spec calls it optional and no Darkmatter consumer uses it (Rule 2: no
  speculative features).
- **R3: warning code spelled `dm.glob.skipped_symlink`, not
  `compose.glob.skipped_symlink`.** Every existing compose warning code (and the
  DMLS diagnostic codes) uses the `dm.<family>.<code>` form (`dm.expression.unknown_identifier`,
  `dm.expression.parse_failure`, `dm.remote_cache.io_failure`,
  `dm.schema.missing_simplified_envelope`, `dm.transclusion.broken_path`).
  No `compose.` prefix exists. The closest spelling is `dm.glob.skipped_symlink`,
  one code shared by `find_files()` and `::file-links`.

- **Phase 2 implementation decisions** (details in [Phase 2](#phase-2)):
  error enum has `Unresolvable`/`InvalidContext`/`Io` and no
  `From<FileReferenceError>` (`resolution_failure()` instead); errors live in
  `glob/error.rs`; the literal-miss hint is `DetailedResolution::glob_hint()`
  rather than error `Display`; `$x/*.md` is a bare pattern, not
  `MalformedPrefix`; `\` is never an escape; a pattern with nothing after its
  prefix is malformed; `matches`/`roots` skip a pattern the context cannot
  root; file identity is canonical parent + name; `%` with an absolute
  payload searches below its directory.

- **Phase 3: caller-supplied values carry their origin into the validator.**
  The spec says a caller value's `cwd` is the launch directory; the
  validator is per document and could not tell a caller value from a
  frontmatter one. `FileValues::Resolved` and `ValidatorCache` now carry
  `CallerOrigins` (property → origin context, built from the schema stage's
  `caller_input_records`), and the `x-darkmatter-match` keyword picks its
  property's origin from its schema location. Part of the cache key, carried
  by `EffectiveSchema` into phase rebuilds, and public as
  `DarkmatterSchemas::with_caller_input_records` plus
  `caller_input_records_for_overrides` so a host that validates caller
  overrides itself (Claudine's pre-validation and launch schema) judges them
  as composition does. This touches Claudine's library in Phase 3, ahead of
  its Phase 5 track, because its tests broke otherwise.
- **Phase 3: `GlobReference::matches_without_context` (new biscuit-file
  API).** The structural validator (no request) must still judge an existing
  absolute path by its full path, because root-union coercion picks an arm
  that way (`contested_glob_decides_which_arm_coerces_siblings`). Darkmatter
  may not construct a context outside the builder (context guard), so
  biscuit-file gained a context-free judgment: bare patterns from the
  filesystem root, absolute patterns as written, every context-dependent
  pattern neither admits nor rejects. Consequence (pre-existing, unchanged):
  coercion cannot be steered by a `./`, `&`, `^`, or `@` pattern; see Phase 3
  "Open finding".
- **Phase 3: `match()` definition check lives in the simplified grammar**
  (`schemas/simplified/grammar.rs` `match` arm, via
  `file_match::definition_error`), the definition-check path `md` and DMLS
  share (R10). `x-darkmatter-match` compilation keeps its own schema error
  for raw JSON Schema input.
- **Phase 3: warning code `dm.glob.skipped_symlink`** (R3 spelling, see
  above). `find_files()` warnings travel through a request-shared
  `GlobWarningSink` (the ICMP sink pattern) drained by the root pipeline with
  `add_warnings`, keyed `(code, link)` so one link reached twice is one
  warning; `::file-links` adds them to its own transclusion report.

- **Phase 4: discovery takes the document's context, no path or boundary.**
  `triggers::scan(&ctx)` and `DarkmatterSchemas::with_trigger_discovery()`
  read the roots from the context they are given, which must be the checked
  document's own context; the registry keeps it and judges `$path` in it
  (`TriggerRegistry::context`, `evaluate_registry(registry, fm, Some(path))`).
  `schema_roots`, `normalize_path`, `normalize_relative_path`, and
  `TriggerRegistry.boundary` are deleted.
- **Phase 4: where discovery runs is unchanged.** `md compose`,
  `md schema validate`, `md clean`, and `md schema triggers` still discover
  triggers only inside a repository (the last still fails outside one with
  `missing-context`), and DMLS only inside a workspace folder. The spec's root
  table says root 3 is "always" present; it does not say discovery must start
  running outside a repository, so the existing gate stays.
- **Phase 4: per-pattern contexts for `$path`.** Bare and `./` patterns are
  read from the trigger's `pattern_cwd` (the folder holding its `schemas/`, or
  the document's `base_dir()` for `SCHEMAS_DIR` and `~/schemas`); every other
  prefix (`&`, `^`, `~`, absolute) is judged in the document's own context.
  Judging `^` from the trigger's folder instead would have made criterion 21's
  `^docs/**` miss a package document for a trigger in `{repo}/schemas`. Each
  pattern compiles to its own single-pattern `GlobReference` (negation kept
  beside it), which is equivalent to the list semantics (R11: per-pattern
  judgment). `../` is not forbidden by the spec and is accepted, read like
  `./`.
- **Phase 4: `$path` definition errors reuse `SchemaError::TriggerMatch`**
  with a message naming the authored pattern (R10): it is the existing
  trigger-definition path that `md` and DMLS both report.
- **Phase 4: symlinked roots.** Roots 1 to 3 must be real directories (a
  symlinked repository `schemas/` was already excluded); `SCHEMAS_DIR` and
  `~/schemas` are user configuration and may be symlinks to a directory.
- **Phase 4: `TriggerTrace` is no longer `serde::Serialize`.** Its roots are
  now `SchemaRoots`; nothing serialized the trace.
- **Phase 4: `SCHEMAS_DIR` joins the `md` test fixture's protected
  application inputs** (`darkmatter/cli/tests/common/protected_env.rs`), so a
  developer's shell value is scrubbed from every spawned `md` and a test
  declares it with `application_input`.
- **Phase 4 `$path` matrix outcomes (logged as the plan asks).** Absent
  (`match: {}`): the existing vacuous-arm load error. `$path: null` and an
  empty `$path:`: "must be a glob string ... got null", distinct from absent.
  Duplicate `$path` key: the YAML layer's existing refusal ("duplicate entry
  with key"), reported as a malformed trigger file.

- **Phase 5: `GlobReference::lists_file` (new public biscuit-file API).**
  `matches` judges a file symlink where it sits (Phase 2), so it admits an
  out-of-tree link that `list_files` skips. Claudine's walk filters while
  walking and cannot use `list_files`, and the plan forbids re-implementing
  containment in Claudine, so biscuit-file exposes the listing verdict for
  one path; `list_files` and `lists_file` share one rule
  (`PreparedSet::escaping_target`). `FileMatchGlobs::lists_file` wraps it.
- **Phase 5: DMLS cache key reuses Darkmatter's context encoder.** The spec
  lists repository root, `cwd`, `base_dir` origin, and a snapshot hash
  (`HOME`, environment, `@` roots). The key uses the new public
  `darkmatter::markdown::compose::file_resolution_context_identity`, the
  existing exhaustive encoder behind the compose graph identity, which
  covers those and more (source path, package and area roots, launch `@`
  scope, vault roots, external-relative opt-in), hashed with
  `biscuit-hash`. The build `generation()` stays in the key. No second
  hand-written identity exists in DMLS.
- **Phase 5: `md` vs DMLS parity is checked at the library level.** DMLS
  tests cannot run the `md` binary, so `dmls/tests/l1/schema_roots_parity.rs`
  compares DMLS's per-repository context and overlay with the context and
  `DarkmatterSchemas` assembly `md schema validate` performs (launched from
  the document's folder); `md`'s own output is pinned by the Phase 4 CLI
  tests over the same library calls. A full LSP session then checks the
  published diagnostics.
- **Phase 5: one-way parity asserts `file_match_admits` and `matches`.**
  The `x-darkmatter-match` validator shares the same `FileMatchGlobs`
  judgment (Phase 3) and is not driven separately per candidate.
- **Phase 6: the allowlist has five files, not the plan's four.** The plan
  names `file_reference/glob/parse.rs` only; `file_reference/glob/roots.rs`
  also names `globset` (it stores each prepared root's `GlobMatcher`). The
  spec's entry is "biscuit-file's `GlobReference` module", which covers both
  files, so `roots.rs` is listed rather than refactored (Rule 3).
- **Phase 6: the guard adds a manifest check.** The spec asks the guard to
  reject "any other glob crate". The `glob` crate cannot be found by a source
  search, because `glob` is also biscuit-file's own `file_reference::glob`
  module (`pub use glob::{..}`), so each scanned package's non-dev
  dependencies are pinned to an exact glob-crate set instead.
- **Phase 6: `Glob` is matched only as `Glob::`.** A bare `Glob` is an enum
  variant in `FileLinksMode` and Claudine's `PathSegment`; globset's `Glob` is
  reached through `Glob::new`, and any file that imports it already names
  `globset`.
- **Phase 7: `::file-links` and `match()` validation compare as sets.** The
  spec compares glob cells "by native-order file list". A `::file-links`
  tree is rendered by biscuit-terminal's `FileSystem` component, which
  orders entries as a directory listing (folders first, by name), so its
  order is presentation, not resolution; validation judges one value at a
  time and has no order. Both report `Observed::FileSet`. `find_files()`
  and both completion walks keep the ordered comparison.
- **Phase 7: completion's expectation drops `_`-prefixed folders.** For the
  same row, completion omits `_completed/` files that `match()` validation
  admits. That is criterion 6's one-way parity (suggestion filters only), so
  the shared expectation encodes it for `MatchCompletion` instead of
  treating it as a divergence.
- **Phase 7: the cross-entry comparison (Wave 14) is the shared
  expectation.** Every entry point on a glob row is compared with the same
  expected list or class, and every pair of observations that passes it
  agrees, so a separate pairwise check could never fail on its own and was
  not kept. Divergences the matrix found were fixed in code (the
  `find_files()` failure row above).
- **Phase 7: which entry points carry glob rows.** Table 1: the compose
  pipeline, `md compose`, and Claudine composition run all three consumers;
  pre-flight, schema validation, `md schema validate`, and DMLS diagnostics
  run `match()` validation. DMLS evaluates no expression and does not list a
  `::file-links` glob (its graph records the glob text as a file use), so it
  has no listing row. Table 2: `match()` validation of a caller value through
  the library (`--set`) and `claudine compose … spec=<value>`; completion
  through `claudine __complete` (TAB walk) and a new
  `EntryPoint::ClaudineChooser` for the ENTER walk (`file_candidate_paths`).
  The chooser needs a terminal, so its runner is a unit test of the
  claudine binary (`Owner::ClaudineCliChooser`), not a spawned process.
- **Phase 7: the tree escape has no Table 2 failure class.** A
  `match()` pattern whose root cannot be supplied admits nothing (passive
  validation, documented on `GlobReference::matches`), so validation and
  completion report `Observed::Unresolved` for it; `find_files()` and
  `::file-links` carry the `InvalidReference` class for the same row in
  Table 1.
- **Phase 6: ruling on `ignore` (Phase 5 hand-off).** The guard admits the
  `ignore` crate as a directory walker (`WalkBuilder`: DMLS discovery and
  watch, Claudine's completion walkers) and forbids its glob matchers
  (`OverrideBuilder`, `GitignoreBuilder`, `TypesBuilder`). No Claudine file
  needs an allowlist entry.

## Windows Evidence

None in Phase 1 (no code paths changed). The `backslash_escape` default
(Spike 1) is the Windows risk that Phase 2's tests must cover.

- **Phase 2:** `just cross-check biscuit-file --os windows` passed, 980/980,
  after one test-helper fix (see [Phase 2](#windows-evidence-1)).

- **Phase 3:** `just cross-check biscuit-file --os windows` failed first on
  the Phase 2 test `glob_reference::literal::a_literal_miss_that_looks_like_a_glob_hints_at_glob_references`
  (added after Phase 2's Windows run): Windows rejects `*`/`?` in a name with
  `InvalidFilename` (OS error 123), which `deepest_existing_ancestor` and the
  candidate probe treated as an `Io` failure, so `docs/*.md` was `Io` instead
  of `NoMatch`. Both now treat `InvalidFilename` as absent
  (`biscuit-file/lib/src/file_reference/resolve.rs`; recorded in the `os`
  skill, `windows.md` item 16). Re-run: **982 passed** (`cross-check-key:
  68f479db95be4e56`). `just cross-check darkmatter --os windows`:
  **7180 passed, 67 skipped** (`cross-check-key: be600858c4ae4b48`, after
  the fix; an earlier run before it also passed). Claudine was not
  cross-checked (plumbing only; CI's push-to-`main` Windows leg covers it).

- **Phase 4:** `just cross-check darkmatter-cli --os windows` first failed
  one new test, `schema_triggers::triggers_command_prints_the_five_schema_roots_in_search_order`:
  the package root printed as `…\cwd\area/pkg\schemas`, because the
  repository catalog spells package roots with `/`. Roots compared correctly
  (canonical identity); only the printed spelling was mixed. Fixed by
  re-collecting each root folder's components (`roots.rs::native_spelling`)
  and recorded in the `os` skill. Re-runs after the fix: darkmatter **7178
  passed, 67 skipped** (`cross-check-key: 02c2c53564bd09ce`), darkmatter-cli
  **825 passed, 69 skipped** (`08d0b6bc72e53093`), dmls **785 passed, 9
  skipped** (`21aa4421d5b6bb3a`).

- **Phase 5:** `just cross-check biscuit-file --os windows`: **pass**.
  `just cross-check dmls --os windows`: **790 passed, 9 skipped**
  (`cross-check-key: 119a8da664b47082`), including the new cache-key and
  parity tests. `just cross-check claudine-cli --os windows`: **2620 passed,
  3 failed, 11 skipped**. The failures are
  `loop_gate_ambient::{lifecycle_reference_loop_example_runs_as_written,
  loop_gate_info_renders_loop_count_on_every_pass}` ("compose failed" after
  the agent prompt of a counting `goose` stub loop) and
  `pr_flow_rehearsal::blocked_push_with_ask_triages_then_fixes_and_commits`
  (exit 1 in the `prompts/pr.md` push route). None of the three uses
  `match()`, completion, globs, triggers, or schema roots (no reference in
  either file or in the prompts they run), and every completion test passed
  on Windows. A baseline run of the same three tests from a clean `HEAD`
  worktree was attempted and refused by the rig's storage preflight
  (40.9 GiB free of the 50 GiB required); it was not forced. They are
  recorded as **unrelated, not confirmed pre-existing**; CI's push-to-`main`
  Windows leg will show them. The out-of-tree symlink completion test is
  Unix-only (creating a symlink needs a privilege on Windows); the
  biscuit-file rule it rests on is covered on Windows by `boundary.rs`'s
  test where links can be made.

- **Phase 6:** `just cross-check darkmatter --os windows
  glob_implementation_guard`: **2 passed** (`cross-check-key:
  e06665dcd83c95cd`). Linux: the archive leg failed twice before any test
  ran (`libbiscuit_file-*.rmeta is not writeable`, the stale read-only kache
  links in this worktree's standing clone that the `os` skill describes);
  the native path (`--features effects-instrumentation`) passed **2 of 2**.
  The guard's path keys are `/`-joined by `source_scan`, so separators do not
  affect it.

- **Phase 7:** no Windows run (the plan defers Windows to Phase 8's
  cross-check). The Windows-sensitive pieces are in the shared test module:
  `file_url_path` strips the leading `/` of a `file:///C:/…` link, candidate
  values are written with `/` separators (`to_portable_string`), and every
  path comparison goes through `PathIdentity` after canonicalization. Phase 8
  should run `entry_point_parity` on Windows for darkmatter, darkmatter-cli,
  dmls, and claudine-cli (plus the claudine-cli `parity_tests` unit test).

## Handed-off Reads

The `2026-09-30-file-refs-use-magic` ambient-state guard still carries the
two entries this feature must delete (Decision 22):

- `darkmatter/lib/src/markdown/compose/file_links/discovery.rs`:
  `resolve_boundary` reads the current directory.
- `darkmatter/lib/src/markdown/schemas/file_match.rs`: `admits` falls back to
  `std::env::current_dir()` when the validator has no `base_dir`.

Both carry the reason "handed off to 2026-09-30-glob-reference" and are
still present at the start of this feature.

- **Phase 3:** both reads are deleted (`resolve_boundary` and `admits`'s
  `current_dir` fallback), and both `HANDED_OFF` allowlist entries (and the
  constant) are removed from `darkmatter/lib/tests/l1/context_construction_guard.rs`.
  The guard passes with no handed-off entries.

## Phase 1

### Readiness check

- `git status` at start: only this log file, untracked and empty. The plan
  expected two modified test files
  (`darkmatter/dmls/tests/l1/repository_contexts.rs`,
  `darkmatter/lib/tests/l1/entry_point_parity.rs`). Neither is modified: both
  were committed in the branch's recent
  `2026-09-30-file-refs-use-magic` review iterations (`fd5521d2c`,
  `d91d9e8ae`, and parents). No unexpected files. Nothing to report.
- `2026-09-30-file-refs-use-magic` is at Phase 8 close: its implementation log
  ends with the Phase 8 final checkpoint and hand-off, which names this
  feature as next. Its later review-to-implement iterations (2–4) are
  committed (`b4190fc09`, `fd5521d2c`), and the Claudine port it was blocked on
  landed in `042b1025d`.
- **Baseline, biscuit-file `just test`:** 1017 tests run, 1017 passed, 0
  skipped.
- **Baseline, darkmatter `just test --no-fail-fast`:** 8842 tests run, 8841
  passed, **1 failed**, 12 skipped. (The default fail-fast run stopped after
  6420 tests on the same failure.)
  - Pre-existing failure:
    `darkmatter::l1 error_snapshots::link::unrecognized_format_mentions_html_and_markdown`
    (`darkmatter/lib/tests/l1/error_snapshots/link.rs:21`). The snapshot
    expects `` Markdown `[text](href)` link `` and the renderer now emits
    `` Markdown `text` link ``: an inline link inside a code span is being
    rewritten by prose rendering. It is unrelated to glob references, file
    resolution, or schemas, and this feature does not touch that path. Left
    as is. The `.snap.new` insta wrote was deleted so the tree stays clean.

### Rulings check

Each ruling's "the implementer first confirms…" clause was checked against
the code, not deferred:

- R1, R2, R5, R7, R8, R10, R11, and R12 need no Phase 1 evidence.
- R3: convention read; see [Departures](#departures-from-spec).
- R4: confirmed by Spike 1 and the code read above; stands.
- R6: searched for all-negation lists; none, so the ruling stands.
- R9: no Rust source reads a `SCHEMA_DIR` or `SCHEMAS_DIR` environment
  variable today (the only hit is `triggers/discovery.rs`'s directory-name
  constant `SCHEMAS_DIR_NAME = "schemas"`). The reader is Phase 4's.

### Dependency note

- `biscuit-file/lib/Cargo.toml`: `globset = { version = "0.4", optional = true }`
  added and enabled by the `file-reference` feature (`dep:globset`). Same
  `0.4` requirement as `darkmatter/lib/Cargo.toml`; `Cargo.lock` gains one
  line (biscuit-file's dependency list) and no new package.
- `cargo check -p biscuit-file` passes with default features and with
  `--no-default-features`.
- `biscuit-file/docs/dependencies.md` documents the dependency as
  **planned** (no code uses it until Phase 2) and records the
  `backslash_escape` rule. The root `docs/dependencies.md` lists only
  per-crate summaries for biscuit-file and already lists `globset`, so it is
  unchanged.

### Tests

Phase 1 changes no behavior, so it adds no tests. The two spikes are
throwaway probes, as the plan directs, and were not committed. The baselines
above are the regression evidence, and they ran after the dependency was
added to `Cargo.toml`.

### Gates at the end of Phase 1 (macOS)

| Area | `just test` | `just lint` |
|---|---|---|
| biscuit-file | 1017 passed, 0 skipped | pass |
| darkmatter (lib, cli, dmls, zed-dmls; `--no-fail-fast`) | 8841 passed, 1 failed (the pre-existing snapshot above), 12 skipped | pass |

No cross-OS run: Phase 1 adds an unused optional dependency and changes no
code path. The `darkmatter` skill needed no update.

## Phase 2

`GlobReference` now exists in biscuit-file and `%` runs on it. No other
package changed.

### What was built

- `biscuit-file/lib/src/file_reference/glob/`:
  - `parse.rs`: pattern grammar `[!][prefix]glob` on top of
    `file_reference::parse::parse` (so prefix errors are `FileReference`'s
    own), `SplitPattern` (literal directory prefix vs. glob tail), and
    `compile`, the only `globset` call in biscuit-file:
    `literal_separator(true)`, `case_insensitive(false)`,
    `backslash_escape(false)`. `escape` delegates to `globset::escape`.
  - `roots.rs`: `prepare` resolves one pattern against a context through the
    new shared `resolve::reference_roots` (the single sigil-to-roots mapping
    `%` and direct resolution use), narrows each root by the literal
    directory prefix, absorbs leading `..` hops into the root, and runs the
    tree / repository containment checks on each walk directory.
    `owning_root` is the R8 helper; `canonical_prefix` canonicalizes the
    longest existing prefix.
  - `list.rs`: merged positive roots in precedence order, a walk per
    outermost walk directory (`follow_links(false)`), native order per root,
    one examination per canonical file, skipped out-of-tree file symlinks.
  - `matches.rs`: lexical membership (canonical parent prefix + file name).
  - `error.rs`: `GlobReferenceError` with `resolution_failure()`.
  - `mod.rs`: `GlobReference` (`new`, `with_file_name_view`, `patterns`,
    `escape`, `list_files`, `take_first`, `matches`, `roots`),
    `GlobListing`, `SkippedEntry`, and the crate-internal
    `take_first_recursive` for `%`.
- `resolve.rs` extractions (extract, not copy): `anchored_repository_root`
  (was duplicated in `resolve_core` and `candidate_plan`),
  `interpolate_pieces` (authored text vs. `{{VAR}}` values), and
  `reference_roots`. `resolve_recursive_core` keeps its root planning, its
  per-root containment check, and its post-match check, and delegates the
  search to `take_first_recursive`; `recursive_subdir_filter` and the old
  walk/sort are deleted.
- `DetailedResolution::glob_hint()` (the literal-miss hint).
- Re-exports from the crate root: `GlobReference`, `GlobReferenceError`,
  `GlobListing`, `SkippedEntry`.

### Departures and decisions made while implementing

Recorded here and summarized under [Departures from Spec](#departures-from-spec):

1. **Error enum shape.** Variants: `RejectedPrefix { pattern, prefix, reason }`,
   `MalformedPrefix`, `InvalidGlob { pattern, message }`, `NoPositivePattern`,
   `RelativeTreeEscape`, `OutsideRepository`, plus three the plan did not
   list: `Unresolvable { pattern, source }` (missing home/vault/variable,
   repository escape, injected sigil), `InvalidContext`, and `Io`. There is
   no `From<FileReferenceError>`: a glob error must name its pattern, which a
   `From` cannot supply. Instead `GlobReferenceError::resolution_failure()`
   gives the same `ResolutionFailure` class a `FileReference` failure of the
   same kind reports (a remote prefix is `UnsupportedRemote`), which is what
   the parity matrix compares. `InvalidGlob` carries the message, not
   `globset::Error`, so `globset` stays out of the public API.
2. **The error lives in `glob/error.rs`**, a sixth file beside R1's five.
3. **Literal-miss hint is data, not `Display`.** A no-match is
   `Ok(None)` / `ResolutionFailure::NoMatch` with no `FileReferenceError`, so
   there is no error `Display` to extend. `DetailedResolution::glob_hint()`
   returns the hint for a no-match whose text contains `*`, `?`, or `[`;
   consumers (Phase 3) append it to their own no-match message.
4. **`$x/*.md` is a bare pattern, not `MalformedPrefix`.** `$` is not a sigil
   in the reference grammar (`FileReference::new("$x/a.md")` is a valid bare
   reference), and the spec requires one prefix grammar. The grammar test
   asserts the bare reading and uses `ftp:x/*.md` (unsupported scheme) and
   `~user/*.md` for the malformed rows.
5. **`\` is never an escape.** `backslash_escape(false)` on every OS. On Unix
   `\` is a literal name character; on Windows it is a path separator (the
   host path grammar) and is spelled `/` in the glob. Literal text is escaped
   only with character classes (`escape`).
6. **A pattern with nothing after its prefix is malformed** (`^docs/`, `!`,
   `^`): it could only name a directory, which a file listing never returns.
7. **`matches` and `roots` skip a pattern whose roots the context cannot
   supply** (for example `&` outside a repository): such a positive pattern
   admits nothing and such a negation rejects nothing. They return `bool` /
   `Vec` per R1; `list_files` and `take_first` report the error.
8. **File identity is the canonical parent plus the file name**, not the
   fully canonical path, so two file symlinks to one target stay two entries
   while `/var` and `/private/var` spellings collapse.
9. **Several positive patterns** merge their roots in first-appearance order;
   a file is listed in the first pass that reaches it. Nearest-root judgment
   per pattern (R11) makes that the owner's pass.
10. **Walk errors below a search directory are ignored** (as the old `%` walk
    did); a search directory that exists but cannot be canonicalized is
    `GlobReferenceError::Io`.
11. **Preparation is not a walk but does read later roots' metadata** for
    `&`/`^` (repository containment of each walk directory) and for
    bare/`./`/`../` (tree containment). `take_first` never *walks* a later
    root; the proof test uses `@`, which has no containment pre-check.
12. **`%` absolute payloads** search below their own directory (see Changed
    Outputs).

### Requirement-to-test mapping

All in `biscuit-file/lib/tests/l1/glob_reference/` (L1, declared in
`tests/l1/main.rs`; no tier markers in any path segment):

| Requirement | Test |
|---|---|
| Wave 3 grammar matrix (control row + one edit per shape, incl. `!` in `FileReference`, duplicates, unknown prefix) | `grammar::every_pattern_shape_has_a_defined_outcome` |
| `InvalidGlob` names the pattern; class | `grammar::an_invalid_glob_names_the_pattern_and_the_cause` |
| Criterion 15 (order, each once, `take_first`, `roots`) | `order::list_files_orders_by_root_precedence_then_depth` |
| Criterion 15 (`take_first` does not walk later roots; Unix) | `order::take_first_never_walks_a_later_root` |
| Criterion 16 (tie-break) | `order::ties_break_component_wise` |
| Criteria 18, 5 (ownership with exclusion, nearest-root negation) | `order::an_excluded_file_is_not_reincluded_by_a_later_root` |
| Criterion 4 (`!&**/_completed/**`) | `order::a_repository_exclusion_composes_with_a_scoped_pattern` |
| Symlinked-spelling dedup (`/var` vs `/private/var` analogue, all OSes) | `order::one_file_reached_through_two_spellings_is_listed_once` |
| Bare = `cwd` then repository; `./` = `cwd` only | `order::bare_patterns_search_cwd_then_the_repository_root` |
| R11 file-name view, negations, criterion 11 (biscuit-file part) | `order::the_file_name_view_widens_only_slashless_patterns` |
| Criterion 14: relative escape is `RelativeTreeEscape`; opt-in succeeds | `boundary::relative_globs_cannot_leave_the_tree` |
| Criterion 14: `~`, `@`, absolute, vault, `{{VAR}}` allowed | `boundary::unbound_roots_may_lie_outside_the_repository` |
| Criterion 14: `&`/`^` outside a repository = `OutsideRepository` | `boundary::repository_sigils_outside_a_repository_are_typed_errors` |
| Criterion 14: directory symlink out of tree not followed | `boundary::a_walk_does_not_follow_a_directory_link` |
| Criterion 25 (biscuit-file part) | `boundary::a_file_link_out_of_the_tree_is_skipped_by_a_bound_glob` |
| Criterion 19 (`[id]` literal vs class, `%`, `escape`) | `literal::brackets_are_literal_in_a_file_reference_and_a_class_in_a_glob` |
| `escape` covers `[ ] * ? { }`, leaves `\` | `literal::escape_makes_every_metacharacter_literal`, `literal::escaped_names_find_exactly_the_literal_file` (Unix) |
| Criterion 23 (`%^README.md` local-first, `%vault:notes.md`) | `literal::recursive_references_are_local_first` |
| `%` shapes: parent suffix, `./`, `{{VAR}}` with glob chars, absolute | `literal::recursive_references_keep_their_payload_literal` |
| Criterion 26 (lexical `matches` on missing paths) | `literal::matches_is_lexical_for_paths_that_do_not_exist` |
| Criterion 26 (case-sensitive) | `literal::matching_is_case_sensitive` |
| Literal-miss hint | `literal::a_literal_miss_that_looks_like_a_glob_hints_at_glob_references` |
| Criterion 13 (biscuit-file part, unfiltered) | `unfiltered::hidden_ignored_and_underscore_files_are_listed` |
| `%` regressions (existing behavior kept) | existing `finalized_reference_resolution`, `implicit_relative`, `precedence_flip`, `file_tree`, `detailed_resolution`, `portable_path::excess_parent` tests, unchanged |

The Input Robustness Matrix does not apply: Phase 2 reads no file format or
configuration. The pattern grammar is covered by the grammar matrix above.

One regression found by the existing suite while implementing:
`finalized_reference_resolution::canonical_containment_accepts_internal_links_and_rejects_external_targets`
failed when the per-root `root/<payload>` containment check was dropped from
`%`; the check was restored, and the test passes.

### Gates (macOS)

| Area | Command | Result |
|---|---|---|
| biscuit-file | `just test` | 1042 passed, 0 skipped |
| biscuit-file | `just lint` | pass |
| biscuit-file | `cargo check --no-default-features` / `--features file-reference` | pass |
| biscuit-file | doctests (`cargo test --doc`) | pass, incl. the new `GlobReference` example |
| darkmatter (lib, cli, dmls, zed-dmls) | `just test --no-fail-fast` | 8841 passed, 1 failed, 12 skipped: the pre-existing `error_snapshots::link::unrecognized_format_mentions_html_and_markdown`, same as the Phase 1 baseline; its `.snap.new` was deleted |
| darkmatter | `just lint` | pass |

`just check-tier-coverage` does not exist as a recipe in the `biscuit-file`
area or at the root under that name; the new tests carry no tier marker, so
they run in L1.

### Windows evidence

`just cross-check biscuit-file --os windows` (host `build-win-native`):

- First run: 979/980. `glob_reference::boundary::a_walk_does_not_follow_a_directory_link`
  failed in the test helper, not the library: `repo.join("docs/shared")`
  kept a `/`, and `cmd /C mklink /J` read `/shared` as a switch. This is the
  known trap in the `os` skill (`windows.md` item 10). The helper now
  re-spells both paths natively before calling `mklink`.
- Second run: **980 passed, 0 skipped** (`cross-check-key: 5aec9ffbd33e5d51`).
  File symlinks were created successfully on that host, so criterion 25's
  biscuit-file test ran there.

### Docs and skills

- `biscuit-file/docs/topics/file-references.md`: "Recursive Search" rewritten
  as local-first; new "Glob References: `GlobReference`" section (calls,
  native order with a Mermaid diagram, pattern rules, boundary and
  symlinks, the literal-miss hint). Phase 8 may expand it.
- `biscuit-file/docs/dependencies.md`: `globset` no longer "planned".
- `.claude/skills/biscuit-file/SKILL.md` (triggers + `GlobReference`
  pointer) and `references/file-references.md` (`%` no longer lexical).
- The `darkmatter` skill needs no change yet: no Darkmatter code changed.

## Phase 3

`match()`, `find_files()`, and `::file-links` run on `GlobReference`; both
handed-off current-directory reads are gone. Packages: `darkmatter`
(lib), `biscuit-file` (one additive API), and `claudine` / `claudine-cli`
(kept compiling and green; the roots-based completion walk is still Phase 5).

### What was built

- **Track A, `FileMatchGlobs`** (`darkmatter/lib/src/markdown/schemas/file_match.rs`):
  a wrapper over `GlobReference::with_file_name_view()` exposing `new`
  (returns the `GlobReferenceError`), `matches(path, ctx)`, and
  `roots(ctx)`. `compile`, `is_match`, `admits_path`, the anchor list, and
  the `current_dir` fallback are deleted. `file_match_admits` judges in the
  caller's origin. The `x-darkmatter-match` keyword judges in a
  `JudgedIn` chosen per keyword: structural (only an existing absolute
  path, via `matches_without_context`), the document's value context, or a
  caller property's origin (`CallerOrigins`, see Departures).
  `definition_error` backs the grammar's `match` check (criterion 8).
- **Track B, `find_files()`** (`compose/expression/functions/mod.rs`): the
  prefix splitter, `collect_glob_matches`, and `resolve_document_directory`
  (now unused) are deleted. It is `GlobReference::new([pattern]).list_files`
  in `ResolutionContext::file_context()`, native order, file-name view off.
  Failures are the new `ExpressionError::GlobReference { function, source:
  Arc<GlobReferenceError> }` (authoring-fatal), and
  `MarkdownError::resolution_failure()` finds its class through the `Arc`.
  A URL is passed un-normalized so it is rejected as remote (normalizing
  collapsed `https://` to `https:/`, a malformed-scheme error; found by
  `find_files_input_matrix`).
- **Track C, `::file-links`** (`compose/file_links/`): `discover` takes the
  document's `FileResolutionContext` (`source_file_resolution_context()`);
  glob mode is `list_files` plus the directive's own rules (extension
  allowlist, self-exclusion, canonical dedup with the lexical display path).
  `--dir` keeps its walk with the boundary taken from `base_dir()`.
  `resolve_boundary`, `split_glob_prefix`, `path_to_forward_slashes`, and the
  `globset` import are deleted. `FileLinksResult.skipped` carries skipped
  links; the engine adds one warning each. `FileLinksError::InvalidGlob` became
  `FileLinksError::GlobReference { line, source }`.
- **Warnings** (`compose/glob_listing.rs`, `context/report.rs`):
  `ComposeWarning::GLOB_SKIPPED_SYMLINK_CODE` (`dm.glob.skipped_symlink`) and
  `ComposeWarning::skipped_symlink` (identity = the link path), plus
  `GlobWarningSink`, shared through `ComposeOptions.glob_warnings` →
  `ResolutionContext.glob_warnings` and drained in `pipeline/mod.rs` with
  `add_warnings`. The sink is classified as non-identity in
  `classify_options`.
- **Caller origins** (`schemas/validate.rs`, `schemas/mod.rs`,
  `compose/schema_validation.rs`, `compose/context/options.rs`):
  `CallerOrigins` / `ValueOrigin`, `build_validator_with_callers`,
  `ValidatorCache::with_caller_origins` (in the cache key and `JudgedIn`),
  `EffectiveSchema.caller_origins` (phase rebuilds keep them), public
  `DarkmatterSchemas::with_caller_input_records`, and public
  `caller_input_records_for_overrides` (the "no explicit records" rule,
  extracted from `schema_validation::caller_input_records` so Claudine uses
  the identical rule).
- **biscuit-file**: `GlobReference::matches_without_context`.
- **Claudine** (compatibility only): the two completion walks call
  `MatchGlobs::new(..)` and `matches(path, &file_resolution_context(ctx))`
  instead of `compile`/`is_match` (still walking the launch directory);
  `load_effective_schema` takes the caller records (pre-validation builds them
  with `caller_input_records_for_overrides`; the launch schema and
  translation use `PrepareOptions::schema_caller_records`).
- **Guard**: both `HANDED_OFF` entries and the constant removed from
  `darkmatter/lib/tests/l1/context_construction_guard.rs`.

### Existing-test reconciliation

First full `just test` after Waves 5 (darkmatter): 8840 run, 10 failed.
Classification:

| Test | Class | Resolution |
|---|---|---|
| `schema_validation::tests::undecided_root_union_resolves_a_caller_file_from_the_launch_area`, `schema_validation_integration::contested_glob_decides_which_arm_coerces_siblings`, cli `compose_schema::shipped_plan_prompt_anchors_caller_file_from_{repository_root,package_area}` | bug | caller values were judged from the document's folder; fixed with caller origins and `matches_without_context` (no test edits) |
| `required_context::structural_validators_resolve_only_context_free_file_values` | bug | the structural validator briefly admitted everything; fixed with `matches_without_context` |
| `schemas_grammar_proptest::round_trip_random_atoms` | intended (criterion 8) | generator restricted to valid glob references |
| `file_links_compose::rich_fixture_renders_full_presentation_contract` | intended | repository icon follows the context; test supplies a repository root |
| `file_links_compose::out_of_bound_path_is_ignored` | intended (criterion 14) | rewritten as `a_relative_glob_leaving_the_repository_is_a_typed_error` |
| `find_files_and_try_frontmatter::find_files_searches_the_first_existing_candidate_only` | intended (criterion 10) | rewritten, `Io` pin kept |
| `find_files_and_try_frontmatter::find_files_input_matrix` | intended + one bug | message fragments updated; URL normalization bug fixed |

Discovery unit tests in `file_links/discovery.rs` were updated for the new
signature and the CWD-boundary behavior they pinned (see Changed Outputs);
the `split_glob_prefix_*` unit tests were deleted with the function.

Claudine `just test` then showed 5 failures in `claudine-cli::l1
compose_schema_cli` (`compose_contested_file_match_selects_and_rejects_union_arms`,
`compose_enforces_each_root_union_arm_match_before_provider_launch`,
`compose_caller_file_resolves_from_the_launch_directory_for_every_schema_shape`,
`compose_late_caller_file_verdict_names_the_launch_directory`,
`compose_unresolved_caller_file_fails_before_initialize_when_non_interactive`):
all bugs of the same kind (Claudine pre-validates and re-validates at
completion with its own `DarkmatterSchemas`, which did not know the caller
origins). Fixed by `with_caller_input_records` and
`EffectiveSchema.caller_origins`; no Claudine test was edited. The Claudine
`match_globs_*` unit tests (criterion 9's `*.png`, `src/**/*.rs`, `!_*.md`)
keep every assertion; only the call spelling changed to `matches(path, ctx)`.

`just test-l2` (darkmatter) then failed one test,
`level2_render_tree_terminal::file_links::level2_file_links_directive_renders_styled_tree_in_real_terminal`:
intended (repository icon follows the context); its fixture now runs
`git init` instead of creating a bare `.git` directory. Re-run: 18 + 69 + 3
passed.

### Requirement-to-test mapping

New tests, all L1 (no tier marker in any path segment), declared in their
crate's `tests/l1/main.rs` or inline `#[cfg(test)]` modules:

| Requirement | Test |
|---|---|
| Criterion 1 (validation half), Incident 2 | `darkmatter::l1 glob_consumers::match_validation_reads_reference_prefixes` |
| Criteria 4, 5, 11 (`match`), 12 (validation) | same test |
| Criterion 7 (caller value from the launch directory) | `glob_consumers::a_caller_value_is_judged_from_the_launch_directory` (mutation-checked: fails with caller origins removed) |
| Criterion 8 | `glob_consumers::match_rejects_patterns_that_are_not_glob_references` |
| Criterion 9 | existing `file_match::tests::*`, Claudine `match_globs_*` (assertions unchanged), `file_match::tests::globs_judge_a_path_relative_to_the_value_cwd` |
| Criterion 10 (`find_files`), 11 (`find_files`), 13 | `glob_consumers::find_files_merges_every_root_most_local_first`, `find_files_and_try_frontmatter::find_files_merges_the_document_folder_and_the_repository_root` |
| Criterion 10 (`::file-links`), 13 (self-exclusion) | `glob_consumers::file_links_merges_every_root_most_local_first` |
| Criterion 14 (both consumers, absolute allowed, `&` outside a repository) | `glob_consumers::relative_globs_stay_inside_the_repository`, `file_links_compose::a_relative_glob_leaving_the_repository_is_a_typed_error`, `discovery::tests::{a_relative_glob_cannot_leave_the_repository, an_absolute_glob_may_name_a_directory_outside_the_repository, without_a_repository_the_context_tree_root_bounds_the_glob}` |
| Criterion 14 (handed-off reads) | `context_construction_guard::production_source_builds_contexts_only_through_the_builder` (no handed-off entries) |
| Criterion 25 (consumer side; count, wording, dedup across two evaluations, in-tree link kept) | `glob_consumers::an_out_of_tree_file_symlink_is_skipped_with_one_warning`, `discovery::tests::escaping_symlinked_file_is_dropped` |
| Parser accepts prefixes | `file_links::parser::tests::parses_reference_prefixed_globs_as_one_token` |
| `matches_without_context` | `biscuit-file l1 glob_reference::literal::matches_without_context_judges_the_full_path` |

`allow_external_relative()` success for a relative escape is covered at the
biscuit-file level (`boundary::relative_globs_cannot_leave_the_tree`); a
compose request has no public opt-in for it, so no consumer-level row
exists. The Input Robustness Matrix does not apply (no file format or
configuration reader changed); the pattern grammar's matrix is biscuit-file's
(Phase 2).

### Open finding (pre-existing, not a regression)

Root-union **coercion** is context-free (`coerce.rs` uses structural
validators), so it can only judge an existing absolute value by bare and
absolute patterns. When the arm is decided only by a `./`, `&`, `^`, or `@`
pattern, coercion picks no arm while contextual validation does, and a
sibling that needs arm-specific coercion (`count: "5"` under `number`) fails
validation. The old code had the same gap (it compared the literal `./…`
glob against the full path). Documented in the schema definition page;
recorded for a later decision (see `message_to_agent` in the spec).

### Gates (macOS unless noted)

| Area | Command | Result |
|---|---|---|
| darkmatter (lib, cli, dmls, zed-dmls) | `just test --no-fail-fast` | 8848 passed, 12 skipped, 0 failed (the Phase 1 snapshot failure no longer reproduces) |
| darkmatter | `just test-l2` | 18 + 69 + 3 passed |
| darkmatter | `just lint` | pass |
| biscuit-file | `just test` / `just lint` | 1043 passed / pass |
| claudine (lib, cli, gen, …) | `just test --no-fail-fast` / `just lint` | 8085 passed, 9 skipped / pass |
| biscuit-file, Windows | `just cross-check biscuit-file --os windows` | 982 passed |
| darkmatter, Windows | `just cross-check darkmatter --os windows` | 7180 passed, 67 skipped |

`just check-tier-coverage` does not exist in this checkout (see Phase 2);
every new test path is free of tier markers and is compiled by a declared
target (`darkmatter/lib/tests/l1/main.rs` declares `glob_consumers`; the
rest are inline unit tests or existing L1 files).

### Docs and skills

- `darkmatter/docs/inline/file-links.md` (glob form, folders table, boundary,
  errors), `darkmatter/docs/topics/schemas/definition.md` (`match()` glob
  references, value directory, nearest root, file-name view, definition
  errors, coercion note), `darkmatter/docs/schemas/expression-functions.yaml`
  and the mirrored row in `darkmatter/docs/topics/darkmatter-expressions.md`
  (`find_files()`).
- `biscuit-file/docs/topics/file-references.md`: `matches_without_context`
  row; fixed a Phase 2 drift (step 5 of the resolution algorithm still said
  `%` "sorts matches lexically across roots").
- Skills: `.claude/skills/darkmatter/SKILL.md` (glob consumers, caller
  origins, `git init` fixtures), `.claude/skills/darkmatter/compose.md`,
  `.claude/skills/darkmatter/schema.md`, `.claude/skills/biscuit-file/SKILL.md`,
  `.claude/skills/os/windows.md` (item 16).

## Phase 4

Five schema roots replace the ancestor walk, and `$path` triggers run on
`GlobReference`. Packages: `darkmatter` (lib), `darkmatter-cli`, and `dmls`
(kept compiling and green; its caching and parity work is Phase 5).

### What was built

- **Root list** (`darkmatter/lib/src/markdown/schemas/roots.rs`, new, public
  as `darkmatter::markdown::schemas::{SchemaRoots, SchemaRoot, SchemaRootKind,
  SchemaRootState, SearchedRoot, InvalidSchemasDir, SCHEMAS_DIR_VARIABLE}`):
  `SchemaRoots::for_document(&ctx)` returns all five roots in order with a
  state each (`Searched`, `Absent`, `Duplicate { of }`, `NotApplicable`,
  `Invalid { value, reason }`), plus `search_paths()` (bare-name lookup) and
  `searched()` (each with its `pattern_cwd`). Inputs: `ctx.package_root()`,
  `ctx.package_area()`, `ctx.base_dir()`, `ctx.env()["SCHEMAS_DIR"]`,
  `ctx.home_dir()`; dedup by `canonicalize_simplified` + `PathIdentity`.
  Nothing reads the process (the context guard still passes).
- **Discovery** (`triggers/discovery.rs`): `scan(&ctx)` enumerates the
  searched roots, shadows by file name, and records each trigger's
  `pattern_cwd`; the registry keeps `roots: SchemaRoots` and the context.
  Module doc rewritten. `DarkmatterSchemas::with_trigger_discovery()` takes no
  arguments; bare-name `$schema` lookup (`resolve.rs`, unchanged code) now
  walks `roots.search_paths()`.
- **`$path`** (`triggers/grammar.rs`, `matcher.rs`, `assemble.rs`):
  `PathGlobs::new` compiles one `GlobReference` (file-name view) per pattern
  and refuses `@`, `vault:`, `%`, `{{` anywhere, invalid globs, and all-`!`
  lists with a `TriggerMatch` error naming the pattern. The matcher takes a
  `PathSubject` (document path + context + the trigger's `pattern_cwd`);
  `PathSubject::detached()` replaces the old `""` path. The thread-local
  `globset` cache and `compile_globs` are deleted.
- **Callers**: compose (`schema_validation.rs`, gate renamed
  `discovers_triggers`), `md clean` (`schemas/clean.rs`), `md schema validate`,
  `md schema triggers` (new output via `OrderedList` + `Prose`), and DMLS
  (`overlay/mod.rs` scans `context.for_source(path)`).
- **Shipped example**: `darkmatter/example-docs/schemas/external.md`.
- **Docs**: `definition.md` ("Repository Trigger Schemas" rewritten with
  "Schema roots" and "Matching a path with `$path`" subsections, the bare-name
  paragraph, `md schema triggers` sample), `dmls-schema-support.md` (diagram,
  DMLS now reads `SCHEMAS_DIR` and `~/schemas`, restart caveat),
  `authoring-schemas.md` (five-root list, `{root}` outside a repository,
  `SCHEMAS_DIR`), `schema-activation.md` and `claudine/docs/rollout-strategy.md`
  (`SCHEMA_DIR` → `SCHEMAS_DIR`). `git grep SCHEMA_DIR` over every `docs/`
  tree and `.claude/skills` now returns nothing. Phase 8 still owns the full
  docs pass (`match()` section examples, README, Claudine docs).
- **Drifted comments fixed** (code is right): `md clean`'s two "git-root
  ancestor walk" comments, DMLS `trigger_boundary`'s doc (it now only gates),
  the `discovery.rs` test-fixture comment, and the `$path` descriptor in
  `about.rs`.

### Requirement-to-test mapping

All new tests are L1 (no tier marker in any path segment).
`darkmatter/lib/tests/l1/main.rs` declares `schema_roots`; the CLI tests are
in the already-declared `darkmatter/cli/tests/l1/schema_triggers.rs`.

| Requirement | Test |
|---|---|
| Wave 7 root list; criterion 27 (order, document's package not the launch's) | `darkmatter::l1 schema_roots::schema_roots_are_package_area_tree_schemas_dir_then_home` |
| Wave 7 input matrix (control, absent, empty, whitespace, relative, missing, `SCHEMA_DIR`, equals `~/schemas`, no home) | `schema_roots::schemas_dir_and_home_input_matrix` |
| Criterion 28 (shadowing both directions, shadowed trigger reported and not evaluated) | `schema_roots::an_earlier_root_shadows_a_later_one_by_file_name` |
| Criterion 29 (`SCHEMAS_DIR` names the folder itself; bare `docs/*.md` from the repository root; unset adds nothing) | `schema_roots::schemas_dir_names_the_schemas_folder_itself`, cli `schema_triggers::schema_validate_applies_triggers_from_schemas_dir_and_home` |
| Criterion 30 (in-between folders) | `schema_roots::in_between_schemas_folders_are_not_discovered`, `discovery::tests::scan_skips_in_between_schemas_folders` |
| Criterion 31 (`~/schemas` trigger and `$schema: user.yaml`) | `schema_roots::home_schemas_apply_in_a_repository_with_no_schemas_folder`, cli `schema_validate_applies_triggers_from_schemas_dir_and_home` |
| Criterion 21 (`^docs/**`, `~/notes/**`, bare `*.md` at any depth, forbidden prefixes incl. after `!`) | `schema_roots::path_triggers_read_reference_prefixes`, `schema_roots::forbidden_path_prefixes_are_definition_errors_naming_the_pattern` |
| Criterion 21 last sentence (trigger vs `match()` parity, incl. nearest root, negation, file-name view, case) | `schema_roots::path_triggers_and_match_validation_give_the_same_verdicts` (mutation-checked: ignoring `pattern_cwd` fails it) |
| Wave 8 `$path` field matrix (control + 15 edits) | `schema_roots::path_field_input_matrix` |
| Criteria 27, 29 through `md schema triggers` | cli `schema_triggers::triggers_command_prints_the_five_schema_roots_in_search_order`, `triggers_command_marks_unset_and_invalid_schemas_dir` |
| Definition errors through `md schema validate` | cli `schema_triggers::schema_validate_reports_a_forbidden_path_prefix_naming_the_pattern` |
| Criterion 33 (shipped example, passive + end-to-end) | cli `schema_triggers::shipped_external_schema_example_names_its_sibling_explicitly` |
| Matcher unit behavior (file-name view, separator, negation, case, no document) | `triggers::matcher::tests::path_*` |

Criterion 32 (`md` and DMLS agree) is Phase 5's: DMLS now calls the same
`scan` on the document's context, but its parity tests are scheduled there.
The plan's "absent from the snapshot even if set in the test process" cell is
covered structurally: the reader takes only `ctx.env()`, the context guard
forbids process-environment reads in `src/`, and the test process's
environment is never mutated (repository rule).

### Repository check

`ls -d */schemas */*/schemas` (plus root `schemas/`): `schemas/`,
`claudine/schemas/`, `darkmatter/schemas/` remain roots; the in-between
`claudine/docs/schemas/`, `darkmatter/docs/schemas/`, and
`darkmatter/example-docs/schemas/` are no longer searched. `md schema validate`
over all 364 documents naming `feature-review.yaml`, `review.yaml`,
`suggestion-review.yaml`, or `memory.yaml` by bare name: 364 valid. Over the
other bare-name documents: two fail as before this change
(`claudine/fixes/_completed/2026-08-31-silent-success-and-startup-stall/log.md`
names `fix-log.yaml`, `darkmatter/features/_completed/2026-07-14-invalid-frontmatter/log.md`
names `feature-log.yaml`; neither file exists anywhere in the repository).

### Existing-test reconciliation

First full `just test` after the change: 2 failed (DMLS overlay unit tests
whose fixture context had no repository; intended, see Changed Outputs), then
1 failing 8/12 runs (`dmls::l1 lsp_session::trigger_payload_failure_retains_effective_schema_and_diagnoses_envelope`;
intended, same cause: the open envelope's own context no longer reached the
workspace `schemas/`; 10/10 after the fixture became a repository). Every other
existing test passed unedited apart from signature updates.

### Gates (macOS unless noted)

| Area | Command | Result |
|---|---|---|
| darkmatter (lib, cli, dmls, zed-dmls) | `just test --no-fail-fast` | 8851 passed, 12 skipped, 0 failed |
| darkmatter | `just test-l2` | 18 + 69 + 3 passed |
| darkmatter | `just lint` | pass |
| claudine, claudine-cli | `cargo check --tests` | compiles (no use of the changed trigger APIs) |
| darkmatter, Windows | `just cross-check darkmatter --os windows` | 7178 passed, 67 skipped |
| darkmatter-cli, Windows | `just cross-check darkmatter-cli --os windows` | 825 passed, 69 skipped (after one fix, below) |
| dmls, Windows | `just cross-check dmls --os windows` | 785 passed, 9 skipped |

`just check-tier-coverage` does not exist in this checkout (as in Phases 2
and 3); every new test path is marker-free and compiled by a declared target.

### Docs and skills

Docs as listed under "What was built". Skills: `.claude/skills/darkmatter/SKILL.md`
(scan signature, five-root bullet), `.claude/skills/darkmatter/schema.md`
(new "Schema roots and `$path` triggers" section), `.claude/skills/os/windows.md`
(item 4: catalog package roots arrive with mixed separators).

## Phase 5

The editor and the shell agree on schema roots, and Claudine's `match()`
completion walks the globs' roots. Packages: `dmls`, `claudine-cli`,
`darkmatter` (one public function and one wrapper), and `biscuit-file`
(one public method).

### What was built

- **Track A, DMLS cache key (criterion 22).** `OverlayCache::schema_for`
  (`darkmatter/dmls/src/overlay/mod.rs`) keys the effective schema on
  content + configuration + trigger registry + the document's context
  identity (`file_resolution_context_identity`, new and public in
  `darkmatter/lib/src/markdown/compose/context/options.rs`, re-exported from
  `darkmatter::markdown::compose`) + the build generation. Before, the key
  carried only the generation, so two contexts with the same generation (any
  `DocumentResolution::from_context`, or a rebuilt context whose catalog
  moved a `^` root) were served one cached schema. The roots still come from
  `SchemaRoots::for_document` through `triggers::scan` (no DMLS root list);
  the snapshot is still the one the server was launched with.
- **Track A, parity (criteria 21 DMLS half, 32).** New
  `darkmatter/dmls/tests/l1/schema_roots_parity.rs` (declared in
  `tests/l1/main.rs`).
- **Track B, Claudine walks (criteria 1, 3–6, 12, 17, 24, Decision 25).**
  `claudine/cli/src/completion/schema_completion/candidates.rs`:
  `file_candidates` (TAB) and `file_candidate_paths` (ENTER) share
  `match_glob_files`, which walks `MatchGlobs::roots` in precedence order
  with the existing walk filters, examines each file once (a `PathIdentity`
  set over the canonical root plus walked names, so the first root's walk
  owns it), keeps it when `MatchGlobs::lists_file` holds, and sorts each
  root's pass by depth then components (native order). TAB values come from
  `candidate_value`: a file under the launch directory goes to `PortablePath`
  as its bare launch-relative reference (kept as authored), any other file
  as a path with the restricted strategy (`SameDirRelative`, `ChildDir`,
  `ImmediateParentDir`, `PeerDir`, `RepoRoot(None)`, `HomeDir`,
  `AbsolutePath`), always `with_ctx` the completion context. The typed
  partial filters the rendered value. The `out.sort()` calls are gone, and
  the two choosers in `commands/schema_interactive/mod.rs` no longer re-sort
  `match()` candidates (the bare-`file` default glob is still sorted, now
  inside `file_candidate_paths`).
- **biscuit-file `GlobReference::lists_file`** and darkmatter
  `FileMatchGlobs::lists_file` (see Departures). `file_match_admits` already
  used `matches` (Phase 3); no change.
- **Found while testing: `PortablePath` compares spellings.** Rendering from
  a canonicalized target against a `/var/…` launch directory found no shared
  route and fell through to the absolute form. `candidate_value` therefore
  passes the path as walked (spelled from the context's own roots).

### Requirement-to-test mapping

All new tests are L1 (no tier marker in any path segment).

| Requirement | Test |
|---|---|
| Criterion 22 (`^` roots differ; regression: failed before the fix) | `dmls overlay::tests::schema_cache_keys_on_the_package_root` |
| Criterion 22 (`SCHEMAS_DIR` differs; passed before by the registry's `Debug` in the key; mutation-checked: still passes with the registry term removed) | `dmls overlay::tests::schema_cache_keys_on_the_snapshot_environment` |
| Criterion 32 (same roots, applied triggers, bare-name file for criteria 27–31's documents: package doc, repository doc, in-between schema) | `dmls::l1 schema_roots_parity::md_and_dmls_give_the_same_roots_triggers_and_bare_names` |
| Criteria 29, 31 through a real LSP session (`SCHEMAS_DIR` and `~/schemas` from the launch snapshot) | `schema_roots_parity::a_session_applies_schemas_dir_and_home_like_md` |
| Criterion 21 (definition error identical in `md` and DMLS) | `schema_roots_parity::a_forbidden_path_prefix_is_the_same_definition_error_in_md_and_dmls` |
| Criterion 1 (Incident 2, exact candidate) | `claudine-cli bin schema_completion::tests::incident_2_completes_a_caret_pattern_from_the_repository_root`; end to end through `claudine __complete` with the prompt in `~/.claudine/prompts`: `claudine-cli::l1 compose_schema_cli::completion_file_match_reads_the_caret_prefix_from_the_launch_repository` |
| Criteria 3, 17 (`^`, `&`, `./` from a nested package; native order, not text order) | `schema_completion::tests::a_nested_launch_lists_each_prefix_in_native_order` |
| Criterion 4 | `schema_completion::tests::a_root_exclusion_removes_files_a_positive_caret_pattern_admits` |
| Criterion 5 | `schema_completion::tests::each_file_is_judged_by_its_nearest_root`, `property_value_bare_match_offers_the_launch_folder_then_the_repository_root` |
| Criterion 6 (one-way parity, every prefix tested: bare, `./`, `&`, `^`, `~`, absolute, negations) | `assert_offers_resolve_and_are_admitted`, called by every test above |
| Criterion 12 | `schema_completion::tests::a_file_name_negation_is_neither_offered_nor_admitted` |
| Criterion 24 (bare inside the launch directory, `../`, `&`, `~/`, absolute, never `{{`, every value resolves back) | `schema_completion::tests::candidates_outside_the_launch_folder_take_the_first_form_that_resolves`, plus the resolve-back check in every test |
| Decision 25 / criterion 25 (completion omits an out-of-tree link, no warning; Unix only, as symlink creation needs privileges on Windows) | `schema_completion::tests::an_out_of_tree_file_symlink_is_omitted_without_a_warning` |
| `lists_file` agrees with `list_files` per path | `biscuit-file l1 glob_reference::boundary::a_file_link_out_of_the_tree_is_skipped_by_a_bound_glob` (extended) |
| Shipped router, root spec after the launch area through the proxy | `claudine-cli::l1 level1_review_router_partial_pty::level1_review_router_partial_offers_a_repository_root_spec_after_the_launch_area` |

The Input Robustness Matrix does not apply: no file format or configuration
reader changed (the `$path` and `SCHEMAS_DIR` matrices are Phase 4's).

### Existing-test reconciliation

| Test | Class | Resolution |
|---|---|---|
| `schema_completion::tests::property_value_match_pattern_anchors_on_cwd_not_repo_root` | intended (criterion 5) | rewritten, see Changed Outputs |
| `level1_review_router_partial_pty::{yolo_confirms…, decline_and_cancel…, proxy_target_schema…}` | intended (Decision 19) | decoy moved out of the partial's reach, see Changed Outputs |
| `level1_review_router_partial_repo_root_launch_widens_candidates_to_the_chooser` | intended | repurposed (premise removed) |
| `context_construction_guard` (claudine-cli) | bug in new code | `PortablePath::from_*` now chains `.with_ctx` at construction |
| `spawn_site_guard` (claudine-cli) | bug in new test | the e2e test uses `common::init_git_repo` |

### Gates (macOS unless noted)

| Area | Command | Result |
|---|---|---|
| darkmatter (lib, cli, dmls, zed-dmls) | `just test --no-fail-fast` | 8856 passed, 12 skipped |
| darkmatter | `just test-l2` | 18 + 69 + 3 passed |
| darkmatter | `just lint` | pass |
| claudine (lib, cli, gen, …) | `just test --no-fail-fast` | 8093 passed, 9 skipped |
| claudine | `just test-l2` | 277 + 3 passed |
| claudine | `just lint` | pass |
| biscuit-file | `just test` / `just lint` | 1043 passed / pass |

`just check-tier-coverage` does not exist in this checkout (as in Phases 2
to 4); every new test path is marker-free and compiled by a declared target
(`dmls/tests/l1/main.rs` declares `schema_roots_parity`; the rest are inline
unit tests or existing L1 files).

### Docs and skills

- Docs: `biscuit-file/docs/topics/file-references.md` (`lists_file` row),
  `darkmatter/docs/topics/schemas/dmls-schema-support.md` (context in the
  cache key), `claudine/docs/topics/completions/shell-completions.md`
  (`match()` values: roots table, order, spelling, chooser order),
  `claudine/docs/topics/completions/auto-complete.md` (provided partials).
- Skills: `.claude/skills/darkmatter/dmls.md` (cache key, parity gate, test
  traps), `.claude/skills/darkmatter/schema.md` (`lists_file`),
  `.claude/skills/biscuit-file/SKILL.md` (`lists_file`),
  `.claude/skills/claudine/SKILL.md` (completion walks and rendering).

## Phase 6

The source-scan guard keeps `GlobReference` the one glob implementation
(criterion 2). Package: `darkmatter` (one new L1 test file and its manifest
metadata); no production code changed.

### What was built

- **`darkmatter/lib/tests/l1/glob_implementation_guard.rs`**, declared in
  `tests/l1/main.rs`. It loads the shared sanitizer
  (`cli/tests/common/source_scan.rs`, by `#[path]`, as
  `semantic_results_never_persist.rs` does) and runs two checks over
  biscuit-file, darkmatter, darkmatter-cli, dmls, claudine, and claudine-cli:
  - **Source:** production files (comments, literals, and `#[cfg(test)]`
    code blanked) are searched on identifier boundaries for `globset`, `wax`,
    `globwalk`, `wildmatch`, `fast_glob`, `glob_match`, `GlobBuilder`,
    `GlobSet`, `GlobSetBuilder`, `GlobMatcher`, `OverrideBuilder`,
    `GitignoreBuilder`, `TypesBuilder`, and `Glob::`. Hits must match an exact
    per-file allowlist (path, count, reason); an unlisted file, a moved count,
    or a stale entry fails, and the message names each identifier and line.
  - **Manifest:** each package's `[dependencies]`, `[build-dependencies]`, and
    `[target.*.*]` tables (a `package = ".."` rename counts as the crate it
    renames) may name crates from `globset`, `glob`, `wax`, `globwalk`,
    `wildmatch`, `fast-glob`, `glob-match`, `ignore` only as pinned:
    biscuit-file `globset`, darkmatter `globset`, dmls `globset` + `ignore`,
    claudine-cli `ignore`, the rest none. Read with `biscuit_file::Toml`.
- **Allowlist (observed counts):** `biscuit-file/lib/src/file_reference/glob/parse.rs`
  6, `…/glob/roots.rs` 3, `darkmatter/lib/src/markdown/compose/toc_linking/filter.rs`
  7, `darkmatter/dmls/src/workspace/discover.rs` 9,
  `darkmatter/dmls/src/overlay/schema.rs` 4. The scan flagged no other file:
  `toc_linking/types.rs` names globset only inside a string literal (a hint),
  and `file_links/types.rs`'s `FileLinksMode::Glob` is an enum variant, so
  neither needed a change or an entry.
- **`source-inputs`** (`darkmatter/lib/Cargo.toml`): added the four
  other-package allowlisted files (biscuit-file `parse.rs`, `roots.rs`; dmls
  `overlay/schema.rs`, `workspace/discover.rs`). darkmatter's own `filter.rs`
  is its own source and may not be declared. The planner's real-workspace
  checks (`test_affected_scope.RealWorkspaceTestInputTests`, 6 tests) pass,
  and a plan for a change to `parse.rs` alone or `overlay/schema.rs` alone now
  adds `binary_id(darkmatter::l1)` on `ubuntu-latest`. The six manifests are
  non-source and spelled as literals, so a manifest edit (for example
  `claudine/cli/Cargo.toml`) schedules the guard without a declaration
  (verified with `affected_scope.py --resolved-plan`).
- **Dependency cleanup:** none needed. Every crate that declares `globset`
  or `ignore` still uses it in production source, so no
  `docs/dependencies.md` changed.

### Requirement-to-test mapping

| Requirement | Test |
|---|---|
| Criterion 2: no glob-library use outside the exact allowlist, exact counts, stale entries fail (real tree) | `darkmatter::l1 glob_implementation_guard::file_references_have_one_glob_implementation` |
| Negative control: planted `GlobBuilder` in a production file; `Glob::new` counted; moved count; stale source entry; undeclared `wax` in a target table; stale manifest entry; and the scope rule (comment, string literal, `#[cfg(test)]` module, `Mode::Glob` variant, a local `glob` module, a renamed `ignore`, `glob` in dev-dependencies all ignored or counted correctly) | `glob_implementation_guard::the_guard_catches_planted_violations_and_honors_the_scope_rule` |
| Mutation check on the real tree | Appending `globset::GlobBuilder::new("*.md")` to `compose/file_links/discovery.rs` turned the first test red (`glob library used in …/discovery.rs (globset@872, GlobBuilder@872)`); file restored, no diff |

Both tests are L1 (no tier marker in any path segment) and compiled by the
declared `l1` target. `test_layout` passed in the full run.

The Input Robustness Matrix does not apply to production code (none
changed). The test's own manifest reader covers the shapes that decide its
result in the negative control: inline-string and inline-table dependency,
`package` rename, target-specific table, and dev-dependencies excluded. A
manifest that fails to parse, or a dependency table that is not a table,
panics with the manifest's path rather than reading as empty.

### Known limit of CI reach

A new glob-library use in a *non-allowlisted* production file of a package
that already depends on the crate (biscuit-file, dmls, claudine-cli) changes
no declared input and no manifest, so a pull request touching only that
file does not run this guard (it runs on every darkmatter package
selection). `source-inputs` accepts files only, so whole `src/` trees cannot
be declared; the spec's form (declare the scanned paths) is followed for the
allowlisted files.

### Gates (macOS unless noted)

| Area | Command | Result |
|---|---|---|
| darkmatter (lib, cli, dmls, zed-dmls) | `just test --no-fail-fast` | 8858 passed (2 slow), 12 skipped |
| darkmatter | `just lint` | pass (after replacing a duplicated `dead_code` allow with `clippy::duplicate_mod`, the established pattern for a second `source_scan.rs` include) |
| scripts/ci | `python3 -m unittest test_affected_scope.RealWorkspaceTestInputTests` | 6 passed |
| darkmatter, Windows | `just cross-check darkmatter --os windows glob_implementation_guard` | 2 passed |
| darkmatter, Linux | `just cross-check darkmatter --os linux --features effects-instrumentation glob_implementation_guard` | 2 passed (archive leg blocked by host links; see Windows Evidence) |

`just test-l2` was not re-run: this phase adds no L2 test and changes no
production code. `just check-tier-coverage` does not exist in this checkout
(as in Phases 2 to 5).

### Docs and skills

- No `docs/` page changed: the guard is a test, and no package behavior
  changed.
- `.claude/skills/darkmatter/SKILL.md`: a "Glob guard" paragraph next to the
  context guards (what it scans, the allowlist, the `ignore` ruling, how to
  extend it).
- `.claude/skills/os/build-hosts.md`: the stale-link failure recurred in
  `<host>--fix-magic-globs`, the native-path workaround for a session that may
  not SSH, and that `--test l1` is rejected in archive mode.

## Phase 7

The glob reference joins both tables of the entry-point parity matrix
(criterion 20). Packages touched: `darkmatter` (one production fix, the
shared matrix, its runner), `darkmatter-cli`, `dmls`, and `claudine-cli`
(runners).

### What was built

- **Shared matrix** (`darkmatter/lib/tests/common/entry_point_parity/mod.rs`):
  - `GlobForm`: `Scoped` (`^**/*spec*.md`), `ScopedExcluding`
    (`^**/*spec*.md`, `!&**/_completed/**`), and `TreeEscape`
    (`../…/**/*spec*.md`, climbing to the fixture root). `::file-links` and
    `find_files()` take one pattern, so they have no `ScopedExcluding` row
    (`GlobForm::reaches`).
  - `GlobConsumer`: `FileLinks`, `FindFiles`, `MatchValidation`,
    `MatchCompletion`; `GlobDocumentCell` (Table 1, a document at
    `repo/area/pkg/docs`) and `GlobValueCell` (Table 2, both launch
    directories) as `Row::GlobDocument` / `Row::GlobValue`.
  - Expectations: `Expected::Files` (native order) or a failure class;
    observations `Observed::Files` (ordered) and `Observed::FileSet`
    (unordered). The native orders are written out as `PACKAGE_ORDER` and
    `REPOSITORY_ORDER` over nine `GLOB_FILES` (eight `*spec*.md` files across
    package, area, and repository, two of them under `_completed/`, plus a
    non-matching `plan.md` decoy), not computed by the code under test.
  - `match()` cells use a root union: the first arm is
    `file(eager; match(<patterns>))`, the second `enum(glob-rejected)`, so a
    candidate is valid only through the glob, and every entry point names
    `glob-rejected` when it rejects one (`validation_verdicts`).
  - `EntryPoint::ClaudineChooser` (owner `ClaudineCliChooser`) for the ENTER
    chooser's walk.
- **Runners:** the darkmatter library (compose pipeline, pre-flight, schema
  validation, and Table 2 `--set` values), `md` (`compose`, `schema
  validate`), DMLS (published diagnostics), claudine-cli (composition,
  `__complete`, supplied `spec=` values), and a new claudine-cli unit test,
  `completion/schema_completion/parity_tests.rs`, for `file_candidate_paths`.
- **Fix (Wave 14):** `darkmatter/lib/src/markdown/errors/blocks.rs`,
  `interpolation_block` adds the `failure: <class>` row for an
  `ExpressionError::GlobReference` cause (see Changed Outputs). The doc
  comment says so, and `darkmatter/docs/errors/file-reference-failures.md`
  shows the glob case.

### Findings while building the matrix

- `md compose` / `claudine compose` of `find_files('../../../../**/*spec*.md')`
  printed an interpolation block with no `failure:` row (fixed above).
  Reverting the fix turns `md_entry_points_agree_on_every_reference` red
  with exactly that cell.
- The first `match()` construction (second arm requiring a `severity`
  property) could not reject in DMLS: DMLS reports a missing required
  property only in strict mode (by design; `required` is a compose-time
  contract). The second arm became `enum(glob-rejected)`, a constraint DMLS
  checks statically. Not a defect.
- A `find_files()` expression error naming a *single* file reference
  (`markdown_title("&nope.md")`) also prints no `failure:` row; the library
  attaches no class to that path (`FileReferenceDiagnostic` has none). It
  is outside the glob scope and was left as is; see the message to the next
  agent.

### Requirement-to-test mapping

| Requirement | Test |
|---|---|
| Criterion 20, Table 1, library: `::file-links`, `find_files()`, `match()` validation through compose; `match()` validation through pre-flight and `DarkmatterSchemas::validate` | `darkmatter::l1 entry_point_parity::darkmatter_entry_points_agree_on_every_reference` |
| Criterion 20, Table 2, library: `match()` validation of a caller `--set` value from the repository root and from the package | same test (`Row::GlobValue`) |
| Criterion 20, Table 1, `md compose` (all three consumers) and `md schema validate` | `darkmatter-cli::l1 entry_point_parity::md_entry_points_agree_on_every_reference` |
| Criterion 20, Table 1, DMLS diagnostics for `match()` | `dmls::l1 entry_point_parity::dmls_entry_points_agree_on_every_reference` |
| Criterion 20, Table 1 and Table 2, Claudine: composition (three consumers), TAB completion (`__complete`), supplied `spec=` validation | `claudine-cli::l1 entry_point_parity::claudine_entry_points_agree_on_every_reference` |
| Criterion 20, Table 2, the ENTER chooser walk (`file_candidate_paths`) | `claudine-cli::bin/claudine completion::schema_completion::parity_tests::chooser_offers_every_glob_row_in_native_order` |
| `find_files()` failure carries `failure: <class>` (regression) | unit: `darkmatter` `errors::blocks::tests::interpolation_block_glob_reference_cause_renders_the_failure_row`; end to end: the `MdCompose × FindFiles × TreeEscape` and `ClaudineComposition × FindFiles × TreeEscape` cells |
| The order comparison is load-bearing | mutation: swapping two entries of `PACKAGE_ORDER` turned exactly `ComposePipeline × FindFiles × Scoped` red (set consumers unaffected, as designed); restored |

All new tests are L1 (no tier marker in any path segment). The runners are
existing, declared targets (`tests/l1/entry_point_parity.rs` in each
package); the chooser test is a `#[cfg(test)]` module of the claudine binary,
which `just test` runs. The shared file is already in each package's
`[package.metadata.ci.tests] source-inputs`, and every includer names it in
`include_str!`.

The Input Robustness Matrix does not apply: no parser, reader, or
configuration loader changed. Per-cell shapes covered instead: present
match (each candidate), non-match (the `plan.md` decoy and the `_completed`
files under the exclusion), a failing pattern (tree escape), and both launch
directories.

### Gates (macOS unless noted)

| Area | Command | Result |
|---|---|---|
| darkmatter (lib, cli, dmls, zed-dmls) | `just test` | 8859 passed (3 slow), 12 skipped |
| darkmatter | `just lint` | pass |
| claudine (lib, cli, gen, …) | `just test` | 8094 passed (11 slow), 9 skipped |
| claudine | `just lint` | pass (the `__eh_frame` linker notice is the existing macOS one) |
| darkmatter, Linux | `./scripts/cross-check.sh darkmatter --os linux --features effects-instrumentation entry_point_parity` | 8 passed |
| darkmatter-cli, Linux | `./scripts/cross-check.sh darkmatter-cli --os linux --features terminal-tests entry_point_parity` | 11 passed |
| dmls, Linux | `./scripts/cross-check.sh dmls --os linux --features terminal-tests entry_point_parity` | 4 passed |
| claudine-cli, Linux | `./scripts/cross-check.sh claudine-cli --os linux --features test-fixtures -E 'test(entry_point_parity) \| test(chooser_offers_every_glob_row)'` | 4 passed (the chooser unit test and the three `tests/l1` parity tests) |

The archive leg on Linux still fails before any test with the stale
read-only kache links (`libbiscuit_file-*.rmeta is not writeable`); every
Linux row above took the native path through a declared `--features` flag,
as the `os` skill describes. `just test-l2` was not re-run: this phase adds
no L2 test, and the one production change is the text of an error block.

### Docs and skills

- `darkmatter/docs/errors/file-reference-failures.md`: the glob case under
  "Where the row appears".
- `.claude/skills/darkmatter/SKILL.md`: the parity-matrix paragraph names the
  glob rows, the chooser runner, and the `glob-rejected` construction.

## Appendix: Baseline Inventory

Call sites by `git grep -n -w` over `*.rs`, at the start of Phase 1. This
list drives Phase 8's changed-outputs record.

### `FileMatchGlobs`, `admits_path`, `admits`, `file_match_admits`

- `darkmatter/lib/src/markdown/schemas/file_match.rs`: definitions,
  `match_keyword_factory` / `MatchKeyword::check` (validation), private
  `admits` (reads `std::env::current_dir()`, handed off), and unit tests
  `admits_path` (`fixes/**/spec.md`) and `file_match_admits` (partial,
  `features/x/spec.md` rejected, `fixes/x/spec.md` accepted) at lines ~351–370.
- `claudine/cli/src/completion/schema_completion/candidates.rs:328`:
  re-exports `FileMatchGlobs as MatchGlobs` for Claudine's candidate walk.
- `claudine/lib/src/composition/schema/supplied.rs:5,334`:
  `file_match_admits` and `root_union_match_patterns` for caller-supplied
  values (`admits_arm_files`).
- Other `admits` hits (`claudine/lib/src/signals/version.rs`,
  `sniff/lib/src/filesystem/repo/nested.rs`, and others) are unrelated
  functions with the same name.

### `find_files_fn`

- `darkmatter/lib/src/markdown/compose/expression/functions/mod.rs:2443`:
  definition, with its own prefix splitter and a `GlobBuilder` using
  `literal_separator(true)` (~2485).
- `darkmatter/lib/src/markdown/compose/expression/functions/paths.rs:7`:
  binding (`find_files`, alias `findfiles`).
- Tests: `darkmatter/lib/tests/l1/find_files_and_try_frontmatter.rs` (8
  functions); `claudine/cli/tests/l1/shipped_prompt_contract.rs` (a shipped
  prompt's `find_files` use).

### `resolve_boundary` (`::file-links`)

- `darkmatter/lib/src/markdown/compose/file_links/discovery.rs:35,76`:
  only caller and definition (reads the current directory, handed off).
  `GlobBuilder::new(..).literal_separator(true)` at ~105.
- Tests: unit tests in `file_links/discovery.rs` and
  `compose/tests/transclusion.rs`; `darkmatter/lib/tests/l1/reference_integration.rs`;
  `darkmatter/lib/tests/level2/level2_render_tree_terminal.rs`;
  `darkmatter/dmls/tests/l1/lsp_session.rs`.

### `schema_roots`

- Definition: `darkmatter/lib/src/markdown/schemas/triggers/discovery.rs:109`
  (`schema_roots(document_dir, boundary)`, the ancestor walk), re-exported
  from `schemas/triggers/mod.rs:36` and `schemas/mod.rs:137`.
- Production caller: `triggers::scan` (`discovery.rs:315`), called from
  `schemas/mod.rs:425`. Roots then reach bare-name `$schema` lookup through
  `triggers/assemble.rs:209,224` →
  `resolve::resolve_yaml_schema_with_roots`, and `schemas/mod.rs:508` →
  `resolve::resolve_schema_with_roots`.
- Tests: `discovery.rs` unit tests (lines ~446–528); `resolve.rs` tests
  `bare_name_import_resolves_via_schema_roots` (3363) and
  `bare_name_example_resolves_via_schema_roots` (3436), plus the
  `resolve_yaml_schema_with_roots` cases at ~3057–3195; `schemas/mod.rs:3450`
  (`triggers::scan` direct).

### Trigger glob matcher (`$path`)

- `darkmatter/lib/src/markdown/schemas/triggers/matcher.rs`:
  `compile_globs` (~432, default `Glob::new`, bare-basename `**/` expansion,
  thread-local cache) and `path_matches` (~410, all-negation admits nothing).
- Grammar and lint: `triggers/grammar.rs` (tests at ~658 and ~745, list
  `**/*.md`, `!**/_*.md`), `triggers/lint.rs`, `schemas/errors.rs`,
  `schemas/about.rs`.

### `resolve_recursive_core` (`%`)

- `biscuit-file/lib/src/file_reference/resolve.rs:129` (only caller, from
  the shared resolve core when `parsed.recursive`) and `:525` (definition).
- Tests naming recursive references: `biscuit-file/lib/tests/l1/`
  `detailed_resolution.rs`, `file_tree.rs`, `finalized_reference_resolution.rs`,
  `implicit_relative.rs`, `magic_local_roots.rs`, `precedence_flip.rs`,
  `reference_grammar.rs`, `completion_round_trip.rs`, and
  `portable_path/{configuration,excess_parent,inputs,originating_context}.rs`;
  unit tests in `file_reference/{mod,parse,resolve}.rs`;
  `darkmatter/lib/tests/l1/reference_integration.rs`.

### Other glob-crate users (for Phase 6's guard allowlist)

Production `globset` use outside the four consumers above:
`darkmatter/dmls/src/overlay/schema.rs` (`Glob::new`, ~1084),
`darkmatter/dmls/src/workspace/discover.rs` (~98), and
`darkmatter/lib/src/markdown/compose/toc_linking/filter.rs` (~64). Outside
Darkmatter (`sniff`, `research`, `biscuit-terminal`), uses are out of this
guard's scope.
