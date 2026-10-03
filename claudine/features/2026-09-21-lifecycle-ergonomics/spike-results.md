# Schema spike results

**Purpose.** Measure, not build: can `action.yaml` + `lifecycle.yaml` + one
`trigger-schema` express the stack grammar in SimplifiedSchema, be discovered
by the ancestor walk, and validate at an acceptable cost at depth 5 with every
verb generated — or does one of the three feared outcomes hold (generated
entries must collapse to a catch-all; depth must drop to 2–3; a Darkmatter
change is needed)?

## Environment

- Host: Apple M4 Max, 16 cores, 128 GiB, macOS Darwin 27.2.0 arm64; rustc 1.98.1.
- The host carried load averages of 22–35 for the whole session (other
  agents' builds). Every wall-clock figure below is an upper bound; the
  cleanest sample is called out where it exists.
- `md` built from this worktree at commit `584656431` (dirty), release profile:
  `target/release/md`. The installed `~/.cargo/bin/md` (2026-09-24) behaves
  identically on every probe that was cross-checked.
- Darkmatter spike harness: a scratch crate pinned to the repo `Cargo.lock`,
  `darkmatter = { path = ".../darkmatter/lib" }`, release profile.

## What was built

All under `claudine/features/2026-09-21-lifecycle-ergonomics/spike/`:

- `gen-action-schema.py` — generator (PyYAML) for every schema but the trigger.
- `schemas/action.yaml` (48 KB YAML): `action` = 143 verb keys + `min-keys(1)`
  + `max-keys(1)`; `bundle` = the same 143 keys + `when: expression` +
  `no_error: boolean` + `min-keys(1)`. Every verb is `[short-form, long-form…]`
  with `no_error` in each long form. Sources: 21 hand verbs from the Action
  Inventory, 12 side-effect verbs (13 signatures) from `effects/catalog.rs`,
  110 expression functions from `expression-functions.yaml`.
- `schemas/lifecycle.yaml` (124 KB YAML): `item-0 = bundle`; for N = 1..5,
  `item-N` = the 143 verbs + `when` + `then: [item-(N-1)[]@this, bundle@./action.yaml]`
  + `else: [same]` + `no_error` + `min-keys(1)`; `stack-N = [item-N[]@this,
  bundle@./action.yaml]`; `stack = stack-5@this`; `loop` = iteration keys +
  `gate: stack-5@this` + the 21 hand verbs as loose keys.
- `schemas/lifecycle-d1..d4.yaml`, `lifecycle-collapsed.yaml` (depth 5 over an
  action table whose generated verbs are one `<string>: any`), `lifecycle-collapsed-d3.yaml`.
- `schemas/claudine.yaml`: `kind: trigger-schema`, `match: any: [initialize:
  any(required), …six events]`, payload `initialize: stack@./lifecycle.yaml`
  … `loop: loop@./lifecycle.yaml`.
- Fixtures: `depth1`, `depth3`, `depth5`, `depth6`, `root-dictionary`,
  `bundle-in-then`, `loop-gate`, `wrong-verb` (typo'd verb inside `then:`),
  `long-forms`; and `fixtures/variants/*.md` with inline `$schema:` pointers.

## Measurements

### Discovery (`md schema triggers`, from `spike/fixtures/`)

Boundary = repo root. Roots found nearest-first: `spike/schemas`,
`claudine/schemas`, `<repo>/schemas`. `claudine/schemas/claudine.yaml` is
reported *shadowed by* `spike/schemas/claudine.yaml`. Trigger `matched`, arm 1,
for every fixture with an event key. No explicit `$schema:` was needed.

### `md schema validate` per trigger fixture (3 runs, median, ms)

| fixture | quiet sample | loaded sample | exit | schema verdict / first problem |
|---|---|---|---|---|
| depth1 | 3846 | 4075 | 1 | `/start/3 "stop" is not of type "object"` — the bare `- stop` item (Finding 6) |
| depth3 | 3792 | — | 0 | valid |
| depth5 | 3803 | 4103 | 0 | valid |
| depth6 | 4053 | 4012 | 1 | rejected: `/success/0/then/0/then/0/then/0/then/0/then/0 Additional properties are not allowed ('else', 'then')` |
| root-dictionary | 4248 | 4183 | 0 | valid (`no_error: true`, `stop: null` accepted) |
| bundle-in-then | 6720* | 4126 | 0 | valid (`then:` as dictionary with `no_error`) |
| loop-gate | 3978 | 4080 | 0 | valid (`gate:` list with nested `when`) |
| wrong-verb | 3743 | 4147 | 1 | `/start/0/then/0 Additional properties are not allowed ('inf0' was unexpected)` |
| long-forms | 3843 | — | 0 | valid (say long form, retry with `with`, side-effect array and long form, expression fn, `set`) |

Process baseline (`md schema validate` on a document with no schema): 7 ms.
\* one outlier run of 6.7 s; the other two were 4.3 s.

### Resolved JSON Schema size and resolve time (harness, `resolve_schema` on the 7-key payload)

| library | depth | resolved JSON | resolve ms | inlined `bundle` tables | `md schema validate --no-trigger-schemas` median ms |
|---|---|---|---|---|---|
| lifecycle-d1 | 1 | 1 906 406 | 109 | 42 | 234 |
| lifecycle-d2 | 2 | 4 438 950 | 232 | 98 | 496 |
| lifecycle-d3 | 3 | 9 504 038 | 657 | 210 | 1136 |
| lifecycle-d4 | 4 | 19 634 214 | 1179 | 434 | 2595 |
| lifecycle (d5) | 5 | 39 894 566 | 2683 | 882 | 5939 |
| lifecycle-collapsed-d3 | 3 | 1 374 098 | 82 | 210 | 266 |
| lifecycle-collapsed (d5) | 5 | 5 748 818 | 403 | 882 | 1043 |

Table count is exact: `C(stack-N) = 2^(N+2) − 2` copies per event, seven
consumers (six events + `loop.gate`), so depth 5 inlines the 143-key table
882 times (≈ 45 KB of JSON each). Size and time roughly double per level.

### In-process harness (DMLS proxy; `with_trigger_discovery` + `effective_for` + `validate`)

| document | first call ms | warm median ms (5) |
|---|---|---|
| depth1.md | 3576 | 1921 |
| depth5.md | 3373 | 1903 |
| loop-gate.md | 3387 | 1956 |
| variants/lifecycle-d3.md | 2763 | 2374 |
| variants/lifecycle-collapsed.md | 3163 | 3641 |

This is a proxy, not DMLS: DMLS keeps a trigger registry and validator cache
alive per workspace, so its per-edit cost should be below the "warm" column,
which still re-scans and re-resolves the 40 MB schema on every call. No DMLS
bench exercises diagnostics (`dmls --bench-index` times indexing only), so the
LSP path was not measured. The hard floor DMLS cannot avoid is one full
resolve + validator compile per schema change: 2.7 s + compile at depth 5.

## Findings

1. **Discovery works as the spec assumes.** `spike/fixtures/*.md` found
   `spike/schemas/` with no `$schema:`; nearest-root filename shadowing is
   reported; the boundary is the repo root. `kind: trigger-schema` is the
   accepted spelling (`envelope.rs:27`); `schema-trigger` is docs-only drift.
2. **A trigger match arm must contain a presence-requiring condition.**
   `start: any` is rejected as a *vacuous arm*; `start: any(required)` is the
   spelling that works.
3. **`kind: schema` libraries accept only `kind` and `types`.** `description`
   and an exported `$schema` are rejected ("tagged schema documents support only
   `kind` and `types`"). The authoring doc's `kind: schema` + `$schema` + `types`
   example does not load. Consequence: `lifecycle.yaml` cannot be validated as
   a whole file; the seven event keys must be wired by the trigger payload (or
   a document's inline `$schema:`).
4. **`Name[]@file` is rejected when `Name` is a union-typed named type**
   ("cannot apply `[]`/constraints to the union-typed named type"). Union named
   types themselves work (`stack-N` as `[item[]@this, bundle@…]` is fine when
   used bare), but a list item cannot be `bundle | conditional | string`. The
   spike therefore types the list item as one merged mapping (verbs + `when` +
   `then` + `else` + `no_error`, `min-keys(1)`); the parser keeps exclusivity.
5. **`max-keys(1)` / `min-keys(1)` on the 143-key literal object is accepted and
   enforced** (`{}` → "has less than 1 property"; two verbs → "has more than 1
   property"), both via `$constraints` and as a postfix on an import
   (`bundle(max-keys(1))@./action.yaml`).
6. **The bare zero-argument item (`- stop`) is not expressible** alongside
   object items: array items are one type expression, and Finding 4 blocks a
   union item type. `- stop: null` validates (`any` short form).
7. **Depth 6 is rejected by the depth-5 schema**, because `item-0` has no
   `then`/`else`; the error path names the sixth nesting exactly. (The spec's
   "sixth is a parse error" is therefore also visible in the editor.)
8. **Diagnostics are usable but coarsely positioned.** The typo'd verb yields
   `Additional properties are not allowed ('inf0' was unexpected)` at JSON path
   `/start/0/then/0`, and the `then:` union (list arm vs bundle arm) is reported
   through the closest arm, not as `anyOf` noise. But `line`/`column` are those
   of the top-level key (`line 2, column 1`), not the offending line (5).
9. **`@this` is required** for every local reference (`item-4@this`); a bare
   name is "unknown type". Confirmed as the doc says.
10. **`required` is stripped from a reused named type.** `types: { req:
    expression(required) }` used as `when: req@this` makes `when` optional, and
    used as a top-level property makes that property optional. `required`
    written directly on the property inside a mapping type (`when:
    expression(required)`) is kept.
11. **Two valid-YAML spellings crash the standalone loader** with "could not
    project SimplifiedSchema expression spans through YAML source": (a) block
    sequences whose `-` sits at the parent key's indentation (PyYAML's default
    output; `yq`, many editors); (b) a plain scalar folded across physical
    lines. Both are load-time failures of the whole file. The generator indents
    sequences and dumps with `width=inf` to avoid them.
12. **`->` with an empty description is a grammar error**; three catalog
    functions have empty descriptions, so the generator omits the arrow.
13. **The catalog has 110 expression functions, not 257** (`grep -c "^    -
    name:" expression-functions.yaml` = 110; the runtime registry
    `expression_function_descriptors()` loads the same file). Two are variadic
    (`and`, `or`); one has zero parameters; 5 have two overloads, 1 has three.
    With 21 hand verbs and 12 side-effect names the table is 143 keys, not ~300.
14. **Cost at depth 5 with the full table: 39.9 MB resolved JSON, 2.7 s to
    resolve, ~4 s per `md schema validate`, ~1.9 s warm in-process.** At depth 3
    it is 9.5 MB / 0.66 s / 1.1 s. Collapsing the generated verbs cuts depth 5
    to 5.7 MB / 0.40 s / 1.0 s, and depth 3 to 1.4 MB / 0.08 s / 0.27 s. The
    schema-side cost is entirely duplication: 882 inlined copies of one table.
15. **No caching helps the first load.** `DarkmatterSchemas` caches compiled
    validators process-wide, but `resolve_schema` inlines eagerly and is not
    cached across `with_trigger_discovery` calls; the harness "warm" column stays
    at ~1.9 s.

## Traps hit

- The spike directory was committed by another session mid-spike
  (`0b1564e93`, 01:39) while I was told not to commit, and the working tree was
  later reverted to that snapshot, silently undoing the generator fixes for
  Finding 11. Symptom: `md` passed 27 runs, then failed 10, on files with
  identical mtimes. The generator was rewritten whole; the tree now carries
  uncommitted modifications to the committed spike files.
- A scratch crate must pin the repo's `Cargo.lock`; otherwise it compiles a
  different dependency set. (This was ruled out as the cause of the flip above,
  but is still needed for like-for-like timing.)
- My harness first imported `resolve_schema` from `schemas::` — it lives at
  `schemas::resolve::resolve_schema`.
- zsh: `echo =====` is a glob (`=` expansion); use quotes.

## Verdict on the three feared outcomes

| fear | verdict |
|---|---|
| Generated entries must collapse to a catch-all? | **Not required for correctness, but it is the single biggest cost lever.** Full table at depth 5 is 40 MB / ~4 s per validate; collapsed is 5.7 MB / ~1 s. Everything in the fixtures validates either way. |
| Depth must drop to 2–3? | **Not for expressiveness — depth 5 works and rejects depth 6.** For cost, depth 3 with the full table (9.5 MB, 1.1 s) is the highest depth in the same cost band as collapsed depth 5. Depth ≤ 3 *and* collapsed (1.4 MB, 0.27 s) is the only combination that is cheap. |
| A Darkmatter change is needed? | **Yes, for two grammar gaps and one performance root cause.** (a) `Name[]@file` over a union-typed name (Finding 4) — without it the item shape is a merged mapping and `- stop` cannot be typed. (b) The two loader crashes (Finding 11) will bite the first hand-edited schema. (c) Emit named types as `$defs`/`$ref` instead of inlining; that removes the 2^N duplication entirely and would also make recursion representable, making the unroll unnecessary. Everything else in the spec's schema plan holds without Darkmatter work. |

## Recommendations for the spec

1. **Action Inventory, row "257 expression functions" and rule 2** — change to
   110; state that the count is whatever the catalog holds at generation time.
2. **Action Inventory, rule 1** — record that the bare-name short form
   (`- stop`) is parser-only and not schema-typed while Darkmatter arrays cannot
   carry a union item; `- stop: null` is the schema-visible spelling.
3. **Claudine Schema Structure, `lifecycle.yaml` row** — replace "the
   `when`/`then`/`else` item; the bundle as a pattern-keyed object" with the
   shape that loads: one merged item mapping per level (verbs + `when` +
   `then` + `else` + `no_error`, `min-keys(1)`), `then`/`else` typed as the
   property-level union `[item-(N-1)[]@this, bundle@./action.yaml]`. Drop the
   pattern-keyed bundle: literal verb keys are what make the typo diagnostic
   (Finding 8) possible.
4. **Expressiveness limits** — add three "cannot" bullets: arrays of union-typed
   named types; a `kind: schema` file that also exports `$schema`; `required`
   carried by a reused named type. Add: a trigger arm needs `(required)`.
5. **Darkmatter Provisioning** — replace "when composing" boundary prose with
   the observed facts: boundary is the repository root; shadowing is by filename,
   nearest root wins; `md schema triggers` prints all three.
6. **Schema spike section** — record the decision this spike forces:
   *either* generated verbs collapse to `<string>: any` in the list/bundle
   tables (hand verbs stay typed) at depth 5, *or* depth drops to 3 with the
   full table, *or* the schema work is sequenced after a Darkmatter change that
   emits `$defs`/`$ref` for named types (which also unlocks recursion and makes
   the unroll and the depth question disappear). Recommend the third as the
   target and the first as the interim, because the collapsed table keeps the
   typo diagnostic for every hand verb and loses it only for generated ones.
7. **Package Ownership and Sequencing** — add the Darkmatter defects as
   filed-not-blocking items: Finding 4 (`[]` over union names), Finding 11
   (indentless sequences and folded scalars crash the loader), Finding 10 vs
   the doc (`required` stripping, already noted as pending), Finding 8
   (diagnostic line/column stops at the top-level key).
8. **Conditional Nesting** — keep the limit of five; note that the schema
   rejects the sixth level with a path to the exact node, so the DMLS and the
   parser agree.
9. **Location** — no change needed; discovery from `claudine/schemas/` and the
   shadowing rule behaved as written.
