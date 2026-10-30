---
area: darkmatter
status: implemented
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-23
review_iterations: 2
completed: true
implemented: true
clarified: false
created: 2026-10-23
owner: Ken Snyder <ken@ken.net>
packages:
    - darkmatter
    - darkmatter-cli
    - biscuit-hash
related:
    - 2026-10-21-merman
human_review: false
needs_rulings: false
implemented_by: "claude/opus"
---

# Frontmatter key order: author order is the contract, the hash ignores it

## Problem

`2026-10-21-merman` added `merman-core`, which enables `serde_json`'s
`preserve_order` feature. Cargo unifies features per build, and Darkmatter
depends on `biscuit-visualized`, so every build of Darkmatter (and of every
CLI that links it) now gets insertion-ordered `serde_json::Map`s instead of
sorted ones. The draft reports 45 Darkmatter L1 failures from this feature
change; that count is diagnostic context, not an acceptance target.

Ken ruled on 2026-10-23 that insertion order is the behavior we want, but two
things are wrong:

1. **The non-strict frontmatter hash depends on nested key order.** It was
   always meant to be Markdown-aware: reordering frontmatter keys must not
   change it. Darkmatter’s
   [hash_frontmatter_map](../../lib/src/markdown/hash/mod.rs), which computes
   the frontmatter fingerprint, sorts only the top-level
   keys and renders each value with `serde_json::to_string`, which follows
   the nested maps' own iteration order. Nested maps used to be sorted only
   because `serde_json` sorted them by default; with `preserve_order` the
   hash changes when an author reorders a nested mapping. The benchmark
   manifest shows it: `replace_heavy`'s frontmatter hash moved from
   `9e3319d168b8b47a` to `7c6e3ac00a355eb2`, and `compose_schema_transclusion`'s
   also changed, with no content change.
2. **Darkmatter gets insertion order by accident.** Nothing in Darkmatter's
   manifest asks for `preserve_order`; it arrives through Merman. A build
   that did not include Merman would quietly sort again.

## Rulings (Ken, 2026-10-23)

- **R-K1. Author key order is respected.** Frontmatter keeps the author's key
  order at every depth. Top-level order was already retained; nested
  authored order was retained separately as metadata, while ordinary nested
  JSON maps could iterate in sorted order. Insertion order is the contract, not an accident of a dependency's feature flags.
- **R-K2. Merge order.** When two documents' frontmatter is merged, the base
  document's order is kept; keys the base does not have are appended
  afterward, in the merged document's order. (Darkmatter’s top-level
  [merge_with and set_defaults](../../lib/src/markdown/frontmatter.rs)
  operations already keep positions using an insertion-ordered map; this
  ruling makes it a stated contract and extends it to
  nested mappings wherever Darkmatter merges them.)
- **R-K3. The default frontmatter hash is order-independent at every depth.**
  The default `md hash` is two hashes, one for the body and one for the
  frontmatter; the frontmatter half must not change when keys are
  reordered, at any nesting level.
- **R-K4. Where the code lives.** The order-independent hashing of a JSON
  value belongs in `biscuit-hash`, so every package that hashes structured
  data has one canonical way to do it; Darkmatter calls it.

## Design boundaries

Author order and hash order serve different purposes. Parsed and serialized
frontmatter should read in the author's order; a semantic fingerprint should
ignore object-key order. Canonicalization must therefore operate on a borrowed
value without sorting the document's own maps in place.

For example, these mappings retain their respective orders when written, but
have equal non-strict frontmatter hashes:

```yaml
# First document                 # Second document
settings:                       settings:
    zebra: 1                        alpha: 2
    alpha: 2                        zebra: 1
```

“Canonical JSON” below means compact `serde_json` output with recursively
sorted object keys. It is a repository hashing format, not a claim of
compliance with an external canonical-JSON standard. Strings, escaping, and
number spellings follow `serde_json`; this fix does not normalize Unicode,
make `1` and `1.0` equivalent, or change float parsing.

## Required behavior

### Preserve order at authoring boundaries

- Enable `serde_json/preserve_order` explicitly in
  [Darkmatter's library manifest](../../lib/Cargo.toml), with a brief
  contract comment. Enable `biscuit-hash/json` on Darkmatter's existing
  dependency. Do not rely on Merman's transitive feature selection.
- Parsing, ordinary frontmatter serialization, and `md frontmatter` edits
  preserve object-key order at every depth, including objects inside arrays.
  Replacing an existing property keeps its position; a new property appends.
  This is an order guarantee, not a guarantee to preserve YAML comments,
  quoting, anchors, or whitespace during general serialization. Existing
  text-preserving editors retain their stronger contracts.
- Programmatically supplied values retain the order their serializer supplies.
  No author order can be recovered from an unordered map. Audit conversions
  through unordered maps on paths that expose or persist authored mappings;
  carry an ordered representation through those paths instead of reconstructing
  author order from randomized iteration.
- Keep the parsed order metadata available. In Darkmatter,
  [Frontmatter::mapping_key_order](../../lib/src/markdown/frontmatter.rs)
  supplies source order to schema and authoring consumers. It describes parsed
  source, not a live index of later mutations; do not remove it or redefine it
  as part of this fix. Update comments that imply nested JSON maps must still
  be sorted, and test agreement between parsed values and their metadata.

### Share canonical JSON rendering in biscuit-hash

Add an opt-in `json = ["dep:serde_json"]` feature in
[biscuit-hash's library manifest](../../../biscuit-hash/lib/Cargo.toml).
It must not enable `preserve_order`, `xx_hash`, or `blake3` itself.

Expose a renderer of a borrowed `serde_json::Value` that produces a compact
JSON string. Object keys are sorted by Rust string ordering (lexicographic
UTF-8 byte order), recursively, including objects inside arrays. Array
positions and scalar values are preserved. Do not mutate the input or silently
substitute an empty string on serialization failure; either make the accepted
`Value` rendering infallible by construction or return an explicit error.

Provide thin hash conveniences using that same rendered representation:
XXH64 with the existing seed and return type under `json` plus `xx_hash`, and
BLAKE3 with the existing return type under `json` plus `blake3`. Public names
are the implementer's choice. The renderer must also compile with `json`
alone and default features disabled. Avoid duplicate recursive algorithms for
the two hash functions; callers needing only rendering should pay for no hash
algorithm. A borrowed traversal with sorted key references avoids cloning an
entire value tree just to reorder it. No performance spike is required.

### Preserve Darkmatter's hash identities

In Darkmatter,
[hash_frontmatter_map](../../lib/src/markdown/hash/mod.rs) computes the
frontmatter fingerprint shared by the legacy methods and the option-based
hash API. Change only non-strict **value rendering** to use the shared
canonical renderer. Preserve all of these existing details:

- sort top-level keys;
- render each entry as the unescaped key, `:`, and canonical JSON value;
- join entries with the literal two-character sequence `\n`, **not** a newline;
- hash the resulting bytes with the existing XXH64 helper;
- keep empty or absent frontmatter equal to `xx_hash("")`;
- keep option-based ignored-property filtering and all body and keys-only
  hash rules unchanged.

Do not replace this historical entry format with a JSON serialization of the
whole map or with a hash of each value. Either change would invalidate stored
hashes. The benchmark manifest's recorded identities must pass unchanged.
Compatibility with pre-Merman output applies when the parsed values and ignore
policy are unchanged; it does not cover separate float-parsing changes.

Strict hashing continues to hash YAML serialization of the parsed map without
sorting. It is sensitive to author key order at every depth, but does not hash
the original YAML source bytes: changing comments or scalar quoting alone need
not change it. Strict hashes of nested mappings previously emitted in sorted
order can change once author order is honored. Document this intentional
correction and the need to refresh an affected strict baseline using the normal
save path; do not automatically migrate stored values or bump timestamps.

### Audit other structured hashes

Record each audited hash, its purpose, whether key order matters, and the
resulting action in the implementation log. At minimum review:

| Package and source | Required treatment |
| --- | --- |
| Darkmatter: [MarkdownToc construction](../../lib/src/markdown/toc/mod.rs) | Canonicalize the whole frontmatter object for `frontmatter_hash_normalized`; retain its existing empty-map sentinel and its separate raw hash. This identity supports semantic comparison and is distinct from the historical `md hash` entry format. |
| Darkmatter: [schema_content_hash](../../lib/src/markdown/schemas/validate.rs) | Canonicalize the schema value so reordered object keys share a validator cache entry. Preserve array order, which can carry meaning. |
| DMLS: [schema_cache_key](../../dmls/src/overlay/mod.rs) | This hashes document text plus a serialized configuration struct, not just a JSON value. Keep the text component byte-sensitive. Only canonicalize configuration fields that are unordered JSON objects if such fields exist; do not canonicalize the document text or convert the whole struct merely for a cosmetic one-line change. |

Search the remaining Darkmatter and DMLS hash inputs for serialized JSON
objects. Order-sensitive identities may remain where order is intentional;
explain that choice. The audit does not authorize unrelated cache redesign.

### Keep merge precedence separate from key position

For every successful merge, retain base mapping positions and append new keys
in incoming order. Selecting an incoming value for an existing key must not
move that key. For example, base `{z: 1, a: 2}` plus incoming `{a: 3, b: 4}`
produces order `z, a, b` whether `a` keeps `2` or takes `3`.

Darkmatter's [Frontmatter::merge_with and set_defaults](../../lib/src/markdown/frontmatter.rs)
operate at the top level. Preserve that shallow behavior: replacing a nested
object replaces the whole object and retains the incoming object's internal
order. Do not turn these APIs into recursive merges. On conflict,
`ErrorOnConflict` still reports an error; this fix introduces no rollback or
atomicity promise.

Darkmatter already has recursive composition merges in
[effective_state.rs](../../lib/src/markdown/compose/context/effective_state.rs),
including transclusion `set` overrides. Apply base-first order at each
recursively merged mapping. The receiving document is the base for document
plus inherited/external state; the child is the receiving document during
transclusion. Preserve existing scalar, array, null, runtime `ctx`, and
special `replace` precedence rules. Arrays remain leaves, never concatenated.

Reader's note: the current composition builder implements document precedence
by reversing the arguments to a merge in which the incoming value wins. That
also reverses key-order priority. Preserve document-first order independently
of value precedence, rather than retaining that argument reversal as the
ordering policy. The builder also accepts top-level `HashMap`s; merely
enabling `preserve_order` cannot recover order lost before the merge. Inspect
its callers and use ordered inputs wherever authored order is observable.
Internal maps used solely for keyed lookup need no conversion.

### Update tests and documentation deliberately

Review each failed output assertion before updating it. The draft identifies
these initial candidates; the list is not permission to accept all snapshots:

- `frontmatter::tests::parsed_frontmatter_retains_nested_mapping_order_as_metadata`;
- `fn_list_formatting::as_json5_uses_the_formatter_object_idiom`;
- `schema_validate_baseline::schema_validate_legacy_json_output_is_byte_identical`;
- `schemas_convert_snapshots::*`;
- `suggest_constraint_phase3::conversion_snapshot_covers_valid_and_invalid_metadata`.

Each changed output must follow author or deterministic construction order.
Unexpected randomized order requires a code fix. If a test intentionally
compares a legacy command's bytes, preserve the comparison between commands;
only update historical expected ordering after confirming both routes agree.
Use parsed YAML or explicit map insertion to build order fixtures; value
equality alone cannot prove iteration order.

Update [Darkmatter's hash reference](../../docs/cli/hash.md) with recursive
normalization and strict-hash compatibility. Add a frontmatter topic under
`darkmatter/docs/topics/` for parsing, order, and shallow merge behavior, and
update [the recursion topic](../../docs/topics/frontmatter-recursion.md)
with receiving-document-first recursive order and compact examples. Link from
relevant README overviews without turning them into command references.

Document the `json` feature in the biscuit-hash README, crate docs, and a
`biscuit-hash/docs/` topic covering rendering, hash algorithms, and feature
combinations. Update dependency documentation (including Darkmatter's
`docs/dependencies.md`) and the darkmatter and biscuit-hash skills for the new
contracts. Current topic docs must not link back to this specification.

## Acceptance

1. The benchmark test
   `benchmark_fixtures::benchmark_manifest_matches_recorded_identities` passes
   with the manifest unchanged. Add a small fixed expected-hash regression for
   the historical entry format so a future delimiter or whole-map change fails.
2. Separate pairs of parsed documents reorder only top-level keys, only nested
   keys two levels deep, or only keys inside an array element. Each pair has
   equal non-strict frontmatter hashes and different strict frontmatter hashes.
   Cover the shared library helper and frontmatter components of the supported
   option-based kinds. Nested-only changes leave even strict keys-only hashes
   unchanged. Body-only output is unaffected.
3. In biscuit-hash, verify recursive object-order independence, array-order
   sensitivity, empty objects/arrays, scalar roots, Unicode keys, and escaped
   strings. Assert exact canonical bytes for a representative fixture and
   equality with compact `serde_json` output for sorted input. Confirm input
   iteration order is untouched. Hash conveniences must equal the existing
   byte hash helpers applied to those canonical bytes.
4. Confirm normalized TOC hashes ignore top-level and nested key reordering,
   including array-contained objects, while array-element reordering changes
   them. Confirm schema object reordering shares a cache identity where changed.
5. Test shallow merges under every strategy and defaults, including overwrites
   without movement, incoming key order, and a nested object replacement.
   Test recursive document/external merges with both value-preference choices,
   transclusion inheritance, and `set` overrides. Assert order by enumerating
   keys and verify the existing value/null/array precedence remains intact.
6. Exercise `md frontmatter` through its normal invocation path using the
   repository test toolkit: read, edit, save, reread, and repeat. Assert nested
   and top-level order survives, existing-key replacement keeps position, and
   new keys append. Exercise `md hash --save` and `--diff` against a saved
   non-strict baseline after a nested-only reorder: no semantic change, no
   baseline rewrite or timestamp bump, and no diff. Keep these as ordinary
   process tests; no terminal or browser focus is needed.
7. `cargo tree -e features -i serde_json -p darkmatter` shows Darkmatter itself
   requesting `preserve_order`. Run `just test` and `just lint` in both package
   areas; the Darkmatter recipe includes the CLI and DMLS. Also check the
   biscuit-hash library with default features disabled for `json`,
   `json,xx_hash`, and `json,blake3`, plus the ordinary default-feature build.
   Use nextest for feature-specific tests. The area's all-features test recipe
   covers `json` once added; ensure lint covers the new feature-gated code too.
8. The implementation log records the hash audit, justified output updates,
   strict-baseline correction, and any departures from this specification.
   Completion means implementation ready for review; the author closes the
   review cycle and moves the fix afterward.

## Open Questions

None identified that requires a new product decision. The order contract and
value-precedence rules above are settled; the implementer chooses the smallest
ordered representation needed at composition boundaries after tracing callers.
If preserving those boundaries requires a broader public API change than this
fix can reasonably contain, record that specific impact for author review
rather than weakening the order guarantee or sorting output as a workaround.

Planning (2026-10-23) traced that boundary. Changing `EffectiveState` and
`EffectiveStateBuilder` to ordered maps touches six Claudine call sites, each of
which already holds an ordered map. That is contained, so it is done in this fix
and no author review is pending.

## Out of scope

- The other dependents that now see insertion order (claudine, messenger,
  research, playa, biscuit-icon, biscuit-speaks CLIs). Their suites are run
  as part of `2026-10-21-merman`'s later gates; any failure there is a
  separate fix.
- `float_roundtrip`, which Merman also enables, changes how some floats parse.
  No current failure is attributed to it; if one appears, record it.
