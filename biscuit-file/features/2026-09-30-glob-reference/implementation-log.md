---
spec: "/Volumes/coding/wt/rusty-biscuit/fix-magic-globs/biscuit-file/features/2026-09-30-glob-reference/spec.md"
plan: "biscuit-file/features/2026-09-30-glob-reference/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
packages:
    - biscuit-file
    - darkmatter
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
