---
name: darkmatter
description: Expert guidance for the Darkmatter Rust library, `md` CLI, and DMLS language server. Use when composing or rendering Markdown, working with frontmatter, expressions, schemas, file references, remote content, hashing, browser or terminal output, or extending Darkmatter's CLI and editor integrations.
---

# Darkmatter

Darkmatter is the monorepo's Markdown composition, schema, expression, and
rendering system. The package area contains the `darkmatter` library,
`darkmatter-cli` binary (`md`), `dmls` language server, and `zed-dmls`
extension.

## Non-negotiable boundaries

- Capture one request-scoped composition context. Pass the same
  `FileResolutionContext`, repository observation, remote policy, cache state,
  identity, and meta-schema controls through `ComposeOptions`; never recapture
  CWD or repository state in a downstream resolver.
- Resolve file-like values through `biscuit_file::FileReference`. Preserve its
  source context, explicit-vs-implicit syntax, and typed errors.
- Treat composition as effectful and validation as passive. Schema parsing,
  trigger matching, completion, hover, and validation must not perform I/O,
  execute expressions, fetch remotes, or mutate documents.
- Keep network access deny-all by default. Route every allowed remote read or
  write through `biscuit_file::FetchPolicy` and exact-host consent.
- Render terminal output with `TerminalRenderable` components. The browser tier
  is headless and must use browser-protocol input, never host focus or OS input.
- Preserve source spans and typed diagnostic provenance through parsing,
  composition, schema validation, and DMLS projection.

## Choose the owning surface

Documentation under `docs/` becomes the authority when implementation lands.
Write author-facing guidance for humans in the surrounding document's style;
put necessary implementation details in supporting documents under `docs/` and
link to those. Never link a document in `docs/` to a specification or a
feature/fix design document. Keep temporary implementation-status notes explicit
without making readers consult planning artifacts. Preserve documented file
locations; an unimplemented grammar is not a reason to move schema definitions
away from their references or replace their content with placeholders.

| Work | Start with |
|---|---|
| Compose APIs, stages, expressions, file resolution, cache | [compose.md](compose.md) |
| SimplifiedSchema, triggers, validation, meta-types | [schema.md](schema.md) |
| Public modules and extracted library surfaces | [library-surfaces.md](library-surfaces.md) |
| DMLS architecture, protocol behavior, and rollout history | [dmls.md](dmls.md) |
| Render tree, style lowering, disclosure blocks, code blocks | [rendering.md](rendering.md) |
| Terminal rendering options | [terminal.md](terminal.md) |
| Frontmatter model | [frontmatter.md](frontmatter.md) |
| Error/status block conventions | [errors.md](errors.md) |
| Document comparison | [comparison.md](comparison.md) |
| Module layout | [structure.md](structure.md) |
| Parser details | [pulldown-cmark.md](pulldown-cmark.md) |
| Frontmatter ecosystem comparison | [frontmatter-crates.md](frontmatter-crates.md) |

Load only the topic needed for the task. For render-tree implementation work,
also load the `renderable` skill; for terminal components, load
`biscuit-terminal`; for file-reference or JSON/YAML/TOML conversion work, load
`biscuit-file`.

## Composition authority

`ComposeOptions` is the request authority. It carries the captured resolution
context plus remote configuration, cache root and policy, compose identity,
baseline/meta-schema controls, and rendering options. A source-derived file
reference must resolve against that captured context.

The root compose pipeline is ordered:

1. Frontmatter interpolation pass 1.
2. Schema validation and coercion.
3. Frontmatter shell expansion.
4. Frontmatter interpolation pass 2.
5. Literal replacement.
6. Conditional page blocks.
7. Body interpolation.
8. Shell directives and shell blocks.
9. Link resolution.
10. Concurrent transclusion.
11. Inline cleanup and optional fixed-width reflow.
12. Root-only link normalization.

Keep this order stable. Whole-value `{{ ... }}` and `$(...)` values are
executable state: they must resolve or fail, never leak as literal syntax.
Demand-driven context capture must observe only referenced `ctx.*` groups.

Read [compose.md](compose.md) before changing a stage, expression function,
context property, cache key, transclusion directive, or file-resolution path.

## Schema authority

Darkmatter uses `SimplifiedSchema`, compiled to Draft 2020-12 JSON Schema.
`DarkmatterSchemas` owns baseline merging and validator caching. The standalone
classifier is `parse_standalone_schema_document`; it recognizes either a pure
root `$schema` document or a `kind: schema` document with a `types` mapping.

Important contracts:

- Optional schema properties accept missing or `null` values.
- Validation keeps source positions, origin information, typed problem codes,
  pending values, and file-reference diagnostics.
- Eager `file(eager)` values may normalize only on successful composition;
  validation-only APIs remain read-only.
- `literal(value)` preserves YAML scalar typing and lowers to JSON Schema
  `const`; quoted and native YAML forms are observably different.
- `expression`, `yaml`, and `json` are passive content formats. They parse or
  coerce but never execute.
- `type-definition` and `schema` meta-types delegate to the same passive
  parsers used by authoring and DMLS.
- Trigger matching is schema-based and passive. A schema-trigger document is a
  kinded document and must retain its root `kind` declaration.

Read [schema.md](schema.md) for imports, unions, pattern dictionaries,
suggestions, triggers, and DMLS schema behavior.

The authored replacement schema entry point is `darkmatter/schemas/darkmatter.yaml`.
It declares all globals, including `doc`, and imports types from `partials/`.
Runtime migration is pending: do not confuse the existing embedded document
baseline with this global catalog. The planned document baseline is its resolved
`doc` definition; `ctx` and `current` share a context type, and there is no
`current_env`. Register the global catalog explicitly rather than auto-applying
its root as frontmatter properties.


## Remote and cache safety

HTTP(S) composition is supported for `::file`, `::code`, and read-side
expression arguments only where a remote runtime is present. Ordinary rendered
links are never fetched. Frontmatter interpolation and `$()` branching are
local-only; a remote URL there fails loudly.

- CLI callers opt in with `md compose --allow-host <host>`.
- Use the two-category vocabulary everywhere (help, docs, comments):
  **semantic-result artifacts** (composed documents, `::file` children,
  `::code`/`::toc-linking` results, snapshots) are memory-only until a
  `ContentPolicy` exists (R18); **transport artifacts** (raw HTTP bodies) are
  the one class `--cache-root` / `ComposeOptions::with_cache_root(...)` may
  persist (R36, the fix's Q1). Local transclusion stays run-local even after
  `ContentPolicy`. `docs/topics/caching.md` is the user-facing contract, and
  `cli/tests/help.rs::test_compose_help_states_the_transport_cache_boundary`
  pins the `md compose --help` wording. The semantic-result persistence path was deleted, not kept
  dormant: `RunLocalCache` is memory-only, `CacheAccessMode` governs run-local
  reuse only, and `CacheStats` has no persistent counters (transport activity
  is `RemoteFetchStats`). `lib/tests/semantic_results_never_persist.rs` pins
  this: `RunLocalCache` may not name `FileStore`/`RemoteFetchRuntime`,
  `FileStore` is allowlisted (exact counts) to the remote transport cache, and
  the deleted symbols may not reappear. Update its allowlist deliberately when
  a transport-cache file legitimately changes its `FileStore` uses.
- Configuring a cache root mutates nothing. `FileStore::at` only records the
  path and creates directories inside the write that needs them. A missing root
  stays missing and an existing one stays byte-identical unless an artifact is
  written. Never add eager `create_dir_all` or a platform-cache fallback.
- Freshness is controlled by `RemoteReadConfig` and the CLI remote freshness,
  refresh, and TTL flags. Response `Cache-Control` outranks all of them:
  `no-store` is never written (an on-disk `no-store` entry is purged), and
  `no-cache` is revalidated before every reuse, even under `Optimistic`, with
  no stale serve under `Fallback`. The write path accepts only
  `StorableDirectives`, so a TTL override structurally cannot store a
  `no-store` response; keep it that way.
- Transport-cache I/O failures (write or purge) are non-fatal. They flow
  through `RemoteFetchStats::cache_warnings` into `ComposeReport.warnings`
  (stage `remote_cache`, code `dm.remote_cache.io_failure`); never swallow
  them with `let _ =`.
- Remote manifests persist `redacted_url` (`redact_url`: no userinfo or
  fragment, query shown as `?<redacted>`), never the raw URL; the entry key
  stays `xx_hash` of the full URL. Warnings may name the redacted form only.
  Manifest `CACHE_VERSION` (now `2`) is separate from the `v{N}` directory's
  `STORE_LAYOUT_VERSION` (`1`), so old entries stay at the same path: another
  version is a miss left on disk, but a `no-store` one is still purged via the
  version-stable `RemoteUrlManifestHeader`. Bump the layout version only when
  the path scheme changes.
- Host policy must be checked before the transport cache is read.
  `fetch_with_cache` reads the store before its policy-enforcing client runs,
  so the early `check_allowed` in `RemoteFetchRuntime::register_and_fetch` is
  the only thing keeping a denied host's cached bytes out of output. A cache
  root never authorizes a host. `denied_host_never_reads_a_fresh_cached_entry`
  and the CLI `test_compose_denied_host_never_reads_a_seeded_cache_entry` pin
  this ordering.
- `absolute` and `relative` are local path transforms, never remote fetches.
- `EffectEngine::http_post` uses the same host policy as remote reads.

## Rendering authority

All public Markdown rendering folds through the render tree. Darkmatter builds
one context-aware `renderable::Document`, then performs one target fold.
Component policy is lowered during construction; do not add a post-fold HTML or
terminal decoration pass.

`DarkmatterPage` is only a viewport/page-frame assembler. It may own page
margin, padding, background, max-width centering, and browser wrapper metadata,
but it must not inspect component node kinds or mutate component content.

The browser contract is strict:

- Browser tests are headless and cannot activate windows or inject host input.
- Assert DOM state, computed style, accessibility state, or used geometry.
- Browser HTML and CSS must be derived from the same lowered layout/style
  values; never repair output with string replacement.
- Remote links and images retain safe structured attributes and policies.

Read [rendering.md](rendering.md) before changing style claims, code-block
themes, disclosure blocks, browser safety, or the render-tree fold.

## CLI orientation

The binary is `md`. Major command families include:

- `md compose`, `clean`, `read`, `toc`, and `delta` for document processing.
- `md schema validate|detect|about` for schema workflows.
- `md hash` for Markdown-aware frontmatter/body hashes.
- `md frontmatter get|set|rm` for structured frontmatter changes.
- `md code-block` for direct terminal, HTML, or Markdown code rendering.
- `md graph` and reference commands for document/reference inspection.

Rendering flags are presentation policy. `CliStyleClaims` captures explicitly
supplied global CLI style claims so command handlers can merge them with
frontmatter without mistaking defaults for user intent. Keep parsing in
`cli/src/args`, command execution in `cli/src/commands`, and output in shared
renderable components.

## Testing and verification

Use the Darkmatter package-area recipes:

```sh
cd darkmatter
just build
just test
just lint
```

Use `just test-l2` only for real-terminal behavior and `just test-browser` for
headless browser behavior. Parser, schema, prompt, template, or configuration
changes require both a passive shipped-artifact corpus test and an end-to-end
test through the normal invocation path. Persisted values require a repeated
read/write/read round trip.

The expression-grammar corpus gate is
`lib/tests/dasherized_identifier_corpus.rs`. It walks root `prompts/`,
`.claude/commands/`, `darkmatter/prompts/`, and `claudine/prompts/` through
the library's own extractors: `ExpressionFinder`,
`scan_darkmatter_directives`, `parse_frontmatter_shell_value_spanned`, and
`frontmatter_expression_values`, which finds Expression-typed properties of
any name under each document's passively resolved effective schema, the
classification DMLS uses. Claudine's `when`/`while`/`until` keys stay as a
separate check because Claudine's schema types them as `string` but
evaluates them as lifecycle conditions. The gate requires every extracted
expression to parse. Run it after any lexer or parser
change. A red result names a shipped prompt, and that prompt usually already
fails to compose. Check with `md compose` before blaming the grammar.

Identifiers may contain `-`: `read_variable` joins a `-` only mid-identifier
and only when the next character continues an identifier, so `spec-name` is
one name while `a - b`, `a -b`, `4-2`, and `foo--bar` stay subtraction. The
rule lives in the lexer, never in `is_identifier_char`. Any cursor-side scan
(DMLS completion partials) must call `expression::identifier_prefix_start`
rather than add `-` to a character class, which would merge `foo--bar`.

A `{{ … }}` that cannot be parsed or evaluated fails full-document
composition regardless of `fail_fast`, including one a rescan finds in
replacement output. `interpolate_text`/`interpolate_value` take an explicit
`ExpressionFailurePolicy`. Pass `Strict` from document stages. Use `Lenient`
only for `compose_subtree(..., Lenient)` and preflight discovery (see
[compose.md](compose.md#error-handling)). The error carries the authored
span (`SourceRef::OnDiskSpan`) whenever it is provable, including after an
earlier stage rewrote the body and inside block (`|`, `>`), multi-line,
tagged, anchored, or aliased frontmatter scalars. `interpolation_block` renders the file and
authored line and column for every cause, and
`interpolation/fatality_characterization.rs` is the drift guard.

A well-formed identifier that resolves to nothing warns as
`dm.expression.unknown_identifier` (once per root per document) unless the
author handled its absence (`x || d`, `x ? …`, `is_null(x)`) or the root is
known to the final state, a caller input, or the effective schema. Read
[compose.md](compose.md#unknown-identifiers-dmexpressionunknown_identifier)
before changing a runtime evaluation surface: new surfaces must observe
through the shared `AbsenceScope`, not a new walk. DMLS reports the same code
at `WARNING` through the static twin, `expression::static_variable_reads`
(see [dmls.md](dmls.md#unknown-identifiers)).

Each issue is reported once. A coded `ComposeWarning` family declares its
identity in its constructor (`from_schema_advisory`,
`unknown_context_variable`, `expression_failure`). `add_warning` and
`merge` collapse warnings whose `(source, code, subject)` match, and never
compare messages. Schema advisories key on the referenced path. Root and
expression subjects are per source document:
`run_compose_pipeline_node` calls `attribute_to_document` before a child
report merges upward. Push new warnings through `add_warning`/`add_warnings`,
not `report.warnings.push`. Membership is O(1) through a private
`WarningIndex` cache: it indexes direct pushes lazily and rebuilds when the
vector shrinks, but an element replaced in place is not seen.
`interpolate_text` tracks a failed span across
rescans and never re-evaluates it. A new coded family, such as
`dm.expression.unknown_identifier`, adds a `WarningSubject` constructor
rather than a message-based check.

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
`cli/tests/spawn_site_guard.rs` rejects raw spawns, post-build CWD/PATH/
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

Do not run workspace-wide Cargo gates for a Darkmatter-only change. Use Sniff
and GitNexus first to include actual downstream consumers such as Claudine when
a public Darkmatter type or behavior changes.
