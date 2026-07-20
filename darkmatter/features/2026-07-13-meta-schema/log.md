---
feature: 2026-07-13-meta-schema
description: "Implementation log for the meta-schema feature's review-to-implement cycles"
deferred_perf_measurement: false
implementation_2: "2026-07-18T09:03:32-07:00"
implementation_3: "2026-07-19T21:33:53-07:00"
implementation_4: "2026-07-19T23:27:33-07:00"
---

# Meta Schema — Implementation Log

## Implementation of Review Findings #3

> **started at:** 2026-07-19T21:33:53-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'darkmatter/features/2026-07-13-meta-schema/review-3.md'
- this is iteration 3 of the review-to-implement cycle
- a prior attempt on this same iteration was interrupted mid-Finding-1; its partial
  library edits are present in the working tree and are carried forward rather than
  reverted (see the resumed-state notes below)
- the review contains **3** findings:
        - **High** — Standalone schema documents bypass declaration-level reference validation
        - **High** — DMLS still reconstructs semantic paths instead of consuming structural paths
        - **Medium** — The canonical Level-2 release gate is still red (unrelated CLI rendering tests)
- impacted package area (per the spec's implementation surface map): `darkmatter`,
  covering the `darkmatter` lib, `darkmatter-cli`, and `dmls` crates — so
  `just test` / `just lint` from that area is the verification scope

### Resumed state carried forward from the interrupted attempt

- `SchemaDeclaration` gained `PartialEq`; `SchemaReference` got a hand-written
  `PartialEq` keyed on `(kind, file_reference.raw())` because resolution-time
  `FileReference` state is not part of a freshly classified reference's identity
- `parse_standalone_schema_payload_with_source` now returns
  `SourceAware<SchemaDeclaration>` via `parse_schema_declaration` instead of
  `SourceAware<SimplifiedSchema>` via `parse_yaml_schema`
- `StandaloneSchemaDocument.schema` (field) became `.declaration` with a
  `.schema()` accessor returning `Option<&SimplifiedSchema>`
- the pure-payload mapping/sequence carrier guard was removed so a scalar
  reference reaches the declaration parser
- downstream `dmls` consumers were **not** yet migrated when the attempt was
  interrupted, so the area is expected to be red on entry

### Prior (interrupted) attempt log

> **started at:** 2026-07-19T21:12:27-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'darkmatter/features/2026-07-13-meta-schema/review-3.md'
- this is iteration 3 of the review-to-implement cycle
- the review contains **3** findings:
        - **High** — Standalone schema documents bypass declaration-level reference validation
        - **High** — DMLS still reconstructs semantic paths instead of consuming structural paths
        - **Medium** — The canonical Level-2 release gate is still red (unrelated CLI rendering tests)
- impacted package area (per the spec's implementation surface map): `darkmatter`, covering the `darkmatter` lib, `darkmatter-cli`, and `dmls` crates — so `just test` / `just lint` from that area is the verification scope
- findings are implemented serially:
        1. High — standalone declaration-parser parity
        2. High — DMLS structural path consumption
        3. Medium — canonical L2 gate (repair or formally scope AC13)

### Finding 1 (High) — Standalone declaration-parser parity

- starting the work on 'standalone-declaration-parser-parity' at 21:16:19-07:00
- resumed the work on 'standalone-declaration-parser-parity' at 21:35:12-07:00
        - carried forward the interrupted attempt's edits to `reference.rs`, `simplified/mod.rs`, `simplified/source.rs`, and `simplified/standalone.rs` rather than reverting them, and completed the `.schema` field → `.declaration` / `.schema()` migration across every downstream consumer
        - ran GitNexus upstream impact analysis before editing; all three key symbols returned **HIGH** risk — `parse_standalone_schema_document` (55 impacted, 5 direct, across Overlay/Diagnostics/Schemas/Providers), `StandaloneSchemaDocument` (35 impacted), `parse_standalone_schema_payload_with_source` (35 impacted). No CRITICAL, and no execution flows were reported affected
        - discovered the compile breaks extended beyond the two known `resolve.rs` sites to five call sites total: `resolve.rs:382`/`:924`, `dmls/src/providers/frontmatter.rs:394`/`:1025`, plus unit tests in `dmls/src/overlay/schema.rs` and `dmls/src/overlay/mod.rs` and integration test `lib/tests/suggest_constraint_phase4.rs`
        - decided `resolve.rs:382` (`parse_yaml_referenced_file`) routes a whole-file reference through the existing `resolve_reference`, matching how root-union `FileRef` arms already resolve, so the target file becomes the origin and the dependency edge
        - decided `resolve.rs:924` (`load_named_types`) returns an explicit `SchemaDocument` error ("whole-file reference schema documents cannot supply named imports") rather than following the chain, since imports are defined only among a file's own top-level entries; this mirrors the sibling root-union error instead of papering over `None`
        - decided the two DMLS provider sites treat `schema() == None` as an active-but-empty document: a whole-file reference contributes no passive namespace names and no semantic type regions, and is deliberately **not** malformed
        - discovered `dmls/src/diagnostics/frontmatter.rs` (`standalone_declaration_diagnostic`) already routed scalar and sequence payloads through `parse_schema_declaration`, so the DMLS diagnostic layer was already correct — the inconsistency was solely in the library parse path, which is why `$schema: "   "` already produced `invalid_schema_shape` while `$schema: ["   "]` produced nothing
        - detected a retired-contract test: `suggest_constraint_phase4.rs::claimed_malformed_envelopes_are_schema_document_errors` asserted `$schema: string` is malformed, but under declaration-parser parity `string` is a valid bare-name reference — the same thing inline `$schema: myschema` means — so rejecting it would reintroduce the exact inline/standalone divergence this finding is about. Removed that case, added `pure_scalar_payloads_are_whole_file_references` documenting the deliberate contract change, and added `types: ./other.yaml` to the malformed list to pin tagged mapping-only enforcement
        - diagnosed `cargo fmt --check` as an unusable signal in this area: it reports drift in files never touched by this work (`dmls/src/bench.rs`, `dmls/src/capabilities.rs`), confirming the local-rustfmt-vs-`main` drift `CLAUDE.md` warns about. No write-mode formatting was run; new code was hand-matched to surrounding style and verified only against `just lint`
- work completed for 'standalone-declaration-parser-parity' at 22:06:07-07:00
        - added five Level-1 library parser tests to `lib/tests/meta_schema_phase6.rs` covering the valid scalar reference (asserting the exact `FileReference` span `9..21` and `PathQualified` kind), whitespace-only scalar and arm, remote scalar and arm, mixed valid/invalid union, tagged mapping-only enforcement, and a valid all-reference root union
        - added one Level-1 in-memory LSP test `standalone_reference_declarations_match_the_shared_declaration_parser` to `dmls/tests/lsp_session.rs` asserting exact LSP diagnostic ranges over the offending arm for the invalid cases and zero schema diagnostics for the two valid cases; every fixture target is a non-existent path or a remote URL, so a passing run is itself evidence that no I/O occurs
        - gates: `just build` **PASS**; `just test` **PASS** (darkmatter 5,920/5,920, darkmatter-cli 561/561, dmls 606/606, 214 skipped); `just lint` **PASS** (clippy `-D warnings` clean)
        - **surfaced a separate pre-existing defect (not fixed here).** A self-referential whole-file schema reference stack-overflows the process: `a.yaml` containing `$schema:\n  - ./a.yaml` aborts with `fatal runtime error: stack overflow` on the **existing** contract, independent of this change — `resolve_standalone_root_union` recurses through `resolve_reference` → `load_schema_from_path` with no cycle guard (`MAX_IMPORT_DEPTH` guards only named-type import expansion). This change makes the scalar shape `$schema: ./a.yaml` reach the same unguarded loop. Left unfixed under surgical-change discipline; it warrants its own finding, since a fix means threading a visited-set or depth bound through `load_schema_from_path` / `parse_yaml_referenced_file` / `resolve_standalone_schema` / `resolve_standalone_root_union` / `resolve_reference` — all HIGH-risk symbols

### Finding 2 (High) — DMLS structural path consumption

- starting the work on 'dmls-structural-path-consumption' at 22:07:41-07:00
        - recorded GitNexus upstream impact before editing: `frontmatter_authoring` is **HIGH** risk (40 impacted symbols, 1 direct caller, 3 modules — Overlay direct, Diagnostics and Providers indirect); `enclosing_path` LOW (27 impacted, 4 direct); `standalone_type_definition_diagnostic` LOW (3 impacted); `entry_at_offset` LOW (0 impacted). The HIGH-risk change is fully covered by the DMLS suite, which is green
        - added the missing structural accessors to `FrontmatterAst` (`dmls/src/overlay/frontmatter.rs`) rather than reconstructing paths in the providers: `entry_by_key_path`, `key_path_at`, `key_path`, `index_of`, `container_at_offset`, `enclosing_key_path`, `children_of_key_path`, `key_entry_on_line`, `child_entry`, plus a shared `pointer_for` RFC-6901 builder. `entry_by_dotted` was kept but documented as usable only for the library contracts that already hand DMLS a dotted string (`style:` warnings)
        - **defect 1 — semantic activation:** `overlay/schema.rs:frontmatter_authoring` now walks `ast.key_path_at(index)` instead of `entry.dotted.split('.')`, so a property named `build.target` resolves as one key and activates its declared `type-definition`/`schema` behavior. The `$schema` descendant skip became a structural path check rather than a `"/$schema/"` pointer-prefix string test
        - **defect 2 — completion:** `providers/frontmatter.rs` gained `value_cursor` (owner key + value partial taken from the authored entry, so `"build.target"` and `"host: port"` are not split at a raw `:`) and rewired `enclosing_path` onto `ast.container_at_offset`, so a quoted `"$schema"` ancestor now decodes to the reserved `$schema` path. The reverse line scan (`enclosing_path_by_indent`) survives only as the fallback for a cursor no still-placed AST entry describes
        - chose per-entry staleness verification over a blanket `overlay.stale` veto: `still_placed` re-reads the entry's key token at its recorded span and compares it to the decoded key, so a malformed edit elsewhere in the frontmatter block leaves untouched keys structurally owned instead of surrendering every structural answer at once. A span that has genuinely moved is rejected rather than trusted
        - **defect 3 — standalone inner-definition diagnostics:** `diagnostics/frontmatter.rs` replaced `format!("{payload_key}.{key}")` with `entry_by_key_path(&[payload_key, key])`. Two sibling dotted-format lookups in the same file (`$schema.{property}` prepare ranging, and the nested-property range in `semantic_problem_range`) were converted to `entry_by_key_path` / `child_entry` for the same reason
        - extended the fix past the three named sites to the same defect class, per the review's "use those products end-to-end" instruction: `present_child_keys` now uses structural parentage instead of a dotted-prefix strip; `expression_values`, `schema_file_targets`, and `schema_hover` take `key_path_at`/`key_path` instead of `dotted.split('.')`; the `ctx.*` hover discriminator is a path check instead of a `dotted.starts_with("ctx.")` test; and `graph/substrate.rs:inline_schema_file_keys` uses structural parentage instead of `dotted.starts_with("$schema.")`
- work completed for 'dmls-structural-path-consumption' at 22:39:12-07:00
        - added four Level-1 in-memory LSP regressions to `dmls/tests/lsp_session.rs` covering everything the review demanded: quoted `"$schema"` as an ancestor, keys containing `.`, `:`, `/`, and `~` (with `~0`/`~1` pointer round-trip), nested mappings, CRLF (each completion case runs under both LF and CRLF), and a malformed current buffer retaining last-good semantic ownership of untouched keys
        - verified the new tests are **not vacuous**: with the four fix sites temporarily reverted in place, all four fail; restored, all four pass. The standalone-diagnostic case initially passed against the old code because a dotted join and `format!("{payload}.{key}")` coincide for a single dotted key, so the fixture was strengthened to the collision the dotted spelling is actually blind to — a nested `build: {target: …}` authored alongside a flat `"build.target": …`, where the dotted lookup ranged the valid sibling instead of the rejected definition
        - no existing test encoded the retired text-derived contract as a behavior assertion. One unit test, `test_enclosing_path_builds_full_ancestor_chain`, was renamed to `test_enclosing_path_by_indent_builds_full_ancestor_chain` to follow the indentation walk to its new name; its assertions are unchanged and it now covers the fallback rather than the primary path
        - gates: `just build` **PASS**; `just test` **PASS** (darkmatter 5,920/5,920, darkmatter-cli 561/561, dmls **611/611**); `just lint` **PASS**. dmls went 606 → 611 (+4 LSP regressions, +1 unit test); no test was weakened or deleted
        - orchestrator independently re-verified with `cargo check -p dmls --all-targets` (clean) after a stale mid-edit rust-analyzer diagnostic reported a type error at `providers/frontmatter.rs:500`

### Finding 3 (Medium) — Canonical Level-2 release gate

- starting the work on 'canonical-l2-release-gate' at 22:40:23-07:00
        - confirmed the three failures are **deterministic, not load artifacts**: reproduced 4/4 retries at ~2s each with host load down at 8 (load was 18/66/74 at the start of the investigation and settled during it)
        - established the failures are **pre-existing on `main`**: `git diff main...HEAD` shows zero changes under `darkmatter/lib/src/markdown/render/` and `markdown/highlighting/`, and `level2_code_block_styling.rs` is byte-identical to `main`
        - identified a single shared root cause for all three failures in `level2_code_block_styling.rs`: the code panel renders dark (luma 25/44) where a light inverted panel (>175) is asserted
        - attributed the defect to **test staging, not product code**. `query_osc_color_with_timeout` (`biscuit-terminal/lib/src/discovery/osc_queries/query.rs:69-113`) attempts a live OSC-11 query first for `TerminalApp::Wezterm` when no multiplexer is present and returns on success, so `COLORFGBG` is never consulted in a WezTerm pane; under tmux the query is skipped and `COLORFGBG` is honored. The test file's own comment already documents this and had abandoned its light-terminal mirror for exactly this reason — the dark direction has since gone the same way
        - confirmed the inversion contract is green at three independent points, which is what rules out a product defect:
                - L1 `resolve_for_surface_inverts_default_dark_terminal_to_light_panel`
                - library L2 `level2_page_code_panel_is_contiguous_inverted_rectangle`
                - tmux-staged L2 `level2_schema_about_{dark,light}_terminal_uses_*_code_theme`, with exact OneHalf RGB assertions in **both** directions
        - found the review's `md`-not-found harness failures **did not reproduce**: all 8 `level2_errors` tests passed in a clean canonical run
        - recorded the underlying latent hazard behind that reported failure: `level2_errors.rs` invokes bare `md compose` through the pane `PATH` (lines 98, 135, 180) instead of the `md_shim` that `common/level2.rs` adopted specifically so L2 cannot pass against a stale host binary. It passes here only because a host `md` is installed
        - declined to repair the gate under Rule 3 (surgical changes): restaging the three tests onto tmux requires porting `run_md_env` / `run_md_after_shell_prefix` in the shared `common/level2.rs` helper that backs all 69 CLI L2 tests, which is non-trivial work in code paths unrelated to meta-schema and would put the entire CLI L2 tier at risk for a defect this feature did not introduce
        - routed the repair to the already-filed `_unscheduled/wezterm-sgr-race-test-fixes/spec.md`, which names this exact test family
- work completed for 'canonical-l2-release-gate' at 22:55:12-07:00
        - added a **proposed** scoped AC13 exception to `spec.md` naming exactly the three excepted tests, their out-of-scope justification, the contract-is-green evidence, the passing-slice evidence, the repair path, and the `level2_errors` note. AC13 is not relaxed for anything else
        - the exception is marked **PROPOSED — not approved, awaiting Ken's ratification**. Until it is ratified, AC13 is formally unmet and the feature cannot claim the area-level release gate is green
        - gates after the change: `just test` **PASS** (5,920/5,920, 561/561, 611/611); `just lint` **PASS**; darkmatter library L2 **18/18**; `schema about` CLI L2 **3/3**; DMLS L2 **3/3**
        - canonical `just test-l2` remains **red at 87/90** (library 18/18, CLI 66/69, DMLS 3/3 — run separately because the canonical run aborts in the CLI tier). This is reported honestly and is **not** claimed green

### Successful Completion

The implementation of review cycle 3 has completed successfully in 1 hour and 22 minutes. During this implementation all 3 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 2 were fixed, 1 was deferred (see reasons below):

- **Medium — The canonical Level-2 release gate is still red** — deferred as a
  **proposed scoped AC13 exception** rather than repaired. The three failing
  tests (`level2_code_block_clears_inherited_dim_before_theme_colors` and two
  siblings in `level2_code_block_styling.rs`) are pre-existing on `main`,
  outside every meta-schema execution path, and caused by WezTerm-harness test
  staging that the single-source color-mode resolver invalidated — not by
  product code. The inversion contract they claim to test is proven green at L1,
  in the library L2 tier, and under the tmux L2 harness with exact RGB
  assertions in both directions. Repairing the gate requires converting
  `run_md_env` / `run_md_after_shell_prefix` in the shared `common/level2.rs`
  helper that backs all 69 CLI L2 tests from WezTerm to tmux — non-trivial
  out-of-scope work that would risk the entire CLI L2 tier under Rule 3. It is
  routed to the pre-existing `_unscheduled/wezterm-sgr-race-test-fixes/spec.md`.
  **This deferral requires Ken's ratification of the proposed exception; until
  then AC13 is formally unmet.**

> **Note:** no performance measurement was required or deferred by this review,
> so `deferred_perf_measurement` remains `false`.

Two defects were surfaced during this cycle that are **out of scope for this
review** and warrant their own findings:

- **Unbounded whole-file schema-reference recursion (crash).** A self-referential
  standalone schema reference stack-overflows the process on the pre-existing
  contract; Finding 1 makes the scalar shape reach the same unguarded loop. A fix
  means threading a visited-set or depth bound through five HIGH-risk resolver
  symbols.
- **`level2_errors.rs` bypasses the `md_shim`.** It invokes bare `md compose`
  through the pane `PATH`, so it can pass against a stale host-installed binary
  rather than the build under test.

The files changed by this implementation cycle are:

- `darkmatter/lib/src/markdown/schemas/reference.rs`
- `darkmatter/lib/src/markdown/schemas/resolve.rs`
- `darkmatter/lib/src/markdown/schemas/simplified/mod.rs`
- `darkmatter/lib/src/markdown/schemas/simplified/source.rs`
- `darkmatter/lib/src/markdown/schemas/simplified/standalone.rs`
- `darkmatter/lib/tests/meta_schema_phase6.rs`
- `darkmatter/lib/tests/suggest_constraint_phase4.rs`
- `darkmatter/dmls/src/overlay/frontmatter.rs`
- `darkmatter/dmls/src/overlay/schema.rs`
- `darkmatter/dmls/src/overlay/mod.rs`
- `darkmatter/dmls/src/providers/frontmatter.rs`
- `darkmatter/dmls/src/diagnostics/frontmatter.rs`
- `darkmatter/dmls/src/graph/substrate.rs`
- `darkmatter/dmls/tests/lsp_session.rs`
- `darkmatter/features/2026-07-13-meta-schema/spec.md`
- `darkmatter/features/2026-07-13-meta-schema/log.md`
- `darkmatter/features/2026-07-13-meta-schema/review-3.md`

## Implementation of Review Findings #2

> **started at:** 2026-07-18T09:03:32-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'darkmatter/features/2026-07-13-meta-schema/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- the review contains **4** findings:
        - **High** — DMLS reconstructs parser state and ranges from text instead of consuming the required sidecar
        - **High** — a semantic arm in a valid mixed union produces false diagnostics and suppresses sibling completions
        - **High** — invalid standalone outer declarations use the wrong code and whole-document range
        - **Medium** — trimmed schema references are syntax-checked and resolved with different strings
- impacted package area (per the spec's implementation surface map): `darkmatter`, covering the `darkmatter` lib, `darkmatter-cli`, and `dmls` crates — so `just test` / `just lint` from that area is the verification scope
- findings are implemented serially, foundation-first, so that later DMLS work can build on the shared parser authority:
        1. Medium — trimmed schema references (library, isolated)
        2. High — sidecar-backed parser-state authority for DMLS
        3. High — mixed-union gating for diagnostics and completion
        4. High — standalone outer-declaration diagnostic contract

### Finding 4 (Medium) — Trimmed schema references

- starting the work on 'trimmed-schema-references' at 09:04:31-07:00
        - confirmed the defect: `classify_schema_reference` computed `trimmed` for the HTTP(S) and bare-name decisions but built the `FileReference` from the untrimmed `reference`, so `" ./schemas/post.yaml "` was classified on one string and resolved on another
        - a whitespace-only value was non-empty to `FileReference::new`, so it fell through as `PathQualified` rather than being rejected
        - discovered the duplicated policy the finding warns about: `resolve_reference` in `resolve.rs` was calling `reference.trim()` a **second** time, independently of the classifier
        - fixed `darkmatter/lib/src/markdown/schemas/reference.rs` so classification, `FileReference::new`, and the `reference` field on both `RemoteUnsupported` and `Unresolved` all derive from the single `trimmed` value
        - added an explicit whitespace-only guard returning `SchemaError::Unresolved { reference: "", source: InvalidSyntax("empty reference string") }` before `FileReference::new` is reached
        - fixed `darkmatter/lib/src/markdown/schemas/resolve.rs` so `resolve_reference` takes the canonical string back off the classifier via `classified.file_reference().raw()` — syntax check, bare-name root lookup, sibling suggestion, path resolution, and error reporting now use one string by construction
        - aligned two sibling call sites in `resolve.rs` carrying the identical defect: `resolve_namespace` (`Name@file` imports) and `resolve_one_example` (`example(...)` artifacts) both used `trimmed` for the remote/bare-name checks but passed the untrimmed string to `FileReference::new`
        - verified nothing downstream depends on the untrimmed error text — no references to `RemoteUnsupported` or the `$schema` resolution message anywhere in `claudine/`
- work completed for 'trimmed-schema-references' at 09:20:04-07:00
        - added `padded_schema_references_are_classified_and_resolved_as_one_trimmed_string` to `darkmatter/lib/tests/schemas_source_projection.rs`, beside the existing `schema_declaration_parser_classifies_syntax_without_io` sentinel
        - the test covers leading, trailing, both-sided, and tab/newline padding for path-qualified and bare-name references; asserts kind and canonical `raw()` on the classifier; asserts the same product from `parse_schema_declaration` on a quoted padded YAML scalar
        - it asserts `""`, `" "`, `"   "`, `"\t"`, `"\n"`, and `" \t\n "` all reject as an empty reference with the structured error
        - resolver parity is proven by resolving every padded form (plus padded bare names against a schema root) through `resolve_schema_with_roots` and asserting an identical `json_schema` to the unpadded reference
        - the passive-operation proof is retained: classifying `"  ./no/such/dir/missing.yaml  "` and `" missing.yaml "` succeeds, which can only hold if classification performs no existence check or file I/O
        - deliberate non-change: `parse_schema_declaration_with_source` still maps `SchemaSpanKind::FileReference` onto the **authored** text including padding — spans must point at what the author actually typed so an LSP squiggle lands on real characters, and that is independent of the value used for resolution
        - gates green: `just test` → darkmatter 5816 passed / 140 skipped, darkmatter-cli 561 passed / 71 skipped, dmls 584 passed / 3 skipped, zero failures; `just lint` clean with zero warnings

### Finding 1 (High) — Sidecar-backed parser-state authority for DMLS

- starting the work on 'dmls-sidecar-parser-state' at 09:20:04-07:00
        - discovered the root cause: the library already had a **strict** source-aware sidecar (`parse_property_definition_with_source` / `parse_schema_declaration_with_source` in `simplified/source.rs`) that can only answer questions about text which already parses
        - editor completion asks the opposite question — about text that by definition does not parse yet — which is exactly why DMLS had grown its own text heuristics; the missing piece was a **tolerant** entry point, not a second AST
        - new library module `darkmatter/lib/src/markdown/schemas/simplified/cursor.rs` (private module, re-exported from `markdown::schemas`) is the new authority:
                - `SchemaCursor { path: SchemaSourcePath, role: SchemaCursorRole, token: String, token_span: Range<usize> }` — reuses the existing `SchemaSourcePath` sidecar path type, so no new path model and no second spanned AST (spec ruling Q6)
                - `SchemaCursorRole::{ Type, Constraint { subject, array_level }, Argument { subject, constraint, array_level }, InlineObjectKey, ImportReference { name } }`
                - entry points `locate_type_definition_cursor(value_source, value_offset, cursor)` and `locate_schema_declaration_cursor(...)`
                - the implementation drives **`grammar::Lexer`** — the same lexer the real parser uses — over the authored prefix, tracking an explicit frame stack matching the grammar's EBNF (`InlineObject` / `ConstraintList` / `ArgList`), and reproduces the parser's own special cases (`enum`/`literal` item lists lex in argument mode; `[]` before `(` selects the array-level list)
                - no `rfind`, `split`, or indentation heuristic anywhere in it
        - projection goes through the `yaml_scalar` seam via a new **`decode_partial_scalar_at`**, the tolerant sibling of `decode_scalar_at` (an unterminated quote decodes to what exists so far; plain scalars keep trailing whitespace) — no line-ending normalization, so CRLF and multibyte offsets stay exact
        - added a structural locator `locate_schema_value` + `SchemaValueNode` / `SchemaValueKind` / `SchemaValueEntry` to `simplified/source.rs`, a public projection of the private `locate_yaml_value` — this is what diagnostics needs to range something *inside* a value the semantic parser rejects
        - taught `locate_inline` to accept empty flow collections (`[]`, `{}`), which it previously errored on
        - resolved the array-surface question from the existing contract rather than inventing one: added `SchemaConstraintDescriptor::accepts_array_level()` in `about.rs`, derived from the catalog's existing `target_types` prose rather than a new hand-maintained list
- work completed for 'dmls-sidecar-parser-state' at 10:18:31-07:00
        - rewired `darkmatter/dmls/src/providers/frontmatter.rs`:
                - `meta_schema_completion` now uses `SchemaAuthoringState` only for *activation*; everything downstream comes from the cursor API
                - added `semantic_value_start`, which prefers the frontmatter AST's `value_span` and only falls back to the line for a buffer whose YAML is not yet parseable
                - `type_definition_completions` dispatches on `SchemaCursorRole`; new `constraint_completions` handles item-level vs array-level lists, plus `accepts_constraint` comparing catalog entries as whole keywords instead of substrings
        - rewired `darkmatter/dmls/src/diagnostics/frontmatter.rs`: `smallest_invalid_definition_span` now walks `locate_schema_value`'s structural tree deepest-first instead of the frontmatter AST's descendants — this also closes a real gap, since the AST does not enumerate the interior of a sequence item
        - heuristics deleted: `semantic_value_token` (the `,`/`[` splitter with quote-stripping), the `partial.rfind('(')` + `trim_end_matches("[]")` constraint branch, the `accepted_constraints.contains(...)` substring test, and `empty_flow_sequence_span` (the raw-byte `[]` scanner)
        - tests added:
                - library L1: 18 unit tests in `cursor.rs` covering all three named failure states, nested inline objects, flow union arms, arguments, imports, fresh positions, hard-parse-failure tolerance, quoted/multibyte/escaped projection, document offsets, and mid-value cursors
                - library L1: `array_level_constraint_set_is_published_and_parseable` pins the array surface as exactly `required, default, generated, min, max, unique`, and proves `suggest` is rejected
                - DMLS L1: `meta_schema_completion_reads_parser_state_for_partially_authored_values` in `dmls/tests/lsp_session.rs` — 6 cases × LF/CRLF = 12 in-memory `lsp_session` runs (second constraint after nested paren, postfix array constraints, single-quoted inline object, block-sequence union arm with constraint, double-quoted, multibyte-in-value), each also asserting the `textEdit` range replaces the authored token rather than appending
        - **no existing public signature changed** — everything is additive; the only visibility change widens `grammar::Lexer` and five of its methods from private to `pub(super)` so the sibling module can use the one grammar authority, leaving the CRITICAL blast radius on `parse_property_definition` / `parse_schema_declaration` untouched
        - two intentional behavior deltas: `ImportReference` now returns no completions (previously the type-keyword catalog was reachable after `@`, where it matched nothing — a no-op in practice, now correct by construction), and `Argument` / `InlineObjectKey` positions deliberately offer nothing, as they are author-supplied values and names
        - gates green: `just test` → darkmatter 5835 passed / 140 skipped, darkmatter-cli 561 passed / 71 skipped (1 flaky, retried green — the pre-existing `schema_about` leaked-handles flake the review already recorded), dmls 585 passed / 3 skipped; `just lint` clean, exit 0; `no_side_effects.rs` sentinels pass unchanged
        - orchestrator independently re-verified with `cargo check -p dmls --all-targets` after the IDE surfaced stale mid-edit syntax diagnostics: compiles clean, the diagnostics were a language-server snapshot artifact

### Finding 2 (High) — Mixed-union gating for diagnostics and completion

- starting the work on 'mixed-union-gating' at 10:18:31-07:00
        - confirmed the mechanism: `semantic_authored_diagnostics` iterated the overlay's `SchemaAuthoringState::Frontmatter` values and emitted the specialized code whenever the recorded semantic kinds rejected the authored value, consulting the `ValidationReport` only for `relatedInformation` — never to ask whether the *union as a whole* failed
        - activation is intentionally broad (any arm carrying `type-definition`/`schema` records the kind), so a valid mixed union produced a false `dm.schema.invalid_type_definition`
        - **extracted a shared gating helper**: the expression path's policy was inline and expression-specific (a `HashSet<&str>` of `report.problems` paths built at the top of `expression_diagnostics`), now lifted to `union_rejected_paths(report: &ValidationReport) -> HashSet<&str>`
                - its rustdoc `## Notes` records why activation being broader than rejection is exactly what makes the gate necessary
                - both `semantic_authored_diagnostics` and `expression_diagnostics` call it; the expression docblock now points at the shared authority instead of restating the policy
        - gated the specialized diagnostic on `union_rejected.contains(value.pointer)` **before** any parsing work, so a valid document does no redundant AST/YAML/`parse_property_definition` work
- work completed for 'mixed-union-gating' at 10:36:28-07:00
        - completion no longer short-circuits: `completion` in `darkmatter/dmls/src/providers/frontmatter.rs` split its ordinary body into a new `schema_completion` helper, and is now `meta_schema_completion(...).unwrap_or_default()` extended with `schema_completion(...)`, passed through the existing `dedup_completions`
        - semantic items lead, so the pure-`type-definition` ordering and the dedup/first-seen preselection policy are unchanged; sibling union arms merge in behind them
        - the Finding 1 cursor/sidecar API is untouched — no text heuristics were reintroduced
        - tests added to `darkmatter/dmls/tests/lsp_session.rs` (Level-1, in-memory `lsp_session` fixture) with helpers `mixed_semantic_union_doc` (parameterized on arm order) and `codes`:
                - `mixed_semantic_union_gates_the_specialized_diagnostic_on_whole_union_failure` — `type-definition`/`string` and `schema`/`string`, both arm orders, each with a value the `string` arm accepts (specialized code must be **absent**) and a value no arm accepts (must be **present**); 8 documents total
                - `single_arm_semantic_type_keeps_its_specialized_diagnostic` — pins the non-union common path so the fix does not regress it
                - `mixed_semantic_union_completion_merges_sibling_arm_candidates` — `[type-definition, enum(foo, bar)]` in both arm orders must offer `foo`, `bar` **and** the semantic `string` candidate
        - both fixes were verified load-bearing by temporarily disabling each in turn: without the gate the diagnostics test reproduces the review's exact false positive; without the merge the completion test loses `foo`/`bar`
        - discovered a real validator property worth recording: `string` is **not** a discriminating arm for scalar values — `value: 42` against `[type-definition, string]` produced no diagnostics at all because the `string` arm coerces a YAML number rather than type-mismatching, so the whole-union-failure case had to use a block mapping
        - checked that merging adds no noise for `$schema` authoring: values under `$schema:`, the `$schema` scalar itself, and block-sequence arms all resolve through `def_at_path_ctx` against the effective shape, which has no `$schema` property, so the ordinary provider returns empty there
        - one deliberate contract delta flagged: a semantic cursor role that intentionally offers nothing (`Argument`, `InlineObjectKey`, `ImportReference`) previously returned `Some(vec![])` and suppressed the ordinary provider entirely; the ordinary provider now runs there too, which traces to empty for semantic-typed properties in every case but is a genuine contract change rather than a pure merge
        - gates green: `just test` → darkmatter 5835 passed / 140 skipped, darkmatter-cli 561 passed / 71 skipped (no `schema_about` flake this run), dmls 588 passed / 3 skipped; `just lint` clean, exit 0; `no_side_effects.rs` sentinels pass unchanged

### Finding 3 (High) — Standalone outer-declaration diagnostic contract

- starting the work on 'standalone-outer-declaration-diagnostics' at 10:36:28-07:00
        - discovered the standalone fallback had exactly two outcomes: `standalone_type_definition_diagnostic` (inner, mapping-payload properties only) and an unconditional whole-buffer `dm.schema.document_malformed` — everything else (empty root unions, illegal union arms, invalid whole-file references) hit the generic code with a whole-document range
        - discovered the outer/inner boundary is **structural**, not incidental: an outer declaration is a *sequence* payload (root union) or a *scalar* payload (whole-file reference), whereas a *mapping* payload's failure always belongs to one of its property definitions
                - restricting the new path to sequence/scalar payloads is what mechanically preserves the `invalid_type_definition` vs `invalid_schema_shape` split, rather than relying on ordering luck
        - discovered `parse_yaml_schema` does not classify union-arm references (only `parse_schema_declaration` does), so a whitespace-only reference *arm* (`$schema: ["   "]`) parses clean today and never reaches the error path; the finding's reachable reference case is the scalar form `$schema: "   "`, which the Finding 4 trimmed-reference fix now rejects with `Unresolved { reference: "" }`
- work completed for 'standalone-outer-declaration-diagnostics' at 11:01:17-07:00
        - changed `darkmatter/dmls/src/diagnostics/frontmatter.rs`: added `standalone_declaration_diagnostic` between the inner and generic paths, emitting `dm.schema.invalid_schema_shape` from `parse_schema_declaration`'s error with a range from the library's structural locator (`locate_schema_value`)
                - supporting helpers `standalone_payload_node`, `invalid_union_arm_span`, and `arm_is_rejected` (which mirrors `parse_schema_declaration`'s arm handling: mapping → inline shape, string → `classify_schema_reference`, anything else → illegal)
                - extracted `standalone_payload_key` so both standalone helpers derive the envelope's payload key from one place
                - no raw-byte scanning and no decoded-text range reconstruction — the Finding 1 sidecar authority is consumed throughout
        - changed `darkmatter/dmls/docs/diagnostics.md`: the `invalid_schema_shape` row records the standalone outer-declaration cases and their ranging; the `document_malformed` row now states it applies only when no more precise declaration or definition diagnostic claims the failure
                - `dm.schema.invalid_type_definition` is absent from that table entirely — pre-existing drift from an earlier phase, left alone under CLAUDE.md Rule 3 (surgical changes)
        - added `standalone_outer_declaration_errors_are_shape_coded_and_precisely_ranged` to `darkmatter/dmls/tests/lsp_session.rs`, covering 7 documents: pure empty root union, **tagged** empty root union, pure invalid scalar arm, pure invalid local reference, malformed YAML (tab indent), pure malformed inner definition, tagged malformed inner definition
                - each case asserts the exact LSP **range** *and* that the set of `dm.schema.*` document codes is exactly the one expected, so a whole-document range or a code collapse fails the test
                - helpers `range_of` / `whole_document_range` derive expectations from the fixture text rather than hard-coded numbers
        - verified load-bearing: temporarily disabling the new branch reproduces the review's exact defect — `pure-empty-union.yaml` reverts to `dm.schema.document_malformed` over `0:0–1:0`
        - gates green: `just test` → darkmatter 5835 / darkmatter-cli 561 / dmls 589 passed; `just lint` exit 0; `no_side_effects.rs` sentinels pass unchanged, since reference classification is passive and no filesystem access was added

### Final Verification

- the orchestrator re-ran both gates independently from the `darkmatter` package area **after all four findings had landed together**, to confirm the composed result rather than trusting per-finding reports:
        - `just lint` → exit 0, zero warnings and zero errors across `darkmatter`, `darkmatter-cli`, and `dmls`
        - `just test` → exit 0; darkmatter **5835 passed / 140 skipped**, darkmatter-cli **561 passed / 71 skipped**, dmls **589 passed / 3 skipped**; zero failures and no flaky retries on this run
- net test growth across the cycle: darkmatter 5816 → 5835 (+19), dmls 584 → 589 (+5), darkmatter-cli unchanged at 561
- during Finding 1 the IDE surfaced syntax diagnostics in `dmls/src/providers/frontmatter.rs` that contradicted the subagent's green report; `cargo check -p dmls --all-targets` confirmed a clean compile, so those were stale mid-edit language-server snapshots rather than real breakage

### Successful Completion

The implementation of review cycle 2 has completed successfully in 1 hour and 58 minutes. During this implementation all 4 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 4 were fixed, 0 were deferred.

No finding required a performance measurement, so `deferred_perf_measurement` remains `false`.

Two follow-up items were surfaced during the work. Neither is a deferred finding — both fall outside the scope of what the review asked for — but both are recorded here so they are not rediscovered:

- **Invalid reference *arms* inside an otherwise-valid root union** (`$schema: ["   "]`) still produce no diagnostic. `parse_yaml_schema` accepts them as `SchemaArm::FileRef` without classification, so `parse_standalone_schema_document` returns `Ok` and the error path is never entered. Catching this would require running `parse_schema_declaration` on *every* standalone document rather than only on already-failing ones — a behavior expansion past Finding 3, which is scoped to the fallback path. This warrants a design decision rather than an unilateral widening.
- **A pure envelope with a valid scalar reference payload** (`$schema: ./other.yaml`) still reports `document_malformed` with the envelope's own "`$schema` must be a mapping or sequence" message. The declaration parser accepts that string, so there is no declaration-shape error to report; the complaint is genuinely envelope-level. This behavior is unchanged by this cycle.

The files changed during this cycle are:

- `darkmatter/lib/src/markdown/schemas/reference.rs` — single-trim canonical reference construction and the whitespace-only guard
- `darkmatter/lib/src/markdown/schemas/resolve.rs` — resolver parity via `classified.file_reference().raw()`, applied to `resolve_reference`, `resolve_namespace`, and `resolve_one_example`
- `darkmatter/lib/src/markdown/schemas/simplified/cursor.rs` — **new** tolerant parser-state authority (`SchemaCursor` / `SchemaCursorRole`)
- `darkmatter/lib/src/markdown/schemas/simplified/source.rs` — structural locator `locate_schema_value` and empty-flow-collection support in `locate_inline`
- `darkmatter/lib/src/markdown/schemas/simplified/grammar.rs` — `Lexer` widened to `pub(super)` so the cursor module reuses the one grammar authority
- `darkmatter/lib/src/markdown/schemas/about.rs` — `SchemaConstraintDescriptor::accepts_array_level()`
- `darkmatter/dmls/src/providers/frontmatter.rs` — cursor-driven completion, `schema_completion` split, union-merged candidates
- `darkmatter/dmls/src/diagnostics/frontmatter.rs` — `union_rejected_paths` gate, structural-locator ranging, `standalone_declaration_diagnostic`
- `darkmatter/dmls/docs/diagnostics.md` — diagnostic-code table updated for the standalone outer-declaration contract
- `darkmatter/lib/tests/schemas_source_projection.rs` — padded/whitespace-only reference parity tests
- `darkmatter/dmls/tests/lsp_session.rs` — parser-state, mixed-union, and standalone-declaration Level-1 LSP regressions

## Implementation of Review Findings #4

> **started at:** 2026-07-19T23:27:33-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'darkmatter/features/2026-07-13-meta-schema/review-4.md'
- this is iteration 4 of the review-to-implement cycle
- the review contains **3** findings:
        - **High** — Completion loses nested semantic owners and RFC 6901-escaped keys (`dmls/src/providers/frontmatter.rs`)
        - **High** — Referenced schema-file graphs can recurse without bound and lose transitive dependencies (`lib/src/markdown/schemas/resolve.rs`)
        - **Medium** — The canonical Level-2 gate remains red and its AC13 exception is not ratified
- impacted package area (per the spec's implementation surface map): `darkmatter`,
  covering the `darkmatter` lib, `darkmatter-cli`, and `dmls` crates — so
  `just test` / `just lint` from that area is the verification scope

### Finding 1 — Completion loses nested semantic owners and RFC 6901-escaped keys

- starting the work on 'finding-1-completion-structural-owner-path' at 23:29:41
- **root cause — two independent path-truncation defects, not one**
        - `meta_schema_kinds_for_line` (`dmls/src/providers/frontmatter.rs`) reduced the authored position to a single `owner` string (`ancestors.first()`, else `key`) and then searched `SchemaAuthoringState::Frontmatter` values for `value.pointer.strip_prefix('/') == Some(owner)` — comparing a **decoded** single segment against a **serialized RFC 6901 pointer**. It could never match a nested value (`/parameter/type` searched as `parameter`) nor an escaped top-level key (`/a~1b` vs `a/b`, `/c~0d` vs `c~d`). `frontmatter_authoring` was already recording the correct complete pointer; completion was the only lossy stage
        - a second, previously unnoticed truncation sits underneath: `FrontmatterAst::container_at_offset` filtered to `FmValueKind::Mapping` only, so a `- item` line under a **sequence**-valued key had its ancestor chain cut at the nearest enclosing mapping (`parameter:/type:/- str` reported ancestors `["parameter"]`, not `["parameter", "type"]`). Fixing only the pointer comparison leaves every `[]` array form dead
- **changes** (3 files)
        - `dmls/src/providers/frontmatter.rs` — builds the complete structural key path (`ancestors` + `key`, `key` omitted for a `sequence_item`), decodes each recorded pointer through the existing repo authority `darkmatter::markdown::schemas::JsonPointer::parse` (no second escaper hand-rolled), and selects the **longest** recorded pointer that is a segment-wise prefix. Exact match → recorded `kinds`; strict-ancestor match → `TypeDefinition`, which preserves and correctly generalizes the prior "nested under a `schema`-kinded owner degrades to `TypeDefinition`" rule. `$schema`-rooted early returns and the whole `Standalone` arm untouched
        - `dmls/src/overlay/frontmatter.rs` — `container_at_offset` (and sibling `enclosing_key_path`) take `line_start` and now accept `FmValueKind::Sequence`, **but only when `entry.key_span.end <= line_start`**. That line guard is load-bearing: without it a *flow* collection (`ratios: [0.25, ]`) becomes its own line's ancestor and doubles the key, which regressed `suggest_phase1_completion_positions` on the first attempt. `Mapping` behavior deliberately unchanged
        - `dmls/tests/lsp_session.rs` — 2 new Level-1 in-memory LSP session tests
- **tests added**
        - `meta_schema_completion_matches_complete_structural_pointers` — 8 shapes × LF/CRLF = 16 assertions: nested `type-definition` (`/parameter/type`, the shape Feature A itself authors), nested `schema` (`/nested/decl`), `/`-bearing owner key, `~`-bearing owner key, plus the `[]` sequence form of each. Failures are collected and reported together so one broken class cannot mask the other seven
        - `malformed_buffer_retains_nested_and_escaped_semantic_owners` — asserts baseline completion, then `didChange`s a malformed `bad: [unclosed` onto a later line and asserts the owner keeps activation via the last-good tree's per-entry still-placed check
- **load-bearing evidence (measured, not reasoned)**
        - reverting only `meta_schema_kinds_for_line`: **all 16** sub-cases fail with empty candidate lists
        - restoring that and reverting only the `Sequence` container change: the 4 scalar shapes pass and **exactly the 8 `[]` sub-cases fail** — confirming the second defect is real and independently covered
        - with both fixes: 2 tests, 2 passed
- **gates** — `just test` exit **0**: darkmatter **5920/5920**, darkmatter-cli **561/561**, dmls **613/613** (review's 611 + 2 new); `just lint` exit **0**
- work completed for 'finding-1-completion-structural-owner-path' at 23:52:18

### Finding 2 — Referenced schema-file graphs can recurse without bound and lose transitive dependencies

- starting the work on 'finding-2-reference-graph-cycle-and-dependencies' at 23:52:40
- **root cause — two independent defects on the same code path**
        - `resolve_reference` had no frame stack. A standalone document whose whole payload is a reference (`SchemaDeclaration::Reference`) delegates by re-entering `resolve_reference` from `parse_yaml_referenced_file`, and root-union `SchemaArm::FileRef` arms do the same from `resolve_root_union` / `resolve_standalone_root_union`. Nothing bounded that recursion, so `a.yaml → a.yaml` or `a.yaml ⇄ b.yaml` recursed until the stack died
        - both success paths (normal reference at `resolve.rs:344`, bare-name/schema-root at `resolve.rs:305`) executed `resolved.referenced_files = vec![canonical_path(&path)]` — an **overwrite** discarding the nested accumulation. `document → a.yaml → b.yaml` reported only `a.yaml`, violating `EffectiveSchema::dependencies`' invalidation contract for every hop past the first
- **changes — `lib/src/markdown/schemas/resolve.rs`**
        - new private `ReferenceStack` (canonical-path frames, innermost last) with `enter`/`leave`/`describe`, plus `MAX_REFERENCE_DEPTH = 32` mirroring the existing `MAX_IMPORT_DEPTH`. Frames use the file's existing `canonical_path` helper, so `./a.yaml`, `../dir/a.yaml`, and a case-differing spelling on macOS/Windows collapse to one frame
        - new `load_guarded` wraps `load_schema_from_path` so the frame opens before load and closes on both success and failure. The stack is threaded through `resolve_root_union`, `resolve_reference`, `load_schema_from_path`, `parse_yaml_referenced_file`, `resolve_standalone_schema`, and `resolve_standalone_root_union`; `resolve_yaml_schema_with_roots` is the single construction site. All six are private with in-file callers, so **no public signature changed**
        - new `accumulate_referenced_file(nested, canonical)` folds each hop in via `BTreeSet` — deduplicated and deterministically sorted, the order `ResolvedSchema::referenced_files` already documented. Replaces both overwrite sites
        - new `attribute_origin` sets `SchemaOrigin::referenced_file` only when the nested origin is not already `ReferencedFile`. **Deliberate:** for `doc → a.yaml → b.yaml` the origin is now `b.yaml`, not `a.yaml` — `a.yaml` is a pure redirect with no declaration for `relatedInformation` to range against. Single-hop behavior is byte-identical, so no existing test moved
- **changes — `lib/src/markdown/schemas/errors.rs`**
        - added `SchemaError::ReferenceCycle { chain: String }` (rendered `a -> b -> a`) plus its `status_block` arm. `SchemaError` is public and **not** `#[non_exhaustive]`, so every match site was checked first: the only exhaustive match is `status_block` in that same file; `claudine/lib/src/composition/schema/translate.rs` and the three `dmls` sites all match specific variants with a fallback. No downstream break, confirmed by lint across all three crates
- **deliberate widening worth flagging** — `mod.rs:445` feeds `referenced_files` to `triggers::assemble::check_document_schema_not_trigger`. Now that the list is transitive, a delegation chain whose *terminal* hop is a `*.trigger.yaml` envelope is also rejected, not just a direct reference. That is the correct reading of "triggers activate by placement and match, never by reference", but it is a scope change rather than a pure bug fix
- **tests added**
        - new `lib/tests/meta_schema_reference_graph.rs` — 6 Level-1 resolver tests: multi-hop chain (`doc → a → b → terminal`, asserting the resolved schema, all three hops in the dependency list, and origin attribution); self-cycle; two-file cycle; root-union arm cycling back into an open chain; document-level root-union arm entering a cycle; and a diamond (`shared.yaml` named by two arms) proving the guard tracks *open* frames rather than visited ones, so a legitimate repeat still resolves
        - `dmls/src/overlay/mod.rs` — `schema_cache_invalidates_on_terminal_file_in_a_reference_chain`, sibling of the existing single-hop test. It edits **only** the terminal `b.yaml` in a `doc.md → a.yaml → b.yaml` chain and asserts the bundle is reassembled (`!Arc::ptr_eq`) and that `enum(a, b)` → `enum(x, y)` actually re-fails validation
- **load-bearing evidence — measured, not reasoned**
        - *cycle guard*: stashed out **only** the `frames.contains` check while leaving the depth cap in place (so the test process could not die), rebuilt, reran. `two_file_cycle_*` and `root_union_arm_cycling_*` both failed with `chain exceeded 32 levels` — resolution genuinely recursed 32 frames deep with no terminating condition; with no guard at all that recursion is unbounded, so stack overflow is the real pre-fix outcome. The self-cycle test still passed under the partial revert (its assertion only requires `a.yaml` in the message, which the depth error also carries), so the two-file and union tests are the discriminating ones
        - *dependency accumulation*: reverted `accumulate_referenced_file` to the `vec![canonical]` overwrite. `multi_hop_chain_resolves_and_records_every_hop` failed reporting `["…/a.yaml"]` against expected `[a, b, terminal]`, and the diamond test reported `["…/union.yaml"]` against `[shared, union]` — the exact data loss the review described
- **gates** — `just test` exit **0**: darkmatter **5926/5926** (+6), darkmatter-cli **561/561**, dmls **614/614** (+1); `just lint` exit **0**
        - one nextest flake: `markdown::reference::file_tree::model::tests::inline_summary_singular` LKFAIL on try 1, passed on retry — unrelated module, known spurious leak-timeout class
- **orchestrator note** — the IDE surfaced 8 rustc diagnostics in `resolve.rs` (`cannot find function load_guarded`, arity mismatches) that contradicted the subagent's green report. `cargo check -p darkmatter --all-targets` finished clean, so those were stale mid-edit language-server snapshots, not real breakage — the same false signal seen in iteration 3
- work completed for 'finding-2-reference-graph-cycle-and-dependencies' at 00:12:05

### Finding 3 — The canonical Level-2 gate remains red and its exception is not approved

- starting the work on 'finding-3-canonical-l2-gate-and-ac13-exception' at 00:12:20
- this finding was handled by the orchestrator directly rather than a subagent, because its actionable core is a **ratification decision reserved to Ken**, not an implementable code defect
- reproduced the canonical gate on this host **after** findings 1 and 2 had both landed, to confirm no new L2 breakage was introduced:
        - `just test-l2` (fail-fast) → exit **100**: library **18/18** passed, then the CLI tier aborted after 2 passes on `level2_code_block_clears_inherited_dim_before_theme_colors`, leaving 66 CLI tests and the whole DMLS tier unrun — byte-for-byte the behavior review 4 describes
        - `just test-l2 --no-fail-fast` → exit **100**: library **18/18**, CLI **66/69**. The 3 failures are **exactly** the three tests named in the spec's proposed exception and no others
        - DMLS L2 run separately → **3/3** passed
- confirmed the failure mechanism matches the spec's stated root cause rather than anything this cycle changed:
        - the assertion fails with `got luma 44` — a **dark** panel where a light (inverted) one is required. That means the terminal was detected as *light* despite the test staging `COLORFGBG='15;0'`, which is precisely the WezTerm live-OSC-11-wins path the spec documents. A theme-resolution regression would show as a wrong *color*, not as the staged terminal polarity being ignored outright
- **drift detected in the spec's own exception evidence, and corrected**
        - the spec claimed `git diff main...HEAD` reports **zero** changes under both `darkmatter/lib/src/markdown/render/` **and** `darkmatter/lib/src/markdown/highlighting/`. The first half still holds; the second is now false — the unrelated perf commit `864521fae` ("borrow syntax themes and write escapes directly") touches `highlighting/{mod,prose,themes}.rs` on this branch
        - per CLAUDE.md the code is authoritative and the document is wrong, so the paragraph in `spec.md` was corrected to state the accurate diff facts, name the responsible commit, and explain why it is still not the cause (the luma-44 polarity evidence above)
        - re-verified and left standing: `render/` is genuinely zero, and `darkmatter/cli/tests/level2_code_block_styling.rs` is genuinely byte-identical to `main`
        - the exception's **PROPOSED / awaiting-ratification status was deliberately left unchanged** — correcting a factual evidence sentence is not the same as approving the exception, and this session cannot ratify on Ken's behalf
- **DEFERRED.** The finding cannot be closed in this cycle. Its two possible closures are both unavailable here:
        - *ratify the exception* — reserved to Ken; this is a non-interactive session with no one to ask
        - *restore the gate by repairing the three tests* — requires porting `run_md_env` / `run_md_after_shell_prefix` in `darkmatter/cli/tests/common/level2.rs` from the WezTerm harness to tmux. That helper backs the entire 69-test CLI L2 corpus, so the change would put the whole tier at risk to fix a pre-existing defect this feature did not introduce. It is already filed as `darkmatter/features/_unscheduled/wezterm-sgr-race-test-fixes/spec.md`, which names this exact test family. Doing it here would violate CLAUDE.md Rule 3 (surgical changes)
- no performance measurement was required by any finding in this review, so `deferred_perf_measurement` remains `false`
- work completed for 'finding-3-canonical-l2-gate-and-ac13-exception' at 00:14:03

### Final Verification

- the orchestrator re-ran both L1 gates independently from the `darkmatter` package area **after both code findings had landed together**, to confirm the composed result rather than trusting per-finding reports:
        - `just lint` → exit **0**; zero warnings and zero errors across `darkmatter`, `darkmatter-cli`, and `dmls`
        - `just test` → exit **0**; darkmatter **5926 passed / 140 skipped**, darkmatter-cli **561 passed / 71 skipped**, dmls **614 passed / 3 skipped**; zero failures on this run
- net test growth across the cycle: darkmatter 5920 → 5926 (+6, the new resolver reference-graph file), dmls 611 → 614 (+3: 2 completion regressions + 1 chain cache-invalidation test), darkmatter-cli unchanged at 561
- L2 was reproduced but is **not** green — see finding 3 above for the full accounting
- as in iteration 3, the IDE surfaced rustc diagnostics that contradicted a subagent's green report (this time 8 errors in `resolve.rs`). `cargo check -p darkmatter --all-targets` finished clean, confirming them as stale mid-edit language-server snapshots. Worth remembering: the language server's view of a file a subagent has just rewritten is not trustworthy evidence either way — verify with a real compile

### Successful Completion

The implementation of review cycle 4 has completed successfully in 46 minutes. During this implementation all 3 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 2 were fixed, 1 was deferred (see reasons below):

- **Medium — "The canonical Level-2 gate remains red and its exception is not approved."** Deferred because neither available closure is reachable from this session. Closing it by *ratifying* the AC13 scope exception is a decision reserved to Ken, and this is a non-interactive session with no one to ask. Closing it by *repairing* the three failing tests would require porting the shared `run_md_env` / `run_md_after_shell_prefix` helpers in `darkmatter/cli/tests/common/level2.rs` off the WezTerm harness and onto tmux — a helper that backs the whole 69-test CLI L2 corpus — in order to fix a pre-existing, already-filed defect that this feature did not introduce and whose failure mechanism is demonstrably unrelated to meta-schema code. That is a Rule 3 violation and belongs to `darkmatter/features/_unscheduled/wezterm-sgr-race-test-fixes/spec.md`. What *was* done for this finding: the gate was reproduced post-change to prove no new L2 breakage (the 3 failures are exactly the 3 named tests and no others), the failure mechanism was confirmed against the spec's stated root cause, and a factual drift in the spec's own exception evidence was corrected.

No finding required a performance measurement, so `deferred_perf_measurement` remains `false`.

The files changed during this cycle are:

- `darkmatter/lib/src/markdown/schemas/resolve.rs` — `ReferenceStack` cycle/depth guard, `load_guarded`, `accumulate_referenced_file` transitive dependency accumulation, `attribute_origin`
- `darkmatter/lib/src/markdown/schemas/errors.rs` — new `SchemaError::ReferenceCycle { chain }` variant and its `status_block` arm
- `darkmatter/dmls/src/providers/frontmatter.rs` — complete structural key-path matching against encoded RFC 6901 pointers, via the existing `JsonPointer::parse` authority
- `darkmatter/dmls/src/overlay/frontmatter.rs` — `container_at_offset` accepts sequence-valued containers under a load-bearing `key_span.end <= line_start` guard
- `darkmatter/dmls/src/overlay/mod.rs` — `schema_cache_invalidates_on_terminal_file_in_a_reference_chain`
- `darkmatter/lib/tests/meta_schema_reference_graph.rs` — **new**; 6 Level-1 resolver tests (multi-hop chain, self-cycle, two-file cycle, two root-union cycle shapes, diamond)
- `darkmatter/dmls/tests/lsp_session.rs` — nested/escaped/array completion regressions across LF and CRLF, plus malformed-buffer last-good retention
- `darkmatter/features/2026-07-13-meta-schema/spec.md` — corrected the stale `highlighting/` claim in the proposed AC13 exception evidence
