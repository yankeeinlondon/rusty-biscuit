# Session Handoff — 2026-10-27

## Task status

Finalize the map/filter designs with no unresolved product rulings before
planning implementation. The author asked us to reconcile expression and
directive forms, review an architect's inline directive-spec changes, accept
what we agree with, and commit when editing/rulings are finished. Do not implement
Rust yet. Stay in this worktree/branch. Do not commit unrelated changes.

The author settled both remaining rulings after the context reset. Both
map/filter specs are finalized for planning, and implementation remains
unstarted. Do not repeat the settled questions.

## Files and commit status

- Earlier signed commit `e52d4f0fd` contains the original directive spec and
  map/filter topic docs. Signature was verified.
- Current edits include the expression spec, plan, and implementation log;
  architect-reviewed directive spec; and map/filter topic docs synchronized
  with all accepted clarifications and finalized rulings. The closing design
  commit includes these files, the metadata draft, and this handoff.
- A new quick draft exists at
  `darkmatter/features/2026-10-27-schema-narrowing-metadata/spec.md`.
- `darkmatter/docs/iterables.md` has earlier uncommitted edits intentionally left
  out of the prior three-file commit. Do not silently include it now.
- This handoff is session context, not a product specification or shipped doc.
- Local Git identity/signing was verified before the earlier commit:
  Ken Snyder / ken@ken.net, OpenPGP signing enabled. Recheck before committing
  and run `git verify-commit HEAD` afterward. No agent attribution in commits.

## Settled directive behavior

- Optional positional callback; no `fn=`. Both `=>` and `->` everywhere.
- Map default `i => i`; filter default `i => true`. Filter's positional callback
  selects original items; optional `filter=` adds a short-circuiting second
  predicate. This default/extra-filter choice was stated as an assumption after
  an optional clarification received no answer; it is now in the docs/spec.
- Arrays/tuples yield elements; objects yield insertion-ordered `{key,value}`
  entries with preserved types. Shallow patterns supported.
- Map binds input for its callback, then binds result to the same pattern for
  the body. Filter's body retains the original input, not predicate results.
- Source must have known iterable type from schema, namespace, or outer local.
  Optional/null source renders nothing. Unknown source fails absent explicit
  existing-policy recovery. Non-null scalars fail. Required schema properties
  still undergo normal validation.
- Reserved `local` namespace: bare name resolves nearest local before frontmatter;
  `local.foo` accesses local only; `fm.foo` accesses frontmatter. Null shadows.
  Normal local/frontmatter shadowing has no advisory. Protect namespaces/globals.
- Child transclusions receive locals only through explicit parameters, while
  parent target/condition/parameter expressions can read locals.
- Order: block-level `when` first, source once, selection, stable sort, stopping,
  map transformation if applicable, retained body preparation.
- `while`/`until` mutually exclusive, exclude boundary item, existing truthiness.
  All selection/sort/stop callbacks see original items. Lazy without sorting;
  with sorting compute each selected key once in source order.
- Stable ascending/descending numeric or locale-independent string sorting;
  reject null/missing, mixed types, booleans, and containers as sort keys.
- Authored bodies repeat with provenance and whitespace; inserted data never
  becomes executable syntax. Shared structural scanner includes shell blocks.
- All shell sites require existing explicit template authorization even in
  skipped branches. Local-dependent placeholders are template-only, use existing
  scalar validators, and never grant permission from a type. Existing result
  caching and no-cache controls remain.
- Fail on unhandled collection errors, preserve original index/key and nested
  diagnostics; no silent item skipping or effect rollback. Shared budgets needed.

## Settled execution-order ruling

The author explicitly selected document-wide composition stages. Prepare each
document's frontmatter once, evaluate structural collection callbacks and capture
body scopes, then ordinary body interpolation, existing shell stages, link
resolution, transclusion, cleanup/normalization. Do not finish each item's body
before evaluating the next callback. Child concurrency/output placement retain
existing behavior. An explicit effect such as `as_markdown()` inside an
expression runs when that expression evaluates, not in a deferred body stage.
This is recorded as Settled Pipeline Ruling in the directive spec and topic docs.

## Expression alignment completed in planning files

Phase 1 was design only: no source code or prototype was written. The spec now
accepts both arrows, shallow patterns, objects/typed tuples, local namespace,
known-type projection, no shadow advisory, shared iteration budgets.

Two obsolete implementation sketches were replaced in spec, unstarted plan, and
current log handoff: `param: String` becomes reusable `BindingPattern`; a
stack-only borrowed `Locals<'a>` chain becomes immutable retainable lexical
frames with shared binding identities and runtime/passive views. Historical
phase-one proposals remain clearly labeled historical, not active gates.

Intentional differences: expression callbacks are required; sources may be
arbitrary expressions; results are arrays; expression map does not rebind its
result to the input pattern; rendering options remain directive-only.

The directive spec's relationship section was synchronized with this alignment.

## File narrowing ruling and separate metadata draft

The author explicitly ruled that BOTH `file_exists(x)` and `is_file_type(x)`
narrow the checked value to `file` on true. `file_exists` additionally checks
existence at evaluation time. False existence does not imply not-file.

Current implementation registers file_exists as true-implies string:
`darkmatter/lib/src/markdown/compose/expression/functions/paths.rs`.
The specs/plan require changing this through existing NarrowingFact registration.
Facts attach to stable checked values, not a separate reevaluation of the same
read-side call text. No implicit cross-callback memoization is agreed.

The new schema-metadata draft proposes `narrows(value: file; true-implies)` and
analogous true-iff/false-implies forms, plus omitted type for presence. It moves
fact authority from Rust registration to catalog declarations, using the existing
engine. Syntax/artifact migration remain draft choices FOR THAT FEATURE ONLY.

The author explicitly said this must NOT be a prerequisite for map/filter.
This non-blocking boundary is now recorded in both map/filter specs, expression
plan/log, and metadata draft. Use existing Rust facts now; migrate later.

## Settled ruling: dynamic transclusion discovery

The author agreed to extend existing composition preflight after the resumed
session verified its expression preparation and target-shape guards.

- Preflight may enumerate typed collections and evaluate discovery-supported
  expressions needed to identify targets, including target-producing callbacks.
- Inspect candidate children before collection conditions, filtering, or stopping
  exclude them. Apply this recursively through explicit local parameters.
- Reads may occur for items rendering later skips. Schema validation and DMLS
  remain passive; discovery executes no shell commands or repeated bodies.
- Preserve rejection of pending-shell and other unavailable execution-dependent
  targets, including dependencies propagated through locals and callbacks.
- Extend graph metadata with authored-site and iteration-scope identity. Newly
  encountered runtime targets fail without late approval. Graph reuse does not
  promise arbitrary callback-result caching.
- No literal-target-only restriction, explicit target-list declaration, or new
  permission mode was adopted. Known file suitability remains required.

The directive spec, expression spec/plan/log, and both planned topic pages now
record this ruling. No Rust implementation has been performed.

## Settled ruling: expansion budgets

The author accepted 100,000 item visits and 16 MiB (16,777,216 bytes) of
cumulative UTF-8 collection expansion per root request. Visits are shared by
expression/directive calls and nested composition, including rejected items.
Standalone expression sessions use the same default visit limit and shared
counter. Expression arrays are values, not rendered-output bytes.

Nested output counts again when copied into enclosing expansions; cached
fragments count whenever inserted into expansions. Ordinary pipeline passes,
reallocations, unrelated source text, and preflight copies add no charge.
Fail before an append exceeds the limit; exactly reaching it succeeds.
Positive library options and CLI `--max-iteration-visits` /
`--max-expansion-bytes` configure limits, with no unlimited setting. Reject zero,
negative, and overflowing values and include limits in compose identity.
Preflight does not consume the execution allowance.

## Architect review status

Accepted verified clarifications: strict grammar/options, declared type handling,
shared shell-block scanner, template-only local argument timing, scalar fallback,
cache compatibility, container/CRLF rules, expanded acceptance coverage.
Corrected stale shadow-advisory and expression alignment language.
All product rulings are settled, including stage order, discovery, and budgets.

Both directive docs must be fully updated after implementation and frontmatter
set to `kind: canonical-documentation`; spec explicitly requires this. They are
still planned, not canonical shipped behavior today.

## Next work

The design work is complete and ready for implementation planning. The
expression plan exists but Phases 2–5 remain unstarted; the directive feature
still needs its implementation plan. No Rust tests were needed for this
documentation-only work. Verify the closing design commit's signature and
`git diff --check` when reporting delivery.

Keep the earlier `darkmatter/docs/iterables.md` edits out of the closing commit.
Do not move features to `_completed`, run cargo fmt, create branches/worktrees,
or claim functionality is implemented.
