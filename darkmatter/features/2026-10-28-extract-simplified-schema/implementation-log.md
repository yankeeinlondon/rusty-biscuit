---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-schema-narrowing-and-extraction/darkmatter/features/2026-10-28-extract-simplified-schema/spec.md"
plan: "darkmatter/features/2026-10-28-extract-simplified-schema/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
decisions_for_review:
    - |-
        Glob pattern parsing is part of biscuit-file's new minimal `file-reference-syntax` feature, so the minimal build keeps the `globset` crate. The plan said the minimal build must exclude gix, walkdir, and globset.
        Why: the schema grammar checks `match(...)` glob patterns while it parses (via GlobReference::new, which compiles a globset matcher). Dropping globset from the minimal build would mean skipping that check there, so some schemas that are rejected today would be accepted, and the spec forbids changing which schemas parse.
        Alternative: leave globset out and report the glob check as an "unavailable required check" in the minimal build. That is a slightly smaller dependency tree, but the minimal build would then behave differently from the default build.
        If the reviewer prefers the alternative: move `glob/parse.rs`, `glob/error.rs`, and `GlobReference::new` back under `file-reference`, and have Phase 4 make the grammar's `match(...)` check conditional on the `file` feature. The Phase 2 and Phase 8 dependency-tree checks should then also list globset.
    - |-
        The new `biscuit_file::frontmatter` module needs the `yaml` feature. The plan called it "unfeatured".
        Why: recognizing a near-miss `----` fence means checking whether the enclosed text is a YAML mapping, and that needs the YAML parser, which is optional in biscuit-file. `yaml` is on by default, and simplified-schema needs it anyway.
        Alternative: make the YAML parser a mandatory dependency of biscuit-file. That makes every no-default-features user pay for YAML.
        If reversed: drop the `#[cfg(feature = "yaml")]` on `pub mod frontmatter` and make serde_yaml_ng non-optional.
    - |-
        In the biscuit-file feature split, only the path-identity and strategy types of the "portable path" family are in `file-reference-syntax`. `PortablePath`, `PortableReference`, their diagnostics, and `PORTABLE_ENV_VARIABLES` stay under the full `file-reference` feature. The plan listed "portable path types" under the syntax feature.
        Why: PortablePath checks each spelling by running the resolver, which needs the resolution stack.
        Alternative: move the resolver's lexical parts into the syntax feature as well. That is a much larger refactor, with no current consumer that needs it.
        If reversed: the resolution code that PortablePath calls would have to be split out of `resolve.rs`.
    - |-
        Biscuit-file's `FileReferenceError::Git` variant exists only when `file-reference` is enabled, because its payload is a gix type. The error enum is not `#[non_exhaustive]`, so its set of variants depends on the enabled features.
        Alternative: change the payload to a boxed `dyn Error` so the variant always exists. That changes the variant's public type for existing users.
        If reversed: only error.rs and the one match on that variant change.
    - |-
        Plan Task 1.1 placed the inventory under `darkmatter/features/2026-10-25-extract-simplified-schema/`. That directory does not exist, so the inventory is at `darkmatter/features/2026-10-28-extract-simplified-schema/inventory.md`, beside the spec.
    - |-
        The inventory recommends, without implementing yet, two homes that the spec may not have anticipated, for review before Phase 4.
        (a) SchemaError's terminal presentation (its `BlockError` implementation): Rust does not let Darkmatter implement a trait from another crate for a type from another crate, so once SchemaError moves this code must move with it, into an opt-in, non-default `terminal` feature of simplified-schema that only Darkmatter enables. The alternative is a wrapper type in Darkmatter.
        (b) The pure `{{!data:v1:...}}` pending-token codec, plus the plain `{{ }}` scanner it relies on, moves into simplified-schema in Phase 2.4, because the format checks decode pending tokens during validation.
    - |-
        The pending-token codec's helper scanner (the plain `{{ }}` scan) moved as free functions `scan(content, excluded_ranges)`, `scan_plain`, and `find_all_plain` in a new `simplified_schema::expression::scan` module. The plan and inventory said only "move the scan core". Darkmatter keeps its `ExpressionFinder` type as a thin wrapper that finds Markdown code blocks and passes them to `scan`, so every existing caller and the 2,400-line lexer test module are unchanged.
        Alternative: move `ExpressionFinder` itself with a constructor that takes the code regions. That would add a second constructor and make Darkmatter's `new` (which needs the Markdown scanner) impossible on the moved type, so it would still need a wrapper.
        If reversed: only `scan.rs` and `ExpressionFinder`'s three delegating methods change.
    - |-
        `newline_offset_table` and `line_at_offset`, two crate-private helpers in the old `span.rs`, stayed in Darkmatter, which keeps a small `span.rs` that re-exports the library's span items alongside them. The plan said to move only the span helpers the moved code uses, and only Darkmatter's table-of-contents and remote-content code (which stay) use these two.
        If a later phase needs them in the library: move both and make them `pub`.
    - |-
        The `sniff` dependency is declared now, optional and behind the default `file` feature, although nothing calls it until the path-based factories arrive. The plan's Task 2.1 asked for this.
        Alternative: add it in the phase that writes the capture module. That keeps the dependency list strictly to what is used, but it means CI registration and the feature table change twice.
        If reversed: delete the `sniff` line and the `dep:sniff` entry from `file`.
    - |-
        Re-exports left in Darkmatter: `markdown::numeric` is marked `// migration: removed in Phase 7`, because it exists only for unmoved callers. `markdown::literal_token` and `markdown::span` are not marked, because the inventory keeps the literal-token path (Claudine uses it widely) and the span items are part of Darkmatter's public root API (`darkmatter::Spanned`, pinned by the `span_compat` test). `pending::first_token` is `pub` in the library (a Darkmatter caller needs it) but re-exported only `pub(crate)` in Darkmatter, so Darkmatter's public API does not grow.
        If the reviewer wants every re-export marked for removal, Phase 7 must also repoint Darkmatter's internal and Claudine callers of `literal_token`.
    - |-
        Two tests inside the moved literal-token module loaded YAML through Darkmatter's Markdown frontmatter parser, which the library cannot depend on. In the library they now load through the same admitting YAML loader and JSON conversion that the frontmatter parser uses. Darkmatter keeps two copies that still go through the real frontmatter parser, so both facts stay pinned.
        Alternative: move those two tests to Darkmatter only. That loses the library-level round-trip check.
    - |-
        The library counts its own work. Moving the parser meant its `ExpressionParse` work counter could no longer call Darkmatter's counters, so simplified-schema gained a test-only `work-counters` feature with its own `work_counters` module (just `ExpressionParse` for now). Darkmatter's `work-counters` feature turns it on, and Darkmatter's `snapshot()` adds the library's count into its existing `Work::ExpressionParse`, so every test and bench that reads that counter is unchanged. This follows the inventory's plan for work counters; the plan itself did not mention them for this phase.
        Alternative: have Darkmatter pass a counting callback into the parser. That puts an instrumentation parameter on the public parse functions.
        If reversed: delete `simplified-schema/lib/src/work_counters.rs` and the macro in its `lib.rs`, and the fold in `darkmatter/lib/src/work_counters.rs`.
    - |-
        Two parser tests that moved with the parser read Claudine test fixtures (`claudine/cli/tests/fixtures/nested_span_regression/*.md`) with `include_str!`, so a simplified-schema test now reads files from another package. Only the relative path changed, as the move rules require.
        Alternative: copy the two fixtures into `simplified-schema/lib/tests/fixtures/`. That removes the cross-package read, but the copies can drift from the incident files they pin.
        If reversed: copy the files and change the two `include_str!` paths in `simplified-schema/lib/src/expression/parser.rs`.
    - |-
        The plan asked for a library test per schema check (expression values, expression defaults, function examples) proving `unknown_fn(1)` is accepted. Those checks are still in Darkmatter until Phase 4 moves the schema code, so the library tests cover the parser function each check calls, and new Darkmatter tests (`schema_expression_syntax_checks.rs`) cover the three checks themselves through the public schema API. The three checks now import the parser from `simplified_schema::expression` directly, so Phase 4 only has to change those imports to `crate::expression`.
        If the reviewer wants the check-level tests in the library: move them when Phase 4 moves `format.rs`, `convert.rs`, and `grammar.rs`.
    - |-
        `PRECEDENCE_TABLE` (the parser's operator-precedence list) was crate-private. It is now public but hidden from the docs, because a Darkmatter test checks that Darkmatter's operator catalog matches it, and that test now has to reach it from another crate.
        Alternative: move that comparison test into the library. The test reads Darkmatter's catalog, so it cannot move.
    - |-
        The plan's decision 5 expected `error.rs` to be split into a syntax part and a host part. As the inventory found, it has no syntax part: the syntax errors are `LexerError` and `ParseError`, which moved with the lexer and parser. `error.rs` stays whole in Darkmatter, and `ExpressionError` converts the library's `ParseError` as before.
source_files_during_phase_1:
    - biscuit-file/lib/Cargo.toml
    - biscuit-file/justfile
    - biscuit-file/lib/src/lib.rs
    - biscuit-file/lib/src/frontmatter.rs
    - biscuit-file/lib/src/path_text.rs
    - biscuit-file/lib/src/yaml/types.rs
    - biscuit-file/lib/src/file_reference/mod.rs
    - biscuit-file/lib/src/file_reference/error.rs
    - biscuit-file/lib/src/file_reference/context.rs
    - biscuit-file/lib/src/file_reference/resolve.rs
    - biscuit-file/lib/src/file_reference/lexical.rs
    - biscuit-file/lib/src/file_reference/glob/mod.rs
    - biscuit-file/lib/src/file_reference/glob/parse.rs
    - biscuit-file/lib/src/file_reference/glob/error.rs
    - biscuit-file/lib/src/file_reference/portable/mod.rs
    - biscuit-file/lib/src/file_reference/portable/path_identity.rs
    - biscuit-file/lib/tests/l1/main.rs
    - biscuit-file/lib/tests/l1/reference_syntax_only.rs
    - darkmatter/lib/src/markdown/frontmatter.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/schema_extraction_characterization.rs
docs_updated_during_phase_1:
    - biscuit-file/README.md
    - biscuit-file/docs/dependencies.md
    - biscuit-file/docs/topics/file-references.md
docs_created_during_phase_1:
    - darkmatter/features/2026-10-28-extract-simplified-schema/inventory.md
skills_files_updated_during_phase_1:
    - .claude/skills/biscuit-file/SKILL.md
    - .claude/skills/biscuit-file/references/api.md
    - .claude/skills/biscuit-file/references/architecture.md
packages:
    - biscuit-file
    - darkmatter
    - simplified-schema
source_files_during_phase_2:
    - simplified-schema/lib/Cargo.toml
    - simplified-schema/justfile
    - simplified-schema/lib/src/lib.rs
    - simplified-schema/lib/src/numeric.rs
    - simplified-schema/lib/src/numeric/ingress.rs
    - simplified-schema/lib/src/span.rs
    - simplified-schema/lib/src/pending.rs
    - simplified-schema/lib/src/expression/mod.rs
    - simplified-schema/lib/src/expression/scan.rs
    - simplified-schema/lib/tests/l1/main.rs
    - simplified-schema/lib/tests/l1/test_layout.rs
    - simplified-schema/lib/tests/l1/foundations.rs
    - Cargo.toml
    - Cargo.lock
    - justfile
    - darkmatter/lib/Cargo.toml
    - darkmatter/lib/src/markdown/numeric.rs
    - darkmatter/lib/src/markdown/numeric/ingress.rs
    - darkmatter/lib/src/markdown/span.rs
    - darkmatter/lib/src/markdown/literal_token.rs
    - darkmatter/lib/src/markdown/compose/expression/lexer.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/simplified_schema_reexports.rs
docs_updated_during_phase_2:
    - darkmatter/docs/dependencies.md
    - darkmatter/docs/inline/interpolation.md
    - docs/dependencies.md
    - darkmatter/features/2026-10-28-extract-simplified-schema/plan.md
    - darkmatter/features/2026-10-28-extract-simplified-schema/implementation-log.md
docs_created_during_phase_2:
    - simplified-schema/README.md
    - simplified-schema/docs/dependencies.md
    - simplified-schema/docs/topics/features.md
    - simplified-schema/docs/topics/foundations.md
skills_files_updated_during_phase_2:
    - .claude/skills/darkmatter/SKILL.md
source_files_during_phase_3:
    - simplified-schema/lib/Cargo.toml
    - simplified-schema/justfile
    - simplified-schema/lib/src/lib.rs
    - simplified-schema/lib/src/work_counters.rs
    - simplified-schema/lib/src/expression/mod.rs
    - simplified-schema/lib/src/expression/lexer.rs
    - simplified-schema/lib/src/expression/parser.rs
    - simplified-schema/lib/src/expression/ast.rs
    - simplified-schema/lib/tests/l1/main.rs
    - simplified-schema/lib/tests/l1/expression_syntax.rs
    - darkmatter/lib/Cargo.toml
    - darkmatter/lib/src/work_counters.rs
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/lexer.rs
    - darkmatter/lib/src/markdown/compose/expression/parser.rs
    - darkmatter/lib/src/markdown/compose/expression/ast.rs
    - darkmatter/lib/src/markdown/schemas/format.rs
    - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
    - darkmatter/lib/src/markdown/schemas/simplified/grammar.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/simplified_schema_reexports.rs
    - darkmatter/lib/tests/l1/schema_expression_syntax_checks.rs
docs_updated_during_phase_3:
    - simplified-schema/README.md
    - simplified-schema/docs/topics/foundations.md
    - simplified-schema/docs/topics/features.md
    - darkmatter/docs/topics/schemas/parsing/index.md
    - darkmatter/docs/topics/schemas/parsing/scanning.md
    - darkmatter/docs/topics/schemas/parsing/lexing.md
    - darkmatter/docs/topics/schemas/parsing/grammar.md
    - darkmatter/features/2026-10-28-extract-simplified-schema/plan.md
    - darkmatter/features/2026-10-28-extract-simplified-schema/implementation-log.md
docs_created_during_phase_3:
    - simplified-schema/docs/topics/expression-syntax.md
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/SKILL.md
message_to_agent: |-
    Phase 3 is done. `simplified_schema::expression` now holds `scan`, `lexer` (Token, ParseMode, ComparisonOp, LexerError, Lexer, lex_spanned, identifier_prefix_start), `parser` (parse, parse_spanned, parse_condition(_spanned), parse_callback(_spanned), ParseError, Parser, INVALID_CALLBACK_CODE, PRECEDENCE_TABLE), and `ast`, all re-exported at `simplified_schema::expression::*`. Darkmatter's `compose::expression` re-exports `ast` and `parser` as modules and the token items from `lexer.rs` (tagged `// migration: removed in Phase 7`); `ExpressionFinder`, `deferred.rs`, `error.rs`, and the evaluator stay.
    Points for Phase 4:
    (1) The schema checks in `schemas/format.rs`, `schemas/simplified/convert.rs`, and `schemas/simplified/grammar.rs` already import `simplified_schema::expression::…` directly; when they move, change those to `crate::expression::…`. Their check-level tests are in Darkmatter's `tests/l1/schema_expression_syntax_checks.rs` (they use Darkmatter's validation/lowering entry points, so they stay as cross-crate parity tests).
    (2) Work counters: the library has a test-only `work-counters` feature and `simplified_schema::work_counters` (`Work`, `add`, `snapshot`, `WorkSnapshot::{get, since}`) plus a crate-private `crate::note_work!(Kind)` macro (one unit; add a count arm if moved code needs `note_work!(Kind, n)`). When moved code calls `note_work!` (inventory section 2 lists the kinds: ValidatorCompile, AlphanumericCheckCompile, PositionMapBuild, ...), add the kind to the library's `Work` enum and `ALL`, and map it in `library_kind` in `darkmatter/lib/src/work_counters.rs` so Darkmatter's snapshot still reports it under its own `Work` variant. Darkmatter's `work-counters` already enables the library feature.
    (3) The library's `just test` runs with `--features work-counters`, then `--no-default-features`; `just lint` runs default, `work-counters`, and no-default-features clippy passes.
    (4) Phase 2 and Phase 3 are not committed yet: the tree holds both phases' changes, and the `git mv` renames are unstaged (they show as deletions plus untracked files).
---

# Implementation Log for 2026-10-28-extract-simplified-schema (9 phases)

## Phase 1

### 1.1 Boundary inventory

Written to `inventory.md` in this feature directory (the plan names a
`2026-10-25-…` directory that does not exist; the inventory sits beside the
spec). Headline findings that later phases must act on:

- **Pending codec must move (decision 4 → "required").** `format.rs` calls
  `literal_token::decode_leaf` (expression and shell formats) and
  `holds_pending_syntax` while JSON Schema format checks run, so Darkmatter
  cannot pre-classify. Phase 2.4 moves the pure `{{!data:v1:…}}` codec plus the
  plain `{{ }}` scan core of `ExpressionFinder` (`scan_plain`); the
  Markdown-aware finder and `is_deferred_template` stay.
- **`sniff` is needed by the `file` feature (decision 6).** Darkmatter captures
  repository roots through `sniff` (`GitRepo::discover`,
  `detect_repo_structure` in `context/request.rs`).
- **Owner-table corrections:** `file_match.rs`, `timings.rs`,
  `mapping_orders.rs`, `resolve_ctx::{document_file_context,
  resolve_document_file_ref}` move; `frontmatter_shape.rs`, `completion.rs`,
  `coerce.rs`, and `embedded.rs` split; `compose/expression/error.rs` stays
  whole (syntax errors are already `LexerError`/`ParseError`).
- **Nine "needs a fact" rows (F1–F9)** each have a decided home in the
  inventory; none introduces a callback trait.
- **Query surface:** no current helper parses a string path; the facade's
  string-path parser is new and maps onto `completion::def_at_path`
  (`PathStep::{Key, Index}`); empty path → `None`; dotted segment is always a
  literal key (no precedence rule exists to preserve).
- Consumers: strict only in Claudine's completion-phase validation; soft in
  Messenger `research/load.rs`, claudine-gen `inputs.rs`, claudine-cli
  `signals.rs`, DMLS diagnostics/overlay, `md schema validate`; everything
  else inspection-only.

### 1.2 Baseline

- host: macOS (Darwin 27.2.0), this worktree; toolchain `rustc 1.98.1
  (48a229cea 2026-09-01)`; profile `bench` (release), Criterion defaults from
  the bench files.
- commands, each run three times, sequentially:
  `cargo bench -p darkmatter --bench schema_validation` and
  `cargo bench -p darkmatter --bench effective_schema_ownership`.
- values are the median of the three runs' Criterion point estimates. Load
  average during the runs was 4–14 (other host activity plus the read-only
  inventory agent); treat differences under ~10% as noise.
- caveat: the darkmatter release compile for run 1 started within a minute of
  the 1.5 frontmatter edit (behavior-identical delegation to `biscuit-file`;
  not on any benchmarked path).

| Benchmark | Median |
| --- | --- |
| `effective_schema_ownership/darkmatter_baseline/baseline_only` | 5.4 µs |
| `effective_schema_ownership/darkmatter_baseline/baseline_plus_document` | 11.5 µs |
| `effective_schema_ownership/darkmatter_baseline/clone_effective` | 4.7 µs |
| `effective_schema_ownership/synthetic_512_properties/baseline_only` | 24.3 µs |
| `effective_schema_ownership/synthetic_512_properties/baseline_plus_document` | 30.9 µs |
| `effective_schema_ownership/synthetic_512_properties/clone_effective` | 11.8 µs |
| `effective_schema_ownership/control/document_only` | 10.6 µs |
| `effective_schema_initialization/darkmatter_baseline/owned_accessor` | 191.1 µs |
| `effective_schema_initialization/darkmatter_baseline/configure_default_baseline` | 334.1 µs |
| `effective_schema_initialization/darkmatter_baseline/configure_and_resolve_default_baseline` | 338.4 µs |
| `effective_schema_initialization/darkmatter_baseline/merge_empty_document` | 756.4 µs |
| `effective_schema_initialization/darkmatter_baseline/merge_tiny_document` | 750.4 µs |
| `schema_validation/corpus_1000/validate_1000_docs` | 18.14 ms |
| `schema_validation/corpus_1000/validate_1000_docs_warm_cache` | 18.40 ms |
| `schema_validation/claudine_cold/no_trigger_match` | 22.06 ms |
| `schema_validation/claudine_cold/document_events` | 22.68 ms |
| `schema_validation/claudine_cold/nested_sequence_group` | 24.28 ms |
| `schema_validation/claudine_warm/no_trigger_match` | 1.250 ms |
| `schema_validation/claudine_warm/document_events` | 1.361 ms |
| `schema_validation/claudine_warm/nested_sequence_group` | 1.338 ms |
| `resolve_frozen/unrolled_depth5` | 8.18 ms |
| `resolve_frozen/recursive` | 2.43 ms |
| `resolve_frozen_io/unrolled_depth5` | 25.3 µs |
| `resolve_frozen_io/recursive` | 23.0 µs |
| `resolve_frozen_compile/unrolled_depth5` | 5.18 ms |
| `resolve_frozen_compile/recursive` | 1.76 ms |

### 1.3 Characterization tests

`darkmatter/lib/tests/l1/schema_extraction_characterization.rs` (declared in
`tests/l1/main.rs`), 90 L1 tests, all green on unmodified Darkmatter code:
`yaml_loader_matrix` 46 (one `#[test]` per cell via `macro_rules!`),
`json_schema_loader_matrix` 21, `frontmatter_schema_field` 8,
`query_edge_cases` 6, `r6_table` 4, `function_schema` 4, `immutability` 1.

Entry points pinned: strict = `coercion::check_exact`/`accepts_exactly`
(value level) and `EffectiveSchema::validate_raw` /
`validate_for_phase(.., Completion)` (frontmatter level); soft =
`coercion::bind` (`Bound::Kept` = borrowed, `Bound::Converted` = owned) and
`EffectiveSchema::validate` / `validate_for_phase(.., Launch)`; YAML loader =
`parse_standalone_schema_document` (+ `parse_property_definition`,
`parse_yaml_schema`); raw JSON Schema loader = `resolve::resolve_schema` on a
`./schema.json` in a tempdir.

Pinned outcomes that look wrong (asserted as-is; parity authority, not fixed):

1. YAML `kind: schema` without `types` → `SchemaDocument` "found unsupported
   keys: " with an empty list (the "missing `types`" message is unreachable).
2. Leading BOM: YAML loader → "more than one document"; JSON loader →
   "invalid JSON: expected value".
3. Frontmatter `$schema:` and `$schema: null` → `FrontmatterShape` (not absent).
4. Unknown type word (`numbr`) parses as a local named reference, not a
   grammar error.
5. Null property type parses as `null`; `{}` parses as an empty inline object.
6. JSON loader: `$schema` null/`5`/`""`/`{}`, a draft-07 URI, `"kind":
   "schema"`, and a remote `$ref` all load; only a remote `$schema` reference
   is `RemoteUnsupported`.
7. Raw JSON load errors say "invalid standalone schema document".
8. A whole-file `$schema` reference with a sibling `types` map is accepted.
9. `serialize_property_def` drops spaces (`number(min(0);integer)`).
10. Two `suggest` constraints across one union are rejected by the grammar.

R6 row 3 (`number(integer)` + `"2.5"`): exact → `ConversionNotAllowed`; bind →
`ConstraintNotMet`; input unchanged.

### 1.4 biscuit-file feature split

- `file-reference-syntax = ["dep:globset", "url"]`;
  `file-reference = ["file-reference-syntax", "dep:gix", "dep:walkdir"]`.
- `cargo tree -p biscuit-file --no-default-features --features
  file-reference-syntax -e normal`: `globset` present, no `gix`, no `walkdir`.
- Syntax feature: `FileReference` (`new`/`raw`/`class`/`payload`/builders),
  kind/class/error types, `ResolutionFailure`, `ResolutionCandidate`,
  `RootProvenance`, `PathPosition`, `MagicPathTier`, `GlobReference`
  parsing, `GlobReferenceError`, `FileResolutionContext` and its data types,
  `PathIdentity`, `RelativeRoute`, `PortabilityPreference`, `IntentForms`,
  `home_dir`, `user_cache_dir`, `capture_env`.
- `file-reference` only: every resolving method, detailed resolution and
  completion types, `Resolved`, glob listing/matching
  (`matches_without_context` included), `PortablePath` and its diagnostics,
  `find_git_root`, `FileReferenceError::Git`.
- Two lexical path helpers moved from `resolve.rs` to a new
  `file_reference/lexical.rs`; `FileReferenceError::resolution_failure` moved
  to `error.rs`. Paths and type identities unchanged.
- Incidental fixes needed for `--all-targets` in reduced builds: the `parsers`
  bench declares `required-features = ["toml", "yaml", "json5"]`;
  `yaml/types.rs` `test_max_depth_exceeded_toml` gated on `toml`; `--features
  url` alone now compiles (`Resolved` export moved under `file-reference`).
- New L1 module `tests/l1/reference_syntax_only.rs` (6 tests, ungated);
  resolution modules in `tests/l1/main.rs` gated on `file-reference`.
  `just lint` adds a clippy pass for `--no-default-features --features
  file-reference-syntax,yaml`; `just test` (`test-minimal`) adds a nextest pass
  over that module in that configuration.
- Verified by the subagent: `just test` 1187/1187 + 9/9 no-default + 6/6
  syntax-only, doctests 28/28, `just lint` clean, Windows-gnu clippy clean in
  both configurations; `cargo check` clean for darkmatter, claudine,
  messenger, worktree, tree-hugger, biscuit-terminal, content-policy.

### 1.5 Passive frontmatter helper

- New `biscuit_file::frontmatter` (`extract_frontmatter_block`,
  `FrontmatterExtraction`, `FrontmatterEnvelopeError::FenceMismatch`,
  `line_spans`/`LineSpan`, `detect_near_miss_fence`), gated on `yaml` only
  (see decisions). 13 unit tests moved/added there: CRLF, lone CR, BOM, BOM
  does not weaken delimiters, `----` near-miss typed error, dash runs that are
  not frontmatter, missing closer, empty block, body-less, junk after the
  closer stays body, line spans for every terminator.
- `darkmatter::markdown::extract_frontmatter_block` keeps its signature and
  maps `FenceMismatch` to `MarkdownError::FrontmatterFenceMismatch`;
  `FrontmatterExtraction` is re-exported from `biscuit-file` (same path in
  Darkmatter, one type). `parse_frontmatter` uses the shared `line_spans` and
  `detect_near_miss_fence`. Darkmatter keeps the near-miss mapping test and the
  presence-agreement test.
- `cargo check -p biscuit-file --no-default-features --features yaml` clean.

### Phase gates

- `biscuit-file`: `just lint` clean (including the new syntax-only clippy
  pass); `just test` 1187/1187 L1 + 13 new `frontmatter::tests` within them,
  9/9 no-default `path_text`, 6/6 syntax-only `reference_syntax_only`.
- `darkmatter`: `just lint` clean (including the zed-dmls wasm32 check);
  `just test` 13849 passed, 10 skipped, 0 failed (includes the 90
  characterization tests, dmls and darkmatter-cli).
- Dependents of `biscuit-file`: `cargo check` clean for darkmatter, claudine,
  messenger, worktree, tree-hugger, biscuit-terminal, content-policy (the
  default build's public surface is unchanged, so no dependent test run was
  needed beyond darkmatter's full suite).
- Cross-OS: `just cross-check biscuit-file --os windows` passed (1114/1114 on
  build-win-native; no observation was published because the remote tree was
  this uncommitted working tree). Windows-gnu clippy was clean in both feature
  configurations.
- The CI scope calculator was not used to skip anything: source files changed,
  so both gates ran.

## Phase 2

Done inline by the orchestrator (no subagents): the moves were small and
sequentially dependent, so splitting them would only have added coordination.

### 2.1 Package area

- New area `simplified-schema/` (library only): `lib/Cargo.toml` (package
  `simplified-schema`, lib `simplified_schema`, edition 2024, `AGPL-3.0-only`,
  `autotests = false`, one `[[test]] l1` root), `README.md` (overview),
  `docs/topics/foundations.md`, `docs/topics/features.md`,
  `docs/dependencies.md`, and a `justfile` modeled on `biscuit-contract`.
- Features: `default = ["file"]`, `file = ["biscuit-file/file-reference",
  "dep:sniff"]`. Normal dependencies are only what moved code uses:
  `tracing`, `serde`, `serde_json` (`preserve_order`, `raw_value`,
  `float_roundtrip`), `thiserror`, `base64`, `biscuit-file`
  (`default-features = false`, `yaml`, `json5`, `file-reference-syntax`), and
  optional `sniff`. `indexmap` and `jsonschema` are not added yet: nothing that
  moved in this phase uses them (they arrive with the modules that do).
  `json5` is needed by `reject_non_finite_json5`.
- `sniff` is declared now (decision 6, confirmed by the inventory) but unused
  until the capture module lands; see decisions_for_review.
- `just test` runs the default suite and then `test-minimal`
  (`cargo nextest run -p simplified-schema --no-default-features`, skipped in
  archive mode like biscuit-file's). `just lint` adds a
  `--no-default-features --all-targets` clippy pass. `bench` is a
  not-applicable stub until benches exist.
- Registered in the root `Cargo.toml` members and the root `justfile`
  `areas :=`. `sniff repo package-areas` lists `simplified-schema`;
  `just check-canonical` passes (13 canonical recipes);
  `just check-tier-coverage simplified-schema` reports nothing stranded.
  No CI manifest enumerates areas by hand: `scripts/ci/affected_scope.py`'s
  `("claudine", "darkmatter", "sniff", "playa")` tuple is specialized-job
  flags and `scripts/ci/consolidation.py`'s list is the historical
  test-consolidation tool; neither needs the new area.
- No `[package.metadata.ci.tests] source-inputs` yet: no test of this crate
  reads another package's file (Phase 8 revisits).

### 2.2 Numeric model

- `git mv darkmatter/lib/src/markdown/numeric.rs` and `numeric/ingress.rs` to
  `simplified-schema/lib/src/numeric.rs` / `numeric/ingress.rs`. Inline tests
  moved with them unchanged (8 + 6).
- Widened to `pub` in the library (Darkmatter's moved schema code and
  frontmatter loader need them across the crate line): `load_admitted_yaml`,
  `is_yaml_admission_error`, `admit_json_text_numbers`, `JsonNumberError`
  (field `reason` gained a doc line).
- `darkmatter/lib/src/markdown/numeric.rs` is now a re-export file
  (`// migration: removed in Phase 7`): the public items keep their public
  paths, the three loader helpers stay `pub(crate)` in Darkmatter.
- Doc drift fixed: module header no longer says "Darkmatter's" model; doc
  examples use `simplified_schema::numeric`. The internal
  `NON_FINITE_MARKER` string (`"darkmatter: non-finite number"`) is unchanged
  on purpose: it never surfaces (it is matched and converted to
  `NotFinite`), and the extraction is behavior-preserving.
- Named boundary cases: `numberlike`/integer boundary — `admit_integer(2^53)`
  is exact and `2^53 + 1` is `Inexact`; `9007199254740993.0` in any spelling
  is `Inexact`; `1e400` is `NotFinite`; `pear` is `NotANumber` (new L1
  `foundations::integer_admission_boundary_is_two_to_the_fifty_third`). The
  existing `numeric::tests::whole_numbers_are_exact_in_every_spelling`,
  `decimals_round_and_extremes_are_rejected`,
  `json_numbers_and_supplied_floats` (i128/f64 cases) moved unchanged.
  Darkmatter's `tests/l1/numeric_admission.rs` (compose, frontmatter, and raw
  schema entry points) stays in Darkmatter as the cross-crate parity test.

### 2.3 Span vocabulary

- `git mv darkmatter/lib/src/markdown/span.rs simplified-schema/lib/src/span.rs`:
  `Spanned<T>`, `line_of_offset`, `line_col_of_offset`, and the
  `SourceSpan` re-export (still `biscuit_file::SourceSpan`). Its 9 inline
  tests moved unchanged.
- `newline_offset_table` and `line_at_offset` (`pub(crate)`, used only by
  Darkmatter's `toc` and `compose/remote.rs`, which stay) were kept in a new,
  smaller `darkmatter/lib/src/markdown/span.rs` that re-exports the library's
  items. Not migration-tagged: `darkmatter::markdown::span::*` and the
  crate-root `darkmatter::{SourceSpan, Spanned, line_of_offset,
  line_col_of_offset}` are Darkmatter's public API (pinned by
  `span_compat`).
- Criterion 8 test: `foundations::spans_are_byte_offsets_into_crlf_text_with_windows_paths`
  (a `C:\…` path and a `\\?\C:\…` verbatim path on CRLF lines; CRLF counts as
  one break; byte columns; an offset between `\r` and `\n`).

### 2.4 Pending codec (required, per the inventory)

- `git mv darkmatter/lib/src/markdown/literal_token.rs
  simplified-schema/lib/src/pending.rs`, whole file (all 11 inline tests).
- The region-free scan core of `ExpressionFinder` moved to
  `simplified_schema::expression::scan`: `ExpressionLocation`,
  `InterpolationLiteral`, `LiteralTokenLocation`, `ExpressionScanResult`, and
  free functions `scan(content, excluded)`, `scan_plain`, `find_all_plain`
  (code-region ranges are an argument: inventory fact F8). The scanning
  logic is byte-for-byte the old `ExpressionFinder::scan` /
  `scan_legacy_expression` / `is_escaped_opener`, with `self.` fields turned
  into parameters. `expression` is the module Phase 3 extends with the lexer
  and parser.
- Darkmatter's `ExpressionFinder` keeps `new` (Markdown code-region
  detection), `with_code_blocks`, `find_all`, `code_regions`; `scan`,
  `scan_plain`, and `find_all_plain` delegate to the library, and the four
  result types are re-exported from `compose::expression::lexer` (same
  paths). Its 2,400-line lexer test module stays in Darkmatter and still
  drives the scan through `ExpressionFinder`.
- `pending::first_token` widened to `pub` (Darkmatter's
  `compose/frontmatter_interpolation.rs` calls it).
- `darkmatter/lib/src/markdown/literal_token.rs` is now a re-export of
  `simplified_schema::pending` (not migration-tagged: the inventory keeps
  `darkmatter::markdown::literal_token` as a lasting path because Claudine
  and Darkmatter compose use it widely); `first_token` is re-exported
  `pub(crate)` so Darkmatter's public API does not grow.
- Minimal test edits (listed per the Move Protocol): in `pending.rs` tests,
  `yaml_value` parsed through `Markdown::from` frontmatter; it now parses
  through `numeric::load_admitted_yaml` and `serde_yaml_ng::from_value` into a
  JSON map, which is the exact conversion Darkmatter's frontmatter loader
  performs (`parse_yaml_map_rejecting_duplicates`). To keep the Darkmatter
  frontmatter fact pinned too, Darkmatter's `literal_token.rs` keeps two
  tests that load through `Markdown::from`:
  `the_yaml_scalar_loads_back_from_frontmatter_as_the_bare_token` and the
  proptest `the_yaml_scalar_round_trips_through_frontmatter`. Scan calls
  changed from `ExpressionFinder::scan_plain` to `scan_plain`.

### Tests added (requirement → test)

| Requirement | Test |
| --- | --- |
| Scan core is Markdown-free and honors caller exclusions (F8) | `expression::scan::tests::excluded_ranges_hide_only_the_openers_inside_them` |
| Scan semantics preserved (offsets, nesting, unclosed, literals, tokens, escapes) | `expression::scan::tests::*` (10) |
| Scan offsets are bytes of the supplied CRLF text | `expression::scan::tests::offsets_are_bytes_of_the_supplied_text_including_crlf` |
| Spans over CRLF + Windows paths (criterion 8) | `foundations::spans_are_byte_offsets_into_crlf_text_with_windows_paths` |
| Numeric boundaries through the public API | `foundations::integer_admission_boundary_is_two_to_the_fifty_third` |
| Loader admission errors keep their location (YAML path; JSON pointer/line/column on CRLF) | `foundations::loaders_reject_inadmissible_numbers_with_a_location` |
| Codec and scan agree (whole leaf = data; embedded = pending; escaped = text) | `foundations::a_whole_leaf_token_is_data_and_never_pending` |
| Darkmatter paths are the library types; finder passes Markdown regions to the scan | DM `tests/l1/simplified_schema_reexports.rs` (2) |
| Token round trip through Darkmatter's frontmatter loader | DM `markdown::literal_token::tests` (2) |
| Every test file is compiled by a declared target | `test_layout::every_test_source_is_compiled_by_a_declared_target` |

Boundary and fidelity: the crate line (Darkmatter → simplified-schema) is
the boundary; F1 in-process tests at each crate's public API, L1. No CLI,
filesystem, or persistence boundary changed. The Input Robustness Matrix
loaders did not change in this phase (number admission moved unchanged; its
existing cells moved with it).

### Checkpoint

- `cargo tree -p simplified-schema --no-default-features -e normal`: no
  `darkmatter`, `claudine`, `content-policy`, `gix`, `walkdir`, `sniff`;
  `globset` present by the Phase 1 decision (the checkpoint's globset clause
  is waived, as the Phase 1 handoff said). The default build holds `gix`,
  `walkdir`, `sniff` through `file`, and no Darkmatter-family crate.

### Phase gates

- `simplified-schema`: `just test` 49/49 (default) + 49/49
  (`--no-default-features`); doctests 6/6; `just lint` clean in both
  configurations; `just check-tier-coverage simplified-schema` nothing
  stranded; `just check-canonical` 28/28 areas.
- `darkmatter`: `just lint` clean (including dmls, darkmatter-cli, and the
  zed-dmls wasm32 check); `just test` 13819 passed, 10 skipped, 0 failed
  (Phase 1: 13849; minus the 34 inline tests that moved to the library, plus
  4 new Darkmatter tests). After narrowing the `first_token` re-export to
  `pub(crate)`, clippy (`--all-targets`, all CI features) was rerun clean and
  the `literal_token` / `frontmatter_interpolation` unit tests (86) pass.
- Dependents: `cargo check --all-targets` clean for claudine, claudine-cli,
  claudine-gen, messenger. Targeted runs over the code that calls the moved
  symbols: claudine (`composition::closure`, `file_detail`, `schema`,
  `dispatch::template`, `lifecycle::tests::nested_span`, `looping`,
  `claudine_schemas_embedded`) 854/854; claudine-cli (`agent_text_is_data`,
  `override_boundary_guard`) 37/37. Messenger calls none of the moved
  symbols, so its compile check stands in for a test run.
- Cross-OS: `just cross-check simplified-schema --os windows` 49/49 on
  build-win-native.
- No test failed or flaked during the phase.
- The gates were not skipped: source files changed, so the CI scope
  calculator was not consulted for a skip.
- `git mv` staged the renames; the index was reset afterwards (nothing is
  staged), so the moves show as deletions plus untracked files.

## Phase 3

Done inline by the orchestrator (no subagents): the three tasks were
sequential (split, then move, then repoint) and touched the same files.

### 3.1 Split `lexer.rs`

- `git mv compose/expression/lexer.rs simplified-schema/lib/src/expression/lexer.rs`
  (the token lexer is the larger half, so history follows it), then
  Darkmatter's `lexer.rs` was rewritten with only the Markdown half:
  `ExpressionFinder` (`new`, `with_code_blocks`, `find_all`, `scan`,
  `code_regions`, `find_all_plain`, `scan_plain`, `find_code_regions`) and
  its three test modules (`expression_finder`, `interpolation_literal`,
  `backslash_escape`), unchanged.
- Moved: `Token`, `ParseMode`, `ComparisonOp`, `LexerError`, `Lexer`,
  `lex_spanned`, `identifier_prefix_start`, `is_identifier_start/char`, and
  the eleven lexer test modules (`lexer_tokens` … `condition_mode_tokens`),
  unchanged. The lexer's number admission now calls
  `crate::numeric::admit_number_spelling` / `NumberTextError` in the library.
- `is_deferred_template`, `defer_templates`, `is_malformed_template`, and
  `is_deferred_value` were never in `lexer.rs` (they are in `deferred.rs`) and
  stay in Darkmatter.

### 3.2 Move tokens, AST, parser

- `git mv` of `ast.rs` and `parser.rs` into `simplified_schema::expression`.
  Import edits only: `crate::markdown::span` → `crate::span`,
  `crate::markdown::numeric::format_number` → `crate::numeric::format_number`,
  `super::{LexerError, ParseMode, Token, ComparisonOp}` → `super::lexer::…`,
  two test-local `use crate::markdown::compose::expression::…` →
  `crate::expression::…`. Doc examples now name `simplified_schema::expression`.
- Minimal test edit (Move Protocol): `parser::tests::raw_literal_block_indentation`
  `include_str!`s two Claudine fixtures
  (`claudine/cli/tests/fixtures/nested_span_regression/{review-spec-inline,commit}.md`);
  the relative path lost two `../` segments. See decisions_for_review.
- `PRECEDENCE_TABLE` widened from `pub(crate)` to `#[doc(hidden)] pub`
  (Darkmatter's `semantics` test compares its operator catalog with it); its
  `#[allow(dead_code)]` was dropped and its doc now says the mirror is in
  Darkmatter.
- New library module docs: `expression/mod.rs` states the module is *syntax
  only* (decision 5's "documented as syntax only") and re-exports the same
  item set Darkmatter's `compose::expression` re-exported, so
  `simplified_schema::expression::{parse, Expr, …}` works.
- Darkmatter: `compose/expression/mod.rs` replaces `pub mod ast;` / `pub mod
  parser;` with `pub use simplified_schema::expression::{ast, parser};`
  (module re-exports keep every `expression::ast::…` / `expression::parser::…`
  path, including `parser::INVALID_CALLBACK_CODE` and `PRECEDENCE_TABLE`), and
  `lexer.rs` re-exports the token items. These three are tagged
  `// migration: removed in Phase 7`; the existing `pub use ast::{…}`,
  `pub use lexer::{…}`, `pub use parser::{…}` lines in `mod.rs` are unchanged
  and resolve through them.
- `error.rs` stays whole (inventory finding 11): the only syntax errors are
  `LexerError` and `ParseError`, which moved with their modules.
  `ExpressionError`'s `From<ParseError>` (in Darkmatter's `error.rs`) now
  converts the library type; no inherent or foreign-trait impl on a moved
  type exists outside the moved files (checked with `rg`).
- **Work counter (inventory F5).** `Parser::with_mode` calls
  `note_work!(ExpressionParse)`. The library gained a test-only
  `work-counters` feature and a `simplified_schema::work_counters` module
  (`Work::ExpressionParse`, `add`, `snapshot`, `WorkSnapshot::{get, since}`)
  plus a crate-private `note_work!`. Darkmatter's `work-counters` feature now
  enables `simplified-schema/work-counters`, and
  `darkmatter::work_counters::snapshot()` adds the library's count into
  Darkmatter's own `Work::ExpressionParse` slot, so every reader
  (`prepared::tests::counted`, Claudine's `preflight::tests::scaling`, the
  `pipeline_work_alloc` bench) is unchanged. The library's
  `[package.metadata.ci.tests] features = ["work-counters"]`, and its `just
  test` / `just lint` now include the feature.

### 3.3 Syntax-only schema checks

- The three checks stay in Darkmatter until Phase 4 moves `schemas/`; they
  now import the library parser directly rather than through Darkmatter's
  re-export: `schemas/format.rs` (`darkmatter-expression` format,
  `parse_condition`), `schemas/simplified/convert.rs` (`expression` default,
  `parse_condition`), `schemas/simplified/grammar.rs` (inline
  `example(call => result)`, `parse_spanned` + `literal_to_json`).
- Verified passive by reading: each calls only a parse function and (for the
  example's expected side) folds literal nodes into JSON; none evaluates,
  runs an example, or looks up a function or variable.

### Tests added (requirement → test)

| Requirement | Test |
| --- | --- |
| Expression-typed value / default parse accepts an unresolvable call (`unknown_fn(1)`) | lib `expression_syntax::an_unresolvable_call_is_an_expression_typed_value_and_default` |
| Function-example call parse accepts `unknown_fn(1)`; spans and `erase` agree | lib `expression_syntax::an_unresolvable_call_is_a_function_example_call` |
| Example expected literal parses to literal nodes | lib `expression_syntax::an_example_expected_literal_parses_to_literal_nodes` |
| Malformed text is rejected with a byte position (parser and lexer errors) | lib `expression_syntax::malformed_text_is_rejected_with_a_byte_position` |
| The dialects differ only in `\|\|` (parser and lexer) | lib `expression_syntax::the_two_dialects_differ_only_in_how_or_parses` |
| Criterion 8: spans over CRLF text with a Windows path | lib `expression_syntax::spans_are_byte_offsets_into_crlf_text_with_windows_paths` |
| Library counts each parse once (success or failure) | lib `expression_syntax::each_parse_is_counted_once_whatever_the_outcome` (`work-counters`) |
| Darkmatter folds the library count into `Work::ExpressionParse` | existing DM `compose::expression::prepared::tests::counted::*` (would read 0 parses if the fold broke) |
| Value check accepts `unknown_fn(1)`, rejects `unknown_fn(` with the format named | DM `schema_expression_syntax_checks::an_expression_typed_value_calling_an_unknown_function_validates` |
| Default check accepts `unknown_fn(1)`, rejects `unknown_fn(` as `SchemaError::Convert` | DM `schema_expression_syntax_checks::an_expression_default_calling_an_unknown_function_lowers` |
| Example check accepts `unknown_fn(1) => {…}`, rejects a malformed call and a non-literal result | DM `schema_expression_syntax_checks::a_function_example_calling_an_unknown_function_parses` |
| Darkmatter's expression syntax paths are the library's types and functions | DM `simplified_schema_reexports::the_darkmatter_expression_syntax_paths_are_the_library_parser` |

Boundary and fidelity: the crate line (Darkmatter → simplified-schema) is the
boundary; F1 in-process tests at each crate's public API, L1. The DM tests
anchor validation in a temp dir (`context_at`), not the cwd. No CLI,
filesystem, or persistence behavior changed. The Input Robustness Matrix does
not apply: the expression grammar is a text grammar, and no loader changed.

### Checkpoint

- `rg "fn parse_spanned"` finds one grammar: the `Parser::parse_spanned`
  method and the free `parse_spanned`, both in
  `simplified-schema/lib/src/expression/parser.rs`. No `Token` enum,
  `lex_spanned`, or `parse_condition_spanned` remains in Darkmatter.
- Darkmatter's expression tests are unchanged: the finder tests stayed, the
  lexer/parser/ast inline tests moved with their modules (no assertion
  edited), and the integration tests that use `compose::expression::…` paths
  compile against the re-exports.

### Docs

- New `simplified-schema/docs/topics/expression-syntax.md`; README,
  `foundations.md` (diagram), and `features.md` (`work-counters`) updated.
- `darkmatter/docs/topics/schemas/parsing/{index,scanning,lexing,grammar}.md`
  pointed at the new source locations. Their old links (`../../../lib/…`)
  were one directory level short and never resolved; the new links were
  checked to exist.
- `.claude/skills/darkmatter/SKILL.md` boundary note extended to the
  expression syntax and the folded counter.

### Phase gates

- `simplified-schema`: `just test` 317/317 (default + `work-counters`) and
  316/316 (`--no-default-features`; the counter test compiles out);
  `just doctest` 12/12; `just lint` clean (default, `work-counters`,
  no-default-features); `just check-tier-coverage simplified-schema` nothing
  stranded.
- `darkmatter`: `just lint` clean (lib, cli, dmls, zed-dmls wasm32), plus
  `cargo clippy -p darkmatter --all-targets --features
  effects-instrumentation,work-counters,terminal-tests,browser-tests -D
  warnings` clean; `just test` 13562 passed, 10 skipped, 0 failed (Phase 2:
  13819; the about 268 moved lexer/parser/ast inline tests now run in the
  library, and 4 Darkmatter tests were added).
- Dependents: `cargo check --all-targets` clean for claudine, claudine-cli,
  claudine-gen, messenger. Claudine targeted run with `work-counters`
  (`preflight::tests::scaling`, which reads `Work::ExpressionParse` through
  the fold, plus `composition::closure`, `dispatch::template`,
  `lifecycle::tests::nested_span`, `looping`): 309/309.
- Cross-OS: `just cross-check simplified-schema --os windows` 317/317 on
  build-win-native.
- No test failed or flaked. The gates were not skipped (source files
  changed). The `git mv` renames were unstaged; nothing is staged.
