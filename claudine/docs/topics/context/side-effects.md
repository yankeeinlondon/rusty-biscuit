# Side Effects

The side-effect engine is Darkmatter's callable catalog of **mutating**
operations: writing frontmatter, creating files and directories, appending log
lines, and sending HTTP requests. It is the deliberate counterpart to the
read-only [expression engine](expression-engine.md).

## What it is used for

Composition and the expression engine are pure — composing a document never
changes anything outside it. Real-world automation, though, needs to *act*:
stamp a `last_updated`, increment a counter, append to a JSONL log, ensure a
scaffold file exists, post to a webhook. Those are side effects, and Darkmatter
keeps them strictly separate.

> The engine is deliberately **not** wired into the compose pipeline — composing
> a document never invokes a side effect. Only an external orchestrator (e.g.
> Claudine's lifecycle stack) drives it.
> — `darkmatter/lib/src/effects/mod.rs`

This is the division of ownership to keep in mind: **Darkmatter owns the effects
engine; Claudine drives it through lifecycle events; compose stays pure.** The
`claudine context --side-effects` report only *documents* the catalog — it never
runs an effect (see [below](#documentation-only-guarantee)).

## The engine

`EffectEngine` (`effects/mod.rs`) is built through a builder that establishes
the engine's safety envelope:

```rust
let engine = EffectEngine::builder()
    .mutation_root("/path/to/repo")  // filesystem writes are confined here
    .allowed_hosts(["example.com"])  // network is deny-all unless listed
    .auto_rehash(true)               // re-hash Markdown with a `hash:` field
    .build();
```

- **`mutation_root`** — every filesystem write is confined to this directory.
  Writes go through `atomic_write_guarded` (temp file + rename) and a lexical
  containment check refuses any path that escapes the root.
- **`allowed_hosts`** — **deny-all by default**. `http_post` refuses any host not
  explicitly allow-listed, before any network access happens.
- **`auto_rehash`** (default `true`) — after mutating a Markdown document that
  carries a `hash:` frontmatter field, the engine recomputes the hash so the
  document stays self-consistent.

## Runtime-accessible descriptions

Every effect descriptor implements the shared `Described` trait from
`darkmatter::catalog`. This powers exact lookup, fuzzy suggestion, and
plain-text error enrichment for consumers that need to talk about capabilities
programmatically.

The `--side-effects` report renders an optional **Example** column (hidden below
70 columns to preserve the minimum supported width). Examples are taken from
`EFFECT_DESCRIPTORS` and formatted with the same rules as the expression report.

## The capabilities

The verbs live in `effects/verbs.rs` and are catalogued in
`effects/catalog.rs`. Each returns a value (a prior value, a new array, an
absolute path, or an HTTP result):

| Category | Capabilities |
|----------|--------------|
| **Frontmatter Mutations** | `set_frontmatter`, `merge_frontmatter`, `delete_frontmatter`, `increment_frontmatter`, `decrement_frontmatter`, `append_frontmatter`, `prepend_frontmatter` |
| **File & Directory** | `ensure_file(file)`, `ensure_file(file, content)`, `ensure_dir`, `append_line`, `append_jsonl` |
| **Network** | `http_post(url, body)` |

Each capability has a **safety classification** (`EffectSafety`):

- **`FilesystemWrite`** — write confined to the mutation root.
- **`Network`** — access restricted by the deny-all allowlist.
- **`MarkdownMutation`** — Markdown change that honors auto-rehash.

## How the `--side-effects` report is built

`render_side_effects_report` (`claudine/cli/src/commands/context.rs`) renders
directly from `effect_descriptors()` (`EFFECT_DESCRIPTORS`). It groups by
`category` and emits a table — **Capability**, **Description**, **Example**
(above 70 columns), **Safety** — with the safety cell colored by
classification. Ahead of the table it prints a constraint checklist that
restates the engine's guarantees (documentation-only, orchestrator-driven,
mutation-root confinement, deny-all network, auto-rehash, and that *catalog
membership is not authorization*).

Because effects are **typed methods with no string dispatcher**, the descriptor
catalog *is* the authoritative capability surface — there is no runtime name
table to enumerate (unlike the expression engine). A method without a descriptor
is simply not a catalogued capability.

## Intentionally uncatalogued methods

Public `EffectEngine` methods that are deliberately excluded from the capability
surface are listed in `INTENTIONALLY_UNCATALOGUED` in
`darkmatter/lib/src/effects/catalog.rs`. The list is currently empty: every
public mutating method is either catalogued or explicitly reviewed and
documented as excluded. Adding a method to this list requires the same code-
review scrutiny as adding a descriptor, because it is a claim that the method
should remain outside the documented capability surface.

## Documentation-only guarantee

The `--side-effects` report — and every other `claudine context` report — must
construct **no** `EffectEngine` and attempt **no** network access. This is not
left to inspection. Darkmatter ships an optional `effects-instrumentation`
feature that maintains process-wide counters (`engine_build_count`,
`network_attempt_count`), bumped at engine construction and at `http_post`
entry. The CLI test
`metadata_reports_construct_no_engine_and_attempt_no_network` drives all three
documentation reports and asserts neither counter moved. Production builds do not
enable the feature, so the atomics never touch the hot path.

## How to add a capability

1. **Implement it.** Add a method to `EffectEngine` (in `verbs.rs`) honoring the
   mutation-root / allowlist / auto-rehash guards.
2. **Catalog it.** Add an `EffectDescriptor` to `EFFECT_DESCRIPTORS` with its
   `signature`, `description`, `safety`, `category`, `order`, and a verified
   `example`.
3. **Pin it under test.** Add an `EffectVerb` entry to the `cfg(test)`
   `EFFECT_VERBS` table pairing the descriptor signature with a real call to the
   new method. `verb_signature_set_equals_descriptor_signature_set` also asserts
   every descriptor carries an example.
4. The `--side-effects` report needs **no change** — it reads the catalog.

If the method is intentionally *not* a catalogued capability, add its name to
`INTENTIONALLY_UNCATALOGUED` in `effects/catalog.rs` with a written rationale in
the code-review discussion. The allow-list exists because Rust cannot enumerate
a type's public method surface at compile time; it converts the silent orphan-
method gap into a reviewed decision.

## Drift control for side effects

Tests in `effects/catalog.rs` keep the catalog honest:

- **`verb_signature_set_equals_descriptor_signature_set`** — bidirectional set
  equality between `EFFECT_VERBS` (each entry calls a real `EffectEngine`
  method) and `EFFECT_DESCRIPTORS`. A descriptor with no backing method, or a
  verb with no descriptor — including a new overload like
  `ensure_file(file, content)` — fails the build.
- **`every_verb_maps_to_a_reachable_method`** — runs each verb against a sandbox
  engine, proving the method is reachable (a renamed/removed method fails to
  compile or run). `http_post` is proven reachable by its allowlist refusal, so
  the test performs no real network I/O.

**Known limitation:** because Rust cannot enumerate a type's public method
surface at compile time, these tests *cannot* detect a public `EffectEngine`
method that was simply never given a descriptor. Such a method is intentionally
outside the capability surface until a descriptor adds it. See
[Drift Control](drift.md#next-steps).
