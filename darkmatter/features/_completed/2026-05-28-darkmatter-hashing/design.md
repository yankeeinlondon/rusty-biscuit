---
phase: 1
title: Baseline Discovery And API Design
created: 2026-05-28
---

# Darkmatter Hashing — Phase 1 Design

This document is the Phase 1 deliverable: it records the baseline discovery, the
reuse decisions, the new library API surface, the file-layout decision, the
`--body`/`--frontmatter` → `MdHashKind` mapping, and the library/CLI boundary in
code-level terms — all decided **before** any implementation begins (Phase 2+).

No production source files change in Phase 1.

## 1. Baseline Findings

### Current hash behavior (`darkmatter/lib/src/markdown/hash.rs`)

`Markdown` exposes three public methods:

- `hash(body_only, frontmatter_only, strict) -> String` — the only kind today.
  Returns `{:016x}` for a single concern or `{fm}-{body}` for both.
- `hash_frontmatter(strict) -> u64`
  - empty/absent map → `xx_hash("")`.
  - strict → hash of `serde_yaml_ng::to_string(map)`.
  - non-strict → keys sorted, each rendered `key:<json-value>` joined with the
    literal two-char sequence `\n`, then hashed. Values participate.
- `hash_body(strict) -> u64`
  - strict → `xx_hash(content)`.
  - non-strict → `xx_hash_variant` ignoring `LeadingWhitespace`,
    `TrailingWhitespace`, `BlankLine`.

All hashing is xxHash via `biscuit-hash` (`xx_hash`, `xx_hash_variant`,
`HashVariant`). The 16-digit lowercase hex shape (`{:016x}`) is the canonical
component encoding and must be preserved.

### CLI surface (`darkmatter/cli/src/args.rs`, `commands.rs`)

- `Command::Hash { input, body, frontmatter, strict }` — three bool flags, no
  env reads, no save/diff.
- `run_hash(input, body_only, frontmatter_only, strict)` — single-file/stdin
  mode prints `md.hash(...)`; directory mode aggregates per-file `fm-body`
  hashes (splits on `-`, re-hashes the two columns). No business logic beyond
  aggregation lives here.

### Save/write pattern (`run_clean`, `output/string.rs`)

`md clean --save` is the existing write-back precedent:

1. load original, clone, mutate the clone,
2. `original.delta(&cleaned)`; write only if `!delta.is_unchanged()`,
3. `std::fs::write(resolved, cleaned.as_string())`,
4. CLI prints the delta. **The CLI owns `fs::write`; the library owns mutation
   and serialization.**

`as_string()` (`output/string.rs`) re-emits frontmatter via
`serde_yaml_ng::to_string(map)` between `---` delimiters, then appends
`content()` verbatim. This is the canonical serializer the spec mandates for
`--save`. It already writes the body byte-for-byte (`content()` is untouched),
satisfying the "body written byte-for-byte" requirement **provided** hash-save
never runs `cleanup`.

### Frontmatter model (`frontmatter.rs`, `types.rs`)

- `FrontmatterMap = IndexMap<String, serde_json::Value>` — insertion order is
  preserved, satisfying the "preserve key order as much as the model allows"
  requirement for free.
- `Frontmatter::as_map() -> &FrontmatterMap`, `as_map_mut()`, `is_empty()`,
  `get::<T>`, `insert::<T>`.
- `Markdown::{frontmatter, frontmatter_mut, fm_get, fm_insert, content}`.
- Keys are matched byte-exactly; no normalization. This is exactly the
  "exact key matching" the ignore-set requires.

### Error path (`types.rs::MarkdownError`, `MarkdownResult`)

`thiserror`-derived enum, returned as `MarkdownResult<T>`. Malformed stored-hash
parsing will add **one** new variant here (see §3), surfaced to the CLI through
the existing eyre path → exit code 1.

## 2. Reuse Decisions

| Need | Decision | Source |
|------|----------|--------|
| Heading extraction in document order, with level + literal text | **Reuse** `toc` machinery | `toc/mod.rs::extract_elements`, `MarkdownToc::all_headings()` |
| Section content boundaries (heading → next same/parent-level heading) | **Reuse** `toc` hierarchy + byte ranges | `toc/mod.rs::build_hierarchy`, `MarkdownTocNode` prelude/byte ranges |
| Preamble (content before first heading) | **Reuse** `toc` preamble | `MarkdownToc::preamble`, `preamble_hash*` |
| Section alignment for `detailed` (heading/content/level/position) | **Salvage internal helper only**; own the classification + messaging | `delta/mod.rs::compare_sections`, `extract_headings_with_paths`, `ChangeAction` |
| Frontmatter key/value vs keys-only hashing | **New**, but mirror existing non-strict canonicalization (sorted keys) | derived from `hash.rs::hash_frontmatter` |
| Canonical write-back serialization | **Reuse** `Markdown::as_string` | `output/string.rs` |
| Stable key-ordered frontmatter mutation | **Reuse** `IndexMap` semantics | `types.rs::FrontmatterMap` |

**Reuse boundary for the delta engine.** Per the spec, the legacy delta engine's
user-facing *classifications and messages* are not authoritative. We will not
call `Markdown::delta` for explanations. The `detailed` alignment may borrow the
*shape* of `compare_sections`' anchoring strategy (heading match → content-hash
match → positional pairing → leftover add/remove), but the alignment, the
change categories, and the rendered text are owned by the new hashing module.
This keeps the two implementations separate as the plan requires, and avoids
coupling the hashing classifications to `ChangeAction`.

**TOC reuse note (hidden coupling worth a comment in Phase 2).** `toc` hashes
section content with `xx_hash`/`xx_hash_variant` already; the `detailed`
`content-hash` and `body-structure-hash` must use the **same** whitespace-
variant policy as `hash_body` (non-strict: leading/trailing/blank) so that
"whitespace-only body change must not alter non-strict semantic hashes" holds
across kinds. The `toc` node subtree hash uses a different variant set
(`BlockTrimming`), so we compute our own content hashes rather than reading
`MarkdownTocNode`'s precomputed hash. We reuse `toc` only for **structure**
(levels, literal headings, byte ranges, preamble text), not for its hash values.

## 3. New Library API Surface

Target module path: `darkmatter::markdown::hash` (already `pub mod hash`).

### `MdHashKind`

```rust
/// Structural kind of a markdown hash.
pub enum MdHashKind {
    Fm,         // frontmatter (keys + values), single hash
    Body,       // body content, single hash
    Simple,     // {fm}-{body}                       (default, today's behavior)
    Structured, // {fm}-{fm_keys}-{body}-{body_structure}
    Detailed,   // nested object: frontmatter + preamble + sections
}
```

- Library: `FromStr` + `Display` (serde via string) for the five lowercase
  tokens `fm|body|simple|structured|detailed`.
- CLI: `clap::ValueEnum` wrapper lives in `args.rs` (not the library), converted
  into `MdHashKind`.
- **Resolution ordering** (a partial order, not `Ord`): provide
  `fn resolution(self) -> Option<u8>` where `simple=0 < structured=1 <
  detailed=2`, and `fm`/`body` are partial (`None` / a separate marker) — lower
  than `simple`, incomparable to each other. A dedicated
  `enum KindRelation { Same, Higher, Lower, Incomparable }` returned by
  `fn relate(stored, forced) -> KindRelation` drives the `--save` branching in
  Phase 4 so the ordering rules live in one tested place.

### `MdHashOptions` — the explicit option bundle (library reads no env)

```rust
pub struct MdHashOptions {
    /// Active hash property name. Default "hash".
    pub property: String,
    /// EXTRA ignored property names (beyond the always-ignored managed keys).
    pub extra_ignored: Vec<String>,
    /// Forced kind from `--kind`; None = match stored / default simple.
    pub forced_kind: Option<MdHashKind>,
    /// Strict mode (no whitespace normalization / key reordering).
    pub strict: bool,
}
```

- Always-ignored managed keys = `{ property, "last_updated" }`, constructed by
  `fn ignore_set(&self) -> BTreeSet<String>` = managed ∪ `extra_ignored`. The
  stored `ignored` list records **only** `extra_ignored`, sorted, never the
  managed keys.
- `LAST_UPDATED_KEY: &str = "last_updated"` and `DEFAULT_HASH_PROPERTY: &str =
  "hash"` are library constants; the CLI default mirrors them.

### `StoredHash` — parsed view of the on-disk `hash` property

```rust
pub struct StoredHash {
    pub kind: MdHashKind,
    pub value: StoredHashValue,
    pub ignored: Vec<String>, // extras only, sorted; empty ⇒ absent
}

pub enum StoredHashValue {
    /// simple / structured / fm / body — a single string.
    Flat(String),
    /// detailed — nested object.
    Detailed(DetailedValue),
}
```

- `StoredHash::parse(value: &serde_json::Value) -> MarkdownResult<StoredHash>`:
  - JSON string ⇒ `kind = Simple`, `value = Flat(..)`, `ignored = []`
    (shorthand).
  - JSON object ⇒ read `kind`, `value`, optional `ignored`; `detailed.value`
    parses the nested object.
- `StoredHash::to_frontmatter_value(&self) -> serde_json::Value` enforces the
  **promotion invariant**: emit a bare string iff `kind == Simple && ignored
  .is_empty()`; otherwise an object. Omit `ignored` when empty (never
  `ignored: []`). Sort `ignored`.

### `DetailedValue` — persisted detailed shape

```rust
pub struct DetailedValue {
    pub frontmatter: FmHashPair,          // { fm, keys }
    pub preamble: Option<String>,         // hash or null
    pub sections: Vec<SectionTuple>,      // [level, "heading", content_hash]
}
pub struct FmHashPair { pub fm: String, pub keys: String }
pub struct SectionTuple { pub level: u8, pub heading: String, pub content_hash: String }
```

Serde shapes match the spec's YAML exactly: `frontmatter: { fm, keys }`,
`preamble: <hash|null>`, `sections: - [level, "heading", hash]` (tuple
serialization).

### `ComputedHash` — freshly computed, kind-tagged

```rust
pub enum ComputedHash {
    Fm(String),
    Body(String),
    Simple { fm: String, body: String },
    Structured { fm: String, fm_keys: String, body: String, body_structure: String },
    Detailed(DetailedValue),
}
```

- `Markdown::compute_hash(&self, kind, &MdHashOptions) -> ComputedHash`.
- `ComputedHash::to_stored_value(&self) -> StoredHashValue` renders the flat
  string forms (`{fm}-{body}`, four-part, single) or the detailed object.

### `HashComparison` — like-for-like result (uses **stored** ignore-set)

```rust
pub struct HashComparison {
    pub kind: MdHashKind,
    pub frontmatter_changed: bool,
    pub body_changed: bool,
    pub detail: ComparisonDetail,   // structured/detailed extra resolution
    pub ignore_policy: Option<IgnorePolicyAdvisory>, // separate advisory, not a content change
}
```

`ComparisonDetail` carries, per kind, the extra resolution needed to render
explanations (e.g. `fm_keys_changed`, `body_structure_changed`, and for
`detailed` the per-section classification list). `IgnorePolicyAdvisory` records
`{ now_ignoring, previously }` for the advisory line and never counts as a
content change.

### `HashExplanation` — rendered, kind-aware difference report

```rust
pub struct HashExplanation { /* lines + nested body children */ }
impl HashExplanation { pub fn render(&self) -> String; }
```

Built from `HashComparison`. Produces the exact strings in the spec's
"Explaining differences" section for each kind, including the `detailed` nested
body list. Lives behind `Markdown::explain_hash_diff(stored, &opts) ->
MarkdownResult<HashExplanation>`.

### `HashSaveOutcome` — what `--save` decided (CLI writes, library decides)

```rust
pub struct HashSaveOutcome {
    pub explanation: HashExplanation,
    pub new_document: Option<String>, // serialized markdown to write, None = no write needed
}
```

- `Markdown::save_hash(&self, &MdHashOptions) -> MarkdownResult<HashSaveOutcome>`:
  computes kind selection, like-for-like comparison under the **stored**
  ignore-set, applies the `last_updated` rules, mutates a clone's frontmatter
  (`hash` + maybe `last_updated`) and serializes via `as_string()`. Returns
  `new_document = None` when nothing changed (no write). The CLI performs
  `fs::write` only when `Some`. `last_updated` is set to today's local date
  `YYYY-MM-DD`; **the date is passed in by the CLI** (library stays
  deterministic) — add `today: NaiveDate`-style parameter or a small
  `SaveContext { today: String }` so tests inject a fixed date.

### New error variant

```rust
// types.rs::MarkdownError
#[error("Malformed stored hash in '{property}': {reason}")]
MalformedStoredHash { property: String, reason: String },
```

Surfaced through the CLI eyre path → exit code 1.

## 4. File Layout Decision

**Move `markdown/hash.rs` → `markdown/hash/` submodules.** The detailed
comparison + alignment + explanation rendering is substantial and would make a
single file hard to review (the plan's stated tie-breaker). Proposed layout:

```
markdown/hash/
  mod.rs          // pub re-exports; Markdown impl: hash(), compute_hash(),
                  //   save_hash(), explain_hash_diff(); back-compat shims
  kind.rs         // MdHashKind, KindRelation, resolution/relate
  options.rs      // MdHashOptions, ignore_set, constants
  stored.rs       // StoredHash, StoredHashValue, DetailedValue, parse/serialize
  compute.rs      // ComputedHash + fm/body/structured/detailed computation
  compare.rs      // HashComparison, ComparisonDetail, section alignment
  explain.rs      // HashExplanation rendering for every kind
```

`pub mod hash` stays public; today's `Markdown::hash/hash_frontmatter/hash_body`
remain in `mod.rs` unchanged for back-compat (directory-aggregate mode and any
external callers keep working). Phase 2 introduces the new files; the legacy
methods are retained until/unless explicitly superseded.

## 5. `--body` / `--frontmatter` → `MdHashKind` Mapping

- `--body` ⇒ `MdHashKind::Body`; `--frontmatter` (alias `--fm`) ⇒
  `MdHashKind::Fm`. Both are degenerate single-concern kinds.
- `--strict` continues to mean "no whitespace normalization / key reordering"
  and feeds `MdHashOptions.strict`; it is orthogonal to kind.
- **Precedence with the new `--kind`:** `--kind` is the single source of truth
  for forced kind. To avoid ambiguity, `--body` and `--frontmatter` are made
  **mutually exclusive with `--kind`** at the clap layer (`conflicts_with`), and
  are treated as shorthands that the CLI maps to `forced_kind = Some(Body|Fm)`.
  `--body` and `--frontmatter` remain mutually exclusive with each other
  (today's behavior). This keeps one forced-kind input path into the library and
  preserves backward-compatible bare usage (`md hash --body`).
- Bare `md hash` (no kind flags, no `--kind`) ⇒ `forced_kind = None` ⇒ kind
  selection matches the stored hash, else defaults to `Simple`, and prints the
  `{fm}-{body}` string exactly as today (exit 0).

## 6. Library / CLI Boundary (code-level)

**The library** receives only explicit values and reads no environment:

```rust
let opts = MdHashOptions {
    property,        // resolved string
    extra_ignored,   // resolved Vec<String>
    forced_kind,     // Option<MdHashKind>
    strict,          // bool
};
md.compute_hash(kind, &opts)        // bare hash / forced kind
md.save_hash(&opts /*, today*/)     // --save: returns HashSaveOutcome
md.explain_hash_diff(stored, &opts) // --diff: returns HashExplanation
```

**The CLI** (`args.rs` + `commands.rs::run_hash`) owns all of:

- reading `HASH_PROPERTY` (default `hash`) → `opts.property`,
- reading `HASH_IGNORE_PROPERTIES` (CSV, trim, drop empties; additive; cannot
  un-ignore `hash`/`last_updated`) → `opts.extra_ignored`,
- parsing `--kind`, `--body`, `--frontmatter`, `--strict` → `opts.forced_kind`
  / `opts.strict`,
- enforcing `--save` ⊻ `--diff` and `--kind` ⊻ `--body`/`--frontmatter` via
  clap `conflicts_with`,
- supplying today's date for `last_updated`,
- performing `fs::write` when `save_hash` returns `Some(new_document)`,
- mapping outcomes to exit codes: bare/`--save` → 0; operational error → 1
  (eyre); `--diff` with differences (or no stored hash) → 2,
- choosing what to print: bare → hash string; `--save`/`--diff` →
  `HashExplanation::render()`.

This boundary keeps the library deterministic and unit-testable (a test can set
stored ignore-set ≠ env ignore-set and assert "unchanged") and keeps `run_hash`
free of business logic, matching `run_clean`'s split.

## Open Items Deferred To Later Phases (not decided here)

- Exact non-strict canonicalization string for `fm_keys` hashing (Phase 2) —
  will mirror `hash_frontmatter`'s sorted-keys approach, hashing keys only.
- Whether directory-aggregate mode supports new kinds or only bare hashing
  (Phase 7) — leaning "bare hashing only initially; reject `--save`/`--diff`/
  non-simple `--kind` in directory mode with a usage error."
