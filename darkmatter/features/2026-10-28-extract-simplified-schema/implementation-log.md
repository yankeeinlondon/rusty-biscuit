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
message_to_agent: |-
    Phase 1 is done. Read darkmatter/features/2026-10-28-extract-simplified-schema/inventory.md before starting Phase 2. It corrects the plan's owner table, and the corrections that affect Phase 2 are:
    (1) Task 2.4 is REQUIRED, not conditional. format.rs decodes pending tokens while validating, so the pure {{!data:v1:...}} codec (literal_token encode/decode_leaf/holds_pending_syntax, plus TokenError) must move, together with the plain `{{ }}` scan core of ExpressionFinder (scan_plain) that it uses. The Markdown-aware finder and is_deferred_template stay in Darkmatter.
    (2) Decision 6 is confirmed: the `file` feature needs an optional `sniff` dependency (Darkmatter captures repository roots via sniff in context/request.rs).
    (3) biscuit-file now has a `file-reference-syntax` feature (FileReference parsing, GlobReference parsing, FileResolutionContext as a data type; it pulls in `globset` and `url` but not `gix` or `walkdir`). simplified-schema's no-default-features build should depend on biscuit-file with `default-features = false, features = ["yaml", "file-reference-syntax"]`. PortablePath and matches_without_context still require full `file-reference`.
    (4) The minimal build DOES contain `globset`, by decision (see decisions_for_review). Adjust the Phase 2 checkpoint's cargo-tree check to forbid only gix and walkdir.
    (5) biscuit_file::frontmatter (yaml feature) now holds extract_frontmatter_block; Darkmatter wraps it.
    (6) darkmatter/lib/tests/l1/schema_extraction_characterization.rs holds 90 tests that pin current behavior (the parity authority). Move them with their modules; do not change their asserted outcomes.
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
