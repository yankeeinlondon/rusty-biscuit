---
status: draft
created: 2026-09-03
reviewed: true
reviewed_by: codex/default
reviewed_on: 2026-09-03
review_iterations: 1
area: claudine
packages:
    - claudine
    - darkmatter
related:
    - ../_completed/2026-09-01-path-ref-fallback/spec.md
    - ../_completed/2026-09-02-proxy-file-param-provenance/spec.md
---

# The did-you-mean walk dies on its first unreadable entry, and a bare-map schema sidecar is silently accepted as raw JSON Schema

## Summary

Two gaps surfaced while landing PR #67 (2026-09-02). They are unrelated in
code but share a shape: a helper meets an input it did not anticipate and
falls back to "do nothing" without telling anyone.

1. **The repository-local basename suggestion never renders.** The
   `2026-09-01-path-ref-fallback` fix added a bounded "did you mean" walk
   for an explicit operation-file miss. In this repository it produces
   nothing, even from the repository root with a same-named file elsewhere
   in the tree, because the walk abandons the whole search on the first
   entry the walker reports as an error — and `.qwen/skills/*` contains
   dangling symlinks that sort before every package area.
2. **A referenced `schema.yaml` written as a bare simplified map is read as
   raw JSON Schema.** Darkmatter classifies a referenced sidecar as a
   SimplifiedSchema only when it carries an envelope (`$schema:` as the sole
   root key, or `kind: schema` + `types:`). A file such as
   `spec: file(eager; required)` with no envelope falls through to the raw
   JSON Schema branch, where its keys are unknown keywords and every value
   validates. The document's `file(eager)` typing, caller-file projection,
   and interactive collection all silently switch off.

Gap 1 is a defect in a completed fix and is repaired here. Gap 2 is a
documented rule, not a defect; this fix adds the missing diagnostic so the
rule cannot be violated silently.

> **Reader's note (review 2026-09-03):** The review keeps both original
> remedies but tightens their contracts. Repository-walk errors are skipped at
> every iterator position, including the first, while an invalid root is
> rejected before iteration; reaching the budget preserves matches already
> found. The sidecar warning is now a typed schema advisory propagated through
> each consumer rather than being described as an existing shared warning
> channel. Its conservative classifier requires the complete map to parse as a
> SimplifiedSchema and suppresses JSON Schema and custom-vocabulary documents.

## Observed behavior (verified 2026-09-02)

### Gap 1

From the repository root on the `feat/unifi` branch, with
`homelab/docs/unifi/apps/protect.md` and
`homelab/docs/unifi/service-offerings/protect.md` present:

```console
$ claudine inline-compose ./docs/unifi/protect.md --dry-run --codex
 CompositionError: Unresolvable file reference
┃ Cannot resolve `./docs/unifi/protect.md` from launch directory `…/feat-unifi`.
┃ Tried:
┃ - launch directory: `…/feat-unifi/./docs/unifi/protect.md`
┃ Correct the reference and try again.
```

No "Did you mean" section. The diagnostic snapshot
(`CLAUDINE_TEST_DIAGNOSTIC_SNAPSHOT`) shows the recovery seam *did* receive
a repository root and an empty suggestion list:

```json
{"code":"composition.invalid_file_reference","repository_root":"…/feat-unifi",
 "failure":"no_match","suggestions":[],"candidates":["source"]}
```

Ruled out: the walker filters (`_`-prefix elision and the curated skip
list both admit `homelab/docs`), the 20,000-entry budget (roughly 7,700
entries sort before `homelab/`), and the basename extraction
(`protect.md`). Confirmed cause: `find . -type l ! -exec test -e {} \;`
lists eight dangling symlinks under `.qwen/skills/` (`two-face`, `ts_rs`,
`color-eyre`, `agent-observability`, `async-trait`, `viuer`, `syntect`,
`rig`). The `ignore` walker yields an error entry for each, and
`collect_repository_suggestions` returns an empty list on the first error
entry. The L1 test `repository_suggestion_walk_error_returns_empty` pins
exactly this behavior, so the suite is green while the feature is dead in
its own repository.

### Gap 2

The main-side JIT sequence test (`distributed_step_keeps_launch_identity_and_source_schema_and_files`)
writes a sidecar `schema.yaml` containing
`source_marker: string(required)`, `spec: 'file(eager; required)'`, and
`caller_spec: 'file(eager; required)'`, referenced as `$schema: ./schema.yaml`.
A probe inside `prepare_caller_projection` showed the effective schema for
that document was the raw map itself — no `properties`, no `format` — so
the caller-file projection prelude found no file arm and left the caller
value unprojected. The same document composes without any warning. The
repository's own research sidecars (`claudine/docs/research/*/_schema.yaml`)
each carry a comment stating that the root `$schema:` key is required for
this reason, which is documentation of a trap rather than a guard against
it.

## Root cause

### Gap 1 — one bad entry aborts the search

`repository_basename_suggestions` (`claudine/cli/src/completion/operation_file.rs`)
maps every walker error, every failed `strip_prefix`, and every failed
symlink `metadata` call to `Err(())`, and `collect_repository_suggestions`
treats any `Err` as "the walk cannot continue" and returns nothing. The
`2026-09-01-path-ref-fallback` spec asked for exactly that
("walk errors must never replace the primary file-not-found diagnostic"),
but it conflated *the root being unusable* with *one entry failing*. A
dangling symlink or permission-denied entry is an entry problem, and the
`ignore` crate reports it inline while retaining the ability to continue.

The collector also currently consumes `SUGGESTION_ENTRY_BUDGET + 1` items and
then discards any matches accumulated in the first 20,000. That contradicts
the completed fix's "stop after 20,000 visited entries" rule: the budget is a
work bound, not a reason to erase useful results already found within it.

### Gap 2 — the envelope rule has no failure mode

`parse_standalone_schema_document` (`darkmatter/lib/src/markdown/schemas/simplified/standalone.rs`)
returns `Ok(None)` for "ordinary YAML and raw JSON Schema documents" and
`parse_yaml_referenced_file` (`schemas/resolve.rs`) then treats the bytes as
raw JSON Schema serialized in YAML. That is the right division of
authority: a `.yaml` sidecar may legitimately be a raw JSON Schema. But a
mapping whose every value is a string that parses under the simplified
type grammar (`string(required)`, `file(eager; required)`,
`number(default(1))`) is never a useful JSON Schema — the JSON Schema
validator ignores unknown keywords, so the file validates everything. The
resolver can recognize this shape cheaply and today says nothing.

## Design decisions

### D1 — Validate the root once; skip and budget every iterator error

`repository_basename_suggestions` rejects an absent or non-directory
repository root before constructing the walker. After that precondition, every
item yielded by the walker is one budgeted visit:

- `Ok(entry)` may contribute a match;
- a walker `Err`, failed `strip_prefix`, or failed symlink `metadata` skips
  only that item; and
- both successful and failed items count toward
  `SUGGESTION_ENTRY_BUDGET`, regardless of where the failure appears. In
  particular, a first yielded `Err` is not treated specially because it may be
  the motivating first dangling symlink rather than evidence that the root is
  unusable.

Stop before consuming item 20,001 and return the sorted matches accumulated
within the first 20,000 visits. A tree of nothing but broken entries therefore
costs at most 20,000 iterations. The `ignore` walker's per-entry
`ignore::Error` values remain mapped to a unit error so the pure collector stays
testable with an iterator of `Result<SuggestionEntry, ()>`. An invalid root or
a walker that yields no usable entries naturally returns no suggestions.

The path-ref-fallback contract that walk problems never replace the primary
diagnostic is unchanged: no skipped-entry detail surfaces to the user, the
semantic code and existing no-match fields remain unchanged, and only the
already-defined optional suggestion section/detail list may differ.

### D2 — Directory symlinks are still not followed

`follow_links(false)` stays. A *dangling* symlink is an entry error under
D1; a *directory* symlink is still pruned so a repository-local search
cannot escape its captured root (`repository_suggestions_do_not_follow_directory_symlinks`
remains authoritative).

### D3 — The recovery seam is proven against the real seam, not only the pure collector

The existing L1 tests drive `collect_repository_suggestions` with a
synthetic iterator and `repository_basename_suggestions` with a temp repo.
Add one test that seeds a temp repository with a dangling symlink sorting
*before* the directory holding the match (for example `.aaa/broken ->
missing` and `zzz/access.md`) and asserts the suggestion survives. Replace
`repository_suggestion_walk_error_returns_empty` with pure-collector tests
showing that errors in the first and middle positions yield later matches and
consume budget. Add a boundary test proving that exactly 20,000 visits are
examined, item 20,001 is not consumed, and matches found within the budget are
retained. Root validation remains covered at the real seam.

The real dangling-symlink regression is `#[cfg(unix)]`. On Windows, where
creating a symlink may require Developer Mode or elevated privileges, the
platform-independent synthetic iterator tests are the required behavioral
proof. Do not substitute a permission-denied directory: permissions can be
bypassed by the CI identity and would test a different walker condition.

### D4 — A bare-map sidecar that looks like a SimplifiedSchema is a warning, not an error

In `parse_yaml_referenced_file`, after `parse_standalone_schema_document`
returns `None` and before the raw-JSON-Schema fallback, classify the
mapping:

- the root is a non-empty mapping and every value is a string;
- parsing the complete mapping with the existing passive
  `parse_yaml_schema` authority succeeds, which validates both property names
  and every type-and-constraint string without duplicating the grammar;
- no root key is a recognized Draft 2020-12 JSON Schema keyword; and
- no root key begins with `$` or `x-`, conservatively reserving unknown dialect
  keywords and extension vocabularies.

Keep the recognized-keyword set in one named schema-resolution helper and
cover the Draft 2020-12 core, applicator, validation, metadata, format, and
unevaluated vocabularies. A short hand-picked "structural keyword" list is not
sufficient: valid scalar schemas such as `format: string`,
`title: string(required)`, or `$comment: string` would otherwise be false
positives.

When all conditions hold, the resolver attaches a warning to the resolved
schema:

> `<path>` looks like a SimplifiedSchema but has no envelope, so it was
> read as raw JSON Schema and constrains nothing. Wrap the properties under
> a root `$schema:` key (or `kind: schema` + `types:`).

The file is still loaded as raw JSON Schema (behavior unchanged). Making this
an error would reject a valid raw JSON Schema that happens to resemble the
simplified grammar; a conservative warning is proportionate to a silent
no-op.

The classifier is passive and operates on the bytes already read by the
resolver. It performs no second file read, import resolution, file-reference
resolution, network access, or mutation.

### D5 — One typed schema advisory feeds all consumer surfaces

There is not currently one warning channel shared by composition, standalone
validation, and DMLS: `ComposeWarning`, `ValidationReport`, and DMLS
`SuggestionState` are distinct products. Introduce a small typed
`SchemaAdvisory` (name may vary) with a stable kind/code, message, and referenced
file path. `ResolvedSchema` collects these advisories, root-union/reference
merges sort and deduplicate them by `(kind, path)`, and `EffectiveSchema`
retains them alongside dependency paths. The warning code is
`dm.schema.missing_simplified_envelope` with source `darkmatter.schema`.

Consumers project that one semantic advisory without reparsing:

- compose maps it once per root document to a `ComposeWarning` at the schema
  validation stage; repeated pre-shell/post-shell validation and transclusion
  report merging must not duplicate it;
- `DarkmatterSchemas::validate` returns it with the validation outcome, and
  `md schema validate` renders it in normal pretty output and includes a
  structured `warnings` array in JSON output without changing validity or exit
  status; `--quiet` retains its documented success-output suppression; and
- DMLS carries it through `SchemaBundle` and publishes one warning on the
  consuming Markdown document's `$schema` value range. It does not reuse
  `dm.schema.invalid_suggestion`, does not duplicate the warning onto the
  referenced sidecar buffer, and does not depend on the sidecar being open.

Claudine already carries Darkmatter compose warnings through prepared
composition. The new advisory therefore follows the normal Claudine warning
policy, including `--silent` suppression, rather than adding Claudine-specific
schema parsing or rendering.

### D6 — The rule is documented where authors look

Update the authoritative
`darkmatter/docs/topics/schema-definition.md`, its inline guide
`darkmatter/docs/inline/schema-validation.md`, and the Darkmatter skill's
schema authority `.claude/skills/darkmatter/schema.md`. The existing inline
guide currently describes the older, looser `$schema`-mapping disambiguation
rule and must be corrected to the two actual envelopes. State that a referenced
file is a SimplifiedSchema only with an envelope; without one it remains raw
JSON Schema; a map strongly matching the simplified grammar receives the
warning above. The research `_schema.yaml` comments stay as-is.

## Scope

- `claudine/cli/src/completion/operation_file.rs` — validate the root before
  iteration, skip every yielded entry error, and enforce the exact budget and
  five-match cap.
- `claudine/cli/src/completion/operation_file/recovery_tests.rs` — D3
  tests; retire the pinned "any error → empty" test.
- `darkmatter/lib/src/markdown/schemas/resolve.rs` — D4 classification and
  typed advisory in the referenced-file fallback; reuse
  `simplified::parse_yaml_schema`, with no grammar changes.
- `darkmatter/lib/src/markdown/schemas/mod.rs` and compose schema-validation
  plumbing — retain, deduplicate, and project schema advisories through
  `ResolvedSchema`, `EffectiveSchema`, `ValidationReport`, and `ComposeReport`.
- `darkmatter/cli/src/commands/schema/validate.rs` — pretty and JSON advisory
  output with unchanged validity and exit-code semantics.
- `darkmatter/dmls/src/overlay/` and `darkmatter/dmls/src/diagnostics/` — carry
  the advisory through `SchemaBundle` and publish its stable warning code at
  the consumer's `$schema` range.
- `darkmatter/docs/topics/schema-definition.md`,
  `darkmatter/docs/inline/schema-validation.md`, and
  `.claude/skills/darkmatter/schema.md` — D6 wording; re-stamp only Markdown
  files that already carry a managed `hash:` with `md hash <file> --save`.
- Drift: the path-ref-fallback fix's completions topic
  (`claudine/docs/topics/completions/shell-completions.md`) and its skill
  snapshot say suggestions are "advisory"; add that unreadable entries are
  skipped rather than fatal.

## Acceptance criteria

- **AC1 (motivating case).** From the repository root of a checkout that
  contains a dangling symlink under a directory sorting before the match,
  `claudine inline-compose ./docs/unifi/protect.md --dry-run` renders a
  "Did you mean:" section listing the repository-relative `protect.md`
  paths, capped at five, sorted, and `err.detail.suggestions` carries the
  same list in the same order.
- **AC2 (entry errors and budget).** L1: entry errors in the first and middle
  positions yield later matches and consume budget; no item beyond 20,000 is
  consumed; matches found within the budget survive exhaustion; an empty
  iterator and a repository root that is not a directory yield no suggestions.
  The primary diagnostic code and no-match fields are unchanged; only the
  optional suggestions projection may differ.
- **AC3 (containment unchanged).** Directory symlinks are still pruned; the
  existing symlink-escape tests pass unchanged.
- **AC4 (sidecar warning).** A referenced `schema.yaml` whose non-empty root is
  a bare map entirely composed of valid simplified property definitions
  composes with exactly one warning naming the file and the two envelope
  spellings; two schema-validation passes do not duplicate it. The document is
  otherwise composed exactly as before (raw JSON Schema, no constraints).
- **AC5 (no false positives).** A referenced raw JSON Schema (`type:
  object` + `properties`), scalar-keyword schemas (`format`, `title`,
  `$comment`), a custom `$`/`x-` vocabulary document, an enveloped
  SimplifiedSchema, a `kind: schema` document, and a whole-file reference
  scalar produce no warning. A bare map with even one value that is not a
  simplified type string (for example a data file mistakenly referenced)
  produces no warning.
- **AC6 (surfaces and identity).** The same typed advisory appears once in a
  Darkmatter `ComposeReport`, in Claudine's ordinary compose-warning output,
  in default `md schema validate` pretty output, in that command's JSON
  `warnings` array, and as a DMLS warning ranged to the consuming `$schema`
  value. DMLS uses source `darkmatter.schema` and code
  `dm.schema.missing_simplified_envelope`; validity and CLI exit status remain
  unchanged.
- **AC7 (gates).** `just test` and `just lint` in `claudine/` and
  `darkmatter/`; DMLS L1 tests for advisory range/code and dependency-cache
  invalidation; `just cross-check claudine-cli --host windows` and
  `--host linux` for the platform-independent AC2 collector tests and
  production code. The Unix host additionally runs the real dangling-symlink
  fixture. No L2/L3 terminal or browser test is required.

## Non-goals

- Changing which files the suggestion walk considers (the `.gitignore`,
  `_`-prefix, and skip-list rules stay).
- Fuzzy or stem matching for suggestions; exact filename equality remains
  the contract.
- Making the envelope optional, auto-detecting a bare map as a
  SimplifiedSchema, or erroring on it. Raw JSON Schema sidecars remain
  first-class.
- Emitting SimplifiedSchema authoring diagnostics on an unreferenced bare YAML
  file opened directly in an editor; without a consuming `$schema` reference,
  it remains ordinary YAML and intent is unknowable.
- Cleaning up the dangling `.qwen/skills` symlinks; they are useful as a
  standing regression fixture and are outside Claudine's ownership.

## Open questions

None. D1 resolves error and budget semantics, and D4–D5 define the sidecar
classifier, stable advisory identity, propagation, and presentation without
changing either input format's interpretation.
