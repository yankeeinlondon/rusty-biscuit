---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-21T01:32:10-07:00
spec: 2026-07-14-invalid-frontmatter/spec.md
log: darkmatter/features/2026-07-14-invalid-frontmatter/log.md
implemented: true
implemented_by: codex/default
description: "A **feature** review of `2026-07-14-invalid-frontmatter/spec.md`"
feature: 2026-07-14-invalid-frontmatter/review-3.md
previous: 2026-07-14-invalid-frontmatter/review-2.md
---

# Review 3 — Invalid Frontmatter

## Verdict

This feature is **not ready for production**. Review 2's BOM/lone-CR boundary
handling, basic version-1 JSON envelope, failure-path envelope, and public
documentation are implemented, and all 114 focused Level-1 tests run during
this review pass. However, the v1 JSON contract reports invalid source
coordinates after stacked repairs and for lone-CR documents, raw frontmatter
reconstruction can mutate delimiter bytes without an accepted repair, and the
spec's explicit performance and cross-platform acceptance evidence remains
deferred.

Sniff identifies the affected implementation scope as `biscuit-file`,
`darkmatter`, `darkmatter-cli`, and the `dmls` downstream consumer in the
`biscuit-file` and `darkmatter` package areas. This review changes only feature
Markdown/frontmatter and does not edit a Rust symbol or production execution
flow.

## Findings

### High — JSON spans stop indexing the authored document after an earlier repair

The repair pipeline stores diagnostics and applied edits from three different
coordinate spaces: authored YAML, the post-syntax-rescan YAML, and the
post-syntax schema-analysis YAML
(`darkmatter/cli/src/commands/clean/frontmatter_repair.rs:331-399`). The JSON
projection then adds the original YAML offset to every span without translating
later-pass offsets back through earlier edits (`frontmatter_repair.rs:196-220,
229-265`). That is correct only when no prior repair changed the source length.

A direct Level-1 stdin probe with an inline schema, a leading
`title: @daily-report`, and a later `release: 1.20` reproduces the defect. The
reserved-indicator repair inserts two quote bytes before `release`; the emitted
`schema.type-mismatch` and second `applied` span are `73..77`, while the authored
frontmatter ends at byte 76. Slicing the authored document at that range yields
`20\n-`, not `1.20`. The v1 contract requires every diagnostic and repair span
to use whole-authored-document coordinates, so consumers cannot safely apply or
highlight this envelope.

Lone-CR line/column projection is also incorrect. The newly supported input
form is passed to `line_col_of_offset`, but that helper recognizes only `\n`
(`darkmatter/lib/src/markdown/span.rs:100-105`). A diagnostic on line 2 of a
lone-CR document is consequently emitted as line 1 with a document-wide column.

The current spawned-CLI tests establish authored coordinates only for one
single-pass repair and one BOM case
(`darkmatter/cli/tests/clean_json.rs:156-217`). The multi-diagnostic test checks
only code ordering, not whether each span indexes its lexeme
(`clean_json.rs:265-289`). Add Level-1 golden cases that combine a
length-changing syntax repair with later rescan/schema diagnostics and repairs,
and add CRLF/lone-CR line/column cases. Retain an edit map (or convert every
artifact to authored coordinates as each pass is captured) rather than treating
all pass-local spans as authored spans.

### High — Frontmatter delimiters are rewritten outside the accepted edit set

`splice_frontmatter` discards the authored opening and closing delimiter bytes
and always writes canonical `---\n` lines
(`darkmatter/cli/src/commands/clean/frontmatter_repair.rs:482-492`). This bypasses
the analyzer's candidate generation, value-equivalence gate, and edit audit.
For example, an otherwise clean document using the delimiter lines `  ---  `
and ` --- ` exits successfully with `diagnostics: []`, `applied: []`, and
`changed: true`; its delimiter whitespace is silently removed. CRLF/lone-CR
opening and closing delimiter terminators are likewise normalized without
appearing in `applied`—only terminators inside the YAML span are audited.

This violates the C-4 acceptance invariant that every byte outside accepted
edits and ordinary body cleanup remains identical, and it makes `applied` an
incomplete account of frontmatter repairs. Preserve delimiter slices when no
accepted edit targets them. If delimiter terminator normalization is intended
as part of source normalization, represent those changes as document-relative
repairs and include them in the audit. Add Level-1 file/stdin/save/JSON cases for
valid trimmed delimiters and for all line-ending forms around both delimiters.

### High — Performance and cross-platform acceptance are still explicitly open

DECISION: this is DEFERRED for now and is non-blocking.

The spec requires no measurable regression for no-frontmatter and already-clean
frontmatter documents (`spec.md:148-156`), and the ratified acceptance matrix
requires recorded Criterion comparison plus macOS/Windows/Linux compile/test
evidence (`acceptance-matrix.md:76-86`). The feature's own deferred record says
no timing was run and no completed Linux or Windows runtime evidence was found
(`deferred-performance.md:101-117`). Counter tests and a compiling benchmark
vehicle are useful structural evidence, but the record correctly states that
they do not satisfy timed acceptance (`deferred-performance.md:136-144`).

Complete and retain the quiet-host main/branch/main Criterion bracket for both
hot paths, then record successful scoped runtime gates for both affected package
areas on macOS, Windows, and Linux. Until those explicit acceptance rows are
green, this feature cannot be marked production-ready.

## Test-level assessment

| User-facing requirement | Strongest verification | Assessment |
|---|---|---|
| Source normalization, whitespace cleanup, and reserved-indicator repair through file/stdin/`--save` | Level 1 library and spawned CLI | Functional BOM/lone-CR repair is present; delimiter preservation/auditing is a gap (Finding 2). |
| Schema-proven quoting and compose-parity schema resolution | Level 1 library and spawned CLI | Appropriate and passing for isolated and combined functional output; stacked JSON coordinates are wrong (Finding 1). |
| Report-only diagnostics on stderr with exit 0 | Level 1 spawned CLI | Appropriate and passing. No terminal-specific style or glyph contract requires Level 2. |
| Version-1 JSON fields, channels, diagnostics, repair audit, and failure envelope | Level 1 spawned CLI | Basic shape/channel cases pass, but authored-coordinate and complete-audit requirements fail (Findings 1–2). |
| Body fenced YAML remains outside analysis | Level 1 spawned CLI with byte assertions | Appropriate and passing. |
| Idempotency, safety gates, and untouched-byte preservation | Level 1 property/integration tests | Analyzer invariants pass; file-level delimiter reconstruction bypasses the accepted edit set (Finding 2). |
| Hot-path no-regression | Level 1 counters plus a compiling Criterion vehicle | Required comparative timing is absent (Finding 3). |
| Cross-platform operation | Static portability review and macOS Level-1 execution | Required Windows/Linux runtime evidence is absent (Finding 3). |

No requirement concerns a terminal emulator's rendering, input encoder, or OS
keyboard/mouse delivery. Level 2 and Level 3 tests are therefore not required
for the current feature contract.

## Verification performed

- `cargo nextest run -p biscuit-file --test yaml_corpus --test yaml_mutation --test yaml_safety --test parse_count --color never`: **30 passed**.
- `cargo nextest run -p darkmatter --test schema_quoting_safety --test clean_counters -E 'all()' --color never`: **23 passed**.
- `cargo nextest run -p darkmatter-cli --test clean_frontmatter --test clean_json --test clean_schema --color never`: **61 passed**.
- Direct Level-1 stdin probes reproduced the stacked-repair span corruption,
  lone-CR line/column corruption, and unaudited delimiter mutation.
- The full package-area suites, Criterion timing comparison, and Linux/Windows
  runtime gates were not run in this review.
