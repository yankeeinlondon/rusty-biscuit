# Phase 1 — GitNexus Impact Analysis (upstream blast radius)

Index refreshed with `node .gitnexus/run.cjs analyze` (exit 0) before this
capture; `gitnexus://repo/rusty-biscuit` (darkmatter worktree) is current as of
this run. Analysis is `direction: upstream` (what depends on the symbol).

> **⚠️ Two CRITICAL-risk symbols are in the edit path.** Phase 1 edits none of
> them — this record exists so Phases 2–6 keep every change to these symbols
> **purely additive** (a new enum variant, a new match arm, a new keyword
> branch) and preserve existing behavior byte-for-byte. Re-run
> `impact({target, direction: "upstream"})` immediately before editing each one.

| Symbol | File | Risk | Impacted | Direct | Phase | Edit shape |
|--------|------|------|---------:|-------:|-------|-----------|
| `parse_type_expr` (free fn) | `simplified/grammar.rs:67` | **CRITICAL** | 156 | 35 | 2 | add `literal(...)`/`expression` keyword parse branch; no signature/behavior change to existing types |
| `to_json_schema` | `simplified/convert.rs` | **CRITICAL** | 137 | 23 | 3 | add `Literal`→`const` and `Expression`→`{string,format}` arms; existing arms untouched. Affects process `run_subcommand` (10 flows) |
| `coercion_target` | `schemas/coerce.rs` | MEDIUM | 17 | 6 | 4 | add literal-typed + `darkmatter-expression`-format coercion targets; modules Schemas (9), Compose (1) |
| `primitive_matches` | `triggers/matcher.rs` | LOW | 3 | 1 | 3 | add `Literal` equality arm; module Triggers only |
| `SimplifiedType` (enum) | `simplified/types.rs:223` | LOW | 0 direct call edges | — | 2 | add `Literal`/`Expression` variants + keyword maps; exhaustive `match` sites found by compiler, not the call graph (see grep inventory below) |

## Exhaustive-match / keyword-map sites (compiler-enforced, grep inventory)

`SimplifiedType` variants are matched exhaustively across the crate; the call
graph shows 0 edges because these are `match`-arm references, not calls. Adding
a variant forces the compiler to flag each incomplete `match`. Known sites to
update (Phase 2/3):

- `simplified/types.rs` — `as_keyword`, `from_keyword`, and the keyword
  round-trip test list.
- `simplified/grammar.rs` — `parse_constraint_list` enum special-case (mirror
  for literal positional-value path).
- `simplified/convert.rs` — `to_json_schema` type lowering.
- `simplified/serialize.rs` — canonical serialization.
- `schemas/coerce.rs` — `coercion_target`.
- `schemas/about.rs` — descriptor catalog.
- `schemas/detect.rs` — inference (both new types are **never inferred** — non-goal).
- `triggers/matcher.rs` — `primitive_matches`.
- `dmls/src` — provider `SimplifiedType::` match sites (Phases 5–6).

## Format registration (Phase 3)

`schemas/format.rs` registers `darkmatter-yaml` / `darkmatter-json` formats;
`DARKMATTER_EXPRESSION_FORMAT` is added alongside. Internal registration
surface — low external blast radius; verified additive by grep.

## DMLS providers (Phases 5–6)

`dmls/src/providers/frontmatter.rs` and `dmls/src/overlay/expressions.rs` are
the D1/D2/D3 entry points. Not edited in Phase 1; per-symbol upstream impact to
be re-run at the start of Phases 5–6 (the DMLS crate is a leaf binary, so the
upstream radius is bounded by its own provider registry and test suite).

## Verdict for Phase 1

No symbols edited. The two CRITICAL symbols are recorded and flagged. All
planned edits are additive; the guardrail (spec AC #8/#10 — existing schemas'
validation output stays byte-identical) is protected by the captured baseline
in `phase1-baseline.txt` / `phase1-baseline-about.txt`.
