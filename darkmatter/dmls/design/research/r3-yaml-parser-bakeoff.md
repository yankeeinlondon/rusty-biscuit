---
prompt: |-
  DMLS (the Darkmatter Language Server) needs a position-aware YAML parse of
  Markdown frontmatter behind a `FrontmatterAst` facade. Architectural
  decision AD-4 in @darkmatter/features/2026-07-04-dmls/design.md was
  accepted as: prefer `rlsp-yaml-parser`, with a fallback of building a small
  spanned-tree loader directly on `saphyr-parser` events; `serde-saphyr` may
  serve the validation side but is not the geometry source.

  Your task is a hands-on bake-off validating that decision. Create a small
  Rust scratch project and evaluate each candidate against realistic
  Darkmatter frontmatter (draw fixture shapes from
  @darkmatter/docs/schemas/darkmatter.yaml and
  @darkmatter/docs/schemas/claudine.yaml — nested objects, arrays of
  objects, anchors/aliases like the Claudine lifecycle-event anchor, block
  scalars, flow mappings, comments, duplicate keys, and deliberately
  malformed mid-keystroke input like a dangling `key:` or bad indent):

  1. For `rlsp-yaml-parser`: span fidelity on every node kind, comment
     preservation, behavior on malformed input (panic? error? partial
     tree?), YAML 1.2 coverage (anchors, merge keys, multiline scalars),
     crate maintenance signals (release cadence, issues, bus factor).
  2. For a bespoke loader on `saphyr-parser`: how much code is actually
     required for a spanned tree with dotted-path lookup? Prototype it.
     Which edge cases (anchor replay, merge keys) create real work?
  3. For `serde-saphyr`: does it expose a spanned *arbitrary-tree* mode
     usable without target structs, or is it strictly type-driven?
  4. Measure parse time on a 200-line frontmatter block for each.

  Deliverables: a comparison table (span fidelity, comments, error
  recovery, YAML coverage, maintenance, perf), the prototype code findings,
  and a confirmed or revised recommendation for AD-4 including the exact
  `FrontmatterAst` construction path.
last_updated: 2026-07-06
hash: fbc68dbe0303a8cf-58e0574cfb9f63b8
---
# R-3 YAML Parser Bake-Off

## Scope

I built a scratch Rust project at `/tmp/dmls-yaml-bakeoff` and tested the AD-4 candidates against frontmatter-shaped fixtures derived from `darkmatter/docs/schemas/darkmatter.yaml` and `darkmatter/docs/schemas/claudine.yaml`.

Fixture coverage:

- nested schema objects such as `$schema.hash[]`, `ctx`, `style.page`
- arrays of objects such as `sequence`
- Claudine-style lifecycle anchor/alias: `&lifecycle-event` and `*lifecycle-event`
- merge key: `<<: *lifecycle-event`
- block scalar prologue text
- flow mappings and flow sequences
- comments
- duplicate keys
- malformed mid-keystroke input:
    - dangling `owner:`
    - bad indentation
    - mismatched flow collection

The scratch harness used:

- `rlsp-yaml-parser 0.11.1`
- `saphyr-parser 0.0.9`
- `serde-saphyr 0.0.29`

## Results

| Candidate                         | Span Fidelity                                                                                                                                     | Comments                                                                                                                                                       | Malformed Input                                                                                                                                                                                       | YAML Coverage                                                                                                                                                                                                     | Maintenance                                                                                                                                                                                                                            | 200-Line Parse Time               |
|-----------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|-----------------------------------|
| `rlsp-yaml-parser`                | Best fit. Loader returns `Node<Span>` with byte spans on mapping, sequence, scalar, alias, key nodes via traversal, plus anchor/tag span helpers. | Preserved on AST nodes as leading/trailing comments. Top document-prefix comment behavior is selective; attached comments worked.                              | No panic. Dangling `key:` becomes an empty scalar with zero-width value span. Bad indent and bad flow return positioned syntax errors, but no partial AST is returned on hard syntax errors.          | YAML 1.2-oriented. Anchors/aliases are preserved in lossless mode; resolved mode exists. Merge keys are parsed as authored mapping entries, not semantically replayed. Block scalars and flow collections worked. | Young, low adoption. GitHub repo shows 6 stars, 1 fork, 1 issue, 1 PR, 1,450 commits, and 44 releases; changelog has `0.11.1` dated 2026-06-08. External contributions are not currently accepted.                                     | ~3.14 ms                          |
| bespoke loader on `saphyr-parser` | Good raw event spans. Prototype builds a spanned tree and dotted-path lookup, but collection spans need joining and key/value ownership is ours.  | Not available from the parser events I used. Capturing comments would require scanner-level work or a side-channel lexer.                                      | No panic. Dangling `key:` becomes an empty scalar. Bad indent and bad flow return positioned scan errors. No partial tree after hard errors unless DMLS stores the last completed stack state itself. | Parser handles YAML 1.2 events, anchors as numeric IDs, aliases, block scalars, flow collections. Semantic anchor replay and merge-key expansion are real work.                                                   | Stronger adoption base. `saphyr-rs/saphyr` shows 326 stars, 39 forks, 28 issues, 12 PRs; README says it passes the YAML test suite. Latest GitHub release shown was `v0.0.6` on 2025-06-11, while crates.io has `saphyr-parser 0.0.9`. | ~2.00 ms                          |
| `serde-saphyr`                    | Good for typed values through `Spanned<T>`, including referenced/defined locations for aliases and merges. Not a geometry AST.                    | Supports `Commented<T>` for typed deserialization, but docs say freestanding comments are not captured and recommend using `granit-parser` directly for those. | No panic. Excellent snippet errors. Dangling `key:` parsed. Duplicate key rejected by default before arbitrary inspection.                                                                            | Validation-oriented. Anchors/merge behavior is useful when deserializing into target types. It deserializes directly into Rust types rather than constructing an intermediate abstract tree.                      | Active. GitHub shows 199 stars, 18 forks, 25 releases; latest release `0.0.29 Hardening release` on 2026-07-03.                                                                                                                        | ~0.47 ms into `serde_json::Value` |

Timing command: best of 5 runs, 1000 parses per run, on the local macOS host. These are directional, not criterion-grade benchmarks. All candidates are fast enough for Markdown frontmatter-sized LSP work.

Sources checked for maintenance signals:

- [`chdalski/rlsp`](https://github.com/chdalski/rlsp)
- [`saphyr-rs/saphyr`](https://github.com/saphyr-rs/saphyr)
- [`bourumir-wyngs/serde-saphyr`](https://github.com/bourumir-wyngs/serde-saphyr)
- local crate metadata and changelogs from `cargo info` / cargo registry source

## Prototype Findings

The bespoke `saphyr-parser` loader was small but not free.

Scratch size:

- full harness: 425 lines
- core `saphyr-parser` loader, stack, attach logic, and lookup: about 90 lines
- useful prototype including node enum, duplicate-key detection, dotted lookup, reporting, and fixture timing: roughly 170 lines

The MVP loader shape:

```rust
enum MiniNode {
    Scalar { value, style, span },
    Mapping { entries: Vec<(MiniNode, MiniNode)>, span },
    Sequence { items: Vec<MiniNode>, span },
    Alias { id, span },
}
```

It uses a stack of open containers:

```rust
enum Container {
    Map {
        entries: Vec<(MiniNode, MiniNode)>,
        pending_key: Option<MiniNode>,
    },
    Seq {
        items: Vec<MiniNode>,
    },
}
```

That is enough for:

- preserving declaration order
- duplicate local key detection
- dotted-path lookup
- scalar, sequence, mapping, and alias spans
- dangling-key tolerance through empty scalar events

The production gaps are the important part:

- comments are not present in the parser event stream used by the prototype
- anchors are numeric IDs, so DMLS would need its own anchor-name/source map
- alias replay is semantic work and must be bounded against expansion attacks
- merge keys require mapping replay semantics and source-range policy for inherited keys
- hard syntax errors stop the stream, so best-effort partial trees require explicit recovery design
- collection spans need careful normalization to DMLS byte ranges and LSP UTF-16 positions
- duplicate-key policy must preserve all authored entries for diagnostics while choosing a semantic value for schema validation mapping

This validates AD-4's fallback estimate: a small tree loader is plausible, but the first 100 lines only buy the easy part. The LSP-grade edge cases are where ownership cost appears.

## `serde-saphyr` Finding

`serde-saphyr` does not expose the arbitrary, spanned YAML tree DMLS needs as a ready-made public mode.

What worked:

```rust
BTreeMap<String, serde_saphyr::Spanned<serde_json::Value>>
```

This gives spans for top-level typed values. It does not recursively annotate every arbitrary nested key/value because `serde_json::Value` has no span fields.

`Spanned<T>` is excellent for typed validation paths and has useful alias/merge semantics through its `referenced` and `defined` locations. But DMLS needs open-shaped geometry before it knows the effective schema target. For that, `serde-saphyr` remains a validation-side candidate, not the `FrontmatterAst` geometry source.

## Recommendation

Confirm AD-4 as written: use `rlsp-yaml-parser` behind `FrontmatterAst`, keep the bespoke `saphyr-parser` loader as the fallback path, and do not use `serde-saphyr` as the geometry source.

The exact construction path should be:

1. Darkmatter extracts frontmatter text with the existing frontmatter extractor, preserving:
    - delimiter byte ranges
    - `base_line`
    - frontmatter-relative byte offset

2. DMLS calls `rlsp_yaml_parser::LoaderBuilder::new().lossless().build().load(frontmatter_text)`.
3. DMLS lowers the first `Document<Span>` into a DMLS-owned `FrontmatterAst`:
    - `FrontmatterAst::delimiter_range`
    - `FrontmatterAst::root: FrontmatterNodeId`
    - arena of `FrontmatterNode`
    - mapping entries preserving declaration order
    - key span, value span, path segment, node kind, scalar style, comments, anchor/tag/alias metadata
    - duplicate-key side table keyed by normalized path

4. `FrontmatterAst` builds lookup indexes:
    - dotted path: `style.page.margin`
    - JSON Pointer: `/style/page/margin`
    - authored-entry lookup for duplicates

5. DMLS projects frontmatter-relative byte spans through the source map into document LSP ranges.
6. Schema validation remains separate:
    - assemble effective schema through Darkmatter schema APIs
    - run validation on the semantic frontmatter value
    - map validation error paths back through `FrontmatterAst`
    - for merge/alias-derived values, prefer authored source ranges; if semantic expansion is later required, add an explicit `SemanticOrigin` layer rather than mutating the source AST

Important caveat: `rlsp-yaml-parser` does not solve partial AST recovery for hard malformed YAML. For v1, treat this as acceptable if DMLS preserves the previous good AST per document version and emits parse diagnostics from the current failed parse. If mid-error completion inside broken YAML becomes a hard requirement, implement a local recovery wrapper first; only fall back to the bespoke `saphyr-parser` loader if `rlsp-yaml-parser` maintenance or correctness becomes a blocker.
