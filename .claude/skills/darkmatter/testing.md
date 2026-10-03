# Test Fixtures, Source Guards, and the Entry-Point Parity Matrix

Detail behind the "Testing and verification" section of [SKILL.md](SKILL.md).

A fixture that needs a repository runs `git init`: a bare `.git` directory is
not a repository to a prepared request context, so `&`/`^`, the `::file-links`
repository icon, and the relative boundary would all see none.

Deterministic `md` integration tests must launch through
`cli/tests/common/fixture.rs`'s `CliProcessFixture`. Its builder pins
fixture-owned CWD/home/config/cache/temp, scrubs Git/application/rendering
inputs, and supplies a portable minimal PATH. Use its named `host_path`,
`fake_only_path`, `ambient_context`, or `inherit_no_env` policy before
`build()` when the tested behavior requires an escape.

A test whose subject *is* a pinned rendering or darkmatter application input
declares that claim on the builder too — `rendering_input`,
`rendering_input_removed`, `application_input`, `application_input_removed`,
or `plain_terminal(columns, lines)` for the whole fixed-size no-color frame.
The home/config/cache/temp anchors and the Git plumbing are containment and
have no declared override at all: handing one back re-contaminates the child.
`cli/tests/common/protected_env.rs` is the shared classification both the
builder and the guard read.

Do not hand-build an `md` command or undo isolation afterward;
`cli/tests/l1/spawn_site_guard.rs` rejects raw spawns, post-build CWD/PATH/
environment-clear escapes, a post-build `.env`/`.env_remove` naming any
protected key, and stale exemptions.

Tests that build an HTTP client (`remote_fetch` `persistent_cache_tests` /
`integration_tests`, `provider_network`, preflight remote, `effects`) hit
nextest's 30 s timeout when host load far exceeds core count, and do so as a
cluster. Check `uptime` and re-run exactly those tests at `--test-threads 2`
before treating a timeout as a regression; never run `just lint` concurrently
with `just test`.

The ordinary local L1 recipe excludes `slow_` tests and leaves the internal
`terminal-tests` / `browser-tests` build features disabled. Tier recipes enable
their required targets; CI enables both features when constructing all-tier
coverage.

**Context guards.** `context_construction_guard.rs` in darkmatter lib, cli,
dmls, messenger lib/cli, claudine lib/cli, and claudine-gen runs the shared
engine `cli/tests/common/context_guard.rs` over that crate's `src/`:
construction (`FileResolutionContext::new|from_snapshot`, `::from_process`),
optional context (`Option<[&]FileResolutionContext>`, no allowlist allowed),
and ambient state (`std::env::*` reads, `dirs`/`home`/biscuit-file
`home_dir`, `capture_env`, `.resolve()`/`.resolve_from(..)`/`.resolve_target()`,
`PortablePath` without `with_ctx`). Allowlists are exact
`(gate, path, identifier, count, reason)`; a read may be listed only when it
feeds neither file resolution, `ctx.*`, nor `env.*`. A failing guard prints
the `Allowance` to paste; fix the read instead when it resolves a path or
seeds `ctx.*`. The production-scope rule (`#[cfg(test)]` blanking) lives in
`source_scan::production_sources`, shared with
`semantic_results_never_persist.rs`; files below an inline
`#[cfg(test)] mod tests { .. }` (its `tests/` directory) are test-only too.

**Glob guard.** `lib/tests/l1/glob_implementation_guard.rs` keeps
`GlobReference` the one glob implementation. It scans the production source
of biscuit-file, darkmatter, darkmatter-cli, dmls, claudine, and claudine-cli
for glob-library identifiers (`globset`, `GlobBuilder`, `GlobSet[Builder]`,
`GlobMatcher`, `Glob::`, `ignore`'s `OverrideBuilder`/`GitignoreBuilder`/
`TypesBuilder`, other glob crates) against an exact per-file allowlist (the
`GlobReference` module's `parse.rs`/`roots.rs`, `toc_linking/filter.rs`, DMLS
`workspace/discover.rs` and `overlay/schema.rs`), and pins each package's
non-dev glob-crate dependencies (that is what catches the `glob` crate, whose
name biscuit-file's `file_reference::glob` module shares). `ignore`'s
`WalkBuilder` is a walker, not a matcher, and is allowed. Match file
references with `GlobReference`; extend the allowlist only for globs that are
not file references (heading text, editor configuration), and add another
package's allowlisted file to `lib/Cargo.toml` `source-inputs`.

**Schema-variable docs guard.**
`lib/tests/l1/schema_roots.rs::no_doc_or_skill_names_the_singular_schema_variable`
walks `darkmatter/docs`, `claudine/docs`, `biscuit-file/docs`, and the
darkmatter, claudine, and biscuit-file skills, and fails on the whole-word
singular spelling of the variable (plural `SCHEMAS` with its `S` dropped):
the variable is `SCHEMAS_DIR` and names the schemas folder itself. Describe
the singular form in prose; spelling it in a scanned file turns the guard red.

**Entry-point parity matrix.** `lib/tests/common/entry_point_parity/mod.rs`
holds the fixture (monorepo + fixture `HOME` + `outside.md`), the
`EntryPoint` enum with exhaustive `owner()`/`rows()`, both tables, and
`ParityReport`. Runners: `lib/tests/l1/entry_point_parity.rs` (pipeline,
pre-flight, schema validation), `cli/tests/l1/entry_point_parity.rs` (`md`),
`dmls/tests/l1/entry_point_parity.rs`, and
`claudine/cli/tests/l1/entry_point_parity.rs` (`claudine compose --dry-run`
for documents; for Table 2 values, `claudine __complete … compose <value>
resolved=` (completion resolves only the committed prompt, so the value is
that prompt and every target of that runner's second fixture declares
`resolved: enum(t<N>)`; no suggestions is `Observed::Unresolved`), the same
value composed as the prompt argument, and a `target=<value>` schema value).
The same module's `CrossRepositoryFixture` (`launch/` and `source/` Git
repositories with disjoint `magic.md` markers and `order.yaml` enums) backs
the library, `md`, and Claudine runners' `external_*` tests (DMLS's request
belongs to the document's repository by design): a source in another repository takes `&`,
`^`, and bare root lookups from its own repository and `@` from the launch
scope. A CLI that rebuilds a context at a foreign document's directory must
copy the launch scope back (`with_launch_magic_scope`, as
`MdRequest::document_context` does); Claudine completion uses
`claudine::composition::derive_request_context_for_source`, composition's
policy.
`EntryPoint::MdArgument(MdRoute)` is one entry point per `md` route that
reads a file argument (render, compose, clean, toc, get, set, rm, hash, both
delta slots, graph, edit, validate refs, schema validate/detect/triggers,
code-block `--file` and default); each route is observed through its own
result (heading, title, hash, reported path, `Document:` line), Table 2 adds
an absolute path, the four malformed introducers with matching literal files
(`ParityFixture::write_route_files`), and `./@`, and the mutating routes
(`rm`, `edit`) run serially on a restored fixture.
Glob rows (`Row::GlobDocument`/`GlobValue`, `GlobForm` × `GlobConsumer`)
cover `::file-links`, `find_files()`, and `match()` validation (Table 1) and
`match()` completion and caller-value validation (Table 2) against written-out
native orders (`PACKAGE_ORDER`, `REPOSITORY_ORDER`); a `::file-links` tree and
per-value validation compare as sets (`Observed::FileSet`), and completion's
expectation drops `_`-prefixed folders (one-way parity). A `match()` cell is
a root union whose other arm is `enum(glob-rejected)`, never a missing
required property, which DMLS does not report outside strict mode. The ENTER
chooser walk (`file_candidate_paths`) needs a terminal, so its runner is the
claudine binary's unit test `completion/schema_completion/parity_tests.rs`
(`EntryPoint::ClaudineChooser`, `Owner::ClaudineCliChooser`).
Each variant must invoke the feature it names: never stand one entry point in
for another. A new entry
point is an `EntryPoint` variant plus rows; a failing cell is an entry-point
defect, never a table edit. A caller-supplied `../` that leaves the
repository is `InvalidReference` at every entry point, `md`'s arguments
included (no `allow_external_relative`). No runner skips cells by OS:
`RequestSnapshot::from_process` reads the home env-first (`USERPROFILE` on
Windows), so a `CliProcessFixture` home reaches `md` and `claudine`.
`@configured-doc.md` lives only under `home/.claudine/prompts` (Claudine's
user prompt root; the `md` runner passes it as `md --magic-root <DIR>`, and the
library and DMLS runners register it on their snapshot; production DMLS has no
extra-root setting). Every runner runs every form. DMLS runs every consumer at
every editor surface (graph through `index_workspace`; code actions assert no
create-file fix except for a broken Markdown link). Compare failures only by `ResolutionFailure`:
`MarkdownError::resolution_failure()` walks the cause chain (nested children,
`TocLinkingError::Unresolved`, schema `file` values), a tolerated failure's
`ComposeWarning::resolution_failure` carries it, and `md` renders it as a
`failure: <kebab-class>` row on every block and warning
(`darkmatter/docs/errors/file-reference-failures.md` is its user contract);
Claudine's prompt-argument errors carry the same row
(`CompositionError::resolution_failure`).
