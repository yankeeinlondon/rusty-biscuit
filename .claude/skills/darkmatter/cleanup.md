# Cleanup rules and frontmatter repair

Cleanup (`md clean`, `md FILE --save`, DMLS `textDocument/formatting`,
compose's cleanup step, `md` rendering) never changes a document's meaning.
The user-facing record is the area's `docs/` tree.

- `markdown::cleanup::clean_document(&CleanRequest, schema_closure)` is the
  raw-source transaction both `md clean` and DMLS call: frontmatter repair,
  one passive read of `style.markdown` (authored YAML, or the repair
  candidate when that is the only parseable form), one `resolve`, guarded
  body cleanup, reassembly from the emitted frontmatter bytes. Outcomes:
  `Changed` (text plus `applied` repairs), `Unchanged`, `Declined` (source
  returned, `applied: []`, last diagnostic `clean.fidelity-fallback` whose
  `fallback` carries reason, document offset and detail;
  `Fallback::message(line, column)` restates it in a caller's coordinates),
  `Invalid` (Darkmatter's own frontmatter parser cannot load the block to
  emit, including `frontmatter: false` over YAML only the candidate could
  read; source returned, nothing cleaned). Strict YAML is not the acceptance
  test: a block only `frontmatter::load_frontmatter_yaml` loads (bare
  `{{ … }}` values, the fallbacks `Markdown` uses) is kept
  as authored with no repair (tab indentation is not in this set: Biscuit
  File's `yaml.tab-indentation` repair applies), its rules are read from that mapping, and the
  body is cleaned; the strict `yaml.parse` finding stays report-only. Empty frontmatter blocks and BOMs without
  frontmatter are kept; a BOM before frontmatter is an applied repair.
- String wrappers: `cleanup_content` (defaults) and
  `cleanup_content_with(&MarkdownRuleOverrides)` (overrides at CLI level),
  plus `Markdown::cleanup`/`cleanup_with`. They return the input on refusal.
  Compose maps its options (`IncidentalNewlineMode`, `with_list_spacing`
  taking `ListItemPadding`, `with_indentation`, `with_fixed_width`) to explicit
  overrides; a document's own `style.markdown` is not consulted there.
  `strip_incidental_newlines` and `reflow_to_width` stay as standalone,
  unguarded helpers (they normalize CRLF); nothing in cleanup calls them.
- Parse budget: cleanup parses go through `cleanup_parser`; directive and
  expression scanners derive code regions from one
  `compose::parse_utils::scan_code_regions` per text (`Protection::scan`
  passes them to each `*_with_code_regions` scanner), counted separately by
  `scanner_parse_count`. `tests/parse_count.rs` pins both.
- `markdown::cleanup::rules` is the single validator for `style.markdown`
  (document), DMLS `formatting.markdown` (workspace), and the style parser's
  registration. Read values into `Leaf<T>` (absent, null, value); never
  through `#[serde(default)]` options. `resolve` picks each rule
  independently: CLI, document, workspace, runtime default. The baseline
  schema's booleans are literal unions so schema coercion cannot accept a
  quoted value the validator rejects; `base_schema_end_to_end` pins that
  agreement.
- `markdown::cleanup::frontmatter::repair_frontmatter` is the raw-source repair
  both consumers call. Its schema configuration is a lazy closure, so absent or
  empty frontmatter never builds a context. Clean diagnostics
  (`cleanup::diagnostic`) use document coordinates.
- `markdown::cleanup::clean_body(body, &ResolvedMarkdownRules)` is the guarded
  body engine. It never re-serializes Markdown: rules propose bounded byte
  edits against one parse, and one final comparison accepts the candidate or
  returns the input (`CleanOutcome::Declined` plus a `clean.fidelity-fallback`
  finding; `BodyCleanup::fallback` re-anchors it into a document with
  `Fallback::diagnostic(document, body_offset)`). Modules, in pipeline order:
  `protect` (directive lines, quoted `::shell`, and shell-block regions are
  removed from a parse view; expressions are inline-protected), `structure`
  (the block model in source coordinates), `transform` (nine body rules),
  `wrap` (fixed-width, a second stage), `edits` (apply with overlap and
  protection checks), `compare` (the guard).
- Rule-writing traps the corpus found, each now handled:
  pulldown-cmark strips a lazy continuation line's indentation only up to the
  containers it still satisfies (moving it changes code-span text); items of
  one list can sit at different columns; a quote's bare `>` line must move with
  its quote; `ExpressionFinder` can match an "expression" across many lines,
  so rules leave any structure such a span crosses as authored; a bare `|`
  inside a code span still splits a table cell. A rule that cannot apply
  safely leaves the structure as written; the guard is the backstop, and a
  refusal on valid syntax is a rule bug to fix, not an acceptable outcome.
- The old serializer pipeline (`lists`, `emphasis`, `brackets`,
  `blockquote`, `tables`) is deleted; do not reintroduce pulldown-cmark-to-cmark
  re-serialization into cleanup. Ambient `PREFER_ITALICS` restyling is gone;
  `context_construction_guard` no longer allowlists an environment read for it.
- Comparison keys never use `Debug` of pulldown's `CowStr` (its storage
  variant differs for equal values); `compare::tag_key`/`event_key` spell
  fields out.
- User-facing docs: `darkmatter/docs/topics/markdown-formatting.md` (every
  rule, list spacing, precedence), `docs/cli/clean.md`, `docs/inline/cleaning.md`
  (compose's step), and `dmls/docs/editors/README.md` (DMLS formatting and
  `formatting.markdown`). Regenerate their examples from `md clean -` rather
  than writing them by hand.
- Corpus evidence: a passive `clean_document` scan over `git ls-files '*.md'`
  is a throwaway probe, never a CI gate. Run it from a release build
  (`cargo nextest list --cargo-profile release` builds the `l1` binary, then
  run it directly with `--ignored --exact`) and a work queue, not static
  chunks: the debug build takes minutes and a few large frontmatter documents
  dominate (the schema tier, not the body engine). Real `prompts/commit.md`
  coverage lives in `lib/tests/l1/clean_real_world_prompt.rs`.
- Tests: `cleanup/tests/{fidelity_repro,directive_fixed_points,rules,guard,source_fidelity,parse_count}.rs`,
  `cleanup::document::tests`, and the shared parity matrix
  `lib/tests/common/clean_parity/mod.rs`. The library (`lib/tests/l1/clean_parity.rs`),
  `md clean` (`cli/tests/l1/clean_parity.rs`) and DMLS formatting
  (`dmls/tests/l1/formatting_transaction.rs`) all assert it, so add a case
  there when a consumer-visible outcome changes. Cases are string literals
  because `.gitattributes` would normalize a CRLF fixture file.
  `test_hooks::install_broken_pass` (built under `cfg(test)` or the
  `cleanup-test-hooks` feature, which the CLI and DMLS enable as a
  dev-dependency feature) corrupts the candidate before the guard. It is
  process-global and does not reach a spawned `md`. Use it in process (DMLS
  sessions, `darkmatter_cli::commands::run_clean`), and prove binary routes
  with a real decline such as an unpaired `::block`.
