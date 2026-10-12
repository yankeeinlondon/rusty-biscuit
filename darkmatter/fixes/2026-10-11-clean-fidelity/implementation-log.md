---
spec: "/Volumes/coding/wt/rusty-biscuit/fix-clean-fidelity/darkmatter/fixes/2026-10-11-clean-fidelity/spec.md"
plan: "darkmatter/fixes/2026-10-11-clean-fidelity/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - darkmatter/lib/src/markdown/cleanup/rules.rs
    - darkmatter/lib/src/markdown/cleanup/diagnostic.rs
    - darkmatter/lib/src/markdown/cleanup/frontmatter.rs
    - darkmatter/lib/src/markdown/cleanup/mod.rs
    - darkmatter/lib/src/style/descriptor.rs
    - darkmatter/lib/src/style/parse.rs
    - darkmatter/lib/src/style/error.rs
    - darkmatter/lib/src/style/schema/mod.rs
    - darkmatter/lib/src/style/coverage_tests.rs
    - darkmatter/docs/schemas/darkmatter.yaml
    - darkmatter/lib/tests/l1/clean_rules.rs
    - darkmatter/lib/tests/l1/clean_frontmatter_repair.rs
    - darkmatter/lib/tests/l1/base_schema_end_to_end.rs
    - darkmatter/lib/tests/l1/schema_quoting_safety.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/fixtures/clean/style-markdown-control.md
    - darkmatter/cli/src/commands/clean.rs
    - darkmatter/cli/src/commands/clean/json.rs
    - darkmatter/cli/src/commands/clean/frontmatter_repair.rs
    - darkmatter/cli/tests/l1/layout_style_frontmatter.rs
    - darkmatter/dmls/src/config/mod.rs
    - darkmatter/dmls/src/providers/formatting.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/stdio_subprocess.rs
    - darkmatter/dmls/tests/l1/style_markdown_parity.rs
    - darkmatter/dmls/tests/l1/workspace_formatting_settings.rs
    - darkmatter/dmls/tests/fixtures/formatting/dmls.toml
    - darkmatter/dmls/tests/fixtures/formatting/settings.json
docs_updated_during_phase_2:
    - darkmatter/dmls/docs/editors/README.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/cleanup.md
source_files_during_phase_3:
    - darkmatter/lib/Cargo.toml
    - darkmatter/lib/src/markdown/cleanup/mod.rs
    - darkmatter/lib/src/markdown/cleanup/body.rs
    - darkmatter/lib/src/markdown/cleanup/compare.rs
    - darkmatter/lib/src/markdown/cleanup/edits.rs
    - darkmatter/lib/src/markdown/cleanup/protect.rs
    - darkmatter/lib/src/markdown/cleanup/structure.rs
    - darkmatter/lib/src/markdown/cleanup/transform.rs
    - darkmatter/lib/src/markdown/cleanup/wrap.rs
    - darkmatter/lib/src/markdown/cleanup/reflow.rs
    - darkmatter/lib/src/markdown/cleanup/emphasis.rs
    - darkmatter/lib/src/markdown/cleanup/perf_profile.rs
    - darkmatter/lib/src/markdown/cleanup/tests/mod.rs
    - darkmatter/lib/src/markdown/cleanup/tests/body_support.rs
    - darkmatter/lib/src/markdown/cleanup/tests/directive_fixed_points.rs
    - darkmatter/lib/src/markdown/cleanup/tests/emphasis.rs
    - darkmatter/lib/src/markdown/cleanup/tests/fidelity_repro.rs
    - darkmatter/lib/src/markdown/cleanup/tests/guard.rs
    - darkmatter/lib/src/markdown/cleanup/tests/parse_count.rs
    - darkmatter/lib/src/markdown/cleanup/tests/rules.rs
    - darkmatter/lib/src/markdown/cleanup/tests/source_fidelity.rs
    - darkmatter/lib/tests/l1/context_construction_guard.rs
docs_updated_during_phase_3:
    - darkmatter/docs/cli/clean.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/cleanup.md
source_files_during_phase_4:
    - claudine/cli/src/commands/wrap/inline.rs
    - claudine/lib/src/composition/closure/tests.rs
    - claudine/lib/src/system_prompt/prepare/tests.rs
    - darkmatter/cli/Cargo.toml
    - darkmatter/cli/src/commands/clean.rs
    - darkmatter/cli/src/commands/clean/json.rs
    - darkmatter/cli/src/commands/compose.rs
    - darkmatter/cli/src/commands/mod.rs
    - darkmatter/cli/src/commands/render.rs
    - darkmatter/cli/tests/l1/clean.rs
    - darkmatter/cli/tests/l1/clean_fidelity.rs
    - darkmatter/cli/tests/l1/clean_frontmatter.rs
    - darkmatter/cli/tests/l1/clean_json.rs
    - darkmatter/cli/tests/l1/clean_parity.rs
    - darkmatter/cli/tests/l1/clean_rules.rs
    - darkmatter/cli/tests/l1/compose_diagnostic_identity.rs
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/dmls/Cargo.toml
    - darkmatter/dmls/src/providers/formatting.rs
    - darkmatter/dmls/src/router.rs
    - darkmatter/dmls/tests/l1/formatting_transaction.rs
    - darkmatter/dmls/tests/l1/lsp_session.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/lib/benches/clean_hot_paths.rs
    - darkmatter/lib/fuzz/fuzz_targets/markdown_parser.rs
    - darkmatter/lib/src/markdown/cleanup/blockquote.rs
    - darkmatter/lib/src/markdown/cleanup/body.rs
    - darkmatter/lib/src/markdown/cleanup/brackets.rs
    - darkmatter/lib/src/markdown/cleanup/diagnostic.rs
    - darkmatter/lib/src/markdown/cleanup/document.rs
    - darkmatter/lib/src/markdown/cleanup/emphasis.rs
    - darkmatter/lib/src/markdown/cleanup/lists.rs
    - darkmatter/lib/src/markdown/cleanup/markers.rs
    - darkmatter/lib/src/markdown/cleanup/mod.rs
    - darkmatter/lib/src/markdown/cleanup/opaque.rs
    - darkmatter/lib/src/markdown/cleanup/perf_profile.rs
    - darkmatter/lib/src/markdown/cleanup/protect.rs
    - darkmatter/lib/src/markdown/cleanup/reflow.rs
    - darkmatter/lib/src/markdown/cleanup/rules.rs
    - darkmatter/lib/src/markdown/cleanup/tables.rs
    - darkmatter/lib/src/markdown/cleanup/tests/blockquote.rs
    - darkmatter/lib/src/markdown/cleanup/tests/brackets.rs
    - darkmatter/lib/src/markdown/cleanup/tests/emphasis.rs
    - darkmatter/lib/src/markdown/cleanup/tests/escapes.rs
    - darkmatter/lib/src/markdown/cleanup/tests/guard.rs
    - darkmatter/lib/src/markdown/cleanup/tests/lists.rs
    - darkmatter/lib/src/markdown/cleanup/tests/mod.rs
    - darkmatter/lib/src/markdown/cleanup/tests/parse_count.rs
    - darkmatter/lib/src/markdown/cleanup/tests/reflow.rs
    - darkmatter/lib/src/markdown/cleanup/tests/source_fidelity.rs
    - darkmatter/lib/src/markdown/cleanup/tests/tables.rs
    - darkmatter/lib/src/markdown/compose/block_pairs.rs
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/directives_api.rs
    - darkmatter/lib/src/markdown/compose/expression/lexer.rs
    - darkmatter/lib/src/markdown/compose/parse_utils.rs
    - darkmatter/lib/src/markdown/compose/pipeline/phases.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/compose/tests/frontmatter.rs
    - darkmatter/lib/src/markdown/compose/tests/rendering.rs
    - darkmatter/lib/src/markdown/mod.rs
    - darkmatter/lib/src/markdown/render_tree/disclosure_scan.rs
    - darkmatter/lib/tests/common/clean_parity/mod.rs
    - darkmatter/lib/tests/l1/clean_compose_equivalence.rs
    - darkmatter/lib/tests/l1/clean_parity.rs
    - darkmatter/lib/tests/l1/compose_diagnostic_identity.rs
    - darkmatter/lib/tests/l1/expression_regression.rs
    - darkmatter/lib/tests/l1/literal_token.rs
    - darkmatter/lib/tests/l1/main.rs
docs_updated_during_phase_4:
    - darkmatter/lib/README.md
    - darkmatter/dmls/docs/editors/README.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/darkmatter/cleanup.md
source_files_during_phase_5:
    - darkmatter/lib/src/markdown/frontmatter.rs
    - darkmatter/lib/src/markdown/cleanup/document.rs
    - darkmatter/lib/src/markdown/cleanup/body.rs
    - darkmatter/lib/tests/l1/clean_real_world_prompt.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/common/clean_parity/mod.rs
    - darkmatter/lib/tests/fixtures/clean/prompts-commit-464e2d90e.md
    - darkmatter/lib/tests/fixtures/clean/prompts-commit-464e2d90e.provenance.yaml
    - darkmatter/cli/src/args/command.rs
    - darkmatter/cli/src/commands/clean.rs
    - darkmatter/cli/tests/l1/clean_fidelity.rs
    - darkmatter/dmls/src/providers/formatting.rs
docs_updated_during_phase_5:
    - darkmatter/README.md
    - darkmatter/lib/README.md
    - darkmatter/cli/README.md
    - darkmatter/dmls/README.md
    - darkmatter/docs/cli/clean.md
    - darkmatter/docs/inline/cleaning.md
    - darkmatter/docs/lsp/features.md
    - darkmatter/docs/darkmatter-compose-pipeline.md
    - darkmatter/docs/structs/Markdown.md
    - darkmatter/docs/topics/layouts-and-style.md
    - darkmatter/dmls/docs/editors/README.md
    - darkmatter/dmls/docs/features.md
docs_created_during_phase_5:
    - darkmatter/docs/topics/markdown-formatting.md
skills_files_updated_during_phase_5:
    - .claude/skills/darkmatter/cleanup.md
    - .claude/skills/darkmatter/compose.md
source_files_during_phase_6: []
docs_updated_during_phase_6: []
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
source_code:
    - darkmatter/lib/src/markdown/cleanup/rules.rs
    - darkmatter/lib/src/markdown/cleanup/diagnostic.rs
    - darkmatter/lib/src/markdown/cleanup/frontmatter.rs
    - darkmatter/lib/src/markdown/cleanup/mod.rs
    - darkmatter/lib/src/style/descriptor.rs
    - darkmatter/lib/src/style/parse.rs
    - darkmatter/lib/src/style/error.rs
    - darkmatter/lib/src/style/schema/mod.rs
    - darkmatter/lib/src/style/coverage_tests.rs
    - darkmatter/docs/schemas/darkmatter.yaml
    - darkmatter/lib/tests/l1/clean_rules.rs
    - darkmatter/lib/tests/l1/clean_frontmatter_repair.rs
    - darkmatter/lib/tests/l1/base_schema_end_to_end.rs
    - darkmatter/lib/tests/l1/schema_quoting_safety.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/fixtures/clean/style-markdown-control.md
    - darkmatter/cli/src/commands/clean.rs
    - darkmatter/cli/src/commands/clean/json.rs
    - darkmatter/cli/src/commands/clean/frontmatter_repair.rs
    - darkmatter/cli/tests/l1/layout_style_frontmatter.rs
    - darkmatter/dmls/src/config/mod.rs
    - darkmatter/dmls/src/providers/formatting.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/stdio_subprocess.rs
    - darkmatter/dmls/tests/l1/style_markdown_parity.rs
    - darkmatter/dmls/tests/l1/workspace_formatting_settings.rs
    - darkmatter/dmls/tests/fixtures/formatting/dmls.toml
    - darkmatter/dmls/tests/fixtures/formatting/settings.json
    - darkmatter/lib/Cargo.toml
    - darkmatter/lib/src/markdown/cleanup/body.rs
    - darkmatter/lib/src/markdown/cleanup/compare.rs
    - darkmatter/lib/src/markdown/cleanup/edits.rs
    - darkmatter/lib/src/markdown/cleanup/protect.rs
    - darkmatter/lib/src/markdown/cleanup/structure.rs
    - darkmatter/lib/src/markdown/cleanup/transform.rs
    - darkmatter/lib/src/markdown/cleanup/wrap.rs
    - darkmatter/lib/src/markdown/cleanup/reflow.rs
    - darkmatter/lib/src/markdown/cleanup/emphasis.rs
    - darkmatter/lib/src/markdown/cleanup/perf_profile.rs
    - darkmatter/lib/src/markdown/cleanup/tests/mod.rs
    - darkmatter/lib/src/markdown/cleanup/tests/body_support.rs
    - darkmatter/lib/src/markdown/cleanup/tests/directive_fixed_points.rs
    - darkmatter/lib/src/markdown/cleanup/tests/emphasis.rs
    - darkmatter/lib/src/markdown/cleanup/tests/fidelity_repro.rs
    - darkmatter/lib/src/markdown/cleanup/tests/guard.rs
    - darkmatter/lib/src/markdown/cleanup/tests/parse_count.rs
    - darkmatter/lib/src/markdown/cleanup/tests/rules.rs
    - darkmatter/lib/src/markdown/cleanup/tests/source_fidelity.rs
    - darkmatter/lib/tests/l1/context_construction_guard.rs
    - claudine/cli/src/commands/wrap/inline.rs
    - claudine/lib/src/composition/closure/tests.rs
    - claudine/lib/src/system_prompt/prepare/tests.rs
    - darkmatter/cli/Cargo.toml
    - darkmatter/cli/src/commands/compose.rs
    - darkmatter/cli/src/commands/mod.rs
    - darkmatter/cli/src/commands/render.rs
    - darkmatter/cli/tests/l1/clean.rs
    - darkmatter/cli/tests/l1/clean_fidelity.rs
    - darkmatter/cli/tests/l1/clean_frontmatter.rs
    - darkmatter/cli/tests/l1/clean_json.rs
    - darkmatter/cli/tests/l1/clean_parity.rs
    - darkmatter/cli/tests/l1/clean_rules.rs
    - darkmatter/cli/tests/l1/compose_diagnostic_identity.rs
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/dmls/Cargo.toml
    - darkmatter/dmls/src/router.rs
    - darkmatter/dmls/tests/l1/formatting_transaction.rs
    - darkmatter/dmls/tests/l1/lsp_session.rs
    - darkmatter/lib/benches/clean_hot_paths.rs
    - darkmatter/lib/fuzz/fuzz_targets/markdown_parser.rs
    - darkmatter/lib/src/markdown/cleanup/blockquote.rs
    - darkmatter/lib/src/markdown/cleanup/brackets.rs
    - darkmatter/lib/src/markdown/cleanup/document.rs
    - darkmatter/lib/src/markdown/cleanup/lists.rs
    - darkmatter/lib/src/markdown/cleanup/markers.rs
    - darkmatter/lib/src/markdown/cleanup/opaque.rs
    - darkmatter/lib/src/markdown/cleanup/tables.rs
    - darkmatter/lib/src/markdown/cleanup/tests/blockquote.rs
    - darkmatter/lib/src/markdown/cleanup/tests/brackets.rs
    - darkmatter/lib/src/markdown/cleanup/tests/escapes.rs
    - darkmatter/lib/src/markdown/cleanup/tests/lists.rs
    - darkmatter/lib/src/markdown/cleanup/tests/reflow.rs
    - darkmatter/lib/src/markdown/cleanup/tests/tables.rs
    - darkmatter/lib/src/markdown/compose/block_pairs.rs
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/directives_api.rs
    - darkmatter/lib/src/markdown/compose/expression/lexer.rs
    - darkmatter/lib/src/markdown/compose/parse_utils.rs
    - darkmatter/lib/src/markdown/compose/pipeline/phases.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/compose/tests/frontmatter.rs
    - darkmatter/lib/src/markdown/compose/tests/rendering.rs
    - darkmatter/lib/src/markdown/mod.rs
    - darkmatter/lib/src/markdown/render_tree/disclosure_scan.rs
    - darkmatter/lib/tests/common/clean_parity/mod.rs
    - darkmatter/lib/tests/l1/clean_compose_equivalence.rs
    - darkmatter/lib/tests/l1/clean_parity.rs
    - darkmatter/lib/tests/l1/compose_diagnostic_identity.rs
    - darkmatter/lib/tests/l1/expression_regression.rs
    - darkmatter/lib/tests/l1/literal_token.rs
    - darkmatter/lib/src/markdown/frontmatter.rs
    - darkmatter/lib/tests/l1/clean_real_world_prompt.rs
    - darkmatter/lib/tests/fixtures/clean/prompts-commit-464e2d90e.md
    - darkmatter/lib/tests/fixtures/clean/prompts-commit-464e2d90e.provenance.yaml
    - darkmatter/cli/src/args/command.rs
documentation:
    - darkmatter/dmls/docs/editors/README.md
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/cleanup.md
    - darkmatter/docs/cli/clean.md
    - darkmatter/lib/README.md
    - darkmatter/README.md
    - darkmatter/cli/README.md
    - darkmatter/dmls/README.md
    - darkmatter/docs/inline/cleaning.md
    - darkmatter/docs/lsp/features.md
    - darkmatter/docs/darkmatter-compose-pipeline.md
    - darkmatter/docs/structs/Markdown.md
    - darkmatter/docs/topics/layouts-and-style.md
    - darkmatter/dmls/docs/features.md
    - darkmatter/docs/topics/markdown-formatting.md
    - .claude/skills/darkmatter/compose.md
completed_phase: 6
implemented: true
packages:
    - darkmatter
    - darkmatter-cli
    - dmls
    - claudine
    - claudine-cli
---

# Implementation Log for 2026-10-11-clean-fidelity (6 phases)

## Phase 1

Phase 1 is a design-risk phase. It changed no production source. A throwaway
spike was compiled into the `darkmatter` lib test target, run once, and then
removed from the crate. Its source and output are archived, not compiled, at
`fixes/2026-10-11-clean-fidelity/spike/` (`fidelity_spike.rs.txt`,
`report.txt`).

### 1. Root-cause confirmation (current `md clean`, 2026-10-11)

Every reproduction in the spec reproduces byte-for-byte with the current
`target/debug/md clean`: E1, D1, C1, A1, B1 and F1. Additional defaults
observed:

- `one\ntwo` becomes `one two` (Strip is the default).
- `1. a\n1. a\n1. a` becomes `1. 2. 3.` (`increment_ordered_list_bullets: true`).
- CRLF input comes back as LF.

The hypotheses are confirmed:

- **Directive-blind parse.** Confirmed by A1 and B1. Only `::shell-block`
  bodies are masked (`protect_opaque_directive_bodies`). Directive *lines*
  reach pulldown-cmark, become lazy continuations, and are re-prefixed by
  `pulldown-cmark-to-cmark`.
- **Container prefixes lost.** Confirmed by E1, C1 and D1. The serializer, then
  `fix_list_indentation`, `restore_list_markers` and
  `extract_additional_paragraph_contexts`, rebuild prefixes line by line. They
  have no model of a non-paragraph child of an item, so those lines lose the
  item's continuation indent. The indent intended for them is left behind as a
  whitespace-only line, which also explains the introduced whitespace in F1.
- **No output check.** Confirmed: nothing compares the output with the input.

### 2. Entrypoint map

All paths are relative to `darkmatter/lib/src/markdown/` unless stated.

**Counted parses** today go through `cleanup_parser` (`cleanup/mod.rs:51`).
**Real parses** additionally include the uninstrumented `find_code_regions`
(`compose/parse_utils.rs:416`, `Options::all()`), reached through
`OpaqueBodyScan` and `scan_block_pairs`.

| Entrypoint | Location | Default newlines | Counted / real parses today | Phase 4 disposition | Ceiling |
| --- | --- | --- | --- | --- | --- |
| `cleanup_content`, `_compact`, `_loose` | `cleanup/mod.rs:139/162/185` | Strip | 1 / 3–4 | Guarded string wrapper over the body result API; returns the original on refusal; Preserve | 2 counted |
| `cleanup_content_with_indent{,_compact,_loose}` | `:211/234/257` | Strip | 1 / 3–4 | Guarded wrapper; indentation override plus padding override (compact → `none`, loose → `one`); Preserve | 2 |
| `cleanup_content_with_indent*_preserving_incidental` (3 variants) | `:221/244/267` | Preserve | 1 / 2–3 | Collapse into the same guarded wrapper; names may go once nothing calls them (no external users) | 2 |
| `cleanup_to_fixed_width` | `:284` | strip, then reflow | 2 / 4 | Guarded operation: body transform with `fixed-width: N`; verification after reflow | 3 |
| `strip_incidental_newlines` | `cleanup/reflow.rs:58` | n/a | 1 / 2 | **Standalone helper, unguarded**, existing count kept; it already computes source-span edits, which the `unwrap` rule reuses | 1 |
| `reflow_to_width` | `cleanup/reflow.rs:205` | n/a | 1 / 2 | Standalone helper, unguarded, existing count kept. A cleanup that requests a width runs it inside the transaction, followed by the final verification parse | 1 alone; 3 within cleanup |
| `Markdown::cleanup`, `cleanup_compact`, `cleanup_loose`, `cleanup_with_indent{,_compact,_loose}` | `markdown/mod.rs:527-582` | Strip | as above | Guarded; body only (the frontmatter field is untouched); defaults Preserve | 2 |
| `Markdown::cleanup_with_fixed_width` | `markdown/mod.rs:539` | strip + reflow | 2 | Guarded, like `cleanup_to_fixed_width` | 3 |
| `Markdown::strip_incidental_newlines` | `markdown/mod.rs:533` | n/a | 1 | Standalone helper | 1 |
| Compose `ComposeOperation::Cleanup` | `compose/pipeline/phases.rs:107-160` | Strip, forced by `fixed_width` | 1–2 | Calls the guarded wrapper. Default `incidental_newline_mode` stays an explicit compose option; compose is not in scope for `style.markdown` | 2/3 |
| `md clean`, `md FILE --save` | `cli/src/commands/clean.rs:73`, `cli/src/main.rs:165-173` | Strip | 1–2 + YAML | Raw-source transaction (§5) | 2/3 + repair |
| DMLS `textDocument/formatting` | `dmls/src/providers/formatting.rs:26/46`, `router.rs:994` | Strip | 1–2 | Raw-source transaction with the workspace level | 2/3 + repair |

**External callers outside the area** (the Wave 11 consumer check):

- `claudine/lib/src/composition/closure.rs:134,268` and `closure/tests.rs:858`
  call `cleanup_content`. These see behavior change: wraps are kept, `1,1,1`
  is kept, and the output is guarded.
- `claudine/lib/src/composition/prepare.rs:366` sets
  `IncidentalNewlineMode::Preserve` on compose.

**Other in-area callers:**

- `cli/src/commands/render.rs:34-35`: `cleanup_with_indent`.
- `lib/benches/clean_hot_paths.rs`.
- `lib/fuzz/fuzz_targets/markdown_parser.rs`.
- Compose tests: `compose/tests/rendering.rs`.

### 3. Pass dispositions

**Disposition legend:**

- **A** = artifact repair: kept only while the serializer is used.
- **R(rule)** = maps to a rule.
- **X** = removed.

The chosen design (§4) edits authored spans instead of re-serializing. Every
serializer-artifact repair therefore becomes unnecessary and is deleted with
the serializer path in Phase 3. None remains as a separate pass.

| Pass | Location | Disposition |
| --- | --- | --- |
| `protect_opaque_directive_bodies` / `restore_opaque_directive_bodies` | `cleanup/lists.rs:360/410`, `opaque.rs` | Replaced by source protection (§4). Its malformed-scan → source fallback becomes a `declined` result with reason `malformed-region` |
| `extract_list_markers` / `restore_list_markers` | `lists.rs:8/495` | A, so X (markers are never re-serialized) |
| `extract_list_item_contexts`, `extract_additional_paragraph_contexts`, `extract_indented_code_markers`, `fix_list_indentation` | `lists.rs:59/103/214/775` | R(indentation). Rewritten as line-prefix edits; this is the root cause of E1, C1 and D1. Marker-width and marker-padding logic (`list_marker_byte_width`, `commonmark_marker_padding`, `task_marker_byte_width`) is reused |
| `detect_list_indentation` (only reachable from the `None` branch, which no public API takes) | `lists.rs:640`, `mod.rs:407` | X (dead). `indentation: preserve` means "no edit", not "detect" |
| `get_preferred_emphasis_style` (`PREFER_ITALICS`) | `emphasis.rs:11` | X (spec: ambient restyling removed) |
| `preserve_original_emphasis`, `restore_emphasis_placeholders`, `unescape_emphasis_chars`, `restore_backslash_placeholders` | `emphasis.rs` | A, so X (authored bytes are never re-serialized). The `2026-08-27-preserve-backslash-escapes` regression tests remain as fidelity tests |
| `unescape_brackets` | `brackets.rs` | A, so X |
| `fix_blockquote_formatting` (`" > x"` → `"> x"`, `">  > x"` → `"> > x"`) | `blockquote.rs:3` | Partly A, partly unrequested restyling, so X. F1 is caused by the serializer, not by this pass. No rule owns quote-marker spacing |
| `collapse_incidental_soft_break_events` / `SoftBreakModel` | `cleanup/reflow.rs:10`, `reflow/semantic.rs` | R(unwrap). `SoftBreakModel` already yields source-span `StripEdit`s with join-separator logic; it is reused |
| `reflow_to_width` (line-based) | `reflow.rs:205` | R(fixed-width). It normalizes CRLF→LF today (`reflow.rs:208`), so a terminator-preserving path is required. Its protected-line logic (`is_structural_line`, `directive_trimmed`) stays a backstop behind the shared protection |
| `add_text_language_to_empty_code_blocks` | `mod.rs:444` | R(default-language). Becomes an insertion after the fence run |
| `align_tables_in_stream` | `tables.rs:4` | R(table-layout). Becomes a rewrite of the table's own lines, each line keeping its container prefix |
| serializer `increment_ordered_list_bullets: true` | `mod.rs:354` | R(ordered-lists). Becomes number-only edits, applied only when the list is neither uniform nor sequential |
| `normalize_list_spacing` (Normal/Compact/Loose) | `lists.rs:1041` | Compact/Loose become R(list-item-padding: none/one). Normal's "blank at level transitions" behavior is X: the default is `preserve`, and `list-interior-padding` owns item interiors |
| serializer `newlines_after_*` (for example, a blank line inserted between a heading and a paragraph) | cmark defaults | X (no global blank-line normalization). Tests that assert it (for example, `test_cleanup_adds_blank_line_between_header_and_paragraph`) are rewritten in Phase 3 |
| trim leading blank lines; force exactly one final newline | `mod.rs:427-435` | X |
| `list-padding` (new) | none | R(list-padding). New blank-line insertion at the list boundary, using the local terminator; never next to a directive line |
| `list-interior-padding` (new) | none | R(list-interior-padding). New |
| `perf_profile.rs` replica | test-only | Drifted from the real pipeline (it pre-strips). Delete or rebuild in Phase 3 |

### 4. Fidelity spike: chosen representation

**What was prototyped** (archived in `spike/`):

1. **Protection scan.**
   - Whole protected lines are the union of two sets:
     - every `scan_darkmatter_directives` line, with its terminator;
     - every `scan_block_pairs` `Shell` pair's full span. This covers quoted
       `> ::shell-block` regions and their opaque bodies.
   - Inline protected spans are the `ExpressionFinder::find_all` tokens.
2. **Parse view.**
   - The source minus whole protected lines, plus a segment map from view
     offsets to source offsets.
   - This is how directive-aware parsing avoids lazy continuation. A removed
     line cannot lazily continue anything, and a list that continues across
     `::block`/`::end-block` stays one list. That matches compose semantics,
     because compose removes directive lines. Directive-like text in code is
     not reported by the scanners (they skip code regions), so it stays code.
3. **Edits.** One parse of the view yields an edit list in source
   coordinates. A run applies edits, never re-serializes, and applies no edit
   to a protected byte. Prototyped rules:
   - default-language: insertion after the fence run;
   - unwrap: the soft-break span through the next line's container prefix,
     replaced with a space. The edit is refused when the span crosses a removed
     line or an expression;
   - indentation: a per-line whitespace delta applied to every line of a
     nested list's subtree. The target is clamped to the CommonMark-valid band
     `[parent content column, parent content column + 3]`.
4. **Comparison** of view(input) against view(output):
   - normalized events, with text coalesced;
   - soft break becomes a space only under unwrap;
   - empty fence info becomes `text` only under default-language;
   - reference definitions from `Parser::reference_definitions()`, including
     unused ones;
   - each protected line's exact bytes, whether its neighbors are blank, and
     its semantic anchor (the event index it precedes);
   - expression spellings.

**Results** (`spike/report.txt`):

- **All rules off.** Byte-identical, and idempotent, for E1, D1, C1, A1, B1,
  F1, a quoted shell block, a multi-line expression, an unused reference
  definition, mixed CRLF/LF, directive-looking code, and a directive between
  two paragraphs.
- **D1 + default-language.** Only `text` is added, at column 8. This matches
  acceptance criterion 1 exactly.
- **D1 and B1 + indentation 2.**
  - The nested list moves to column 3, the nearest valid continuation indent,
    because the `1.` marker width sets the minimum. Its fence, its paragraph
    and its child lines move together.
  - B1's `::block`/`::end-block` lines are untouched while the list spans
    them.
  - Structure is equal and the result is idempotent.
- **Unwrap.**
  - Joins are made inside paragraphs. A soft break inside `{{ name\n }}` is
    left alone.
  - MIXED keeps the untouched `\r\n` terminators, giving
    `one two three\r\n\r\n- a\n- b\r\n`.
  - The soft break next to a `::file` line between two paragraphs is refused.
- **Guard catches.** The comparator rejected every corrupting candidate the
  prototype produced:
  - the soft-break join into a directive line, before the boundary fix;
  - E1's re-indent of the list inside a quote inside the list, which split the
    quote;
  - a fence-info normalization slip.

  This is direct evidence for the fail-closed backstop.

**Lessons that Phase 3 must design for:**

1. **Container prefixes as segments.** A line's prefix must be modeled as a
   stack of container segments (list indent, `>` marker plus its optional
   space, and so on). An indentation delta applies to the segment owned by the
   target list, not to leading whitespace. E1 under indentation 2 or 8 failed
   (safely) without this.
2. **Columns from source bytes.** pulldown-cmark ranges for a nested
   `List`/`Item` start at the parent's content offset, not at the marker.
   Marker columns must be read from source bytes.
3. **Segment boundaries.** View→source mapping at a segment boundary must
   count as crossing protection when the next segment starts later; line
   starts map to the later segment. Otherwise an edit lands on the terminator
   just before a directive line.
4. **Marker padding.** Indentation 8 under a narrow marker needs
   parent-marker padding (`-    item`), as in today's `fix_list_indentation`.
   This is the case "a requested indent cannot be used literally"; it is
   reused, not reinvented.
5. **Parse counts.**
   - Counted: the spike used 3 per run (planning, input comparison, output
     comparison). Reusing the planning parse's events for the input comparison
     gives the specified **2**.
   - Uncounted: each protection scan reaches `find_code_regions` three times
     (directives, block pairs, `ExpressionFinder`'s private copy).
   - Phase 3 should compute code regions once per text and pass them to the
     scanners through `_in`-style variants. That gives one scanner parse per
     text, counted by a separate `scanner_parse_count`.
   - `find_code_regions` uses `Options::all()`, which differs from
     `cleanup_parser_options()`. Phase 3 either keeps the scanners' own options
     (they define what compose recognizes) or proves equivalence. Directive
     recognition must match compose, so **keep the compose options for
     scanning** and record it.
6. **CRLF.** Existing `strip_incidental_newlines` and `reflow_to_width` start
   with `replace("\r\n", "\n")`. Inside the transaction they must run on
   authored terminators. `SoftBreakModel` edits already carry source spans.

**Discarded alternative: serializer plus repair.** This means keeping
pulldown-cmark-to-cmark and repairing prefixes afterward. The spec's
"all-body-rules-off ⇒ exact body bytes" invariant, CRLF and BOM preservation,
and "disabled regions must not undergo incidental serializer normalization"
would each need a reverse-mapping pass. Every pass in §3 marked A exists only
because of the serializer. The spike shows bounded edits meet every invariant
with no artifact repair. **Decision: bounded source edits plus final semantic
verification. Serializer replacement is not needed, because the serializer is
removed from cleanup.**

### 5. Frozen contracts (Phase 2 to 4 module ownership)

There is **one module tree, `markdown::cleanup`**. No second `clean` module
and no generic pipeline framework are added.

| Module (new unless noted) | Owner phase | Contents |
| --- | --- | --- |
| `cleanup/rules.rs` | P2 rule model | `Leaf<T>`, the rule value types, `MarkdownRuleOverrides`, `ResolvedMarkdownRules`, `RuleSource`, leaf validation, `resolve` |
| `cleanup/diagnostic.rs` | P2 repair pipeline | `CleanStage`, `CleanCode`, `CleanDiagnostic`, `FallbackReason` (moved from the CLI and extended) |
| `cleanup/frontmatter.rs` | P2 repair pipeline | `repair_frontmatter` orchestration, `AuthoredCoordinateMap`, and edit projection, moved from `cli/src/commands/clean/frontmatter_repair.rs` |
| `cleanup/protect.rs` | P3 protect | Protection scan, parse view, segment map, container-prefix segments |
| `cleanup/compare.rs` | P3 compare | Semantic comparison and first-divergence location; test-only broken-pass seam |
| `cleanup/edits.rs` | P3 transforms | `SourceEdit` set; apply with overlap and protection checks; the local terminator for inserted lines |
| `cleanup/mod.rs` (existing) | P3/P4 | Body result API and string wrappers |
| `cleanup/document.rs` | P4 transaction | `clean_document` (the raw-source transaction) |
| `style/schema/markdown.rs` + `descriptor.rs` rows + `docs/schemas/darkmatter.yaml` | P2 rule model | `style.markdown` declarations. Validation delegates to `cleanup::rules`, the single validator |
| `dmls/src/config/mod.rs` (existing) | P2 workspace | `formatting.markdown` and legacy migration |
| `cli/src/commands/clean.rs` and `clean/json.rs` | P4 CLI | Flags to overrides, JSON v1 adaptation, terminal rendering |
| `dmls/src/providers/formatting.rs` and `router.rs` | P4 DMLS | Edits; `window/showMessage` |

**Contract sketch.** Names are frozen. Field-level additions are allowed if
they do not change these semantics.

```rust
// cleanup/rules.rs
pub enum Leaf<T> { Absent, Null, Value(T) }          // absent and null recorded separately; both fall through
pub enum FixedWidth { Off, Columns(u16) }            // 1..=1000; YAML/JSON `true` => Columns(120)
pub enum Indentation { Preserve, Two, Four, Eight }  // integers only; "4" (string) invalid
pub enum ListItemPadding { Preserve, None, One }
pub enum ListInteriorPadding { Preserve, None, ExceptNestedLists, All }
pub struct DefaultLanguage(String);                  // validated: no line breaks, control chars or backticks; "" = add none
pub struct MarkdownRuleOverrides {                   // one per precedence level
    pub frontmatter: Leaf<bool>, pub table_layout: Leaf<bool>, pub fixed_width: Leaf<FixedWidth>,
    pub unwrap: Leaf<bool>, pub indentation: Leaf<Indentation>, pub ordered_lists: Leaf<bool>,
    pub list_padding: Leaf<bool>, pub list_item_padding: Leaf<ListItemPadding>,
    pub list_interior_padding: Leaf<ListInteriorPadding>, pub default_language: Leaf<DefaultLanguage>,
}
pub enum RuleSource { Cli, Document, Workspace, Default }
pub struct ResolvedMarkdownRules { /* each value plus its RuleSource */ }
impl ResolvedMarkdownRules {
    pub fn defaults() -> Self;                       // runtime defaults; independent of the baseline schema
    pub fn all_body_rules_off() -> Self;
    pub fn joins_soft_breaks(&self) -> bool;         // unwrap || a positive width, after precedence
    pub fn permits_tightness_change(&self) -> bool;  // only when a padding rule's source != Default and its value != Preserve
}
pub fn resolve(cli: &MarkdownRuleOverrides, document: Option<&MarkdownRuleOverrides>,
               workspace: Option<&MarkdownRuleOverrides>) -> ResolvedMarkdownRules;
/// Passive. Pending executable values and invalid leaves produce diagnostics and
/// invalidate the whole `style.markdown` object (returns Err).
pub fn document_rules(frontmatter: &serde_yaml_ng::Value, yaml_source: &str)
    -> Result<Leaf<MarkdownRuleOverrides>, Vec<CleanDiagnostic>>;
/// Shared by the TOML and JSON readers in DMLS.
pub fn rules_from_json(value: &serde_json::Value) -> Result<MarkdownRuleOverrides, Vec<RuleError>>;

// cleanup/diagnostic.rs
pub enum CleanStage { Syntax, Schema, Rules, Fidelity }
pub enum CleanCode { Yaml(YamlDiagnosticCode), FidelityFallback /* "clean.fidelity-fallback" */,
                     InvalidRules /* "clean.invalid-rules" */ }
pub enum FallbackReason { MalformedRegion, SerializationFailure, SemanticMismatch }
pub struct CleanDiagnostic { pub stage: CleanStage, pub code: CleanCode, pub span: SourceSpan,
    pub classification: YamlCertainty, pub message: String, pub repairs: Vec<YamlRepair>,
    pub reason: Option<FallbackReason> }
// The fidelity fallback is NonDeterministicFind, repairs: [], span 0..len. Its
// message names the first affected input block as 1-based line and byte column;
// an output-only block uses the nearest input boundary, or EOF.

// cleanup/mod.rs: body level (no frontmatter)
pub enum CleanOutcome { Unchanged, Changed, Declined, Invalid }
pub struct BodyCleanup { pub text: String, pub outcome: CleanOutcome, pub diagnostics: Vec<CleanDiagnostic> }
pub fn clean_body(body: &str, rules: &ResolvedMarkdownRules) -> BodyCleanup; // never returns Invalid
// The existing cleanup_content* wrappers are rule presets that return clean_body(..).text.

// cleanup/document.rs: raw source
pub struct CleanRequest<'a> {
    pub source: &'a str,
    pub cli: MarkdownRuleOverrides,                  // explicit flags only; a CLI default is not an override
    pub workspace: Option<&'a MarkdownRuleOverrides>,// DMLS only
    pub schema: CleanSchemaConfig,                   // existing; the caller builds it from its own context
    pub document_path: Option<&'a Path>,             // None for stdin/untitled: no trigger discovery
}
pub struct CleanResult {
    pub text: String,                                // == source unless outcome == Changed
    pub outcome: CleanOutcome,
    pub frontmatter_span: Option<SourceSpan>,
    pub diagnostics: Vec<CleanDiagnostic>,           // authored coordinates, stable source order
    pub applied: Vec<AppliedRepair>,                 // only repairs actually emitted; [] unless Changed
    pub rules: ResolvedMarkdownRules,
}
pub fn clean_document(request: &CleanRequest<'_>) -> Result<CleanResult, SchemaError>;
// SchemaError covers a missing or unreadable referenced schema (existing typed errors).
// Outcome mapping for the CLI: Invalid => exit 1, no save; Declined => original, exit 0, applied [].
```

**Dependencies.**

- The P2 rule model has no dependencies. The P2 repair pipeline depends on the
  diagnostic types, which the repair task owns. P2 workspace depends only on
  the `rules_from_json` signature above.
- P3 protect and compare depend on nothing from P2 except `ResolvedMarkdownRules`.
- P4 depends on all of them.

### 6. Robustness tables compared with existing loaders (departures recorded before editing)

**YAML document (`style.markdown`).**

- A duplicate key is already invalid YAML: verified with
  `md clean --json dup.md`, which reports `yaml.parse` and
  `yaml.duplicate-key` and exits 1. Matches **I**.
- Today the style parser (`style/parse.rs`) has no `markdown` key. It would
  report `style.markdown` as `UnknownKey` (and reject it under
  `--strict-style`), so registration is required (P2).
- The `StyleFrontmatter` fields are `#[serde(default)]` Options. That is
  acceptable for render styles, but the cleanup reader must not reuse it for
  rules, because `Option` conflates absent with null. `cleanup::rules`
  reads the raw `Value` into `Leaf<T>`.

**TOML workspace (`.dmls.toml`).** Departures from today's loader, all
required by the plan:

1. `FormattingConfig` is `#[serde(default)]` with no `deny_unknown_fields`.
   Unknown keys are silently ignored today, and `formatting.*` unknown keys
   must become **W**. This is scoped to `formatting` only, not to unrelated
   sections (ruling 6).
2. `fixed_width: Option<u16>` has no range check today; it becomes 1–1000.
3. `discover` skips a malformed `.dmls.toml` with only `tracing::warn!`. The
   **W** cells require a client-visible diagnostic, and **no client
   notification channel exists in DMLS today**: there is no `showMessage` or
   `logMessage` anywhere in `dmls/src`. P2 workspace adds the warning record;
   the P4 DMLS task routes it through `router.rs`.
4. `toml` already rejects duplicate keys and trailing garbage as parse errors,
   so those become **W** naturally.

**JSON editor settings.**

1. `recompute` (`config/mod.rs:392`) serializes the *file* config, including
   serde defaults, to JSON and deep-merges the overlay. This is exactly the
   "serialized defaults manufacture legacy keys" hazard in ruling 6. A file
   with the new `formatting.markdown` would serialize as
   `cleanup: "default"`, which makes it look mixed. The fix is to normalize
   each *authored* source to `MarkdownRuleOverrides` before merging, and to
   merge rules leaf by leaf.
2. `merge_json` replaces a value with `null`, so an editor `null` leaf would
   erase a valid file leaf. Under the plan, **N** must release the leaf to the
   lower level instead. This is handled by the leaf-level merge in item 1.
3. On a deserialization failure, `recompute` falls back to the *file* config.
   That already matches "invalid overlay retains valid file settings", but it
   must now also surface a diagnostic.
4. Duplicate names: the settings arrive as `serde_json::Value`, which keeps the
   last duplicate member. The loader cannot observe duplicates (ruling 7, cell
   **X: Value**). This boundary is documented and tested, not changed.
5. `initializationOptions` and `workspace/configuration` are not used today.
   Settings arrive only through `didChangeConfiguration`. The matrix's "normal
   editor configuration request" fixture is therefore a
   `workspace/didChangeConfiguration` payload.

### 7. Acceptance criteria: owners and planned tests

| AC | Owner (phase/task) | Planned tests (L1 unless noted) |
| --- | --- | --- |
| 1 Reproductions (exact, all-off, indentation/unwrap/width/padding variants, multi-digit markers, code children) | P3 verify rule fidelity | `lib/src/markdown/cleanup/tests/fidelity_repro.rs` |
| 2 A1/B1 compose equivalence, `when` true/false | P4 invocation parity | `lib/tests/l1/clean_compose_equivalence.rs` (fixture-owned data, no shell or network) |
| 3 Directive fixed points per rule (after a quote, nested lists, a table) | P3 protect | `cleanup/tests/directive_fixed_points.rs` |
| 4 One test per rule value, list-spacing examples, required separators, ordered sequences | P3 verify rule fidelity | `cleanup/tests/rules.rs` |
| 5 DMLS migration, mixed-shape rejection, compact/loose equivalence | P2 workspace; P4 parity | `dmls/src/config` unit tests plus `dmls/tests/l1/` protocol test |
| 6 Precedence (CLI > document > workspace > default) and invalid-object fallback | P2 rule model; P4 parity | `cleanup/tests/rules_resolution.rs`; CLI and DMLS protocol tests |
| 7 Fidelity guard via the test-only broken-pass seam; mutation list; tight/loose; reflow-introduced; refusal after a YAML repair | P3 compare; P4 transaction, CLI and DMLS | `cleanup/tests/guard.rs`; `cli/tests/l1/clean_fallback.rs`; DMLS protocol test |
| 8 Frontmatter parity (CLI vs DMLS), `frontmatter: false` | P2 repair; P4 parity | `lib/tests/l1/clean_document.rs`; CLI and DMLS fixtures sharing one input |
| 9 Idempotence for AC 1–4 fixtures | P3 verify | helper asserted in each fixture test |
| 10 `prompts/commit.md` @ `464e2d90e` fixture | P5 corpus | `lib/tests/l1/clean_real_world.rs` plus a checked-in fixture |
| 11 Corpus scan counts | P5 corpus | log evidence (not a CI gate) |
| 12 Parse ceilings plus separate scanner counts | P4 transaction | `cleanup/tests/parse_count.rs` (extended) |
| 13 CRLF, mixed endings, BOM, empty frontmatter, missing final newline, all-off, unused reference definitions, malformed regions; CLI and DMLS routes | P3 protect; P4 CLI and DMLS | `cleanup/tests/source_fidelity.rs`; `cli/tests/l1/clean*.rs` (`CliProcessFixture`); `dmls/tests/l1/` |
| 14 Docs | P5 docs | n/a |

### 8. Author-only decisions found

None block Phase 2. Ruling 7's raw-JSON duplicate-name widening stays out of
scope, as the plan says. Two implementation choices are recorded rather than
escalated, because both follow from the spec:

- Scanners keep the compose `Options::all()` for code-region detection (§4,
  lesson 5).
- Inside `markdown::cleanup`, `fix_blockquote_formatting`'s quote-marker
  normalization is removed outright, with no rule replacing it (§3). The spec
  lists "blockquote spacing fixes" among the passes to classify; it is neither
  an artifact repair under the edit model nor mapped to a rule.

### Verification

**Requirement-to-test mapping for this phase.** Phase 1 changed no
behavior, so it added no tests. The spike's assertions (`spike/report.txt`)
are design evidence, not regression tests. Each acceptance criterion's
planned test home and owner are in §7.

**Gates run on macOS.** The tree after the spike was removed has no source
changes.

- `just lint`: **passed** (exit 0, including the zed-dmls wasm32 check).
- `just test`: 8980 tests run, 8974 passed, 5 failed, 1 timed out, 12 skipped.
  None of these failures involves cleanup. All are pre-existing:
  - **5 failures** come from one committed, unmodified file,
    `prompts/_reviews/feature-review.md` (last touched in `c5ad95142`). Its
    frontmatter does not parse: "could not find expected ':' at line 112
    column 9". The failing tests are:
    - `dasherized_identifier_corpus::shipped_corpus_expressions_all_parse`
    - `dasherized_identifier_corpus::corpus_walk_reaches_every_present_surface`
    - `feature_review_incident::the_original_typo_would_have_failed_to_compose`
    - `feature_review_incident::the_typo_is_one_identifier_under_the_current_grammar`
    - `feature_review_incident::the_shipped_feature_review_composes_and_names_its_spec`

    That prompt is outside this fix's scope and was not edited.
  - **1 timeout**, `entry_point_parity::darkmatter_entry_points_agree_on_every_reference`,
    hit the 30s limit under full-suite `--no-fail-fast` load. It took 7.5s in
    the first run and **passes alone in 5.2s**.

**Cross-OS.** No production code changed, so no cross-OS run was needed.
Line-ending and BOM risks are recorded for Phase 3 (§4, lesson 6).

## Phase 2

### 1. Rule model (`cleanup/rules.rs`, `cleanup/diagnostic.rs`, style registration)

- `markdown::cleanup::rules` is the single rule validator. `rules_from_json`
  reads one level (a converted YAML mapping, TOML table, or editor JSON) into
  `MarkdownRuleOverrides` of `Leaf<T>` (`Absent`/`Null`/`Value`), rejecting
  unknown names, quoted numerals, wrong types and lists. `document_rules`
  wraps it for frontmatter: passive, rejects strings that still hold `{{`/`$(`
  syntax, and returns one `clean.invalid-rules` diagnostic per problem in
  document coordinates, discarding the whole `style.markdown` object.
  `resolve(cli, document, workspace)` picks each leaf independently and
  records its `RuleSource`. `joins_soft_breaks()` applies the width
  implication after precedence; `permits_item_padding_change()` /
  `permits_interior_padding_change()` keep which padding rule was explicit.
- `cleanup::diagnostic` holds `CleanStage` (`syntax`, `schema`, `rules`,
  `fidelity`), `CleanCode` (serializes `yaml.*`/`schema.*` unchanged, plus
  `clean.fidelity-fallback` and `clean.invalid-rules`), `FallbackReason`, and
  `CleanDiagnostic`. **Departure from the §5 sketch:** spans (and repair spans
  inside a diagnostic) are document coordinates, not YAML-relative, so one
  type serves frontmatter, rules and fidelity findings. `document_rules` takes
  a third `yaml_offset` argument for that reason.
- **Style registration.** `descriptor.rs` gains ten `markdown.*` leaves with
  `LeafType::MarkdownRule` and `sub_spec: 0` (never `KnownButInactive`), and
  `markdown` is a container. `parse.rs` validates the whole `style.markdown`
  value through `rules_from_json` and reports a new typed
  `StyleParseError::InvalidRule { path, reason }`; rendering never reads the
  rules, so `StyleFrontmatter` has no `markdown` field (documented on the
  struct). Snake-case spellings are rejected there, not aliased, because the
  rules have no aliases.
- **Baseline schema.** `docs/schemas/darkmatter.yaml` declares
  `style.markdown`. Probing showed Darkmatter's schema coercion accepts
  `unwrap: "true"` against `boolean` and `default-language: 5` against
  `string`, which the rule validator rejects. The five booleans are therefore
  `literal(true)`/`literal(false)` unions and `default-language` is
  `string(not-empty)` plus `literal('')`; unions of non-boolean/numberlike
  arms are not coerced. `fixed-width` (`number|boolean`) and `indentation`
  (literal union) were already uncoerced (`"80"`, `"4"` rejected).

**Tests added (all L1):**

| Test | Proves |
| --- | --- |
| `lib/tests/l1/clean_rules.rs::yaml_matrix_walks_every_rule_and_shape` | YAML matrix from the checked-in fixture `lib/tests/fixtures/clean/style-markdown-control.md`: control row (all ten from the document), then per column absent (F, other nine unaffected), null in both spellings (N, falls through), wrong whole type including quoted booleans/numerals (D, span on the authored value), one wrong element and every element wrong (D), empty `""`/`[]`/`{}` (D except `default-language: ""`), duplicate key and trailing garbage (I: invalid YAML; duplicate reported as `yaml.duplicate-key`) |
| `…::semantic_invalid_values_discard_the_whole_object` | width 0/1001/80.5, executable `{{ }}`/`$( )` values, indentation 3, unknown enum values, backtick/newline/tab language |
| `…::unknown_and_snake_case_rule_names_are_rejected_not_ignored`, `…::every_problem_is_reported_in_source_order` | unknown names; stable order |
| `…::object_roots_cover_absent_null_wrong_and_empty_shapes` | `style`/`style.markdown` rows of the object-roots table |
| `…::precedence_resolves_each_rule_independently`, `…::an_invalid_document_object_falls_back_to_the_other_levels` | AC 6 at the rule level |
| `…::width_implies_unwrapping_only_after_precedence` | `true` = 120, width implies unwrap, `false` does not cancel unwrap, `--ignore-incidental-newlines` overrides a document width |
| `…::tightness_changes_are_permitted_only_by_an_explicit_padding_value`, `…::default_and_all_body_rules_off_profiles` | provenance and both profiles |
| `…::rules_do_not_depend_on_the_baseline_or_a_document_schema` | baseline disabled; a document schema cannot authorize `indentation: 3` |
| `lib/tests/l1/base_schema_end_to_end.rs::base_style_markdown_schema_agrees_with_the_rule_validator` | baseline schema and runtime validator agree on 30 shapes, including the coercion traps |
| `…::base_style_markdown_defaults_match_the_runtime_defaults` | every baseline `default(...)` equals `ResolvedMarkdownRules::defaults()` |
| `lib/src/style/parse.rs::tests::valid_markdown_rules_produce_no_warnings_and_pass_strict`, `…::invalid_markdown_rules_return_the_rule_validators_reason` | parser registration and strict mode |
| `cli/tests/l1/layout_style_frontmatter.rs::style_strict_style_accepts_markdown_cleanup_rules`, `…::style_invalid_markdown_rule_fails_with_its_path` | end to end through `md --strict-style --output html` on the shipped fixture |
| existing `style::coverage_tests` (count 129 → 139, per-rule samples) and `base_style_schema_matches_runtime_descriptor` | descriptor and baseline in lockstep |

### 2. Repair pipeline moved into the library (`cleanup/frontmatter.rs`)

- `darkmatter::markdown::cleanup::frontmatter::repair_frontmatter(source,
  document_path, schema)` runs on raw text before any `Markdown` exists. Pass
  order, repair proofs and diagnostic spellings are unchanged (syntax pass,
  gated rescan, schema tier). `AuthoredCoordinateMap`/`EditProjectionPass`
  moved unchanged as `pub(crate)`.
- **Departure from the §5 sketch (`CleanRequest.schema: CleanSchemaConfig`):**
  the schema configuration is supplied by a lazy `FnOnce() ->
  Result<CleanSchemaConfig, E>`, invoked at most once and only when the
  schema tier runs. Building a document context does repository discovery,
  and the CLI's existing contract
  (`clean_schema.rs::test_absent_and_empty_frontmatter_perform_no_schema_work`)
  forbids that work for absent or empty frontmatter. The library never
  captures CWD, home or repository itself. Errors are typed:
  `FrontmatterRepairError::{Extraction, Schema(SchemaError), Context(E)}`.
- `FrontmatterRepair { source, frontmatter_span, repaired, diagnostics,
  applied }`: diagnostics and `applied` are document coordinates. Phase 4 uses
  `discard_repairs(original)` (for `frontmatter: false` or a fidelity refusal:
  original bytes, `applied: []`, diagnostics kept) and `settings_yaml(original)`
  (the authored YAML when it parses, else the repaired candidate with
  `from_candidate: true` and `authored_span()` projecting candidate offsets
  back to authored document coordinates).
- CLI: `cli/src/commands/clean/frontmatter_repair.rs` is deleted. `clean.rs`
  keeps flags, the context-building closure (`schema_config`), terminal
  reporting; `clean/json.rs` holds the version-1 envelope, which now copies
  document spans directly. JSON output is unchanged (all existing
  `cli/tests/l1/clean*.rs` pass unmodified).

**Tests added:** `lib/tests/l1/clean_frontmatter_repair.rs` (10 tests):
repair before `Markdown` exists plus a repeated repair/read fixed point;
`discard_repairs` byte-identical with BOM, CRLF, comment and padded
delimiters; BOM repair at `0..3` with later spans shifted; absent and empty
frontmatter never invoke the schema closure (counting closure, and an `Err`
closure still succeeds); parseable frontmatter invokes it exactly once;
closure failure is `Context`; a missing `--schema`-style override on stdin
input is a typed `Schema` error naming the file; unrepairable duplicate-key
YAML is returned unchanged with its `yaml.duplicate-key` diagnostic and no
settings YAML; `settings_yaml` authored and candidate cases with projection.

### 3. DMLS workspace settings (`dmls/src/config/mod.rs`)

- `formatting.markdown` replaces `formatting.cleanup`/`formatting.fixed_width`.
  `DmlsConfig.formatting.markdown: MarkdownRuleOverrides` is `#[serde(skip)]`:
  each authored source (`.dmls.toml`, the editor overlay) is validated by
  `rules_from_json` and normalized on its own, then merged leaf by leaf (an
  editor value wins; editor `null`/absent releases to the file leaf). The
  other sections keep their deep merge, with `formatting` removed from the
  overlay first, so serialized defaults never manufacture legacy keys.
- Legacy keys translate per source with one migration warning each per load
  (`compact` → `list-item-padding: none`, `loose` → `one`, `default`/`null` →
  no override, `fixed_width` 1..=1000 → `fixed-width`); unsupported values are
  rejected, never coerced. A legacy key next to `formatting.markdown` (even
  `{}`, even a `null` legacy value) in one source rejects that source's
  formatting. Unknown `formatting`/`formatting.markdown` keys are rejected;
  other sections are not made stricter.
- Visible diagnostics: `ConfigState::diagnostics() -> &[ConfigDiagnostic]`
  (`severity`, `source` = file path or "editor settings", `key`, `message`),
  replaced on every load/reload/`apply_client_settings`. A malformed,
  duplicate-key or unreadable TOML file is now an `Error` record, not only a
  log line. Phase 4 routes these through `window/showMessage`; today they are
  also logged.
- `load_file`/`discover` return `LoadedConfig { config, diagnostics }`;
  `CleanupVariant` is removed. `providers/formatting.rs` has a minimal
  adapter (`list-item-padding` none/one → compact/loose cleanup; a width →
  `reflow_to_width`) until Phase 4 replaces it.
- Interpretations recorded: the overlay's `formatting` still applies when its
  other sections fail to deserialize (and vice versa); a migrated legacy key
  keeps its warning even if its source is rejected for another key; TOML
  dates and non-finite floats are rejected by name before validation.

**Tests added (L1):** `dmls/tests/l1/workspace_formatting_settings.rs`
walks the TOML and JSON matrices from checked-in fixtures
(`dmls/tests/fixtures/formatting/dmls.toml`, `settings.json`, read through
`manifest_dir!()`): `toml_formatting_matrix` (control row with every rule from
`RuleSource::Workspace`; per rule absent, wrong type, one/every element wrong,
empty shapes, duplicate key, trailing garbage; object roots; legacy
translation and unsupported values; mixed shape; unknown keys; semantic
invalids; `[wiki]` unknown keys still accepted; the file's other settings
survive each rejection) and `json_formatting_matrix` (the same walk plus an
explicit `null` per leaf that releases to the file value and is never read
as `false`/`preserve`/`""`; `formatting: null`, `markdown: null`, legacy
`null` with a warning; mixed with a `null` legacy key; bare section without
the `dmls` wrapper). Plus `file_legacy_keys_merge_with_editor_rules`,
`migration_warnings_are_recomputed_not_accumulated`,
`null_settings_clear_the_editor_overlay`,
`rejected_file_formatting_leaves_editor_rules_usable`,
`non_object_settings_keep_the_previous_overlay`,
`duplicate_json_member_names_are_resolved_before_the_loader` (ruling 7 cell
"X: Value").

The malformed-JSON transport cell is
`dmls/tests/l1/stdio_subprocess.rs::malformed_settings_json_is_rejected_by_the_transport`,
run against the compiled binary over real pipes. `lsp-server` rejects the
payload (`malformed LSP payload`) and stops reading, so the following valid
request is never answered and the server exits non-zero. The loader never
sees malformed JSON, so a malformed message cannot leave rules half-applied.
This pins existing transport behavior; ruling 7 keeps it out of scope.

### 4. Wave 3 foundation parity

- **Exports.** `markdown::cleanup::{rules, diagnostic, frontmatter}` are
  public modules; `style::StyleParseError::InvalidRule` is new. Downstream
  users of the changed types: `rg StyleParseError|dmls::config|CleanupVariant`
  outside the area found none (Claudine uses only `cleanup_content`, whose
  behavior is unchanged in this phase).
- **Shipped-schema corpus and descriptors.** Ran the `meta_schema*`,
  `schema_phase_validation`, `inline_document_text`, `schemas_*` and
  `base_schema*` L1 groups (188 tests), and the guards and layout tests
  (`*guard*`, `test_layout`, `clean*`; 198 tests): all pass.
- **DMLS agreement.**
  `dmls/tests/l1/style_markdown_parity.rs::style_markdown_completion_and_validation_agree_with_the_rules`
  runs through the LSP protocol: the shipped control document has no
  frontmatter diagnostics; key completion under `style.markdown` offers every
  rule not yet written; value completion offers the
  `list-interior-padding` members; for twelve values, DMLS flags the line
  exactly when the runtime validator rejects the value (including `"4"`,
  `"true"`, `"80"` and `5`).
- **Checkpoint.**
  - Precedence and malformed-config behavior are observable: section 1
    tests and the DMLS matrices.
  - Raw repairs run before `Markdown` construction:
    `clean_frontmatter_repair::unparseable_frontmatter_is_repaired_before_markdown_exists_and_is_a_fixed_point`.
  - No new environment capture or execution sites: the context-construction,
    path-lookup and spawn-site guards pass, and the rule reader is passive.
- **Docs.** `dmls/docs/editors/README.md` replaced its `formatting.cleanup`
  example with `formatting.markdown`, plus a migration note. Editor display
  of config findings is marked **planned**. The full `style.markdown` topic
  page and the `docs/cli/clean.md` and `docs/lsp/features.md` updates remain
  Phase 5 (AC 14).
- **Skill.** `.claude/skills/darkmatter/cleanup.md` (new) records the rule
  validator and library repair entry point; `SKILL.md` links it (199 lines).

### Verification

**Requirement-to-test mapping (Phase 2):**

| Requirement | Test(s) |
| --- | --- |
| Typed rule leaves, absent ≠ null, precedence, width implication, provenance | `lib/tests/l1/clean_rules.rs` (section 1 table) |
| YAML robustness matrix, every column × row, plus semantic invalids and object roots | `clean_rules::yaml_matrix_walks_every_rule_and_shape`, `…::semantic_invalid_values_discard_the_whole_object`, `…::object_roots_cover_absent_null_wrong_and_empty_shapes` |
| Baseline disabled; document schema cannot authorize unsupported values | `clean_rules::rules_do_not_depend_on_the_baseline_or_a_document_schema` |
| Baseline and runtime agree; defaults match without coercing quoted values | `base_schema_end_to_end::base_style_markdown_schema_agrees_with_the_rule_validator`, `…::base_style_markdown_defaults_match_the_runtime_defaults` |
| Style parser registration; strict rendering accepts valid rules | `style::parse::tests::*markdown_rules*`, `cli/tests/l1/layout_style_frontmatter.rs::style_strict_style_accepts_markdown_cleanup_rules`, `…::style_invalid_markdown_rule_fails_with_its_path` |
| Library raw-source repair, disabled-repair preservation, lazy schema, typed errors | `lib/tests/l1/clean_frontmatter_repair.rs` (section 2) |
| `md clean` behavior unchanged after the move | existing `cli/tests/l1/clean*.rs` (125 with guards), unmodified |
| TOML/JSON matrices, migration, mixed shape, unknown keys, visible diagnostics | `dmls/tests/l1/workspace_formatting_settings.rs` (section 3) |
| Malformed JSON at the transport; duplicate JSON names | `stdio_subprocess::malformed_settings_json_is_rejected_by_the_transport`, `workspace_formatting_settings::duplicate_json_member_names_are_resolved_before_the_loader` |
| DMLS completion and validation agree with the library | `dmls/tests/l1/style_markdown_parity.rs` |

**Gates (macOS):**

- `just test` (darkmatter area, `--no-fail-fast`): 9020 run, 9015 passed,
  5 failed, 12 skipped. The 5 failures are the pre-existing ones recorded in
  Phase 1, all from the unparseable `prompts/_reviews/feature-review.md`:
  `dasherized_identifier_corpus::shipped_corpus_expressions_all_parse`,
  `…::corpus_walk_reaches_every_present_surface` and three
  `feature_review_incident::*`.
- `just lint`: passed, including the zed-dmls wasm32 check.

**Cross-OS.** The OS-sensitive changes are in DMLS: config diagnostics that
carry file paths, temp-dir config loading, and a subprocess exit-status
assertion. `just cross-check dmls --os windows` ran 807 tests: 806 passed and
1 failed. The failure, `lsp_session::frontmatter_union_value_completion`, is
an existing file-path completion test that this phase does not touch. It
passed when rerun alone on Windows (`just cross-check dmls --os windows
lsp_session::frontmatter_union`: 3/3), so it was a one-off under full-suite
load, not a regression. It is recorded here and not in the `os` skill,
because a single observation does not establish an OS-specific pattern. Test
fixtures are LF on every OS (`.gitattributes` `* text=auto eol=lf`). Linux
runs in pull-request CI.

## Phase 3

Phase 3 ran in two sessions. The first built Wave 4 and most of Wave 5 and
then stopped before logging; its work was still uncommitted in the tree. The
second session (this entry) audited that work against the plan, filled the
gaps, removed its scratch tooling, and ran the gates. Everything below
describes the tree as it stands at the end of the phase.

### 1. Protection (`cleanup/protect.rs`)

- `Protection::scan` reuses compose's own recognizers, so directive-looking
  text inside code stays code. Protected material comes in two kinds:
  - Whole lines: every `scan_darkmatter_directives` line, every quoted
    `::shell` line, and the full span of every `scan_block_pairs` `Shell` pair
    (the opaque bodies).
  - Inline spans: every `ExpressionFinder` expression, literal and token.
- A malformed region is a typed `MalformedRegion` that becomes a `Declined`
  outcome with reason `malformed-region`. Malformed regions are unpaired
  page/shell blocks, trailing content, and malformed disclosures. The old
  silent source fallback is gone.
- `ParseView` is the body with protected lines **removed**, plus a segment map
  back to source offsets. This is the Phase 1 spike's representation.
  Following Phase 1 lessons 2 and 3, the map's `start_to_source` and
  `end_to_source` treat a segment boundary as crossing protection.
- `Lines` keeps each line's own terminator (LF, CRLF, or a lone CR). Edits
  reuse the neighboring terminator, and `clean_body` strips and restores a
  BOM.

### 2. Comparison (`cleanup/compare.rs`)

- `Snapshot` is built the same way from the input and the candidate: same
  protection, same `cleanup_parser_options()` (no smart punctuation, no
  definition lists), same normalization.
- What a snapshot holds:
  - The coalesced structural and inline token sequence.
  - The exact protected lines, with neighbor-blank state, event anchor and
    container placement.
  - Expression spellings and every reference definition, unused ones included.
  - Ordered-list numbers, code-block bytes and terminators, and fence info.
- The allowances come only from the resolved rules:
  - a soft break becomes a separator under `joins_soft_breaks()`;
  - an unlabeled fence gains exactly the `default-language`;
  - ordered numbers after the first become sequential under `ordered-lists`;
  - tight/loose may change only for lists that
    `tightness_exempt_lists(model, rules)` reaches through an explicitly
    selected padding rule.
- `Divergence` gives the first input offset that differs. `Fallback::diagnostic`
  turns it into a `clean.fidelity-fallback` finding: whole-document span,
  `NonDeterministicFind`, `repairs: []`, and a message naming the 1-based line
  and byte column. An output-only block is located at the end of the input.
- Test-only seam: `cleanup::test_hooks::install_broken_pass` (type
  `BrokenPass`) corrupts the candidate after every rule has run. It is
  compiled under `cfg(test)` or the new `cleanup-test-hooks` feature in
  `lib/Cargo.toml`, which is off by default and will be used by the Phase 4
  CLI and DMLS suites.

### 3. Body transforms (`cleanup/{body,structure,transform,wrap,edits}.rs`)

- `clean_body(body, &ResolvedMarkdownRules) -> BodyCleanup { text, outcome,
  diagnostics, fallback }` is the guarded engine. It never re-serializes:
  - `structure::Model` records containers, lists, items, inline runs, tables
    and fences in source coordinates.
  - `transform::plan` proposes `SourceEdit`s for nine body rules.
  - `wrap::plan` is the fixed-width second stage.
  - `edits::apply` refuses overlapping edits and any edit that touches a
    protected line; a conflict becomes `Declined` with reason
    `serialization-failure`, never a silent string fallback.
- Rule ownership follows the spec. `list-padding` never pads next to a
  directive line, at a container edge, or at a nested list's boundary (that
  boundary belongs to `list-interior-padding`). Required separators survive
  `none`. Indentation clamps to the band the marker width allows and pads the
  parent marker when the target is out of reach (`1.   top` for 8). Every
  child line of the moved item moves with it: fences, quotes and their bare
  `>` lines, and indented code.
- `wrap` counts container prefixes in display width (`UnicodeWidthStr`),
  breaks only at single ASCII spaces outside code, HTML, math, link
  destinations and expressions, and never starts a line with a block marker.
- Counted parses for `clean_body`:
  - 1 for an unchanged body;
  - 2 for a changed body (the planning parse is also the input side of the
    comparison);
  - at most 3 when a width re-wraps an already edited stage.
  `parse_count::guarded_body` pins these counts. Scanner parses are not yet
  counted separately (see the handoff below).
- **Phase 3 removed ambient `PREFER_ITALICS` restyling** from the library.
  `get_preferred_emphasis_style`, the restyle branch in
  `preserve_original_emphasis`, the flanking helpers, the public
  `EmphasisStyle` enum (no users anywhere in the workspace), and the
  restyle-only unit test are deleted. Two places followed:
  - The `context_construction_guard` allowlist entry for `cleanup/emphasis.rs`
    is removed. Any new environment read there now fails that guard, which is
    the regression test for the removal.
  - `docs/cli/clean.md` drops the `PREFER_ITALICS` bullet.

  The CLI test fixtures still scrub `PREFER_ITALICS` from child environments
  (`cli/tests/common/{fixture,protected_env}.rs`). That scrubbing is harmless
  and was kept.
- **Phase 3 deliberately left one thing alone.** The legacy `cleanup_content*`
  wrappers, `cleanup_to_fixed_width`, `Markdown::cleanup*`, and their callers
  (`md clean`, DMLS formatting, compose, Claudine) still run the old
  serializer pipeline. Wave 6 (Phase 4) owns rerouting them to `clean_body`
  and deleting that pipeline. Unconditional blank-line and final-newline
  normalization therefore still exists in the legacy path only. `clean_body`
  has none, as `final_newline_presence_is_kept` and
  `every_body_rule_off_returns_every_document_byte_for_byte` prove.

### 4. Corpus probe (evidence, not a test)

Before deleting the first session's `tests/scratch_probe.rs`, I ran it once.
It cleans every `git ls-files '*.md'` body (5434 files) under all 18 profiles
in `body_support::profiles()` (defaults, all-off, and every non-default value)
and re-cleans each changed result.

- Results: **0 panics, 0 non-idempotent results, and 1 declined file** under
  every profile.
- The declined file is `biscuit-tui/docs/style-guide/_style-guide.md`, and
  the refusal is correct. Its body opens with `::block when="state.name ==
  "Design Language"` (broken quoting) and closes that block with the typo
  `::endblock`, so the region is genuinely unpaired. Phase 5 records it under
  AC 11 as an intentionally malformed example, not a regression.
- Defaults changed 3464 of the 5434 files and left 1969 unchanged.
- The probe and the test-only `Snapshot::token_divergence` helper it used were
  deleted; neither is a CI gate.

### 5. Gaps closed in this session

- AC 1's exact-text matrix:
  `fidelity_repro::e1_c1_d1_produce_exact_text_under_every_named_rule_value`
  covers 30 cases (E1, C1 and D1, each under 10 values: indentation
  2/8/preserve, `unwrap: true`, `fixed-width: 80`, `list-item-padding`
  none/one, and `list-interior-padding` none/except-nested-lists/all). Each
  expected output was reviewed by hand before it was pinned. It replaces the
  earlier 12-case `e1_c1_d1_list_rules_produce_the_documented_text`.
- Clippy (`len_zero`, `type_complexity`) is fixed. `test_hooks::BrokenPass` is
  the alias the guard tests use.

### Verification

**Requirement-to-test mapping (Phase 3, all L1 in the `darkmatter` lib unit target):**

| Requirement | Test(s) |
| --- | --- |
| AC 1: E1–F1 exact under defaults (D1 gains `text` at column 8) and all-off | `fidelity_repro::defaults_leave_every_reproduction_byte_identical_except_d1_gains_a_language`, `…::d1_language_is_added_at_column_eight`, `…::every_body_rule_off_returns_every_reproduction_exactly` |
| AC 1: E1/C1/D1 under indentation 2/8/preserve, unwrap, width 80, every padding value (exact text and structure) | `fidelity_repro::e1_c1_d1_produce_exact_text_under_every_named_rule_value`, `…::reproductions_keep_their_structure_under_every_rule_value` (independent HTML oracle, all 18 profiles) |
| AC 1: multi-digit markers, code children with a clamped indent, B1 reindent across directives | `fidelity_repro::multi_digit_markers_set_the_minimum_continuation_indent`, `…::code_children_move_with_their_item_when_the_requested_indent_is_clamped`, `…::b1_directive_lines_stay_fixed_while_the_list_spanning_them_is_reindented` |
| AC 3: directive fixed points after a quote paragraph, ordered/unordered lists nested two levels, and a table, under every rule | `directive_fixed_points::directive_lines_are_unchanged_under_every_rule_in_every_context` (+ `the_contexts_are_really_formatted_by_the_defaults` control), quoted shell, disclosures, directive-like code, CRLF directive, unpaired blocks |
| AC 4: each rule's default and every value; list-spacing examples exact; required separators; width true→120 and implication; empty language; ordered `1,1,1`/`3,3,3`/`3,4,5`/`1,3,8`/`2,2,5` | `tests/rules.rs` (all tests), including `list_item_padding_none_and_one_match_the_documented_example`, `list_interior_padding_matches_the_documented_example_from_every_starting_point`, `required_separators_survive_none_under_both_padding_rules`, `fixed_width_true_means_one_hundred_twenty_and_implies_unwrap`, `default_language_custom_and_empty_values`, `ordered_lists_*`, plus `corpus_regressions*` |
| AC 7 (library part): mutations of link target/title, emphasis, hard break, HTML, task state, table alignment, reference definitions (used and unused), expressions, code, info strings, joined soft break; tight/loose; explicit padding scope; ordered numbers; reflow-introduced corruption; output-only block location | `tests/guard.rs` (all tests) |
| AC 9: idempotence | `body_support::clean_accepted` asserts a second run is `Unchanged` in every AC 1–4 test; `source_fidelity::every_profile_is_idempotent_on_the_corpus` |
| AC 13 (library part): CRLF, mixed, lone CR, BOM, final newline, empty bodies, unused references, malformed region location, no trailing whitespace | `tests/source_fidelity.rs` (all tests) |
| AC 12 (body part): parse counts | `parse_count::guarded_body::*` |
| `PREFER_ITALICS` removed | `lib/tests/l1/context_construction_guard.rs` (allowlist entry removed; guard rejects any env read in `cleanup/emphasis.rs`) |

**Gates (macOS):**

- `just lint` (darkmatter area): **passed**, including the zed-dmls wasm32
  check, after fixing three clippy findings in the first session's code.
- `just test --no-fail-fast`: 9103 run, 9095 passed, 5 failed, 3 timed out,
  12 skipped.
  - The **5 failures** are the pre-existing ones from Phases 1 and 2, all from
    the unparseable `prompts/_reviews/feature-review.md`:
    `dasherized_identifier_corpus::{shipped_corpus_expressions_all_parse,
    corpus_walk_reaches_every_present_surface}` and the three
    `feature_review_incident::*` tests.
  - The **3 timeouts** hit the 30s limit under full-suite load. All three
    pass when rerun alone:
    - `compose::type_tests::test_compose_context_capture` in 1.9s;
    - `serialized_code_neighbors::serialized_neighbors_render_in_every_table_cell_position`
      in 2.5s;
    - `entry_point_parity::darkmatter_entry_points_agree_on_every_reference`
      in 4.8s (already recorded in Phase 1).

    None of them touches cleanup.
  - Every cleanup test passes, including the new exact matrix.
- No test name or module segment uses a tier marker. Every new test is an L1
  unit test in the lib target, declared through `cleanup/tests/mod.rs`. The
  run above confirms `just test` selected and passed them.

**Cross-OS.** No cross-check was run, and I judge the risk low. Phase 3's
production changes are pure in-memory text transforms, with no paths,
filesystem, processes, or environment reads; one environment read was
removed. The OS-sensitive concern is line terminators, and that is handled
in code and tested with string literals: `Lines` keeps LF, CRLF and lone-CR
terminators per line, and `source_fidelity::{crlf_*, mixed_*,
a_lone_carriage_return_*}` cover them. These tests read no checked-out
files, so `core.autocrlf` cannot change their inputs. Linux and macOS are
proven by pull-request CI; Windows by the push to `main`.

## Phase 4

Phase 4 rerouted every cleanup consumer to the guarded engine, assembled the
raw-source transaction, and wired the CLI and DMLS to it. The library work
(Wave 6 and the Wave 8 parity fixtures) was done directly; the old-test
triage, the CLI adapter and the DMLS adapter ran as three parallel subagents
against the frozen library contract and were reviewed afterwards.

### 1. Transaction (`cleanup/document.rs`)

- `clean_document(&CleanRequest { source, overrides, workspace,
  document_path }, schema_closure) -> Result<CleanResult, FrontmatterRepairError<E>>`.
  Order: `repair_frontmatter` on the raw text; one passive read of
  `style.markdown` from `settings_yaml` (authored YAML, or the repair
  candidate when only that parses, with rule-diagnostic spans projected back
  through `SettingsYaml::authored_span`); one `resolve`; `clean_body` on the
  body of the text being emitted; reassembly from that text's own
  frontmatter bytes.
- **Departure from the §5 sketch:** the request carries `overrides:
  &MarkdownRuleOverrides` (named for what it is, not `cli`), and the schema
  configuration stays the Phase 2 lazy closure rather than a
  `CleanSchemaConfig` field, so absent or empty frontmatter still builds no
  context.
- Outcomes:
  - `Invalid` when the frontmatter that would be emitted does not parse:
    neither authored nor candidate parses, or `frontmatter: false` while only
    the candidate parses. Source returned, nothing cleaned, `applied: []`.
  - `Declined` when the body is refused: source returned, `applied: []`
    (accepted candidate repairs are discarded), frontmatter findings kept as
    report-only, the fallback finding appended last and located in authored
    coordinates (the body is a suffix of both the source and the repaired
    text, so its offset is unaffected by repairs).
  - `Changed`/`Unchanged` by comparing the emitted text with the source;
    `applied` is non-empty only for `Changed`.
- Empty frontmatter blocks are now kept (the old CLI `assemble` dropped
  them). A BOM before frontmatter is still removed as the existing applied
  repair; with `frontmatter: false` it stays.
- `CleanDiagnostic.reason: Option<FallbackReason>` became
  `fallback: Option<Fallback>` (reason, document offset of the first affected
  block, detail) and `Fallback::message(line, column)` formats the message,
  so DMLS can restate the location in the client's position encoding. The
  version-1 JSON envelope is built separately by the CLI and is unaffected.

### 2. Wrappers, compose, render (Wave 6)

- `cleanup/mod.rs` now exposes `cleanup_content` (defaults) and
  `cleanup_content_with(&MarkdownRuleOverrides)` (overrides at CLI level),
  both `clean_body(..).text`. Removed: `cleanup_content_{compact,loose}`,
  every `cleanup_content_with_indent*` variant (including the three
  `_preserving_incidental` ones), `cleanup_to_fixed_width`,
  `ListSpacingMode`, `DEFAULT_INDENT`. `Markdown::cleanup` stays;
  `Markdown::cleanup_with(&MarkdownRuleOverrides)` replaces
  `cleanup_{compact,loose,with_indent*,with_fixed_width}`. No wrapper keeps
  the old newline stripping. `Indentation::from_spaces` is new.
- The old serializer pipeline and its modules are deleted: `lists.rs`,
  `emphasis.rs`, `brackets.rs`, `blockquote.rs`, `tables.rs`,
  `perf_profile.rs`. The list-marker width helpers the standalone reflow
  helpers still use moved to `cleanup/markers.rs`; `opaque.rs` shrank to the
  range scan `reflow/semantic.rs` uses; `reflow.rs` lost the dead
  `collapse_incidental_soft_break_events`.
- `strip_incidental_newlines` and `reflow_to_width` remain public standalone,
  unguarded helpers with their existing counts (Phase 1 policy). Nothing in
  cleanup, compose, the CLI or DMLS calls them any more.
- Compose: `ComposeOptions.list_spacing` is a `rules::ListItemPadding`
  (Normal/Compact/Loose became Preserve/None/One, cache tags unchanged) and
  `with_indent_size(usize)` became `with_indentation(rules::Indentation)`
  (the cache encoding still writes spaces, `0` for preserve). The cleanup
  operation builds explicit overrides (`unwrap` from
  `IncidentalNewlineMode`, `fixed_width`, `indentation`,
  `list_item_padding`) and calls `cleanup_content_with`; it does not read the
  document's `style.markdown`, as Phase 1 decided.
- `md` rendering (`cli/src/commands/render.rs`) and `md compose`
  (`cli/src/commands/compose.rs`) map `--indent` through
  `Indentation::from_spaces` (the parser only admits 2, 4, 8).
- `lib/README.md`'s cleanup section described the removed API and the old
  Normal/Compact/Loose behavior; it now shows `cleanup`, `cleanup_with`,
  `clean_document` and the compose options.

### 3. Scanner parses (AC 12)

- `compose::parse_utils::scan_code_regions` is the one place directive and
  expression scanners parse to find code; it returns all code regions plus
  code blocks only (what `ExpressionFinder` skips) and records a separate
  `scanner_parse_count` under `cfg(test)`. `find_code_regions` and
  `ExpressionFinder`'s private helper delegate to it, so every scanner parse
  in the crate is counted.
- `scan_block_pairs_with_code_regions`, `scan_darkmatter_directives_with_code_regions`,
  `scan_disclosures_with_code_regions` and `ExpressionFinder::with_code_blocks`
  take precomputed regions; the original entry points delegate.
  `Protection::scan` computes regions once per text, so a cleanup run scans
  once per text it inspects (input, re-wrap stage, candidate) instead of four
  or five times. The scanners keep compose's `Options::all()` (Phase 1
  lesson 5).

### 4. CLI routes (`md clean`, `md FILE --save`)

- `run_clean` makes one `clean_document` call. `open_argument`, the lazy
  `schema_config` closure and its document-context derivation are unchanged.
  `rule_overrides` turns only explicit flags into overrides: `--indent` →
  `indentation`, `--compact`/`--loose` → `list-item-padding` `none`/`one`,
  `--fixed-width N` → width (implying unwrap), `--ignore-incidental-newlines`
  → `unwrap: false` and `fixed-width: false`. `CleanOptions.list_spacing`
  became `list_item_padding: Option<ListItemPadding>`.
- Outcomes:
  - `Changed`/`Unchanged`: print, or `--save` writes only on `Changed`.
  - `Declined`: the original is printed (`--save` writes nothing), the
    finding goes to stderr, exit 0. `--json` gives `changed: false`,
    `applied: []` and the fallback finding, with stderr empty.
  - `Invalid`: exit 1 with no save. `--json` prints the envelope (now with
    `applied: []`); human mode keeps the existing YAML error.
- Human stderr keeps "frontmatter suggestions (not applied)" for
  non-auto-applied Syntax/Schema findings and adds a "markdown cleanup"
  section for `clean.invalid-rules` and `clean.fidelity-fallback`, using
  `Prose`/`UnorderedList`.
- One addition: under `--save`, when repairs were applied and the body delta
  is empty, the "No changes" delta report is skipped. Otherwise it would
  contradict the repair notice, which
  `test_clean_save_reports_frontmatter_repair_rather_than_no_changes` forbids.
- **Hook departure (AC 7):** the broken-pass hook is process-global and does
  not cross into a spawned `md`. The CLI therefore proves the hook in process
  (`clean_fidelity::broken_pass_refusal_is_never_saved` calls
  `darkmatter_cli::commands::run_clean` with `save: true`; the file stays
  byte-identical, and the test fails with the hook removed). The
  `CliProcessFixture` tests use real declines (an unpaired `::block`) through
  the binary. Decline handling in the CLI does not depend on the reason.
  `cli/Cargo.toml` gained a darkmatter dev-dependency with
  `cleanup-test-hooks`; as a dev-dependency feature it is on in every test
  build, so no `[package.metadata.ci.tests] features` entry is needed.

### 5. DMLS formatting

- `providers::formatting::formatting(ctx) -> FormattingOutcome { edits,
  warnings }` runs `clean_document` on the unsaved buffer with the workspace
  `formatting.markdown`. Only `file:` URIs pass a document path; an untitled
  buffer gets no trigger discovery. The schema closure builds
  `CleanSchemaConfig::new` from `ctx.file_context()`. Outcome mapping:
  - `Changed`: one whole-document edit;
  - `Unchanged`: nothing;
  - `Declined`: no edit and one warning, with `fallback.message(line,
    column)` at the client's negotiated position encoding;
  - `Invalid` or a repair error: no edit and one warning.

  The provider sends nothing itself.
- `router.rs` sends each warning as `window/showMessage` (Warning). It also
  shows `ConfigState::diagnostics()` once after the initial load and once
  after each `workspace/didChangeConfiguration` (Error and Warning
  severities), never per formatting request. `dmls/docs/editors/README.md`
  drops its "planned" marker for that display.
- No capability changed: no range, on-type, or save formatting.
- The provider's old `Markdown::cleanup`-equivalence unit tests were replaced
  by exact-output tests. `lsp_session::formatting_is_byte_equivalent_to_library_cleanup`
  became `formatting_returns_the_library_clean_transaction_output`.

### 6. Old test triage and consumer updates

- The pre-Phase-4 cleanup tests (`cleanup/tests/{mod,lists,emphasis,escapes,
  blockquote,brackets,tables,reflow}.rs`, `compose/tests/{rendering,frontmatter}.rs`)
  were ported. Their helpers (`clean`, `clean_with`, `clean_unwrapped`,
  `clean_indent`) go through `clean_body`, fail on a decline, assert
  idempotence, and check that the public wrapper agrees, so an "output equals
  input" assertion cannot pass on a silent refusal. Seven tests were deleted:
  - one asserted Normal-mode blank insertion at list level returns;
  - two became duplicates once assertions were exact;
  - three exercised the deleted `align_tables_in_stream`, or were covered by
    the exact table tests;
  - two wide-marker idempotence tests were merged into one.

  The standalone-helper tests use `strip_then_reflow` in place of
  `cleanup_to_fixed_width`. No engine defect was found.
- Expectation updates for documented behavior changes, outside cleanup's own
  tests:
  - **Trailing whitespace is no longer stripped.** An empty interpolation
    leaves its spaces. Affected:
    - `lib/tests/l1/compose_diagnostic_identity.rs` and its CLI twin;
    - `lib/tests/l1/literal_token.rs`;
    - `claudine/lib/src/system_prompt/prepare/tests.rs`, where a leading
      space is kept.
  - **No serializer `[` escaping.** Affected:
    `lib/tests/l1/expression_regression.rs::regression_terminal_in_interpolation`.
  - **No forced final newline.** Affected: the three
    `compose::schema_validation::tests::*` that expected `\n`.
  - **No blank line inserted after a heading.** Affected:
    `claudine/lib/src/composition/closure/tests.rs::cleans_the_agents_body_and_hashes_the_cleaned_text`
    and `claudine/cli/src/commands/wrap/inline.rs::inline_completion_cleans_the_agents_body_on_disk`.
    Both now use an unlabeled fence, which the defaults do change.
  - The CLI's own clean tests (`clean.rs`, `clean_json.rs`,
    `clean_frontmatter.rs`) were updated likewise: soft breaks are kept,
    separate lists are not merged, `1)` is not respelled, and no
    whitespace-only lines appear.

### 7. Invocation parity (Wave 8)

- `lib/tests/common/clean_parity/mod.rs` holds 13 hand-written cases
  (literals, so CRLF and BOM bytes survive checkout normalization). Each case
  has an input, the exact text to emit, and an outcome:
  - repairable frontmatter;
  - repair disabled, with valid YAML and with invalid YAML;
  - unrepairable YAML;
  - empty frontmatter;
  - a BOM with and without frontmatter;
  - CRLF, CRLF frontmatter, and mixed endings;
  - a missing final newline;
  - reference definitions (used and unused);
  - an unpaired directive after repairable frontmatter.
- The same cases run through `clean_document`
  (`lib/tests/l1/clean_parity.rs`, including idempotence), through `md clean`
  printed, `--json` and `--save` (`cli/tests/l1/clean_parity.rs`), and
  through `textDocument/formatting`
  (`dmls/tests/l1/formatting_transaction.rs::every_parity_case_matches_through_formatting`).
  All three agree byte for byte. The CLI and DMLS include the module by
  `#[path]` plus `include_str!`, and both list it in
  `[package.metadata.ci.tests] source-inputs`.
- AC 2: `lib/tests/l1/clean_compose_equivalence.rs` composes A1 (with a fence
  the defaults label) and B1 with each gate true and false, before and after
  `clean_document`. Under the defaults the composed bytes are equal. Under
  `indentation` 2/8, `unwrap`, `fixed-width`, and the explicit padding rules,
  the composed documents render to the same HTML. **Finding:** byte equality
  cannot hold under `indentation: 8`, because cleanup's documented
  parent-marker padding (`1.   first`) survives compose's own cleanup step
  (indentation 4). That is a spelling change by design, not a structural
  one. The test asserts a non-empty set of profiles changed each input, so it
  is not vacuous. Data is fixture-owned, with no shell, network, or captured
  repository.

### 8. Defects found and fixed in this phase

- **Fallback detail used body-relative line numbers.** Reported by the CLI
  subagent. With frontmatter present, the message said "at line 7" (a
  document line) followed by the scanner's "starting at line 4" (a body
  line). `Protection::scan` now words malformed-region details without line
  numbers (for example, "`::block when=\"x\"` is never closed by an
  `::end-block`"); the location is the re-anchored "line L, column C".
  Regression test:
  `cleanup::document::tests::a_malformed_region_declines_the_whole_document`,
  which asserts the exact message.
- The fallback offset points at the first diverging *inline* (for example, a
  link), not the start of its paragraph. Both the CLI and DMLS rely on this,
  and it is more precise than the block start, so it was kept. The tests pin
  it (`line 5, column 5`, and the UTF-16/UTF-8 columns in DMLS).

### 9. Observations for later phases (not changed here)

- biscuit-file's deterministic `yaml.line-ending` repair converts CRLF to LF
  only between the frontmatter delimiters. The delimiter lines keep CRLF, so
  a CRLF document's frontmatter block ends up with mixed terminators. It is an
  accepted, reported repair (`crlf-frontmatter` parity case), but it is a
  candidate follow-up fix in biscuit-file.
- `Markdown`'s frontmatter split drops the body's final newline. The old
  cleanup re-added one, so composed `Markdown::content()` for documents with
  frontmatter used to end in `\n` and now does not. `md compose` prints with
  `println!`, so its stdout still ends in exactly one newline, where the old
  output also had a trailing blank line.
- Composition leaves the spaces around an interpolation that evaluates to
  empty (for example, `A  `). The old serializer stripped them as a side
  effect. If that matters, it belongs to interpolation, not cleanup.

### Verification

**Requirement-to-test mapping (Phase 4):**

| Requirement | Test(s) |
| --- | --- |
| Transaction order, outcomes, discard on refusal, invalid YAML emits nothing, only emitted repairs reported, authored coordinates | `cleanup::document::tests::*` (13 tests), including `a_body_refusal_discards_the_candidate_frontmatter_repair`, `corruption_introduced_by_reflow_is_refused_after_the_rewrap`, `unrepairable_yaml_is_invalid_and_nothing_is_cleaned`, `frontmatter_false_read_from_a_repair_candidate_leaves_invalid_yaml_invalid`, `an_invalid_rule_in_a_repair_candidate_is_reported_at_its_authored_position` |
| AC 6 precedence, all four levels, and invalid-object fallback | `document::tests::each_rule_resolves_overrides_then_document_then_workspace_then_default`; CLI `clean_rules::*` (7); DMLS `formatting_transaction::workspace_rules_apply_and_the_document_overrides_them`, `providers::formatting::tests::{document_style_markdown_beats_the_workspace, invalid_document_rules_are_ignored_while_workspace_rules_apply}` |
| AC 5 compact/loose equal to `list-item-padding` none/one; one migration warning per change | CLI `clean_rules::compact_and_loose_match_document_list_item_padding`; DMLS `a_legacy_setting_warns_once_per_configuration_change_not_per_request`, `config_file_findings_are_shown_at_load_with_their_severity` |
| AC 7 broken pass through consumers, `--save` writes nothing, `--json` fallback, one DMLS warning, refusal after a candidate repair with `applied: []` | CLI `clean_fidelity::{broken_pass_refusal_is_never_saved, declined_clean_*, top_level_save_leaves_a_declined_document_untouched}`; DMLS `an_injected_refusal_reports_a_utf16_column_by_default`, `…_utf8_column_when_negotiated`, `an_unpaired_block_directive_declines_with_one_warning`; library mutation list in `cleanup/tests/guard.rs` (Phase 3) |
| AC 8 frontmatter parity, `frontmatter: false` | `clean_parity` (library, CLI, DMLS); CLI `disabled_frontmatter_repair_*`; DMLS `frontmatter_false_with_repairable_invalid_yaml_returns_no_edits_and_one_warning` |
| AC 2 A1/B1 compose equivalence | `lib/tests/l1/clean_compose_equivalence.rs` |
| AC 12 parse ceilings with separate scanner counts and short-circuits | `cleanup/tests/parse_count.rs`: every wrapper ≤ 2/2; unchanged 1/1; malformed 0 cleanup / 1 scanner; width ≤ 3 with scanner = cleanup; `clean_document` 2/2; standalone helpers 1/1; counters observe repeats |
| AC 13 source fidelity through public routes | CLI `clean_fidelity::{crlf_terminators_are_kept, byte_order_mark_without_frontmatter_is_kept, empty_frontmatter_block_is_kept, missing_final_newline_is_not_added, every_body_rule_off_returns_the_document_unchanged, stdin_routes_clean_and_report}`; `clean_parity` in all three suites; DMLS `formatting_reads_the_unsaved_buffer_not_the_file` |
| DMLS: no edits for unchanged/declined/invalid; no range or on-type formatting | DMLS `an_unchanged_document_returns_no_edits_and_no_message`, `unrepairable_frontmatter_returns_no_edits_and_one_warning`, `formatting_is_advertised_without_range_or_on_type_formatting` |

**Gates (macOS):**

- `just test --no-fail-fast` (darkmatter area): 9135 run, 9124 passed,
  8 failed, 3 timed out, 11 skipped.
  - **5 failures** are the pre-existing ones from Phases 1–3, all caused by
    the unparseable `prompts/_reviews/feature-review.md`:
    `dasherized_identifier_corpus::{shipped_corpus_expressions_all_parse,
    corpus_walk_reaches_every_present_surface}` and three
    `feature_review_incident::*`.
  - **3 failures** were caused by this phase's behavior changes and are
    fixed (section 6): `compose_diagnostic_identity::one_root_in_two_transcluded_documents_warns_once_per_document`,
    `literal_token::a_decoded_token_is_data_everywhere_it_flows` and
    `expression_regression::regression_terminal_in_interpolation`. All three
    pass after the fix.
  - **3 timeouts** hit the 30s limit under full-suite load and pass when
    rerun alone: `serialized_code_neighbors::serialized_neighbors_render_in_every_table_cell_position`
    (4.9s), and both `entry_point_parity::*_entry_points_agree_on_every_reference`
    (7.3s and 9.8s). All three were recorded in earlier phases.
  - The lib unit target alone: 6260 run, 6260 passed after the fixes.
- `just lint`: **passed**, including the zed-dmls wasm32 check, after fixing
  two clippy findings in the new parity tests.
- **Direct consumer, Claudine.**
  - `cargo nextest run -p claudine`: 4736 passed, after the two
    expectation updates in section 6.
  - `cargo nextest run -p claudine-cli`: the cleanup-related failure is fixed
    (`inline_completion_cleans_the_agents_body_on_disk`). The remaining 17
    failures are unrelated to cleanup:
    - **Shipped-prompt drift**:
      `shipped_prompt_route_drift::{fixture_body_matches_the_shipped_body,
      shipped_implement_prompts_have_not_drifted_from_their_fixture}` and
      `shipped_prompts::shipped_implement_plan_real_artifact_executes_mapping_set_before_provider_launch`.
      The shipped `prompts/_implement/implement-plan.md` has drifted from
      its fixture and from the asserted mapping.
    - **Unparseable `feature-review.md`**: three `shipped_prompts`/
      `shipped_prompt_contract` corpus tests and
      `feature_review_cli_preserves_numeric_iteration_and_dependent_paths`.
    - **Missing binary**: ten `codex_app_server::*` tests copy
      `claudine-fake-codex`, which is not built here (`NotFound`).
    - **Timing**:
      `wrap_perf::composition_setup_and_provider_handoff_order_matches_phase_1_baseline`
      passes alone (0.7s).

**Cross-OS (native Windows, `just cross-check <pkg> --os windows <filter>`):**

| Package | Filter | Result |
| --- | --- | --- |
| `darkmatter-cli` | `clean` | 132 run, 132 passed, including CRLF/BOM stdin and file routes and the parity suite |
| `dmls` | `formatting` | 32 run, 32 passed, including `file:` URIs and the parity suite |
| `darkmatter` | `clean` | 427 run, 427 passed, including the transaction and parity tests |

Linux and macOS are proven by pull-request CI, and WSL2 by the nightly run;
nothing here is Linux-specific. A filterset passed through the recipe
(`-E 'test(/…/)'`) fails with a shell syntax error, so use a plain name
filter.

**Tier placement.**
- No new test name or module uses a tier marker.
- The new lib tests are in the lib unit target (`cleanup/document.rs`) or in
  the `l1` binary (`clean_compose_equivalence`, `clean_parity`, declared in
  `lib/tests/l1/main.rs`).
- The CLI tests are `clean_fidelity`, `clean_rules`, and `clean_parity`,
  declared in `cli/tests/l1/main.rs`.
- The DMLS test is `formatting_transaction`, declared in
  `dmls/tests/l1/main.rs`.
- `just test` selected and ran all of them.

**Repository state.** While this phase ran, an external process committed
the in-progress tree as `2be693c89`…`43ff7441a`. The `43ff7441a` commit
carries the first draft of this section. The same process moved
`fixes/2026-09-30-file-refs-use-magic` into `_completed`, unstaged. This run
did neither and touched neither. The edits made after those commits are left
unstaged:

- the three Claudine test files;
- `lib/tests/l1/{compose_diagnostic_identity,expression_regression,literal_token}.rs`;
- this log's sections 4–9 and this verification;
- the plan check-offs and the frontmatter of the plan, the log and the spec.

## Phase 5

Phase 5 checked cleanup against real documents and rewrote the docs to
describe what the code now does. The corpus scan turned up one defect, which
is fixed here (section 3).

### 1. Real-world fixture (AC 10)

- `lib/tests/fixtures/clean/prompts-commit-464e2d90e.md` holds the exact
  bytes of blob `464e2d90ee389470a3cf198c4ff5a6cf3f50d9cd` (`prompts/commit.md`,
  retrieved with `git cat-file blob 464e2d90e`). `git rev-parse
  c5ad95142~1:prompts/commit.md` gives this blob; `c5ad95142` replaced it.
- The fixture carries no metadata, so it stays byte-identical to the blob.
  Its provenance (path, blob, retrieval command) is in the sidecar
  `prompts-commit-464e2d90e.provenance.yaml`, and
  `clean_real_world_prompt::the_fixture_is_the_recorded_blob_byte_for_byte`
  hashes the fixture as a Git blob (gix, already a dev-dependency) and
  compares the result with the sidecar. `.gitattributes` sets `eol=lf`, so a
  Windows checkout keeps the same bytes.
- Under the defaults the guard accepts the prompt and changes exactly one
  line: its one unlabeled fence becomes `text`. No frontmatter repair is
  applied, every soft break is kept (same line count and the same
  `SoftBreak` count), and the output is a fixed point
  (`defaults_accept_the_prompt_and_only_label_its_unlabeled_fence`).
- `the_cleaned_prompt_composes_to_the_same_bytes_as_the_original` composes
  the original and the cleaned prompt in a fresh `gix::init` repository in a
  temporary directory. The two transcluded files are fixture-owned
  stand-ins. It composes with `message` both absent and set, because
  `message` gates two `::block` regions, one of them inside a list. The two
  composed outputs are byte-identical, and no directive line survives
  composition. No shell runs and nothing is read remotely.

### 2. Corpus scan (AC 11)

**Method.** A throwaway `#[ignore]` probe in the `l1` binary, run once from a
release build and then deleted (not a CI gate). It runs
`clean_document` with default overrides, `workspace: None`, and no saving
on every path `git ls-files -z -- '*.md'` lists. It passes the file's path
and a schema closure built with `request_support::context_at(<document
dir>)`, which mirrors `md clean FILE`: a passive context with schema-root
and trigger discovery. Nothing is composed and no expression or shell runs.
Panics are caught, and each `Changed` result is cleaned a second time to
check idempotence. The scan ran twice: once before the section 3 fix and
once after it. The final numbers are below.

**Counts (final run, 5436 tracked paths):**

| Result | Count | Notes |
| --- | --- | --- |
| changed | 3513 | all idempotent; 116 also applied a frontmatter repair |
| unchanged | 1877 | |
| declined | 1 | see below |
| invalid | 23 | frontmatter Darkmatter itself cannot load |
| error (typed schema error) | 13 | `FrontmatterRepairError::Schema` |
| unreadable | 9 | tracked paths missing from the worktree |
| panic | 0 | |
| non-idempotent | 0 | |

The first run, before the fix, gave changed 3497, unchanged 1873, invalid
43, and the same declined, error, and unreadable rows.

**Dispositions.**

- **Declined (1): an authored defect, not a regression.**
  `biscuit-tui/docs/style-guide/_style-guide.md` opens a region with
  ``::block when="state.name == "Design Language"`` (broken quoting) and
  closes it with the typo `::endblock`, so the region really is unpaired.
  The refusal (`MalformedRegion`) is correct. It is the same file the
  Phase 3 body-only probe found.
- **Invalid (23): authored defects, unchanged from before this fix.** In
  each one, Darkmatter's own frontmatter parser, the one `Markdown` and
  compose use, cannot load the block:
  - 19 have a duplicate key: 16 `packages` and 3 others. Most are
    `_completed` plans and reviews.
  - `claudine/features/_completed/2026-04-17-perf-flag/review-1.md` fails
    with "mapping values are not allowed".
  - `biscuit-clipboard/features/2026-05-02-kickoff/plan.md` and
    `claudine/features/_completed/2026-05-02-flattened-bridge/plan.md`
    fail with "did not find expected key".
  - `prompts/_reviews/feature-review.md` is the known unparseable prompt
    behind the five pre-existing L1 failures.
- **Error (13): typed schema errors from authored `$schema` values.** The
  installed pre-change `md` fails on the same files. The spec requires these
  to stay typed errors and never be replaced by a guessed schema.
  - Missing schema files: `fix-log.yaml` and `feature-log.yaml` (bare
    names, in two `_completed` logs), `./_schema.yaml` (the two
    `claudine/gen` agent-cli fixtures, whose sibling is `_schema.r1.yaml`),
    and `./implement.yaml` (a DMLS mapping-only corpus fixture).
  - Six intentionally malformed schema probes under
    `messenger/features/2026-09-17-research-metadata-pipeline/spikes/schema-pilots/probes/`.
  - Two authored grammar errors: `darkmatter/docs/research/parsing/_fleet_future.md`
    (unknown type `crate`) and
    `sniff/reviews/_completed/2026-06-29-cross-platform/review-1.md`
    (boolean values in `$schema`).
- **Unreadable (9): accounted for, not skipped.** These are the nine files of
  `darkmatter/fixes/2026-09-30-file-refs-use-magic/`. They are still tracked
  but deleted from the worktree, because the external process noted in
  Phase 4 moved them, unstaged, to `_completed/`.
- **Valid supported syntax that was refused.** The first run found 20
  documents, including the shipped prompts `prompts/_reviews/cycles.md` and
  `prompts/_reviews/comprehensive-review.md`. All were fixed in this phase
  (section 3) and are accepted by the final run. No refusal of valid
  supported syntax remains, so no `_unscheduled` fix was needed.
- **Performance observation (not changed).** Body cleanup took milliseconds
  even on 240 KB documents. The slow cases (up to about 10 s alone in
  release, for example
  `claudine/features/2026-09-21-lifecycle-ergonomics/spike/fixtures/variants/lifecycle.md`)
  are all in the frontmatter schema tier. The installed pre-change `md
  clean` is equally slow on them (7.6 s), so this predates the fix and is
  out of its scope.
- No repository Markdown was rewritten. `git status` shows only this phase's
  own files.

### 3. Defect fixed: supported frontmatter was reported `Invalid`

- **Symptom.** In the first scan, 20 documents came out `Invalid`. Their
  frontmatter fails strict YAML but loads in Darkmatter. Through `md clean`
  that means exit 1 and no cleaning. The installed pre-change `md` exits 0
  on them; the reproduction below is `prompts/_reviews/cycles.md`. Two
  shapes cause it:
  - a bare interpolation value, `review: {{ ctx.area }}/reviews/…`
    (`cycles.md`, `comprehensive-review.md`);
  - tab indentation inside a block scalar (`.claude/skills/lsp/*`, the
    `audio-programming` research docs, and others).
- **Cause.** The base branch enforced "unrepairable YAML" by building a
  `Markdown`, whose parser (`parse_yaml_with_fallbacks`) falls back to tab
  normalization and then to `{{ }}`/`$( )` protection. The Phase 4
  transaction used strict YAML (`settings_yaml`) as the acceptance test
  instead, which departed from the "existing error contract" the spec
  requires to be kept.
- **Fix.** `frontmatter::load_frontmatter_yaml` (`pub(crate)`) exposes the
  fallback parse.
  - `clean_document` now reports `Invalid` only when that parse also fails
    on the block being emitted.
  - When only Darkmatter can load the block, it is emitted as authored, no
    repair is applied, the body is formatted, and the strict-YAML
    `yaml.parse` finding is still reported as a suggestion.
  - Document rules are read from Darkmatter's loaded mapping, so
    `style.markdown` still applies. A `clean.invalid-rules` finding falls
    within the frontmatter span, because strict-YAML key location is
    unavailable there.
  - The module docs and `## Returns` were updated to match.
- **Tests** (each fails without the fix):
  - `cleanup::document::tests::frontmatter_only_darkmatter_loads_is_kept_and_its_body_is_cleaned`
    (the `cycles.md` frontmatter and the `lsp.md` tab shape);
  - `…::rules_are_read_from_frontmatter_only_darkmatter_loads` (a document
    rule applies, `frontmatter: false` is not invalid, and an invalid rule
    is reported within the block);
  - a new `bare-interpolation-frontmatter` row in the shared parity table,
    run through `clean_document`, `md clean` (printed, `--json`, `--save`)
    and DMLS `textDocument/formatting`;
  - CLI `clean_fidelity::frontmatter_only_darkmatter_loads_is_kept_and_the_body_formatted`
    (exit 0, stdout, `yaml.parse` on stderr, JSON `changed: true` and
    `applied: []`, and `--save`).
  - Duplicate keys and `title: @daily-report`, which Darkmatter rejects too,
    stay `Invalid`. The existing tests for them still pass.
- **Observation (not changed).** `area: {{ ctx.area }}` alone is valid YAML,
  because it reads as a nested flow mapping. An existing frontmatter repair
  respells it `{{ctx.area}}` and reports that as an applied repair. That
  behavior predates this fix, and the expression is equivalent, so the
  parity row uses `cycles.md`'s shape, which strict YAML rejects.

### 4. Documentation (AC 14)

The examples in every page were produced with this branch's `md clean -`.
None were written by hand.

- **Created** `docs/topics/markdown-formatting.md`, the `style.markdown`
  topic page, covering:
  - the five guarantees, with a Mermaid flow of the transaction;
  - every key with its default and a compact example;
  - the list-spacing section for a reader new to tight and loose lists: the
    boundary diagram, what tight and loose render as, and the same input
    under every value of both padding rules, plus the required-separator
    example;
  - precedence as a Mermaid diagram, with the CLI flag mapping;
  - the invalid-object fallback and quoted-value rejection;
  - frontmatter that only Darkmatter loads;
  - why rendering does not read the settings.
- **Rewrote** `docs/inline/cleaning.md`. It was an empty file. It now
  describes compose's cleanup step: it shares the engine with `md clean`,
  takes its settings from `ComposeOptions` rather than the document,
  defaults `unwrap` on, can be disabled with `.disable(ComposeOperation::Cleanup)`,
  and produces output with no forced newline and no stripped whitespace.
- **Updated** `docs/cli/clean.md`:
  - the overview now states the invariants and that the document's rules
    drive cleanup;
  - flag-to-rule mapping, including `--compact` and `--loose`;
  - both stderr sections;
  - a new "When Cleanup Declines" section (human, `--json`, `--save`,
    exit 0, `clean.invalid-rules`);
  - exit status, including frontmatter that only Darkmatter loads;
  - a rule-by-rule transformation table that replaces the old list, which
    described the deleted serializer;
  - examples. The old fixed-width example was wrong: its first line was 82
    columns, so it was regenerated.
- **Updated** `docs/lsp/features.md`: the `textDocument/formatting` row and
  §4.10 now describe the implemented whole-document transaction and the
  decline warning. `rangeFormatting` is no longer listed as part of it.
- **Updated** `dmls/docs/editors/README.md` with a new "Formatting" section:
  frontmatter repair during formatting, precedence over
  `[formatting.markdown]`, no range or on-type formatting, and the
  `window/showMessage` warning when formatting declines. Migration was
  already documented in Phase 4.
- **Updated** `dmls/docs/features.md` and the `dmls/README.md` feature row.
  Both still said formatting was "byte-equivalent to `md clean`" through
  `Markdown::cleanup`.
- **Updated** `docs/darkmatter-compose-pipeline.md` (the Cleaning summary),
  `docs/structs/Markdown.md` (the method table still listed the deleted
  `cleanup_compact`/`loose`/`with_indent*`), `docs/topics/layouts-and-style.md`
  (a pointer saying `style.markdown` is not a render style), the area
  `README.md` topic list, `cli/README.md` (the Document Cleanup prose and
  examples), and `lib/README.md`. The lib README example claimed cleanup adds
  a blank line after a heading, which it no longer does.
- **Code comments:**
  - `md clean` flag help for `--compact`, `--loose`, and
    `--ignore-incidental-newlines` now names the rule each one sets. The
    old help implied the default collapses wrapping.
  - The `cli/src/commands/clean.rs` module docs, `CleanOutcome::Invalid`,
    `ParityOutcome::Invalid`, and the `providers/formatting.rs` module docs
    now say Darkmatter cannot load the frontmatter rather than that it is
    invalid YAML.
- **Skills:**
  - `.claude/skills/darkmatter/cleanup.md` describes the `Invalid`
    acceptance test, user-doc locations, and how to run a corpus probe.
  - `.claude/skills/darkmatter/compose.md` updates the compose Cleanup
    bullet, which described the removed newline stripping.
- No crate was added or removed, so `docs/dependencies.md` is unchanged.
- Left alone: `dmls/docs/editors/README.md` already linked to the DMLS
  feature spec for the full key list, which predates this phase. That link
  goes against the docs-never-link-to-specs rule; it is noted here for a
  later docs pass and was not changed in this fix.

### 5. Corpus closure (Wave 10)

- The defect in section 3 changed which documents are `Invalid`, so the
  first scan's results were stale. Affected fixtures were rerun (the parity
  suite in all three consumers, plus the CLI and library clean suites), and
  then the full passive scan was repeated. Section 2 holds the final counts.
- Every refusal has a disposition. The declined document is an authored
  defect. The 23 invalid documents are frontmatter Darkmatter itself cannot
  load. The 13 errors are typed schema errors on authored `$schema` values.
  The 9 unreadable paths are the moved file-refs-use-magic files. No
  refusal of valid supported syntax remains, so no `_unscheduled` fix was
  created.
- The docs match the accepted behavior, including the section 3 fix.
- Spec departures: none new. Section 3 is an implementation ruling that
  restores the spec's "existing error contract" for unrepaired YAML. The
  contract is decided by Darkmatter's own loader, as on the base branch, not
  by strict YAML.
- Checkpoint:
  - AC 10: `clean_real_world_prompt` (3 tests).
  - AC 11: sections 2 and 5 of this log.
  - AC 14: section 4.
  - No repository Markdown was rewritten.

### Verification

**Requirement-to-test mapping (Phase 5):**

| Requirement | Test(s) / artifact |
| --- | --- |
| AC 10 fixture is blob `464e2d90e` byte for byte, provenance recorded | `clean_real_world_prompt::the_fixture_is_the_recorded_blob_byte_for_byte` (Git blob hash = sidecar `blob`) |
| AC 10 default acceptance, guard passes, every hard wrap kept, idempotent | `clean_real_world_prompt::defaults_accept_the_prompt_and_only_label_its_unlabeled_fence` (exact expected text, `SoftBreak` and line counts, second clean `Unchanged`) |
| AC 10 cleaned output composes, fixture-isolated | `clean_real_world_prompt::the_cleaned_prompt_composes_to_the_same_bytes_as_the_original` (fresh `gix::init` repo, stand-in transclusions, `message` absent and set, identical composed bytes) |
| AC 11 corpus counts and dispositions | log sections 2 and 5 (throwaway probe, deleted) |
| Section 3: frontmatter only Darkmatter loads is kept and the body cleaned | `cleanup::document::tests::frontmatter_only_darkmatter_loads_is_kept_and_its_body_is_cleaned` (original `cycles.md` frontmatter and the `lsp.md` tab shape) |
| Section 3: rules read from that mapping; `frontmatter: false` is not invalid; invalid rules reported in the block | `cleanup::document::tests::rules_are_read_from_frontmatter_only_darkmatter_loads` |
| Section 3 through every consumer | parity row `bare-interpolation-frontmatter` in `lib/tests/l1/clean_parity.rs`, `cli/tests/l1/clean_parity.rs`, and `dmls/tests/l1/formatting_transaction.rs` |
| Section 3, CLI end to end (exit 0, stdout, stderr suggestion, JSON, `--save`) | `cli/tests/l1/clean_fidelity.rs::frontmatter_only_darkmatter_loads_is_kept_and_the_body_formatted` |
| Section 3 negatives (duplicate key, `@` scalar with repair off) stay `Invalid` | existing `unrepairable_yaml_is_invalid_and_nothing_is_cleaned`, `frontmatter_false_read_from_a_repair_candidate_leaves_invalid_yaml_invalid`, and the `unrepairable-frontmatter` and `repair-disabled-invalid-yaml` parity rows |

The two section 3 unit tests were confirmed to fail with the
`load_frontmatter_yaml` call removed (2 failed, 13 passed), and to pass with
it in place.

**Gates (macOS):**

- `just test --no-fail-fast` (darkmatter area): 9141 run, 9136 passed, 5
  failed, 11 skipped. The 5 failures are the pre-existing ones from Phases
  1–4, all caused by the unparseable `prompts/_reviews/feature-review.md`:
  `dasherized_identifier_corpus::{corpus_walk_reaches_every_present_surface,
  shipped_corpus_expressions_all_parse}` and three
  `feature_review_incident::*`. No timeouts this run.
- `just lint`: **passed**, with no warnings, including the zed-dmls wasm32
  check.
- `just check-tier-coverage darkmatter`: 0 stranded.
- No consumer outside the darkmatter area calls `clean_document`. A source
  search found only unrelated names in `claudine/gen` and `messenger`.
  Claudine is unaffected, because compose's cleanup does not go through
  `clean_document`.

**Cross-OS:** `just cross-check darkmatter --os windows clean_real_world_prompt`
ran 3 tests and all 3 passed on native Windows. That covers the blob-hash
check under a Windows checkout, where `.gitattributes` `eol=lf` keeps the
bytes. The section 3 change is OS-neutral text logic, and its parity row runs
in every CI environment. Linux and macOS are proven by pull-request CI, and
WSL2 by the nightly run.

**Tier placement.**
- `clean_real_world_prompt` is declared in `lib/tests/l1/main.rs`.
- The new unit tests are in the lib target.
- The CLI test is in the declared `clean_fidelity` module.
- The parity row is in the shared `#[path]` module, which both the CLI and
  DMLS list in `source-inputs`.
- No name uses a tier marker. `just test` selected and ran all of them.
- The fixture and its sidecar are read through `include_str!`, which CI can
  index.

## Phase 6

Phase 6 validated the finished change and prepared it for review. It changed
no source, documentation, or skill files; the only edits are to this log,
the plan, and the spec frontmatter. The fix is **implementation complete,
ready for review**. It stays in the active `fixes/` directory, and nothing
was staged or committed.

### 1. Scoped gates (macOS, this tree)

- `just test --no-fail-fast` (darkmatter area): 9141 run, 9136 passed, 5
  failed, 11 skipped. These are the same 5 failures as in every earlier
  phase: `dasherized_identifier_corpus::{shipped_corpus_expressions_all_parse,
  corpus_walk_reaches_every_present_surface}` and
  `feature_review_incident::{the_shipped_feature_review_composes_and_names_its_spec,
  the_original_typo_would_have_failed_to_compose,
  the_typo_is_one_identifier_under_the_current_grammar}`.
  - **Cause, now traced to its commit.** `prompts/_reviews/feature-review.md`
    does not parse: "could not find expected ':'" at frontmatter line 112. A
    bare `stop` sits inside a `finalize.stack` mapping, at file lines 112
    and 143.
  - The break came from `c5ad95142` ("chore: update commit and review
    prompts"). That commit is on this branch only (`git merge-base
    --is-ancestor c5ad95142 main` is false), and `main`'s copy of the file
    has no `stop` lines.
  - No cleanup code is involved, but CI on this branch's pull request reads
    the prompt and will fail the same way. It is raised as the spec's
    `human_review` item rather than fixed here, because what `stop` was
    meant to do is the author's call.
- `just lint`: passed, including the zed-dmls `wasm32-wasip2` check.
- `just check-tier-coverage darkmatter`: 0 stranded.

### 2. Direct consumers

- **Which crates were checked.** These crates depend on `darkmatter` outside
  the area (found with a `Cargo.toml` search): research, biscuit-icon-cli,
  claudine, claudine-cli, claudine-gen, playa-cli, sniff-cli, messenger,
  messenger-cli, and biscuit-speaks-cli. Only Claudine calls the cleanup
  surface (`cleanup_content` and compose's cleanup).
- **Compilation.** `cargo check --tests` passes for the eight crates other
  than Claudine.
- **Claudine.** `cargo nextest run --no-fail-fast -p claudine -p
  claudine-cli`: 8158 run, 8141 passed, 17 failed. Phase 5 changed
  `body.rs`/`document.rs` after Phase 4's Claudine run, which is why it was
  run again. The `claudine` crate passes in full. The 17 failures are all in
  `claudine-cli`, and none involves cleanup:
  - 9 `codex_app_server::*` tests copy `claudine-fake-codex`, which is not
    built here, so they fail with `NotFound`. This is unchanged since
    Phase 4.
  - 4 tests fail on the unparseable `feature-review.md`:
    `shipped_prompts::{shipped_prompt_corpus_parses_frontmatter,
    shipped_lifecycle_artifacts_use_mapping_only_set}` and
    `shipped_prompt_contract::{shipped_prompts_have_parseable_schemas_and_expressions,
    feature_review_cli_preserves_numeric_iteration_and_dependent_paths}`.
  - 3 tests fail because `c5ad95142` changed
    `prompts/_implement/implement-plan.md` but not Claudine's fixture copy:
    the two `shipped_prompt_route_drift::*` tests and
    `shipped_prompts::shipped_implement_plan_real_artifact_executes_mapping_set_before_provider_launch`.
  - `steering::automatic::tests::three_warnings_send_three_helper_messages_then_nothing`
    failed once under full-suite load. Run alone it passed 3 times out of 3.
    It is a timing flake, playing the part Phase 4's `wrap_perf` flake did.

### 3. Cross-OS evidence (this tree, `just cross-check`)

The hosts came from `BUILD_LINUX`, `BUILD_WIN`, and `BUILD_WSL`, all set. The
WSL leg runs a nextest archive, which is how CI runs it. Every run below
executed the tests; none is compile-only evidence. The runs repeat Phase 4's
Windows evidence because Phase 5 changed `body.rs`/`document.rs` after it.

| Package (filter) | Linux | native Windows | WSL2 |
| --- | --- | --- | --- |
| `darkmatter` (`clean`) | 433/433 | 432/432 | 433/433 |
| `darkmatter-cli` (`clean`) | 133/133 | 133/133 | 133/133 |
| `dmls` (`formatting`) | 32/32 | 32/32 | 32/32 |

macOS evidence is the full `just test` run in section 1. The `clean` filter
selects every cleanup suite: the transaction, the rules matrix, parity, the
compose-equivalence and real-world prompt tests, frontmatter repair, and
JSON. The Windows library count is one lower because of a test that runs on
Unix only. No terminal or browser behavior changed, so L2 and browser runs
were not needed.

### 4. Static review

- **Registration.** Every new test file is declared in its binary's
  `main.rs`: `lib/tests/l1` (`clean_rules`, `clean_frontmatter_repair`,
  `clean_compose_equivalence`, `clean_parity`, `clean_real_world_prompt`),
  `cli/tests/l1` (`clean_fidelity`, `clean_rules`, `clean_parity`), and
  `dmls/tests/l1` (`formatting_transaction`, `style_markdown_parity`,
  `workspace_formatting_settings`).
  - The library unit tests are under `cleanup/tests/` and
    `cleanup/document.rs`.
  - No name carries a tier marker, and the layout gate tests pass.
- **Feature gates.** `cleanup-test-hooks` is turned on through the CLI's and
  DMLS's dev-dependency on `darkmatter`, so every CI build of those test
  targets compiles it. No `required-features` was added.
- **CI-visible repository reads.**
  - Read through `include_str!`: `clean_real_world_prompt` (fixture and
    sidecar) and `clean_rules` (`style-markdown-control.md`).
  - Read through `manifest_dir!().join(literal)`: the DMLS formatting
    fixtures and `docs/schemas/darkmatter.yaml`.
  - The shared parity matrix, `lib/tests/common/clean_parity/mod.rs`, is
    listed in the `source-inputs` of both `darkmatter-cli` and `dmls`.
- **Robustness matrix (plan section "Input Robustness Matrix").** Each table
  has a test that walks every row and column from a checked-in fixture:
  - YAML: `lib/tests/l1/clean_rules.rs::yaml_matrix_walks_every_rule_and_shape`,
    plus the object-root, semantic-invalid, and unknown-name tests;
  - TOML: `dmls/tests/l1/workspace_formatting_settings.rs::toml_formatting_matrix`
    (fixture `dmls.toml`);
  - JSON: `…::json_formatting_matrix` (fixture `settings.json`), plus the
    legacy-key, null-overlay, rejected-file, and duplicate-member tests.
  - A grep for code smells in the new readers found nothing load-bearing:
    - the `#[serde(default)]` hits in `dmls/src/config/mod.rs` are on older
      config structs unrelated to formatting;
    - `unwrap_or_default()` in `normalize_formatting` runs only after a
      rejection has already been reported, which is the matrix's W cell;
    - the one `.ok()` comes after a range filter and cannot fail.
- **Shipped schema.** `docs/schemas/darkmatter.yaml` adds only the
  `style.markdown` block. `base_schema_end_to_end` (17 tests) validates it
  through the embedded baseline.
- **JSON compatibility.** `darkmatter-cli::l1 clean_json` passes all 20 tests.
- **Idempotence.** Every accepted case in `cleanup/tests/` goes through
  `clean_accepted`, which checks that a second run is `Unchanged`.
  `every_profile_is_idempotent_on_the_corpus` repeats that check for every
  profile, and Phase 5's corpus scan found 0 non-idempotent documents.
- **Parse counts.** `cleanup::tests::parse_count` passes all 12 tests.
- **Final diff.** Every file in the branch diff under `darkmatter/`,
  `claudine/`, and `.claude/skills/darkmatter/` is in a phase's frontmatter
  list.
  - No fixture or schema changed beyond what the plan lists.
  - No `docs/` page names this fix. `docs/lsp/features.md` matched the
    search only on its `last_updated` date.
  - The other files in the branch diff come from other specs' planning
    commits and the author's prompt commit.
  - The `fixes/2026-09-30-file-refs-use-magic` move that earlier phases saw
    unstaged is now committed (`973083e40`), and the worktree is clean.

### 5. Acceptance criteria and their evidence

| AC | Evidence (all passing in section 1 unless noted) |
| --- | --- |
| 1 Reproductions E1, D1, C1, A1, B1, F1 | `cleanup::tests::fidelity_repro` (9 tests), with the indentation, unwrap, width, and padding variants; multi-digit markers and code children in `cleanup::tests::lists` (42 tests) |
| 2 A1/B1 compose equivalence with `when` true and false | `l1 clean_compose_equivalence` (2 tests) |
| 3 Directive fixed points per rule | `cleanup::tests::directive_fixed_points` (10 tests) |
| 4 One test per rule and value, list-spacing examples, required separators | `cleanup::tests::rules` (39), `cleanup::tests::lists` (42), `l1 clean_rules` (19) |
| 5 DMLS migration, mixed config rejected, `--compact`/`--loose` equivalence | `dmls::l1 workspace_formatting_settings` (8 tests, including `file_legacy_keys_merge_with_editor_rules` and `migration_warnings_are_recomputed_not_accumulated`), `darkmatter-cli::l1 clean_rules::compact_and_loose_match_document_list_item_padding` |
| 6 Precedence and invalid-object fallback | `l1 clean_rules::{precedence_resolves_each_rule_independently, an_invalid_document_object_falls_back_to_the_other_levels}`, `darkmatter-cli::l1 clean_rules::compact_and_loose_flags_beat_the_document` |
| 7 Fidelity guard: broken-pass hook, `--save`, `--json`, DMLS empty edit and one warning, mutation list, reflow failure, refusal after repair | `cleanup::tests::guard` (11), `darkmatter-cli::l1 clean_fidelity` (17), `dmls::l1 formatting_transaction` (14) |
| 8 Frontmatter parity, `frontmatter: false` | `l1 clean_frontmatter_repair` (10), the shared parity rows run by `l1 clean_parity`, `darkmatter-cli::l1 clean_parity`, and `dmls::l1 formatting_transaction` |
| 9 Idempotence | `clean_accepted` in every body test; `cleanup::tests::source_fidelity::every_profile_is_idempotent_on_the_corpus` |
| 10 Real-world fixture | `l1 clean_real_world_prompt` (3 tests; native Windows 3/3 in Phase 5, and in the table in section 3) |
| 11 Corpus check | Phase 5 log, sections 2 and 5 (a passive scan, not a CI gate) |
| 12 Parse counts and scanner accounting | `cleanup::tests::parse_count` (12 tests); Phase 4 log, section 3 |
| 13 Source fidelity and invocation coverage | `cleanup::tests::source_fidelity` (12), `darkmatter-cli::l1 {clean, clean_json, clean_frontmatter, clean_fidelity}` through `CliProcessFixture`, including `md FILE --save`; DMLS protocol route in `formatting_transaction` and `stdio_subprocess` |
| 14 Docs | Phase 5 log, section 4 |

**Implementation rulings and spec departures.** Phase 6 added none. The
rulings already recorded are:

- Phase 1, sections 5 and 6: the frozen contracts and the robustness
  departures.
- Phase 5, section 3: Darkmatter's own frontmatter loader, not strict YAML,
  decides whether frontmatter is `Invalid`.

**Deferred items.** These are observations only, not filed as fixes:

- biscuit-file's `yaml.line-ending` repair leaves CRLF delimiter lines around
  LF YAML.
- `Markdown`'s frontmatter split drops the body's final newline.
- An empty interpolation leaves the spaces around it.
- A repair respells a valid `area: {{ ctx.area }}` as `{{ctx.area}}`.
- The frontmatter schema tier is slow on some large documents. This predates
  the fix.
- `dmls/docs/editors/README.md` links to a DMLS feature spec.

### 6. Handoff

- Both plan tasks in Phase 6 are checked. All six phases are complete.
- **Needs the author before merge:** the spec's `human_review` item.
  `c5ad95142` broke `feature-review.md` and caused `implement-plan.md` to
  drift from Claudine's fixture, so CI will fail on this pull request until
  the author fixes the two prompts.
