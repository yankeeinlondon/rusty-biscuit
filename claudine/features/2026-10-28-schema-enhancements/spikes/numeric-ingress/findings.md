# Numeric Ingress Spike — Findings (2026-10-21)

This spike ran **once**, for Phase 1 of the plan (Necessary Rule 6, Wave 1
"Numeric ingress"). It is not scheduled to run again; production regression
tests replace it from Phase 2 onward. It selects a loading seam only. The
number model does not change: runtime numbers stay finite `f64`.

The throwaway crate is `src/` (standalone `[workspace]`, not a workspace
member; versions pinned to the workspace `Cargo.lock`; target dir under
`/tmp`). Its three captured runs are in `runs/`:

| Run | Features | File |
|---|---|---|
| default | `serde_json/float_roundtrip`, `serde_json/raw_value` (what Darkmatter links today, through other crates) | `runs/default.md` |
| `ap` | default + `serde_json/arbitrary_precision` | `runs/arbitrary-precision.md` |
| `po` | default + `serde_json/preserve_order` | `runs/preserve-order.md` |

Command shape: `cargo run --offline --manifest-path src/Cargo.toml [--features ap|po]`
with `CARGO_TARGET_DIR=/tmp/numeric-ingress-target`.

## Selected seam: a number-spelling sidecar beside today's value

Every loader keeps building today's `serde_json::Value` (with the existing
`MappingOrders` for key order). In addition, it records a **map from JSON
Pointer to the written number text**, only for numbers that are stored as
`f64`. One admission function then receives, per number, one of:

- an exact integer (`i64`/`u64` already stored losslessly),
- the written spelling (from the sidecar), or
- an externally supplied `f64` (finiteness check only).

Admission parses the decimal spelling exactly, rounds to the nearest `f64`,
and rejects when a mathematically whole input differs from the rounded value
("cannot be represented exactly"), when the magnitude overflows ("not
finite"), or when a nonzero value rounds to zero (underflow). Decimals that
are not whole may round (`0.1` admits).

This mirrors how `MappingOrders` already carries information that the
`serde_json::Value` tree cannot. It needs no change to the shared value type
and no workspace-wide feature.

### Per loader

| Path | Loader today (file:line) | How the seam recovers spelling |
|---|---|---|
| YAML frontmatter | `serde_yaml_ng` 0.10.0, `darkmatter/lib/src/markdown/frontmatter.rs:644-648` | a second, guided pass over the same text reads each number back as its written text (`serde_yaml_ng` `de.rs:1472`); a document whose numbers are all integers skips the pass |
| JSON (raw schema, transclusion data, `md set`) | `serde_json`, `darkmatter/lib/src/markdown/schemas/resolve.rs:1152`, `.../transclusion/parser.rs:401`, `md set` | the same second pass, reading each number as `RawValue` text |
| JSON5 | `json-five` 0.3.1 | its public round-trip model keeps number text |
| Expression literals | `darkmatter/lib/src/markdown/expr/lexer.rs:921` | the token text is already in hand at the lexer |
| SimplifiedSchema numeric metadata (defaults, enum members, constraints) | `darkmatter/lib/src/markdown/schemas/simplified/grammar.rs:1941` | `Arg.lex` already keeps the text |
| DMLS overlay | `rlsp-yaml-parser` 0.11.1 | every node already carries its text and span |

Nested key order and source spans are unchanged (runs C, D and E): the
sidecar is recorded in visit order, alias resolution carries the spelling of
the anchored node, and `rlsp-yaml-parser` spans index exactly the spelling.

## Per-input outcomes (default run)

"Today" is the value Darkmatter receives now; "admission" is the spike's
admission function applied to the recovered spelling.

| Input | YAML today | JSON today | JSON5 today | Expression lexer | Admission |
|---|---|---|---|---|---|
| `9007199254740992` | exact | exact | exact | `9007199254740992.0` | admit |
| `9007199254740993` | exact `u64` | exact `u64` | exact | rounds to `…992.0` | **reject: inexact** |
| `9007199254740993.0` | rounds to `…992.0` | rounds | rounds | rounds | **reject: inexact** |
| `9.007199254740993e15` | rounds | rounds | rounds | parse error (no exponent syntax) | **reject: inexact** |
| `-9007199254740992` | exact | exact | exact | exact | admit |
| `-9007199254740993` | exact `i64` | exact | exact | rounds | **reject: inexact** |
| `.0` | `0.0` | parse error (not JSON) | `0.0` | parse error | admit `0.0` |
| `1.0` | `1.0` | `1.0` | `1.0` | `1.0` | admit |
| `1e3` | `1000.0` | `1000.0` | `1000.0` | parse error | admit `1000` |
| `0.1` | `0.1` | `0.1` | `0.1` | `0.1` | admit |
| `18446744073709551615` | exact `u64` | exact `u64` | exact | rounds | **reject: inexact** |
| `99999999999999999999` | **whole frontmatter parse error** (read as `u128`, which `serde_json::Value` cannot hold) | `1e+20` | error | `1e20` | **reject: inexact** |
| `1e400` | arrives as the string `"1e400"` | parse error "number out of range" | **`null`** | parse error | **reject: not finite** |
| `1e-400` | `0.0` | `0.0` | `0.0` | parse error | **reject: underflow** |

External `f64`: `9007199254740992.0` and `0.1` admit; `inf` and `NaN` reject
as not finite.

## Required parser features

- Darkmatter must declare `serde_json` features `raw_value` and
  `float_roundtrip` itself. Today both arrive only through feature
  unification from other crates (`raw_value` via axum/schematic,
  `float_roundtrip` via jsonschema); relying on that is fragile.
- `preserve_order` (spike ruling 4) is orthogonal to this seam; run `po`
  shows it changes only map order.

### Rejected: `serde_json/arbitrary_precision`

- It does nothing for YAML or JSON5: both round to `f64` before
  `serde_json` sees the number (runs F: YAML and JSON5 `9007199254740993.0`
  still arrive as `…992.0` with `ap` on).
- Cargo feature unification would turn it on for all 67 workspace crates
  that link `serde_json`, including tauri, rmcp and lsp-types.
- It changes `Value` equality: `Value(1.0) != Value(1.00)` with `ap` on.
- The known untagged-enum/flatten breakage did **not** reproduce in this
  probe, but the equality and blast-radius issues are enough.

## Existing defects found (production behavior today)

These are current bugs the numeric task must fix, each needing a
regression test:

1. YAML `99999999999999999999` fails the **whole** frontmatter parse instead
   of a per-field error.
2. JSON5 `1e400` / `Infinity` and YAML `.inf` / `.nan` silently become
   `null`.
3. Numeric enum members saturate through `i64`.
4. `min(inf)` in a schema constraint is accepted.
5. Expression literals display a saturated value.
6. `1e-400` silently becomes `0.0` on every path.

## Decisions for the author

These do not block selecting the seam; they decide individual cells in
Phase 2's numeric task.

1. **YAML integers beyond `u64`.** Keep today's whole-document parse error,
   but make it a located per-field error (recommended), or report it as
   "cannot be represented exactly"? The spec's criterion 30 only requires
   that it fails validation.
2. **Expression literal syntax.** Expression literals have no exponent or
   leading-dot form (`1e3`, `.5` are parse errors). Keep that (recommended:
   the spec does not add the syntax; criterion 21 applies the exactness
   check to the literal `9007199254740993`, which already lexes).
3. **JSON `1e400`.** `serde_json` rejects it at load with "number out of
   range". Does that load error satisfy the spec's "not finite" outcome for
   JSON input (recommended: yes, it is a located error and nothing is
   admitted)?
4. **Content-format `json` text containing `1e400`.** It is rejected today as
   malformed JSON. Keep (recommended), since the `json` type validates the
   text and does not admit numbers.
