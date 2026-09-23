---
phase: 1
created: 2026-07-18
artifact: decisions
status: ratified
---

# Phase 1 Decisions — Contract Freeze

Every decision below was ratified in Phase 1 against the current code
(`impact-report.md`), observed behavior (`baselines.md`), the spec's Scope
section, and the plan's Current-Code Constraints. Later phases implement
these as written; changing one requires an explicit re-ratification recorded
here. Open Questions from `spec.md` resolved here are marked **[OQ]**.
Plan "recommended" rulings adopted as-is are marked **[plan]**.

## D1 — Source-first public API (biscuit-file)

`biscuit-file` gains a source-first analysis entry point that operates on raw
text and never requires a successfully parsed value:

```rust
// biscuit-file (crate root re-exports; implementation under `yaml/`)
pub fn analyze_yaml(source: &str) -> YamlAnalysis;

pub struct YamlAnalysis { /* parse outcome, diagnostics, candidates */ }
pub struct YamlDiagnostic {
    pub code: YamlDiagnosticCode,
    pub span: SourceSpan,
    pub classification: YamlCertainty,
    pub message: String,
    pub repairs: Vec<YamlRepair>,
}
pub struct YamlRepair {
    pub span: SourceSpan,
    pub replacement: String,
    pub explanation: String,
}
pub enum YamlCertainty {
    Deterministic,                             // auto-apply eligible
    DeterministicFindNonDeterministicSolution, // report-only
    NonDeterministicFind,                      // report-only
}
```

- `analyze_yaml` returns structured diagnostics for **both** parseable and
  unparseable input. Unparseable input yields the parse diagnostic (with the
  structured location projection of `serde_yaml_ng::Error::location()`) plus
  any bounded parse-recovery candidates; parseable input additionally yields
  normalization/whitespace/lint findings.
- `YamlAnalysis` retains the structured parse outcome so clean input is
  parsed **once** (Performance contract) and candidates are generated only
  for a matching diagnostic — never for diagnostic-free documents.
- Convenience methods on successfully parsed, source-backed values delegate
  to the same engine: `Yaml::diagnose() -> Vec<YamlDiagnostic>` and
  `Yaml::repair_candidates() -> Vec<YamlRepair>`. `Yaml` constructed via
  `from_value` has no authored source (see D5) and both methods return empty
  results — value-level diagnostics without source spans are out of v1 scope.
- Repair application goes through one shared edit-set utility (Phase 2):
  UTF-8-boundary validation, overlap/out-of-range rejection, end-of-source
  toward beginning application, and an audit record of applied/rejected
  candidates. `YamlAnalysis::apply()` returns the patched source plus audit.
- Schema-aware diagnostics use the same shapes; `YamlDiagnosticCode` carries
  biscuit-file-owned codes, and Darkmatter maps its schema-aware findings onto
  the same struct with darkmatter-owned codes (one uniform JSON shape, D8).

## D2 — Per-repair safety matrix (replaces a single contradictory gate)

| Class | Proof required to auto-apply |
|-------|------------------------------|
| **S1 parse-equivalent edit** (source normalization; whitespace around flow delimiters/commas, mapping colons, sequence markers) | Original parses; candidate parses; candidate `serde_yaml_ng::Value` **exactly equals** original. When an effective schema exists: candidate's schema result set is identical to the original's. |
| **S2 schema-proven invalid-to-valid quoting** (`release: 1.20` → `"1.20"`) | Raw non-coercing schema query identifies **exactly one** plain-scalar node failing **solely** because a string is required; candidate quotes the exact authored lexeme (no other byte changes); candidate parses; candidate passes the **complete** effective schema; the remaining problem set is a strict subset of the original's (no unrelated regression). Value equality is deliberately **not** required — the type change is the point. |
| **S3 invalid-YAML parse-recovery quoting** (`title: @daily-report` → `"@daily-report"`, no original parsed value) | Original fails to parse; the structured error location lands on a plain-scalar lexeme matched by the D3 bounded grammar; candidate quotes exactly that lexeme and nothing else; candidate parses; the parsed value at that position is a string **byte-equal to the original lexeme text**; the lexically-scanned mapping/sequence context of the original matches the parsed context (same key, same nesting); when an effective schema exists, the candidate passes it. The lexeme-equality + context match substitutes for the unavailable Value-equality proof. |
| **S4 combined edits** | Multiple S1 candidates in one run apply only after pair-wise non-overlap validation and a final reparse proving the combined result still satisfies each candidate's class proof. S3 is iterative (D3), one repair per round, each round re-proven. |

Everything outside S1–S4 is report-only in v1 regardless of how tempting the
candidate looks (duplicate keys, key correction, shape repair, anchor/alias,
multi-document, all `non-deterministic-find` lints).

## D3 — Bounded grammar for the no-schema reserved-indicator repair

Auto-apply (S3) is bounded to:

- **Contexts**: a block-mapping value (`key: <lexeme>`) or block-sequence
  entry (`- <lexeme>`) at any nesting depth, where `<lexeme>` is a
  single-line scalar. Flow-collection contexts (`[ ]`, `{ }`) are report-only
  in v1 (lexeme boundary is ambiguous against `,`/`]`/`}`).
- **Trigger**: the original YAML **fails to parse**, and the parser's
  structured location lands on or immediately after a scalar whose first byte
  is one of the YAML indicator characters `` ` `` `@` `%` `&` `*` `!` `|` `>`
  `'` `"` `,` `[` `]` `{` `}` `#` `:` `?` `-`. (The flagship `@` / `` ` ``
  cases are reserved indicators that can never start a token; the others
  cover aliases, anchors, tags, and flow tokens misused as string content.)
- **Lexeme boundary**: from the first non-space byte after the mapping colon
  or sequence dash to the last non-trailing-whitespace byte of the line.
- **Report-only (never auto-applied in v1)**:
  - the remainder contains ` #` (comment-vs-content ambiguity);
  - the scalar spans multiple lines, or starts a quoted scalar that is
    unterminated (quote-repair is a different, unratified grammar);
  - the input parses but has the wrong shape — `host:localhost`, `-80` under
    a sequence intent, `token: abc #123`. No parse error means no S3 proof;
    these are covered, if at all, by report-only lints;
  - indentation/tab repairs, delimiter/bracket repairs, and any candidate
    requiring more than one lexeme change per round.
- **Iteration**: after one accepted repair the document is reparsed; a new
  parse error may start another round. Bounded at 8 rounds; each round must
  repair exactly one error at a strictly advancing error offset. Exhaustion
  leaves the document in its original bytes with all findings report-only —
  a partial parse-recovery chain is never emitted.

## D4 — Clean pipeline order **[plan]**

1. Raw input load: resolve the file reference once, read file/stdin bytes
   once, carry the optional resolved path.
2. Frontmatter extraction via `extract_frontmatter_block` (sole boundary
   authority). Absent or empty block → skip steps 3–5 entirely (zero YAML,
   schema, and trigger work).
3. Schema-agnostic analysis and accepted repairs (biscuit-file engine, D1–D3)
   on the extracted YAML text.
4. Markdown parse of the repaired document. If the frontmatter still does not
   parse (repair impossible), keep today's contract: miette frontmatter-parse
   error, exit 1, diagnostics reported on stderr, file untouched (baseline
   F3/F9 preserved).
5. Effective-schema resolution (D7) and schema-aware diagnostics/accepted
   repairs (S2); the validator context is built at most once per run.
6. Existing Markdown body cleanup (cleanup passes, optional fixed-width
   reflow) on the parsed body.
7. Raw-preserving assembly: the (possibly repaired) raw frontmatter block
   plus the cleaned body. **Never** routed through `Markdown::as_string()`
   frontmatter reserialization.
8. After accepted frontmatter edits, extraction is re-run so every span used
   downstream refers to the current source.

## D5 — Raw-source ownership for `YamlSource::Path` **[plan]**

- `Yaml` retains the source text read at construction in a new **private**
  field (`retained_source: Option<String>`), populated by `new` (path),
  `from_str`, and `from_bytes`. The public `YamlSource` enum and its variants
  are unchanged (non-breaking; see impact report constraint 3).
- Diagnostics and repairs for path-backed values read the retained copy —
  never a second filesystem read. This eliminates the TOCTOU race where a
  file changes between parse and repair. **[OQ: retain-vs-reread → retain]**
- `Yaml::from_value` sets `retained_source: None`; span-based diagnostics and
  repairs are unavailable for it (D1). Behavior for `Text`/`Bytes` sources is
  unchanged in kind: they already retain their input, now exposed through the
  same accessor.
- Accessors: a crate-public `Yaml::source_text() -> Option<&str>` (name
  finalized at implementation) is the single read path.

## D6 — CLI behavior

- **Default auto-apply**: deterministic repairs (S1–S3) are folded into
  `md clean` by default in both output modes — "in place" in the spec means
  into the produced document, not a new file-writing default. No opt-out flag
  ships in v1.
- **stdout vs in-place**: unchanged. Default mode writes the cleaned document
  to stdout; `--save` rewrites the file and renders the delta report to
  stdout; `--save` + stdin remains exit 1 (baseline F8a).
- **Frontmatter byte preservation**: output is assembled per D4 step 7.
  Frontmatter bytes outside accepted edits are preserved verbatim — this
  intentionally replaces today's full reserialization (baseline F2) and
  eliminates the silent `1.20` → `1.2` corruption (baseline F4).
- **`--json`**: new flag on `md clean`. Emits the machine-readable envelope
  (D8) as the **sole stdout payload** (the cleaned Markdown is not also
  printed). With `--save --json` the file is written and the envelope is
  printed; the human delta report and stderr suggestion rendering are
  suppressed in JSON mode (findings live in the envelope).
- **STDERR suggestions**: in human (non-JSON) mode, report-only findings are
  rendered to stderr with `TerminalRenderable` components (`Prose`,
  `UnorderedList`, a code/span component). They never change the exit code
  (spec) and never duplicate analyzer logic into presentation.
- **Verbose delta with originally invalid frontmatter**: when the original
  cannot construct a baseline `Markdown` (baseline F3 class), save-mode delta
  reporting falls back to a raw text-level summary (original raw text vs
  final output) instead of `Markdown::delta`. When the original parses, the
  existing `DeltaReport` path is unchanged.

## D7 — Clean schema-option semantics

`md clean` gains four flags, resolved once per run through a library-owned,
clap-free configuration surface:

| Flag | Meaning | Conflicts |
|------|---------|-----------|
| `--baseline-schema PATH` | Replace the default Darkmatter baseline schema with the given SimplifiedSchema file | `--no-baseline-schema` |
| `--no-baseline-schema` | Disable the Darkmatter baseline (`ctx`, `hash`, `style`, `replace`) | `--baseline-schema` |
| `--schema REF` | **Replace** the document's own `$schema` (inline or file reference) with the given schema — an operator override, matching `md schema validate --schema`'s explicit-schema posture | — |
| `--no-trigger-schemas` | Disable repo trigger discovery and bare-name schema-root lookup | — |

- Effective layering (unchanged from compose): baseline → matching triggers →
  document schema, where "document schema" is the `--schema` override when
  given, else the document's `$schema`.
- **stdin**: trigger discovery needs a document path and is silently inert
  for stdin (same as compose's non-file sources); `--schema` and
  `--baseline-schema` remain meaningful for stdin.
- **Top-level `md <input> --save` shorthand**: takes no schema flags; runs
  with defaults (baseline on, trigger discovery on for file input).
- Resolution is lazy: it runs only when a non-empty frontmatter block exists
  (D4 step 2 gate), and its result is cached for the whole invocation.

## D8 — Cross-cutting rulings

- **Unconstrained keys** **[OQ]**: when the effective schema does not
  constrain a key, the schema-proven quoting tier (S2) **skips it silently**.
  Absence of a schema constraint is not evidence of an error; a suggestion
  here would be noise. Schema-free lints may still flag the key report-only
  under their own rules.
- **JSON envelope** **[OQ: field names + stability]** — v1-stable shape:
  ```json
  {
    "version": 1,
    "source": { "kind": "file" | "stdin", "path": "…" | null },
    "frontmatter": { "present": true, "span": { "start": 4, "end": 120 } } | { "present": false },
    "diagnostics": [
      {
        "code": "yaml.reserved-indicator",
        "classification": "deterministic",
        "message": "…",
        "span": { "start": 11, "end": 24,
                  "start_line": 2, "start_column": 8,
                  "end_line": 2, "end_column": 21 },
        "repairs": [
          { "span": { "start": 11, "end": 24 },
            "replacement": "\"@daily-report\"",
            "explanation": "…" }
        ]
      }
    ],
    "applied": [ /* repairs actually applied, same shape as repairs[] */ ],
    "changed": true
  }
  ```
  - Enum spellings: classification is snake_case —
    `deterministic`, `deterministic_find_non_deterministic_solution`,
    `non_deterministic_find`. Diagnostic codes are dotted lowercase
    (`yaml.*` biscuit-file-owned, `schema.*` darkmatter-owned), stable once
    shipped.
  - `version: 1` pins the envelope; additive fields may appear in later
    versions, existing fields never change meaning.
  - Golden fixtures pin every field, enum spelling, span, empty `repairs`
    arrays, and multi-diagnostic ordering (Phase 6).
- **Line/column indexing**: byte offsets are 0-indexed and end-exclusive
  (`SourceSpan = Range<usize>`); lines are 1-indexed; columns are 1-indexed
  **byte** columns — identical to `darkmatter::markdown::span` conventions.
  UTF-16/character projection is the consumer's job.
- **Idempotency** **[OQ]**: repeated `md clean` is a **byte-level fixed
  point** — cleaning cleaned output changes nothing. This is an acceptance
  test (acceptance matrix A-CLI-9). The pre-existing phantom "whitespace
  changes only" delta report on already-clean files (baseline F8b) is a
  `DeltaReport` quirk outside v1 scope; raw-preserving assembly is expected
  to reduce it but no guarantee is made.
- **BOM scope**: a single UTF-8 BOM at stream start is removed as an S1
  normalization. No other BOM positions or encodings are in scope.
- **Line-ending scope**: inside the frontmatter block, CRLF and lone CR are
  normalized to LF as S1 repairs (each candidate individually proven). Body
  line-ending handling is unchanged and outside this feature.
- **Corpus licensing/pinning** **[OQ]**: the YAML Test Suite subset is
  vendored into the repo (MIT-licensed upstream), pinned by upstream commit
  SHA recorded in a manifest with per-case upstream IDs; tests never touch
  the network. Mutation fixtures are derived from real monorepo frontmatter
  with the same pinning discipline.

## Decisions explicitly deferred (not ratified, not v1)

- Hard per-document millisecond performance budget (post-benchmark).
- `--strict` / exit-code gate for findings.
- Opt-out flag for deterministic frontmatter repair.
- Auto-fix promotion of any report-only class (duplicate keys, key
  correction, shape/type repair, anchor/alias, multi-document).
- Tolerant-CST (tree-sitter) candidate generation; `serde_yaml_ng` remains
  the sole parse authority in v1.
