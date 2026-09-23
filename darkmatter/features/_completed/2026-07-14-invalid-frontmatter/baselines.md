---
phase: 1
created: 2026-07-18
artifact: baselines
---

# Phase 1 Baselines — Pre-Change `md clean` Behavior

Captured 2026-07-18 against the current working tree (HEAD `d672388dd` plus
uncommitted in-flight work from other features) using a debug binary built
from that tree (`cargo build -p darkmatter-cli`). Fixture inputs are preserved
in this directory under `baselines/` so later phases can re-run them verbatim.

## Functional Baselines

| # | Scenario | Exit | stdout | stderr |
|---|----------|------|--------|--------|
| F1 | No frontmatter, stdout mode | 0 | cleaned body only | empty |
| F2 | Clean frontmatter, stdout mode | 0 | **frontmatter reserialized**: `tags:\n  - alpha` → `tags:\n- alpha`; blank line after closing `---` dropped | empty |
| F3 | Malformed `title: @daily-report` | 1 | empty | miette `MarkdownError: frontmatter parse failed` — "found character that cannot start any token at line 1 column 8", source snippet, file hyperlink |
| F4 | Schema-coercible `release: 1.20` | 0 | **`release: 1.2`** — float reserialization silently loses the trailing zero (data corruption) | empty |
| F5a | stdin via `-` | 0 | same as file mode | empty |
| F5b | stdin implicit (piped, no arg) | 0 | same as file mode | empty |
| F6 | `--save` on clean frontmatter | 0 | `DeltaReport` rendered to **stdout**; file rewritten in place | empty |
| F7 | `--save --verbose` | 0 | `DeltaReport` + statistics + visual diff; reports `EOF newline changed: original=missing, updated=present` | empty |
| F8a | `--save` with stdin | 1 | empty | `Error: --save requires an input file path (stdin is not supported)` |
| F8b | `--save` twice (idempotency) | 0 | file bytes stable across runs (sha1 identical) — **byte-level fixed point holds**, but the report still claims "Whitespace changes only (1)" on the second run (phantom delta quirk) | empty |
| F8c | Top-level `md <input> --save` shorthand | 0 | identical to F6 | empty |
| F9 | `--save` on malformed frontmatter | 1 | empty | same miette error as F3; file untouched |
| F10 | Missing input file | 1 | empty | `Error: Failed to load file: "<path>"` |

### Channel contract (pre-change)

- Cleaned document → stdout (default mode).
- Delta report → stdout (`--save` mode, human-readable, ANSI-styled).
- Errors → stderr with exit 1. No exit codes other than 0/1 observed.
- Suggestions/warnings channel: does not exist yet (no findings concept).

### Baseline-derived invariants for v1

- **I-F3**: today invalid frontmatter is an unrecoverable exit-1 error before
  any cleanup runs — the flagship gap this feature closes.
- **I-F4**: today `release: 1.20` is silently corrupted to `1.2` by
  reserialization — the raw-preserving assembly (Phase 6) must make this
  whole class impossible even when no repair applies.
- **I-F2**: current output reserializes all frontmatter (comment loss, quote
  loss, indentation drift). v1 intentionally changes this: frontmatter bytes
  outside accepted edits are preserved verbatim (see `decisions.md` D6).
- **I-F8b**: byte-level idempotency already holds; the phantom "whitespace
  changes only" report on an already-clean file is a pre-existing
  `DeltaReport` quirk, documented but not v1 scope (see `decisions.md` D8).

## Performance Baselines

### In-process (Criterion, saved baseline `phase1-before`)

Vehicle: `darkmatter/lib/benches/clean_hot_paths.rs` (new this phase).
Re-run comparison:

```text
cargo bench -p darkmatter --bench clean_hot_paths -- --baseline phase1-before
```

| Case | Mean | 95% CI |
|------|------|--------|
| `no_frontmatter/full_pipeline` (parse → cleanup → serialize) | 9.66 µs | 9.60–9.74 µs |
| `no_frontmatter/parse_only` | 675 ns | 641–710 ns |
| `clean_frontmatter/full_pipeline` | 16.68 µs | 16.64–16.73 µs |
| `clean_frontmatter/parse_only` | 4.88 µs | 4.85–4.90 µs |

### End-to-end CLI (release build, 50 invocations each, macOS host)

| Case | min | p50 | mean | p95 |
|------|-----|-----|------|-----|
| `md clean no-fm.md` | 22.5 ms | 24.2 ms | 24.7 ms | 25.9 ms |
| `md clean clean-fm.md` | 22.8 ms | 25.2 ms | 26.3 ms | 33.3 ms |

End-to-end times are process-startup dominated (~22 ms floor); the Criterion
numbers are the sensitive regression signal for Phase 7.

### Counter expectations v1 must satisfy (instrumented in Phase 7)

- No-frontmatter input: zero YAML analysis, zero schema resolution, zero
  trigger discovery.
- Already-clean frontmatter: exactly one YAML parse, zero candidate reparses.
- Schema/trigger state: built at most once per `clean` invocation.
