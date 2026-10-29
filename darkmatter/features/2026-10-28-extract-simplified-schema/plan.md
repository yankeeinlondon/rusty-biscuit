---
total_phases: 9
created: 2026-10-25
phase: 1
agent: claude/sonnet
yolo: true
human_review: false
human_review_items: []
packages:
    - simplified-schema
    - biscuit-file
    - darkmatter
    - darkmatter-cli
    - dmls
    - claudine
    - claudine-cli
    - claudine-gen
    - messenger
---

# Plan: Extract SimplifiedSchema

## Summary and Definition of Done

`darkmatter/lib/src/markdown/schemas/` (about 53k lines, 1,128 unit tests) holds
the SimplifiedSchema language (`simplified/`), validation and JSON Schema
lowering (`validate.rs`, `format.rs`, `json_shape.rs`), named-type resolution
(`resolve.rs`, `resolve/`), and conversion (`coercion/`), welded to Darkmatter's
Markdown, composition, expression, and lifecycle code. The work moves the
language into a new `simplified-schema` package area (library only), leaves host
policy in Darkmatter, and puts a first-class `SimplifiedSchema` object in front
of it. It is a **behavior-preserving** extraction: current observable behavior
is the parity authority, and no function signature, coercion rule, catalog
entry, or runtime recovery path changes.

The work falls into these pieces:

- **Foundations (`biscuit-file`, characterization):** the passive frontmatter
  helper, the `FileReference` feature split, the baseline measurements, and the
  pinned pre-extraction behavior every later phase is judged against.
- **Package scaffold:** `simplified-schema/lib/`, workspace and CI
  registration, `file` feature, recipes.
- **Shared passive code:** numeric model, span vocabulary, passive expression
  syntax (lexer/parser/AST/errors).
- **Language core:** AST, grammar, serializers, subtype, query/cursor, source
  maps, named-type resolution, conversion engine, JSON Schema lowering,
  validation.
- **The facade:** `SimplifiedSchema` with factories, `TryFrom<&str>`, layering,
  strict/soft validation, views, YAML export.
- **Host and consumer migration:** Darkmatter adapters and baseline layering,
  DMLS, Claudine (lib, CLI, generator), Messenger.
- **Evidence and docs:** benchmarks, guards, corpus checks, OS evidence, docs,
  skills, dependency records.

**Done** means all sixteen acceptance criteria in the spec hold, each backed by
a test or a recorded measurement, and:

- `just test` and `just lint` pass in `simplified-schema/`, `darkmatter/`,
  `biscuit-file/`, `claudine/`, and `messenger/`; `just test-l2` passes where an
  L2 test was touched (none is expected to be added).
- `darkmatter/lib/src/markdown/schemas/` holds no second parser, converter,
  subtype implementation, numeric model, or conversion engine
  (`simplified-schema` is the only one).
- The Cargo graph of the schema-only example is clean in both the default and
  `--no-default-features` configurations (Phase 8 records it).
- Docs, skills, and dependency records describe the final boundary and never
  name this spec.
- The implementation log records every measurement, departure from this plan,
  and smell-grep result. The author moves the spec to `_completed`; this plan's
  terminal state is "implementation complete, ready for review".

## Conventions Used by Every Phase

- Every moved test moves **with its module**; a moved test is never rewritten to
  fit new behavior. A test that must change to compile gets the minimal edit
  (import path, constructor spelling) and the edit is listed in the log.
- A task that moves code runs the **Move Protocol**:
    1. Move the file (use `git mv` so history follows) and fix paths.
    2. Leave a `pub use` re-export in Darkmatter at the old path so unmoved
       callers compile (type identity is preserved; one implementation).
    3. Run `just test` in both areas. Red tests are fixed before the next task.
    4. Pass over every `///`, `//!`, and `//` comment of each moved symbol and fix
       or delete drift in the same change.
- Migration-only re-exports are tagged `// migration: removed in Phase 7` and
  deleted in Phase 7 once repository consumers have moved.
- Never run `cargo fmt`. Never commit unless the prompt says so.
- All paths are tested with Windows-style (`C:\`, `\\?\`) and CRLF inputs
  wherever the moved code handles paths or spans (criterion 8).

## Phase 1: Decisions, Characterization, and Foundations

Settle the design, record the boundary, capture the evidence later phases
compare against, and build the two pieces that live below the new crate
(`biscuit-file`).

### Decisions

1. **Facade name and the existing enum.**
    - **decision**: the new facade struct takes the public name
      `SimplifiedSchema`. The existing `schemas::simplified::SimplifiedSchema`
      enum (`Single`/`Union`) becomes `SchemaRoot` inside the library; it is a
      public, inspectable type (spec: "do not flatten root unions").
    - **alternatives considered**: name the facade `Schema` (loses the language
      name the spec fixes); keep both under one name via a module path (an
      ambiguous import for every consumer).
    - **why**: the spec calls the object `SimplifiedSchema` and the language
      `SimplifiedSchema`; the enum is an internal shape, not what a consumer
      holds.
    - **downsides**: a rename ripples through about 20 `dmls/src` and 14
      `cli/src` files and the moved tests; mechanical, done in Phase 4 once.

2. **Module-to-owner map is fixed here, refined by the inventory.**
    - **decision**: the owner table below is the starting map. Task 1.1 turns it
      into a per-file inventory; where the inventory finds a module that the
      table does not cover, apply the rule *"if it calls the expression
      evaluator, Markdown scanner, lifecycle, `ComposeContext`, or baseline
      policy, it stays in Darkmatter; otherwise it moves."*
    - **alternatives considered**: move `schemas/` whole and carve Darkmatter
      pieces out (drags host policy into the new crate); leave a thin shim crate
      (spec forbids permanent parallel APIs).
    - **why**: matches the spec's Ownership table, and the rule is mechanical
      enough for an agent to apply without a ruling.
    - **downsides**: a few modules (`mod.rs` at 4.6k lines, `resolve.rs` at 6.3k)
      mix both kinds and must be split rather than moved.

   | Moves to `simplified-schema` | Stays in Darkmatter |
   | --- | --- |
   | `simplified/*` (AST, grammar, types, convert, subtype, serialize, query, cursor, source, yaml_scalar, lint, signature, alphanumeric, definitions, standalone) | `DarkmatterSchemas`, `EffectiveSchema` host assembly in `schemas/mod.rs`; `catalog.rs` (shipped catalogs, `lifecycle::host_profile`); `roots.rs`; `triggers/`; `embedded.rs` (`Name@darkmatter:<stem>`) |
   | `validate.rs`, `format.rs`, `json_shape.rs`, `phase.rs`, `example.rs`, `positions.rs` (pure parts), `advisory.rs`, `reference.rs`, `discriminant.rs`, `scope.rs` (type scope) | `bound_operands.rs` (lifecycle), `file_match.rs` (workspace/glob), `clean.rs`, `rewrite.rs`, `about.rs`, `timings.rs`, `detect.rs` (document-kind detection), `frontmatter_shape.rs` |
   | `resolve.rs`, `resolve/` (named-type expansion, graph, recursion, merge) minus schema-root policy | `schemas/tests/` cases that exercise Markdown, baseline, or triggers |
   | `completion.rs` suggestion model (pure part); suggestion lint | completion presentation, DMLS wiring |
   | `coercion/` (engine, shape, union, error), `numeric.rs`, `numeric/ingress.rs` | `compose/` (argument binding orchestration, pending classification, lifecycle) |
   | `compose/expression/{lexer(token part),parser,ast,error(syntax part)}` | `compose/expression/` everything else (evaluator, functions, host, scope, catalog, call_check, resolve_ctx, template finder) |
   | `markdown/span.rs` `Spanned<T>` (re-exports `biscuit_file::SourceSpan`) | `literal_token.rs` unless the engine requires its codec (see decision 4) |

3. **Passive frontmatter helper lives in `biscuit-file`, unfeatured.**
    - **decision**: move `extract_frontmatter_block` and its extraction struct
      into a `biscuit_file::frontmatter` module that builds with no features and
      returns its own `FrontmatterEnvelopeError`; Darkmatter wraps it and maps the
      error to `MarkdownError::FrontmatterFenceMismatch`. The `biscuit-file` CLI's
      separate extraction is untouched.
    - **alternatives considered**: a new tiny crate (spec: no extra package);
      keep it in Darkmatter and duplicate in the schema crate (spec forbids a
      second parser).
    - **why**: spec names `biscuit-file` as the example home; today the function
      returns `MarkdownResult`, which drags `biscuit-terminal` error types, so it
      needs its own error to be passive.
    - **downsides**: two error types for one failure, bridged by one `From`.

4. **Literal-token codec and pending facts.**
    - **decision**: `schemas/` and `coercion/` call `literal_token::{encode,
      decode_leaf, holds_pending_syntax}` and `compose::is_deferred_template`.
      Task 1.1 records exactly which of those the moved code reaches. The pure
      string codec of the `{{!data:v1:…}}` token moves into `simplified-schema`
      (module `pending`, `base64` dependency) **only if** the conversion or
      validation engine calls it; all classification, origin tracking, and the
      decision to treat a value as pending stay in Darkmatter, which classifies
      before calling the library. The library receives plain `serde_json::Value`s.
    - **alternatives considered**: a callback trait the library calls to ask "is
      this pending" (spec: no callback/precomputed-finding framework); move all of
      `literal_token.rs` (carries host semantics).
    - **why**: spec section "Existing orchestration" keeps pending-state
      classification in Darkmatter and forbids a callback framework; the engine
      stays pure if it never needs to ask.
    - **downsides**: if the engine truly interleaves the check with coercion,
      Darkmatter must pre-walk values; the Phase 5 parity tests catch any
      difference.

5. **Error types are split along the host line, not wrapped.**
    - **decision**: `ExpressionError` and `error.rs` split: syntax errors
      (lexer/parse errors, spans, hints) move; errors carrying `ContextGroup`,
      host-function, or evaluation facts stay. `SchemaError` moves whole
      (`RemoteUnsupported` included) and Darkmatter's wrapper variants hold it by
      value so the source chain is intact.
    - **alternatives considered**: move `error.rs` whole (drags `ContextGroup`);
      stringify at the boundary (spec: never stringify provenance).
    - **why**: spec R6 "must not stringify away provenance".
    - **downsides**: some `From` impls multiply in Darkmatter.

6. **Where raw-path context capture lives.**
    - **decision**: a private `capture` module inside `simplified-schema`, behind
      the `file` feature, builds a fresh `FileResolutionContext` once at factory
      entry using `biscuit-file`'s context constructors (and `sniff` only if that is
      how Darkmatter captures repository roots today; Task 1.1 confirms). No
      dependency on Darkmatter.
    - **alternatives considered**: a public loader object (spec rejects); a helper
      in `biscuit-file` (it has no `sniff` dependency and should not gain one);
      capturing in each of the three factories (three capture sites to audit).
    - **why**: spec requires one capture point, no Darkmatter dependency, and "no
      public loader or second argument".
    - **downsides**: `simplified-schema` takes a `sniff` dependency in the `file`
      feature if sniff is the capture route; the no-default-features build does
      not.

7. **Cargo feature names.**
    - **decision**: `simplified-schema` features: `default = ["file"]`; `file =
      ["dep:sniff"?, "biscuit-file/file-reference"]`. `biscuit-file` gains
      `file-reference-syntax` (passive `FileReference` parse and types) which
      `file-reference` (resolution: `gix`, `walkdir`, `globset`, `url`) enables.
      Existing `file-reference` users compile unchanged.
    - **alternatives considered**: invert to opt-out features (breaks existing
      users); split into three `biscuit-file` features (more surface than the spec
      needs).
    - **why**: spec "narrowly split" and "preserve capabilities when enabled".
    - **downsides**: `url` placement decided by what `FileReference` parsing needs
      (Task 1.4 reads `parse.rs`).

8. **No spikes.**
    - **decision**: no spike is scheduled.
    - **alternatives considered**: a spike on feature-split feasibility; a spike on
      staging cost for atomic layering; a spike on context capture.
    - **why**: the spec's context boundary "no longer requires a decision spike"; the
      `biscuit-file` split is read from `parse.rs` in Task 1.4 and fails fast at
      compile time if wrong; layering staging cost is something criterion 7 and
      10 say to *measure* (Task 6.5), and the spec already says to reuse shared
      snapshots rather than copy.
    - **downsides**: if `FileReference` parsing needs `url`/`gix` types the split
      grows by one feature (the plan already allows a `url` pass-through).

9. **Characterize first, then move.**
    - **decision**: Phase 1 adds *pre-extraction* tests in Darkmatter that pin the
      outcomes the Input Robustness Matrix needs (below) and the strict/soft
      examples in R6. They are written against the current code and must pass
      before any file moves; later phases move them with their module.
    - **alternatives considered**: write the tests after the move (they would then
      pin the new code, not the parity authority).
    - **why**: spec says existing behavior is the parity authority.
    - **downsides**: one extra phase of test-only work.

### Wave 1 (parallel: disjoint files)

- [ ] **1.1 Boundary inventory.**
    - Produce `darkmatter/features/2026-10-25-extract-simplified-schema/inventory.md`
      (a snapshot, not a doc): one row per file under `schemas/`,
      `coercion/`, `numeric*`, `literal_token.rs`, and `compose/expression/`
      with owner, symbols that cross the line, and the evidence test.
    - Run `rg` for `crate::markdown::compose`, `crate::lifecycle`,
      `literal_token`, `numeric`, `coercion` from inside `schemas/` (37 + 3 + 13
      + 11 + 11 sites today) and record each crossing with a "stays", "moves",
      or "needs a fact" verdict following decision 2's rule.
    - Inventory every external user of the extracted surface: `darkmatter/cli/src`
      (14 files), `darkmatter/dmls/src` (20), `claudine/cli`
      (`completion/schema_completion/keys.rs`), `claudine/gen`
      (`schema_compat.rs`), `claudine/lib`, `messenger/lib`
      (`research/load.rs`). Record each consumer's operation as **strict**
      (`exact validation`) or **soft** (coercing), per R6.
    - Include the **acyclic dependency graph** (Mermaid) with numeric, span,
      conversion, expression-syntax, and pending-codec types shown, and note the
      edge `simplified-schema -> biscuit-file` (+ optional `sniff`).
    - Include the **public-symbol inventory** (spec R7).
    - Include the confirmation of query edge cases against current helpers (empty
      path, missing match, union result) from `simplified/query.rs` and
      `cursor.rs`; the facade preserves what they return.

- [ ] **1.2 Baseline benchmarks.**
    - On this host (macOS), run the existing `schema_validation` and
      `effective_schema_ownership` Criterion benches (and the recursive/cold/warm
      cases they contain) three times each with `cargo bench --bench …` and keep
      the medians in `implementation-log.md` under `### Baseline`, with host,
      toolchain (`rustc -V`), and profile. Phase 8 re-runs the same commands.
    - One host, one quick sample (no widening); no CI gate.

- [ ] **1.3 Pre-extraction characterization tests (matrix and R6).**
    - In `darkmatter/lib/tests/l1/` add `schema_extraction_characterization.rs`:
        - R6 table rows (all four) through the current exact-validation and
          coercing operations: acceptance, effective value, and (for borrowed vs.
          owned) the equivalent today (value unchanged vs. changed).
        - Input immutability (clone before/after, compare).
        - Input Robustness Matrix cells (see the section below) against the current
          `parse_yaml_schema`, `kind: schema`/`types` and raw JSON Schema parsers,
          recording the actual current outcome in each assertion.
        - Query edge cases (dotted keys, `[0]`, `["a.b"]`, empty path, union).
        - Function-schema parse/serialize round trip, `fallible` default, metadata.
    - Pin whatever the current code does; if an outcome looks wrong, assert it
      anyway and note it in the log (parity authority; spec forbids fixing).

### Wave 2 (parallel: separate packages)

- [ ] **1.4 `biscuit-file` feature split.**
    - Read `biscuit-file/lib/src/file_reference/parse.rs` and `mod.rs`; add
      `file-reference-syntax` (passive grammar, `FileReference`, errors, portable
      path types) with no `gix`/`walkdir`/`globset`; make `file-reference` imply it.
    - Gate `resolve.rs`, `glob/`, `context.rs` capture pieces behind
      `file-reference`; keep every public type's path and identity.
    - Verify: `cargo tree -p biscuit-file --no-default-features --features
      file-reference-syntax -e normal` excludes `gix`, `walkdir`, `globset`; the
      default build and `just test` in `biscuit-file/` are unchanged.
    - Add an L1 test that parses a reference with the syntax feature only
      (`cargo nextest -p biscuit-file --no-default-features --features
      file-reference-syntax`).
    - Update `biscuit-file/docs/dependencies.md` / feature table in
      `biscuit-file/docs/` and `.claude/skills/biscuit-file/`.

- [ ] **1.5 Passive frontmatter helper.**
    - Create `biscuit_file::frontmatter` (unfeatured) containing
      `extract_frontmatter_block`, `FrontmatterExtraction`, line-span helpers,
      BOM handling, near-miss fence detection, preserving `yaml_base_line` and
      `body_span` exactly; own error `FrontmatterEnvelopeError`.
    - Re-point `darkmatter/lib/src/markdown/frontmatter.rs` to call it, mapping the
      error. Keep Darkmatter's public function signature (re-export at old path,
      migration-tagged only if its path is not already part of Darkmatter's API).
    - Tests move with it: CRLF, BOM, `----` near-miss, missing closer, empty block.
    - Build check with `--no-default-features`.

### Phase 1 checkpoint

- [ ] `inventory.md` complete and every "needs a fact" row has a decided home.
- [ ] Characterization tests green on unmodified Darkmatter code.
- [ ] `just test` and `just lint` green in `biscuit-file/` and `darkmatter/`.
- [ ] Baseline numbers recorded.

### Input Robustness Matrix

This work adds loaders that read configuration formats: the schema YAML loader
(`$schema` bodies, `kind: schema`/`types`), the raw JSON Schema loader, and the
Markdown frontmatter extractor, behind three factories and `TryFrom<&str>`.

**Default outcome (stated once):** any invalid shape produces a located,
typed error (`SchemaError` / `FrontmatterEnvelopeError`), nothing is opened or
read beyond the single input, and no schema is published. Because the
extraction is behavior-preserving, **the existing parsers decide each exception**;
Task 1.3 records them first and later tests pin them. Expected exceptions to
confirm (not to invent): `$schema` absent in `TryFrom<&str>` is a typed
*missing-schema* error (distinct from malformed); a YAML scalar alone is not a
schema (typed error); Markdown body text contributes nothing.

**Field shapes**, walked once through each shared field validator
(`$schema`, `kind`, `types`, `returns`/`fallible`, per property: type string):

| Field | absent | explicit null (`k:`/`k: null`) | wrong type whole | empty (`""`, `{}`) | wrong type one element |
| --- | --- | --- | --- | --- | --- |
| `$schema` (root/frontmatter) | missing-schema error | pinned from current | pinned | pinned | n/a |
| `kind` | pinned | pinned | pinned | pinned | n/a |
| `types` | pinned | pinned | pinned | pinned | map entry with non-string type pinned |
| property type text | n/a | pinned | pinned | pinned | n/a |

Each field using a shared validator (all property type texts share the grammar
validator) gets a control case plus one invalid shape proving it is wired to it.

**Document shapes**, once per distinct loader: YAML loader (shared by schema
file, `$schema` envelope, `TryFrom<&str>` YAML), raw JSON Schema loader (JSON via
the same value reader; JSON5 only if the loader accepts it today), frontmatter
extractor:

| Shape | YAML schema loader | Raw JSON Schema loader | Frontmatter extractor |
| --- | --- | --- | --- |
| duplicate key | pinned | pinned | n/a (delegates to YAML loader) |
| trailing / invalid content | pinned | pinned | pinned (junk after closer is body) |
| format-specific | second YAML document, `k:` null, CRLF, BOM | draft-URI dialect vs. remote file open (`RemoteUnsupported`) | `----` near-miss, missing closer |

**Control rows:** the unedited `Person` fixture from the spec must give the
positive result in each loader, and each edit is made to a real shipped schema
fixture, one edit per cell. Tests run in-process against the public API;
spawn the `md` CLI only for one wiring case per loader (control + one invalid
shape, asserting the located error and that nothing launched).

**Smell greps before the matrix is declared done (Phase 8):** `#[serde(default)]`
on a load-bearing field; `Option<T>` where absent and null must differ;
`filter_map(.. as_str())`, `unwrap_or_default()`, `.ok()` on a load-bearing
parse, in the moved loaders. Findings are pinned as existing behavior, not
fixed, and logged. A review finding about one cell lists the state of every
other cell in that row and column.

## Phase 2: Scaffold `simplified-schema` and Move Shared Foundations

Create the package area and move the leaf modules that every later move needs:
numeric model, span vocabulary, and the pending codec if required.

### Wave 3 (sequential: scaffold first)

- [ ] **2.1 Package area.**
    - Create `simplified-schema/lib/` (package `simplified-schema`, lib name
      `simplified_schema`, edition 2024, license consistent with Darkmatter's
      `AGPL-3.0-only`), `README.md` (overview with links into `docs/`), `docs/`,
      `justfile` modeled on a sibling lib-only area (recipes: `test`, `test-l2`,
      `lint`, `bench`), and the `l1` test root following the consolidated-binaries
      rule (`autotests = false`, `tests/l1/main.rs`).
    - Features: `default = ["file"]`, `file`; dependencies limited to
      `serde`, `serde_json` (`preserve_order`, `raw_value`, `float_roundtrip` as in
      Darkmatter), `indexmap`, `thiserror`, `biscuit-file` (`yaml`,
      `file-reference-syntax`), `jsonschema`/validator dependencies already used by
      `validate.rs`, and optional `sniff` per decision 6. Task 2.1 reads Darkmatter's
      `Cargo.toml` and takes only what moved code uses.
    - Register in root `Cargo.toml` members, root `justfile` `areas :=`, and any CI
      manifest that enumerates areas (`sniff repo package-areas` must list it;
      check `.github/ci/environments.json` and `scripts/ci/` for hard-coded lists).
    - Add `[package.metadata.ci.tests] source-inputs` entries for any test that will
      read a Darkmatter source file (Phase 8 fills this in).
    - Load the `rust-devops` skill before touching CI registration.

### Wave 4 (parallel after 2.1: disjoint modules)

- [ ] **2.2 Numeric model.**
    - Move `markdown/numeric.rs` and `numeric/ingress.rs` to `simplified_schema::numeric`
      (Move Protocol). Darkmatter re-exports from `crate::markdown::numeric`.
    - Parity: the existing numeric admission/precision tests move; the
      `numberlike` and `i128`/f64 boundary cases are explicitly named in the log.

- [ ] **2.3 Span vocabulary.**
    - Move `Spanned<T>` and span helpers from `markdown/span.rs` that the moved code
      uses; `SourceSpan` stays `biscuit_file::SourceSpan` (identity preserved).
      Darkmatter's `span.rs` re-exports.
    - Standalone offsets refer to the exact supplied text including CRLF
      (criterion 8): add a CRLF + Windows-path test.

- [ ] **2.4 Pending codec (conditional, decision 4).**
    - Using Task 1.1's verdict, move only the pure token codec the engine calls;
      otherwise record "not required" in the log and skip.

### Phase 2 checkpoint

- [ ] `just test` / `just lint` green in `simplified-schema/` and `darkmatter/`.
- [ ] `cargo tree -p simplified-schema --no-default-features -e normal` shows no
      `darkmatter`, `claudine`, `content-policy`, `gix`, `walkdir`, `globset`.

## Phase 3: Passive Expression Syntax

Move the passive parser so schema checks can parse expression-typed values,
defaults, and function examples without a Darkmatter host.

### Wave 5 (sequential: split before move)

- [ ] **3.1 Split `lexer.rs`.**
    - `compose/expression/lexer.rs` (2,445 lines) holds both the expression token
      lexer and a Markdown-aware template finder using
      `compose::parse_utils::scan_code_regions`. Separate them: the token lexer
      moves; the `{{ }}` template finder, `is_deferred_template`,
      `defer_templates`, and `is_malformed_template` stay in Darkmatter.
    - Replace the lexer's use of `crate::markdown::numeric::admit_number_spelling`
      with the library's `numeric` (Phase 2).

- [ ] **3.2 Move tokens, AST, parser, syntax errors.**
    - Move `ast.rs` (592), the token lexer, `parser.rs` (2,453: `parse`,
      `parse_spanned`, `parse_condition`, `parse_condition_spanned`, and the parse
      modes) and the syntax part of `error.rs` (decision 5) into
      `simplified_schema::expression` (module name `expression`, documented as
      *syntax only*).
    - Darkmatter's `compose::expression` keeps `mod.rs` re-exporting the moved items
      under the same paths (migration-tagged); evaluator, scope, call_check, lint,
      semantics, host, catalog, functions stay.
    - `ExpressionError` variants that wrap lexer/parse errors now hold the library
      type. Parity: the 28 `compose::expression` crossings inventoried in 1.1 compile
      against the library for the syntax-only ones; the rest stay.

- [ ] **3.3 Syntax-only schema checks.**
    - Make the library's schema checks (expression-typed values, expression defaults,
      function-example calls and expected literals) call its own parser. Verify they
      do not evaluate, run examples, or look up named functions or variables.
    - Add a library test per check proving a syntactically valid but unresolvable
      call (`unknown_fn(1)`) is accepted at the syntax level.

### Phase 3 checkpoint

- [ ] No second expression grammar anywhere: `rg "fn parse_spanned"` finds one.
- [ ] `just test` green in both areas; Darkmatter expression tests unchanged.

## Phase 4: Move the Language Core

Move AST, grammar, serializers, subtype, queries, source maps, resolution, and
conversion. This is the largest and most tightly coupled phase; the tasks are
mostly sequential because the pieces import each other.

### Wave 6 (sequential: dependency order)

- [ ] **4.1 AST, grammar, types, yaml scalars, alphanumeric, definitions.**
    - Move `simplified/{types,grammar,yaml_scalar,alphanumeric,definitions,signature}.rs`.
    - Rename the existing `SimplifiedSchema` enum to `SchemaRoot` (decision 1);
      apply the rename across Darkmatter, DMLS, CLI via the re-export plus a
      workspace-wide search/replace; the facade name is reserved for Phase 6.
    - Grammar's use of `crate::markdown::compose::expression` now uses the library.

- [ ] **4.2 Conversion engine.**
    - Move `coercion/{engine,shape,union,error,mod}.rs` and
      `simplified/convert.rs` (4,255 lines) with the numeric model.
    - Preserve the "shape-based engine boundary" and callable special paths (the
      `numberlike` case): do not touch signatures. The engine takes `&Value` and
      shapes; Darkmatter's `coercion::bind` argument-binding orchestration stays and
      calls the library engine directly (no temporary schema documents, no
      JSON-Schema round trip per argument; confirm in the log with the call site).
    - Moves `tests/l1/coercion_engine_parity.rs` and `coercion_core.rs` sources of
      truth into the library test tree; the Darkmatter copy becomes a thin
      integration case.

- [ ] **4.3 Subtype, serialize, query, cursor, source, lint.**
    - Move `simplified/{subtype,serialize,query,cursor,source,lint,standalone}.rs`.
    - `source.rs` (3,014 lines) mixes source maps with Markdown positions: split,
      moving source maps and scalar decoding; document-relative projection stays in
      Darkmatter's adapter.
    - Preserve literal-key precedence and union-query helpers exactly (criterion 13).

- [ ] **4.4 Resolution and recursion.**
    - Move `resolve/graph.rs`, `resolve/recursive.rs`, and from `resolve.rs`
      (6,310 lines) the named-type expansion, cycle checks, use-site constraint
      merge, and layering merge (`merge_baseline` generalizes to the layering
      algorithm; whole-property replacement preserved).
    - Schema-root policy, baseline selection, `Name@darkmatter:<stem>` namespace,
      and searched-candidate/dependency tracking stay as the Darkmatter-supplied
      **source set**: the library resolves over explicitly supplied sources and an
      explicit searched-but-absent-candidate record (`resolve/searched_tests.rs`
      moves with its type).
    - HTTP(S) schema, named import, and example references return
      `SchemaError::RemoteUnsupported` unchanged.

- [ ] **4.5 JSON Schema lowering and validation core.**
    - Move `validate.rs` (3,107), `format.rs` (1,189), `json_shape.rs`,
      `phase.rs`, `example.rs`, `advisory.rs`, `discriminant.rs`, the type-scope
      part of `scope.rs`, and the pure parts of `completion.rs`/`positions.rs`.
    - Keep `darkmatter-*` formats and `x-darkmatter-*` keywords byte-for-byte; add
      a golden test over the shipped catalogs' lowered output (ordering and
      vocabulary) generated from the pre-move code in Phase 1.3 and compared after.
    - `format.rs` references to pending expressions
      (`is_pending_expression_value`) become host facts: the library validates
      values it is handed; Darkmatter passes pending literals through before
      calling (decision 4). Any assertion with no library implementation yields the
      "unavailable required check" failure (R6), never silent success.

### Phase 4 checkpoint

- [ ] `rg` finds no `crate::markdown::compose` or `crate::lifecycle` import in
      `simplified-schema`.
- [ ] Moved unit tests (the large majority of the 1,128) pass in the new crate;
      the count moved is recorded in the log.
- [ ] `just test` in `simplified-schema/`, `darkmatter/`, `darkmatter/dmls`,
      `claudine/`, `messenger/` green; `just lint` green.

## Phase 5: Validation Contract

Give the library the strict/soft API of R6 on top of the moved engine.

### Wave 7 (sequential, then parallel tests)

- [ ] **5.1 Result types.**
    - Define `ValidationSuccess<'input>` (accepted value as `Cow<'input, Value>`,
      diagnostics) and `ValidationFailure` (invalid data vs. could-not-complete /
      unavailable required check; keeps accumulated structured diagnostics with
      instance and schema paths, source identity, spans; no partially converted
      value).
    - Mapping decision (record in log): `validate_strict` = existing exact
      validation; `validate_soft` = existing coercing operation. Existing callers
      keep their current operation and do not auto-route through the new methods.

- [ ] **5.2 `validate_strict` and `validate_soft`.**
    - Both take `&Value`; strict borrows the input on success; soft borrows when no
      coercion occurred and owns the converted value otherwise. Never mutate the
      input; never clone the whole value merely to validate.
    - Prepared state is held inside the schema object (Phase 6) behind interior
      reuse keyed on definition revision and context identity; Task 5.2 exposes the
      internal `Prepared` type the facade owns.

- [ ] **5.3 Contract tests (parallel with 5.2 follow-ups).**
    - The four R6 table rows for both methods, with pointer-identity assertions for
      "borrows the original" (`Cow::Borrowed` and `std::ptr::eq`).
    - Coercion never repairs a violated constraint (`number(integer)` and `"2.5"`).
    - Nested and union coercion, numeric-precision failure, input immutability.
    - Typed-`Err` cases: invalid data vs. unavailable required check, diagnostics
      preserved, no partial value.
    - Compare every case with the Phase 1.3 characterization outputs.

### Phase 5 checkpoint

- [ ] `just test`/`just lint` green in `simplified-schema/`.
- [ ] Criterion 2 satisfied in the library test tree.

## Phase 6: The `SimplifiedSchema` Facade

### Wave 8 (parallel: separate modules of the same crate)

- [ ] **6.1 Object, internal representation, view API.**
    - `SimplifiedSchema` wraps the retained internal form: typed definitions, raw
      JSON Schema, root union (`SchemaRoot`); no flattening to a property map.
    - Borrowed `TypeDefinition` and `FunctionType` views retain parameter names,
      order, types, optional/rest markers, return type, fallibility, metadata.
    - Query by dotted/bracket path (`people[0].name`, `coords[1]`,
      `settings["a.b"]`) plus programmatic segments; empty-path, missing-match,
      and union results map to the existing query surfaces (Task 1.1 notes); the
      query does not clone the schema or expand recursion.
    - Runtime property iteration and caller-provided selectors via the existing
      traits; no generated enums.

- [ ] **6.2 Factories and `TryFrom<&str>`.**
    - `TryFrom<&str>`: content only, no I/O in any feature configuration; uses the
      shared passive frontmatter helper (Task 1.5) and the existing YAML schema
      parsers; classifies by `$schema` structure, not by YAML success; a
      file-looking string is content and yields the typed schema-structure error;
      external references remain parsed references for later resolution.
    - `try_from_markdown(&file_reference)`, `try_from_yaml`, `try_from_json_schema`
      (feature `file`): accept a raw path or a context-bound reference (one input
      trait, `Into`-style; exact type names are a planning detail, chosen here and
      logged). Raw path: capture the request context once at entry via the private
      `capture` module (decision 6), before resolution, imports, or any `.await`.
      Bound: reuse the supplied context, capture zero times. Read the input once and
      classify and construct from that single buffer. Result owns/shares its origins
      and dependency metadata and does not borrow the input wrapper.
    - Typed errors for missing file, missing schema-bearing structure, malformed
      definitions.
    - Async `resolve(uri)` supports existing non-network references and supplied
      sources only; HTTP(S) stays `RemoteUnsupported`.

- [ ] **6.3 Layering.**
    - `layer_under`, `layer_over`, and `as_base`/override aliases as
      `&mut self -> Result<&mut Self, _>`; sources borrowed unchanged; incoming
      lists low to high (last wins); receiver wins under `layer_under`, incoming
      wins under `layer_over`; whole-property replacement via the Phase 4.4 merge,
      keeping root-union/reference restrictions.
    - **Atomic per call:** stage into owned merge working data and commit with a
      single assignment on success; on `Err` the receiver definition and receiver-
      local cache identities are unchanged. Earlier chained successes remain.
      Reuse shared snapshots, no full-schema copy for rollback.
    - Successful definition change bumps a revision that invalidates affected
      prepared state; unchanged definitions keep it.

- [ ] **6.4 Export.**
    - Typed definitions to Draft 2020-12 JSON Schema via the existing lowerer
      (function type keeps its located error).
    - Full-schema simplified YAML export assembled from the existing type/property
      serializers: envelope forms, union-arm metadata, finite named-definition
      graphs without expanding recursion.
    - Raw JSON Schema to simplified export returns a typed unsupported error. No
      reverse converter.

### Wave 9 (after Wave 8)

- [ ] **6.5 Facade tests and staging measurement.**
    - Schema-only example `simplified-schema/lib/examples/validate_person.rs`
      (spec `Person` schema; multiple `serde_json::Value` inputs through one
      object; AST inspected before any validation) — doubles as the criterion 1
      graph target.
    - Layering: spec table (`timeout`), whole-property replacement, atomic failure
      mid-list (valid first layer, unsupported-merge later layer, then validation
      still uses prior state), chain keeps earlier successes, source schemas
      unchanged, returned reference aliases the receiver (`std::ptr::eq`).
    - Factories: criterion 11 and 12 cases (YAML vs Markdown equivalence, body
      text ignored, external references retained with zero I/O).
    - Context instrumentation (criterion 15): a test-only counter on the `capture`
      module asserting raw path = 1, bound = 0, imports and validation = 0 more,
      and CWD/home/env changes after capture do not retarget; source-relative
      imports; Windows paths and CRLF. Counter is `cfg(test)`/a private test seam,
      not a public API.
    - Round trip (criterion 16): parse -> export -> parse equivalence including
      union arms, functions, and finite recursive graphs.
    - Measure layering staging cost (clone vs. staging) and record in the log; act
      only if material per the spec.

### Phase 6 checkpoint

- [ ] Criteria 1, 10, 11, 12, 13, 15, 16 hold in `simplified-schema/`.
- [ ] Both `--no-default-features` and default builds compile and pass their
      applicable test groups (criterion 14, part 1).

## Phase 7: Darkmatter, DMLS, and Consumer Migration

### Wave 10 (sequential: Darkmatter first)

- [ ] **7.1 Darkmatter adapters.**
    - `DarkmatterSchemas`/`EffectiveSchema` now hold the library `SimplifiedSchema`.
      Frontmatter origin tracking, document-relative diagnostic projection,
      pending classification, literal-token handling, lifecycle operand binding,
      default/optional/eager-file application all stay in Darkmatter and call the
      library; applying a soft-converted value to a document remains Darkmatter's
      choice.
    - Baseline: keep the process-wide shared baseline of resolved typed
      definitions and shared lowered JSON Schema; effective schema is built by
      `effective.layer_under([&baseline, …extension_layers, …matched_triggers])`
      preserving low-to-high order, custom/disabled baselines, and the
      no-authored-`$schema` distinction (absence is a typed error from the
      constructor; Darkmatter preserves existing absence handling).
    - Compiled-validator reuse and context-sensitive cache identity unchanged
      (assert with existing tests plus a new reuse assertion).

- [ ] **7.2 Darkmatter CLI and `md schema`.**
    - Update `darkmatter/cli/src` imports (14 files); `md schema` commands and
      output unchanged (existing CLI tests are the evidence).

### Wave 11 (parallel: independent packages after 7.1)

- [ ] **7.3 DMLS.**
    - Update 20 `dmls/src` files; DMLS builds the schema from the current in-memory
      document snapshot (unsaved edits) with no path-only factory and no filesystem
      reread; searched-but-absent candidate invalidation (nearer candidate appears)
      preserved; diagnostics and completion parity tests green.

- [ ] **7.4 Claudine.**
    - `claudine/gen/src/schema_compat.rs` and other schema-only operations import
      `simplified-schema` directly; `claudine/cli` completion
      (`schema_completion/keys.rs`, tests) and `claudine/lib` launch/completion keep
      using Darkmatter where Markdown/triggers/composition are needed. Update
      `claudine/*/Cargo.toml` `source-inputs` lists if moved files appear there.

- [ ] **7.5 Messenger.**
    - Research loader (`messenger/lib/src/research/load.rs`) validates frontmatter
      and keeps deserializing the **original** frontmatter (not the soft result);
      imports updated.

### Wave 12

- [ ] **7.6 Remove migration wrappers.**
    - Delete every `migration: removed in Phase 7` re-export; fix remaining
      imports to the owning crate. Darkmatter keeps only document-, trigger-, and
      composition-aware adapters. Confirm with `rg` that no repo code imports
      `darkmatter::markdown::schemas::simplified`.

### Phase 7 checkpoint

- [ ] `just test`, `just lint` green in `darkmatter/`, `darkmatter/dmls`,
      `darkmatter/cli`, `claudine/`, `messenger/`.
- [ ] Criterion 4 evidence assembled (baseline/layer parity cases: absent
      schema, custom/disabled baseline, editor extensions, matched triggers,
      unsaved DMLS buffers).

## Phase 8: Evidence, Guards, and Cross-OS

### Wave 13 (parallel)

- [ ] **8.1 Corpus checks (criterion 3).**
    - Passive corpus test in the library over shipped schema catalogs, partials,
      triggers (as supplied sources, not discovered), and representative consumer
      sidecars; plus one Darkmatter-side invocation test proving integration.

- [ ] **8.2 Source guards (criterion 6).**
    - Update existing guards for relocated implementations rather than weakening
      them (`darkmatter/lib/tests/l1/dependency_direction_guard.rs`,
      `type_scope_ownership_guard.rs`, `glob_implementation_guard.rs`,
      `path_lookup_guard.rs`, and the CLI `source_scan.rs` users): new ownership
      assertions: no `simplified-schema` dependency on Darkmatter, Claudine, DMLS,
      CLIs, or content-policy runtime; no second parser/converter/subtype/numeric
      in Darkmatter; no `capture` outside the one sanctioned module.

- [ ] **8.3 Dependency graph (criteria 1, 14).**
    - Record `cargo tree -e normal` for the example in the default and
      `--no-default-features` configurations in the log: no Darkmatter/rendering/
      effect stack; no `gix`, `walkdir`, `globset` in the minimal build.
    - Run both configurations' test groups.

- [ ] **8.4 Matrix smell greps.** Run the Input Robustness smell greps on the moved
      loaders and log hits (pinned, not fixed).

### Wave 14 (after Wave 13)

- [ ] **8.5 Benchmarks (criterion 7).**
    - Re-run the Phase 1.2 commands on the same host, toolchain, and profile;
      record relative change per case and its cause in the log. Keep cache-identity,
      shared-compilation, and validator-reuse assertions. Regressions are recorded
      with cause; they do not block this plan (no threshold, no CI gate); the
      author reviews them.

- [ ] **8.6 Cross-OS evidence (criterion 8).**
    - Load the `os` skill first. Use `just ci-local --plan` to see which cells
      execute; reuse qualifying evidence; run `simplified-schema` and the changed
      adapters on macOS (this host) and gather Linux/Windows/WSL2 evidence through
      the rigs the skill names (`just ci-seed` for the nightly cells) before
      reporting. Windows-path and CRLF cases are named in the log. Do not claim an
      OS untestable without consulting the skill.

### Phase 8 checkpoint

- [ ] All sixteen criteria have a recorded test or measurement in
      `implementation-log.md`.

## Phase 9: Documentation, Skills, and Closeout

### Wave 15 (parallel: disjoint files)

- [ ] **9.1 Library docs.**
    - `simplified-schema/docs/`: schema authoring, in-memory use, supplied sources,
      conversion, diagnostics (typed errors and provenance), custom vocabulary
      (`darkmatter-*`, `x-darkmatter-*`), host-dependent capabilities, layering,
      strict vs soft, file feature and context capture (Mermaid for the validation
      flow and the capture boundary). Audience: a developer with no repo
      experience. Move the language reference out of
      `darkmatter/docs/topics/schemas/definition.md` and related pages to
      the new `docs/` as the current authority; Darkmatter topic pages explain only
      integration and link to the language reference. No doc names this spec or any
      `features/` directory.
    - `simplified-schema/README.md` is an overview only.

- [ ] **9.2 Dependency records and drift.**
    - Update `docs/dependencies.md`, `darkmatter/docs/dependencies.md`,
      `biscuit-file/docs/dependencies.md`, and add `simplified-schema/docs/dependencies.md`.
    - Doc-drift pass over `darkmatter/docs/` schema pages, comments of every moved
      symbol (Code Comment Quality rules), and root `CLAUDE.md` if layout
      conventions changed (`areas`).

- [ ] **9.3 Skills.**
    - Add `.claude/skills/simplified-schema/SKILL.md` (compact, links to details);
      update `.claude/skills/darkmatter/schema.md` and `library-surfaces.md`,
      `.claude/skills/biscuit-file/` (feature split, frontmatter helper), and
      `rust-testing`/`os` only if a new fact was learned the hard way.

### Wave 16

- [ ] **9.4 Final validation.**
    - `just test` and `just lint` in every touched area; `just test-l2` where L2
      files changed; review `git status` for stray files; confirm `implementation-log.md`
      is complete; no `_completed` move and no commit unless instructed.

### Final checkpoint

- [ ] All Definition of Done items hold; status is "implementation complete, ready
      for review".

## Risks and Mitigations

| Risk | Mitigation |
| --- | --- |
| `mod.rs` (4.6k) and `resolve.rs` (6.3k) mix host and language code | Task 1.1 inventories symbol by symbol; Phase 4 splits before moving |
| Engine reaches pending/literal-token facts | Decision 4: Darkmatter classifies first; Phase 5 parity tests catch differences |
| Type rename `SimplifiedSchema` to `SchemaRoot` ripples widely | One mechanical rename in Task 4.1 behind the re-export |
| `biscuit-file` split changes a public path | Re-export tests assert type identity; default build unchanged |
| Large moves break source-reading guards and CI scope | Task 8.2 updates guards; `source-inputs` lists kept current |
| Perf regression from interposed API | Phase 1.2 baseline and Phase 8.5 comparison with causes |
