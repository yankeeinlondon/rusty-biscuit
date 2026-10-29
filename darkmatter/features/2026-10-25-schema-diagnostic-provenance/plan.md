---
kind: plan
created: 2026-10-26
total_phases: 4
phase: 1
agent: codex/gpt-6.1-sol
yolo: true
human_review: false
source_files_during_phase_1:
    - darkmatter/lib/src/markdown/schemas/simplified/source.rs
    - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/errors.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/dmls/src/diagnostics/named_types.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/lib/tests/l1/recursive_schema_types.rs
    - darkmatter/lib/tests/l1/schemas_core_type_grammar.rs
    - darkmatter/lib/tests/l1/schemas_named_type_merge.rs
    - darkmatter/lib/tests/l1/schemas_union_postfix.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
    - .claude/skills/darkmatter/schema.md
packages:
    - darkmatter
    - dmls
    - darkmatter-cli
source_files_during_phase_2:
    - darkmatter/lib/src/markdown/schemas/simplified/yaml_scalar.rs
    - darkmatter/lib/src/markdown/schemas/simplified/serialize.rs
    - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
    - darkmatter/lib/src/markdown/schemas/simplified/grammar.rs
    - darkmatter/lib/src/markdown/schemas/simplified/signature.rs
    - darkmatter/lib/src/markdown/schemas/simplified/source.rs
    - darkmatter/lib/src/markdown/schemas/simplified/standalone.rs
    - darkmatter/lib/src/markdown/schemas/errors.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/tests/l1/schemas_named_type_merge.rs
    - darkmatter/lib/tests/l1/schemas_source_projection.rs
    - darkmatter/cli/tests/l1/schema_validate.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/darkmatter/schema.md
source_files_during_phase_3:
    - darkmatter/lib/src/markdown/schemas/simplified/cursor.rs
    - darkmatter/lib/src/markdown/schemas/simplified/yaml_scalar.rs
    - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/diagnostics/named_types.rs
    - darkmatter/dmls/src/overlay/named_types.rs
    - darkmatter/dmls/tests/l1/schema_yaml_spellings.rs
    - darkmatter/dmls/tests/l1/core_schema_types.rs
    - darkmatter/dmls/tests/l1/named_type_references.rs
    - darkmatter/dmls/tests/l1/lsp_session.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/schema.md
source_files_during_phase_4:
    - darkmatter/lib/tests/l1/shipped_schema_corpus.rs
    - darkmatter/cli/tests/l1/schema_validate.rs
docs_updated_during_phase_4:
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/dmls/docs/diagnostics.md
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/darkmatter/schema.md
    - .claude/skills/darkmatter/dmls.md
source_code:
    - darkmatter/lib/src/markdown/schemas/simplified/source.rs
    - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/errors.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/dmls/src/diagnostics/named_types.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/lib/tests/l1/recursive_schema_types.rs
    - darkmatter/lib/tests/l1/schemas_core_type_grammar.rs
    - darkmatter/lib/tests/l1/schemas_named_type_merge.rs
    - darkmatter/lib/tests/l1/schemas_union_postfix.rs
    - darkmatter/lib/src/markdown/schemas/simplified/yaml_scalar.rs
    - darkmatter/lib/src/markdown/schemas/simplified/serialize.rs
    - darkmatter/lib/src/markdown/schemas/simplified/grammar.rs
    - darkmatter/lib/src/markdown/schemas/simplified/signature.rs
    - darkmatter/lib/src/markdown/schemas/simplified/standalone.rs
    - darkmatter/lib/tests/l1/schemas_source_projection.rs
    - darkmatter/cli/tests/l1/schema_validate.rs
    - darkmatter/lib/src/markdown/schemas/simplified/cursor.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/overlay/named_types.rs
    - darkmatter/dmls/tests/l1/schema_yaml_spellings.rs
    - darkmatter/dmls/tests/l1/core_schema_types.rs
    - darkmatter/dmls/tests/l1/named_type_references.rs
    - darkmatter/dmls/tests/l1/lsp_session.rs
    - darkmatter/lib/tests/l1/shipped_schema_corpus.rs
documentation:
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/dmls/docs/diagnostics.md
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
    - .claude/skills/darkmatter/schema.md
    - .claude/skills/darkmatter/dmls.md
completed_phase: 4
implemented: true
---
---
---

# Schema diagnostic provenance implementation plan

## Phase 1 — Establish shared provenance contracts

Deliver three connected changes: retain the typed values participating in constraint
conflicts, retain the structural path and authored location of rejected schema
definitions, and use the library's plain-scalar decoder for incomplete continuation
completion. Success means the library returns trustworthy provenance without
changing merge or schema acceptance rules; terminal and editor diagnostics consume
that provenance consistently; and complete and incomplete anchored continuations
produce matching completions and edits. All seven specification acceptance groups
must pass, including Claudine forwarding and the requested terminal regression gate.

The implementation stays in this worktree. Paths below are repository-relative.
Decisions, evidence, departures, and completed checkpoints are recorded in
`implementation-log.md` beside this plan; the specification is not edited to
describe departures.
Waves are globally numbered dependency steps; tasks within a wave may run
concurrently only when their listed ownership does not overlap. A later wave starts
after the preceding checkpoint. Each implementation task includes its focused
tests and a pass over comments describing changed behavior. Shared export files and
consolidated test registration belong to the integration task named in that wave.

### Decisions

1. **Typed conflict participants**
   - **Decision:** Change `ConstraintConflict.constraints` to
     `Vec<SchemaConstraintParticipant>`, pairing unchanged origins with full typed
     constraints in existing inherited-first order. Migrate all constructors and
     consumers together, including recursive merges and item/container defaults.
   - **Alternatives considered:** Add values to every origin; reread source at display time.
   - **Why:** Origins also serve value-free locations; the resolver already has the
     actual values, including effective values absent from authored text.
   - **Downsides:** This intentionally breaks a public variant's field type.

2. **Effective merge values**
   - **Decision:** Capture schemes before the failing intersection; label every
     scheme participant as effective in DMLS. Preserve strictest-bound origins,
     first origins for equal bounds, numeric default equality, and synthetic
     declaration origins for empty alphanumeric options.
   - **Alternatives considered:** Report original schemes; retain complete merge
     history; omit participants without option tokens.
   - **Why:** These are the specification's settled provenance semantics and explain
     the actual conflict without changing merge behavior.
   - **Downsides:** Effective scheme text may differ from the declaration at its origin.

3. **Canonical constraint messages**
   - **Decision:** Expose `Constraint::to_schema_text(Option<SimplifiedType>)` through
     the existing serializer, and one shared diagnostic-excerpt helper used by
     Display, StatusBlock, and DMLS. Conflict renderers pass `None`. Escape controls
     before truncating to 160 Unicode scalar values, including the final ellipsis;
     terminal markup escaping is applied separately by Prose.
   - **Alternatives considered:** Renderer-specific formatting; unbounded output;
     new authored syntax for compound defaults.
   - **Why:** The serializer is the existing canonical authority. Options need a
     non-panicking path, and defaults must preserve string/number/null identity.
   - **Downsides:** Truncated excerpts and defensive compact-JSON compound defaults
     do not promise a schema-grammar round trip; full values remain in typed data.

4. **One semantic parser**
   - **Decision:** Thread a structural `SchemaSourcePath` through existing semantic
     recursion and cross-arm checks. Keep the public source-free APIs returning
     their original `SchemaError` by discarding internal location metadata. Use
     `LocatedDefinitionFailure` for the two source-aware property/declaration APIs.
   - **Alternatives considered:** Split display property labels into paths;
     reparse candidate scalars; retain parallel old and detailed public APIs.
   - **Why:** Literal keys and indexed arms require structural identity; a second
     suggestion can fail only in the context of earlier, individually valid arms.
   - **Downsides:** Recursive parser plumbing and source-aware callers change together.

5. **Conservative failure projection**
   - **Decision:** Project only verified, aligned nodes whose decoded scalar equals
     the semantic input. Return the semantic error even if projection fails, with
     token span, then rejected-value span, then caller fallback. Apply source offsets
     once; ranges always belong to the supplied document. Preserve successful
     source maps and wrap projection-only errors in the new failure result.
   - **Alternatives considered:** Guess another token; reject unsupported source
     presentations; expand the strict locator grammar.
   - **Why:** Better locations must not change validity, existing fallback, or
     referenced-file ownership. Existing code supports block scalars despite a stale
     source-module header; correct the header to describe that implemented behavior.
   - **Downsides:** Unsupported presentations can retain a whole-value or document underline.

6. **Shared standalone recognition**
   - **Decision:** Share envelope recognition and admitted-YAML loading with the
     existing classifier. The new standalone helper parses YAML once, the semantic
     payload once, and locates at most once on definition failure. Return the first
     inner definition failure; return `None` for valid, unrecognized, invalid-YAML,
     or outer envelope/declaration-shape cases. The classifier retains its existing
     `SchemaDocument` wrapping and global-catalog distinction.
   - **Alternatives considered:** A separate editor detector; exposing all outer
     errors as located definition failures.
   - **Why:** Recognition and numeric admission already have one authority; existing
     DMLS outer diagnostics must keep their classification and fallback.
   - **Downsides:** DMLS still needs its outer diagnostic route when this helper declines.

7. **Plain continuation boundary**
   - **Decision:** Add `decode_plain_scalar_node` using the existing node-property
     and plain decoding implementation. Keep DMLS's backward indentation scan,
     decode from the holder's actual value start across the current document, and
     require the actual cursor within the consumed range, including its final
     insertion position. Use the decoded map for incomplete-buffer edits.
   - **Alternatives considered:** Accept any general scalar decode; extend the
     prefix blacklist; implement a full partial-YAML cursor locator.
   - **Why:** Same-line tags and anchors are shared scalar behavior, while quoted,
     block, alias, collection, and later-line node-property cases cannot qualify.
   - **Downsides:** DMLS retains indentation logic; the library gains a narrow public helper.

8. **Verification boundaries**
   - **Decision:** Test exhaustive semantics and presentation locally at public
     library results; use representative hermetic L1 protocol and CLI sessions for
     wiring, plus the existing no-focus terminal L2 gate. No performance spike,
     benchmark, new dependency, CI environment, or schema-wide cursor API is needed.
   - **Alternatives considered:** Multiply every case across subprocesses/import
     layouts; measure repeated parsing; classify protocol sessions as L2.
   - **Why:** The specification promises provenance, not latency; its design and
     existing code answer the implementation questions without an experimental spike.
   - **Downsides:** Validation establishes correctness, not a measured speedup or
     evidence from operating systems on which no gate was run.

### Wave 1 — Contract integration

- [x] **Define provenance contracts**
  - Own `simplified/source.rs`, `simplified/mod.rs`, `schemas/mod.rs`, and the
    contract-facing changes in `errors.rs`. Introduce the participant and located
    failure types and document their field semantics and UTF-8 byte coordinates.
  - Inventory every constructor, match, source-aware caller, and re-export with
    repository searches, including DMLS, recursive resolution, tests, and Claudine.
    Record migration ownership in the implementation log; do not change source-free
    result signatures or add a second parser surface.
  - Establish the internal failure carrier and path-prefix conventions used by
    Phase 2. Keep this foundation buildable with mechanical adaptations where
    needed; later tasks supply the final semantic behavior, not placeholder values.
  - Checkpoint: the public shapes and call-site inventory are concrete, no unknown
    design choices remain, and the affected packages compile through focused checks.

## Phase 2 — Implement library provenance and scalar decoding

### Wave 2 — Independent value and scalar work

- [x] **Retain conflict values**
  - Own `schemas/resolve.rs`, `schemas/resolve/recursive.rs` as needed,
    `simplified/serialize.rs`, constraint methods, and conflict rendering in
    `schemas/errors.rs`; include resolver and serializer tests.
  - Replace origin-only conflict construction in bounds, schemes, defaults,
    options, and item/container default reconciliation. Snapshot inherited schemes
    before mutation; preserve stream order, tightening, equality, and origin rules.
  - Implement full single-constraint serialization and the common excerpt helper.
    Options render as canonical `alphanumeric(...)` or bare `alphanumeric`.
    Share quoting fixes with normal serialization; distinguish numeric-looking and
    null-looking strings from typed scalars. Compound Rust defaults render compact JSON.
  - Test all four conflict kinds through two-import fixtures, asserting values,
    source paths, occurrences, spans, merge order, and Display. Extend the existing
    numeric/length/item-count/key-count merge table, stricter/equal-bound cases,
    immediate and post-narrowing scheme conflicts, empty/different option sets,
    normalized numeric defaults, and item/container reconciliation. Retain recursive tests.
  - Compare Display and StatusBlock excerpts for controls, terminal markup, long
    Unicode, options, and compound defaults; assert the full structured value survives.

- [x] **Share plain decoding**
  - Concurrent with conflict work; own `simplified/yaml_scalar.rs` and its tests.
    Implement `decode_plain_scalar_node` by sharing node-property skipping and
    plain decoding, returning the existing decoded map and consumed-end shape.
  - Test same-line anchors/tags in either supported order, plain continuation,
    LF/CRLF, blank folding, comment termination, dedent, and sequence indentation.
    Reject quoted/block scalars, collections, aliases, comments, empty holders,
    and properties whose scalar starts on a later line.
  - Checkpoint for Wave 2: focused resolver/serializer/scalar tests pass; the new
    helper agrees with the general decoder on eligible plain values.

### Wave 3 — Located semantic failures

- [x] **Thread failure paths**
  - Own `simplified/mod.rs` and `simplified/grammar.rs`. Carry structural paths
    through mappings, property unions, root unions, nested objects, and
    `$constraints`, while preserving parser traversal and source-free error text.
  - Preserve which arm introduced a cross-arm failure, including the second
    suggestion and duplicate-overload checks. Do not infer paths from display
    labels or reparse arms to reproduce errors.
  - Test nested literal keys containing dots, slashes, brackets, and Unicode,
    indexed mappings, same-prefix second members, and two individually valid
    suggestion arms. Assert path and original semantic error through public APIs
    once the Wave 4 projection is integrated.
  - Checkpoint: source-free semantic regression tests pass and the internal
    failure has the actual rejected node for token and structural failures.

### Wave 4 — Projection and library integration

- [x] **Project definition failures**
  - Own `simplified/source.rs`, `simplified/standalone.rs`, both schema export
    modules, and library source-projection test registration. Depends on Waves 2–3.
  - Implement `LocatedDefinitionFailure` results for property and declaration
    source-aware parsing; migrate every caller identified in Wave 1. Export the
    plain decoder, participant, located failure, and standalone locator.
  - Follow the rejected path in one verified located tree. Project grammar spans
    through the shared scalar decoder, offset exactly once, and keep a trustworthy
    whole-value range when token mapping is unavailable. Wrap projection-only
    failures without changing successful values or source maps.
  - Extract shared standalone recognition/loading rather than copying it. Implement
    `locate_standalone_definition_failure` for both envelopes and pure root-union
    mapping arms; prefix paths with `types` or `$schema`. Preserve classifier
    wrapping, numeric admission, category rules, and global-catalog rejection.
  - Extend `schemas_source_projection.rs`, `schema_yaml_spellings.rs`, and relevant
    inline tests for plain, continued, quoted, and block scalars in both envelopes;
    declaration-level inline frontmatter; nonzero offsets; structural empty unions;
    unsupported multiline-quoted siblings; aliases; and semantic/source mismatch.
    Assert semantic errors survive even when both ranges are unavailable.
  - Checkpoint: public library results prove exact paths and authored bytes,
    successful source-map regressions pass, and source-free errors remain unchanged.

### Input robustness coverage

The Markdown/type-expression text grammar is outside the configuration matrix.
The touched YAML envelope loader and YAML-value dispatch still require shape
regressions because recognition and semantic recursion are being shared. Test
each shared validator once through the crate's public results; reuse existing
coverage instead of creating a Cartesian product of fields and formats.

Default outcome: invalid recognized schema input returns an error through the
existing public classifier, with the best trustworthy location available through
the source-aware route; it executes nothing and publishes no successful document.
Outer loader errors retain their existing location/fallback rather than fabricated
definition ranges. The standalone definition helper returning `None` for these
outer failures never suppresses the classifier/DMLS outer diagnostic.

Exceptions preserving current behavior:

- Recognition: missing or non-`schema` `kind` does not claim a tagged document;
  a sole `$schema` key still claims a pure document. Missing envelopes, nonmapping
  roots, and unrecognized ordinary documents remain unrecognized. Extra pure-root
  keys do not claim a pure envelope. A global catalog is never a document schema.
- Tagged `types` must be present and a mapping; `{}` remains an explicit empty
  schema. Pure `$schema` accepts a mapping, nonempty root union of mappings/local
  reference strings, or a valid whole-file local reference. Empty maps are allowed;
  null, wrong scalar types, and empty root unions are errors.
- Property definitions accept YAML null (including `entry:`) as the null type,
  mapping objects including `{}`, strings containing valid expressions, and
  nonempty unions of strings/mappings/null. Null union members are data; numeric,
  Boolean, and nested-sequence members are errors, never filtered out.
- Optional `categories` absence leaves names unrestricted; `[]` explicitly allows
  none. Preserve the existing category validator and its error behavior, including
  present null, wrong whole-field types, wrong members, and duplicate/name rules.

- [x] **Verify input boundaries**
  - Complete the touched-validator matrix in existing loader/projection tests:
    absent, explicit null, wrong whole type, mixed valid/invalid elements, all-invalid
    elements, and empty values wherever that validator's accepted type admits them.
    A map entry's absence means no property declaration; do not invent an absent
    value argument for APIs requiring a supplied value.
  - For each distinct loading branch actually reused, retain a real shipped-schema
    control and single-edit cases for duplicate keys, trailing invalid content,
    second YAML documents, and YAML-specific null syntax. Assert the classifier's
    public rejection/classification and located helper/fallback behavior; JSON-shaped
    YAML uses the same loader and needs no duplicate matrix.
  - Search touched loading/dispatch code for `#[serde(default)]`, conflating
    `Option<T>`, `filter_map` string extraction, `unwrap_or_default()`, and swallowed
    `.ok()` parsing. Check their actual effect; the classifier's existing unsupported
    key length check must continue rejecting non-string keys despite string filtering.
  - Each field using a shared validator needs a control and one invalid wiring
    case. Any matrix finding records every other applicable shape in that validator
    row and loader column before closure. Checkpoint: no accepted input is silently
    dropped, and valid controls prove every mutation is load-bearing.

## Phase 3 — Consume provenance in DMLS

### Wave 5 — Independent diagnostic and completion integration

- [x] **Publish typed diagnostics**
  - Own `dmls/src/diagnostics/frontmatter.rs`, `diagnostics/named_types.rs`,
    `overlay/named_types.rs`, `overlay/mod.rs`, and their diagnostic tests.
  - Replace standalone scalar search with the library helper; delete
    `grammar_token_span` and `collect_string_scalars`. Select authored token,
    rejected value, then existing enclosing definition/document fallback.
  - Route inline `$schema` through the source-aware declaration parser. Adapt
    existing `schema` and `type-definition` meta-type consumers without weakening
    union acceptance. Preserve codes, severity, semantic wording, related information,
    and local-reference primary diagnostics for imported failures.
  - Read participant origins for related locations and watched files, including
    wrapped failures. Share the library excerpt; use effective wording for schemes
    and the declaration span for implicit empty options with no token.
  - Add representative L1 LSP sessions for values and related locations, narrowed
    schemes, bare options, wrapped dependency watching, standalone token failure,
    inline nested mapping/sequence errors, and referenced-file ownership. Cover
    LF/CRLF, non-ASCII prefixes, and negotiated UTF-8/UTF-16 at publication; retain
    the existing outer-shape and meta-type union tests.

- [x] **Complete anchored continuations**
  - Concurrent with diagnostics; own `dmls/src/providers/frontmatter.rs` and
    separate completion tests. Coordinate any shared `schema_yaml_spellings.rs`
    registration or fixture edits through the diagnostics task to avoid collisions.
  - Pass the actual cursor into holder selection, remove the forbidden-prefix
    list, and call the plain decoder at the candidate's value start using actual
    holder indentation (the dash column for sequence items).
  - Carry the decoded map into expression cursor/replacement projection when no
    usable semantic tree exists. Keep anchors/tags outside edited text and reject
    cursors outside the consumed scalar, allowing insertion at its end.
  - Pair loaded/incomplete anchored and tagged cases for identical candidates and
    authored edits. Test comments, quoted nonholders, blank folding, dedented
    siblings, sequence holders, LF/CRLF, and negotiated positions. Assert passive
    recognition does not load imports or execute values.
  - Checkpoint for Wave 5: focused LSP diagnostic/completion sessions pass,
    guess-by-message helpers and the editor prefix blacklist are absent, and
    participant origin files remain watched.

## Phase 4 — Verify compatibility and publish current documentation

### Wave 6 — Independent documentation and boundary regressions

- [x] **Document public behavior**
  - Own `darkmatter/docs/topics/schemas/definition.md`, including the conflict
    table and definitions/use-sites text; document participant values, effective
    schemes, empty options, bounded excerpts, source-aware return types, byte
    offsets, and conservative fallback with compact examples.
  - Review `darkmatter/docs/errors/README.md` and update any affected catalog
    entry/example. Update `darkmatter/dmls/docs/diagnostics.md` for token ranges
    and related values, and `darkmatter/docs/topics/schemas/dmls-schema-support.md`
    for anchored partial-input completion.
  - Update `.claude/skills/darkmatter/schema.md` and relevant scalar/DMLS guidance
    in the existing skill topic files. Review the README overview for any changed
    public claim and correct it if present; keep detailed references in `docs/`.
    Docs and skills must not link to a feature lifecycle path.

- [x] **Verify shipped consumers**
  - Concurrent with documentation; own corpus, CLI, and Claudine regression tests.
    Extend the passive shipped-schema corpus and retain a normal CLI invocation
    using a shipped artifact through `CliProcessFixture`.
  - At the CLI boundary use the shared-loader valid control and one or two invalid
    cases to prove rendered error location and absence of execution/output
    publication; do not duplicate the public library matrix in spawned processes.
  - Retain source-free variant/message checks, successful source-aware value/map
    checks, and Claudine forwarding coverage. Register new tests in consolidated
    binaries and spell repository reads in CI-indexable forms.
  - Checkpoint: specification acceptance groups 1–6 map to passing assertions;
    documentation describes actual final results rather than planned behavior.

### Wave 7 — Final package gates

- [x] **Run package gates**
  - After Wave 6, run `just test` and `just lint` in `darkmatter/`, `just test`
    in `claudine/`, and `just test-l2` in `darkmatter/`. Use nextest via these
    recipes; keep hermetic DMLS protocol tests in L1 and every L1 test below five
    seconds. Terminal harnesses must never gain focus.
  - Run focused tests while implementing; run the combined final suites once after
    integration, repeating only for changes or unresolved failures. Do not run
    workspace-wide gates, benchmarks, or `cargo fmt`; lint's formatting check is
    a check, not authorization to rewrite formatting.
  - Review the final diff for unintended schema/merge changes, effectful passive
    APIs, stale comments, guessed ranges, duplicated formatting, and omitted public
    consumers. Preserve portability across macOS, Linux, Windows, and WSL2 without
    introducing platform-specific paths or changing CI scope.
  - Record actual gate results and any environment limitation in the implementation
    log, distinguishing passing evidence from tests not run. Close the acceptance
    checklist against all seven specification groups. Leave the feature in its
    active directory with the terminal state **implementation complete, ready for
    review**; do not commit or move it to `_completed`.
