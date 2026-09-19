---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-21T00:36:27-07:00
spec: 2026-07-14-invalid-frontmatter/spec.md
implemented: true
log: darkmatter/features/2026-07-14-invalid-frontmatter/log.md
implemented_by: codex/default
description: "A **feature** review of `2026-07-14-invalid-frontmatter/spec.md`"
feature: 2026-07-14-invalid-frontmatter/review-2.md
previous: 2026-07-14-invalid-frontmatter/review-1.md
next: 2026-07-14-invalid-frontmatter/review-3.md
---

# Review 2 — Invalid Frontmatter

## Verdict

This feature is **not ready for production**. Review 1's core wiring, focused
schema suite, analyzer corpus, and ordinary CLI coverage have been implemented,
and the 105 focused Level-1 tests exercised in this review pass. However, the
file-level frontmatter boundary still misses two required source forms, the
machine-readable contract does not match the ratified v1 contract, and the
spec's explicit performance acceptance remains deferred without measurements.

Sniff identifies `biscuit-file`, `darkmatter`, and `darkmatter-cli` in the two
affected package areas (`biscuit-file` and `darkmatter`). This review changes
only feature-review Markdown and frontmatter; it does not edit a Rust symbol or
production execution flow.

## Findings

### High — File-level BOM and lone-CR documents bypass frontmatter analysis

`extract_frontmatter_block` builds lines only with `split_inclusive('\n')` and
requires the trimmed first line to equal `---`
(`darkmatter/lib/src/markdown/frontmatter.rs:379-421`). A UTF-8 BOM before the
opening delimiter survives `trim()`, while a document using lone CR line endings
is treated as one line. Both inputs are therefore classified as having no
frontmatter, so `repair_frontmatter` takes its zero-work path and never invokes
the YAML engine.

Direct Level-1 process probes reproduce both failures:

- BOM + `title: @daily-report` is emitted as a Markdown heading beginning
  `## \u{FEFF}---`; the invalid scalar remains unquoted.
- Lone-CR frontmatter is emitted as ordinary Markdown beginning
  `---\n\n## title: @daily-report`; it is not normalized or repaired.

This violates the source-normalization requirement and the ratified E-3/E-4
acceptance rows. The analyzer has Level-1 BOM/lone-CR coverage for a standalone
YAML string, but the user-facing `md clean` integration has none. Per the review
rigor rules, that is a **Level-1 integration gap**, not completed coverage. Add
file/stdin/save CLI cases and make the shared boundary authority recognize a
leading UTF-8 BOM and lone CR without weakening its delimiter rules.

### High — `--json` ships a different wire contract and emits no JSON for unrepaired invalid YAML

The ratified v1 envelope contains `version`, structured `source`, structured
`frontmatter`, `diagnostics`, `applied`, and document-level `changed`, with
document-position spans including line/column data
(`decisions.md:208-243`). `CleanJsonReport` instead exposes only `path`,
`frontmatter_offset`, frontmatter-only `repaired`, and `diagnostics`
(`darkmatter/cli/src/commands/clean/frontmatter_repair.rs:82-128`). It omits the
version discriminator, applied-repair audit, document-level change state, and
the promised line/column projection. The Level-1 test at
`darkmatter/cli/tests/clean_json.rs:38-56` explicitly freezes this conflicting
four-field shape, so the test is evidence of the mismatch rather than evidence
that the requirement is met.

The central failure case is also not machine-readable. `run_clean` attempts to
construct `Markdown` before reaching either JSON-output branch
(`darkmatter/cli/src/commands/clean.rs:88-107`), and
`test_json_unrepairable_frontmatter_exits_one_without_envelope` explicitly
requires empty stdout (`clean_json.rs:308-324`). Consequently a consumer asking
for JSON gets ANSI/human error output on stderr and no JSON diagnostic for YAML
that cannot be auto-repaired—the input class for which structured diagnostics
are most important. Preserve the stable exit code, but return the v1 JSON
diagnostic envelope on the error path and align the success envelope and golden
fixtures with the ratified contract.

### High — The explicit no-regression performance acceptance is still unverified

The corrected `clean_hot_paths` benchmark now compiles and the Level-1 counters
prove the intended short-circuits, but no timing comparison has been recorded.
`deferred-performance.md:83-93` explicitly says the main/branch/main Criterion
comparison remains deferred and that the counter tests do not replace it. The
spec requires **no measurable regression** for documents with no frontmatter
and documents with already-clean frontmatter. Until the two corrected
full-pipeline cases are measured on a quiet host and the result is retained,
that production acceptance criterion is open.

The repository has a macOS/Linux/Windows Darkmatter CI definition, but this
review had runtime evidence only on macOS and did not find a completed
three-platform result for both affected package areas. Record the relevant CI
run alongside the performance result rather than treating portable source or
workflow configuration as runtime evidence.

### Medium — Public CLI and library documentation still describes the pre-feature behavior

`darkmatter/docs/cli/clean.md:1-105` documents cleanup as Markdown formatting,
omits all five new frontmatter/schema/JSON flags, and does not warn that
deterministic frontmatter repairs are default-on. The CLI README's cleanup
section likewise contains only the legacy formatting behavior
(`darkmatter/cli/README.md:205-250`). The public `biscuit-file` documentation
also has no discoverable guidance for `analyze_yaml`, retained-source analysis,
diagnostic certainty, or repair application.

This is particularly risky because the new default can mutate frontmatter in
`--save` mode. Update the CLI guide/README and the `biscuit-file` public API
guide before release, including frontmatter-only scope, schema precedence,
STDERR behavior, JSON examples, stable exit codes, and the absence of
`--strict`.

## Test-level assessment

| User-facing requirement | Strongest verification | Assessment |
|---|---|---|
| Ordinary file/stdin repair, body cleanup, and `--save` | Level 1 spawned CLI | Present for LF/CRLF fixtures; **gap for BOM and lone CR** (Finding 1). |
| Report-only suggestions on STDERR with exit 0 | Level 1 spawned CLI | Appropriate and passing. No terminal-specific style/glyph contract requires Level 2. |
| JSON diagnostics and channel contract | Level 1 spawned CLI | Present but freezes the wrong success shape and no-JSON failure behavior (Finding 2). |
| Schema baseline, inline/file schema, overrides, triggers, and stdin isolation | Level 1 spawned CLI | Appropriate and passing. |
| Fenced body YAML remains outside analysis | Level 1 spawned CLI with byte assertions | Appropriate and passing. |
| Idempotency and untouched-byte safety | Level 1 property/integration tests | Appropriate for covered fixture classes; BOM/lone-CR integration is absent. |
| Hot-path no-regression | Level 1 counters plus an unexecuted Criterion vehicle | Structural evidence exists; required timing evidence is missing. |

No requirement concerns a terminal emulator's input encoder, OS keyboard
events, or renderer-specific glyph/SGR behavior. Level 2 and Level 3 tests are
therefore not required for this feature's current contract.

## Verification performed

- `cargo nextest run -p biscuit-file --test yaml_corpus --test yaml_mutation --test yaml_safety --test parse_count --color never`: **30 passed**.
- `cargo nextest run -p darkmatter --test schema_quoting_safety --test clean_counters -E 'all()' --color never`: **23 passed**.
- `cargo nextest run -p darkmatter-cli --test clean_frontmatter --test clean_json --test clean_schema --color never`: **52 passed**.
- `cargo clippy -p biscuit-file -p darkmatter -p darkmatter-cli --all-targets --color never -- -D warnings`: **passed**.
- Direct stdin byte probes confirmed the BOM and lone-CR integration failures.
- The full area test suites, Criterion timing comparison, and Linux/Windows
  runtime gates were not run in this review.
