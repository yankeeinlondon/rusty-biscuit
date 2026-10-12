---
$schema: feature-review.yaml
ready: false
findings:
  - "R1-F1 — high: Same-file binding checks confuse replaced or shadowed names with imported bindings"
  - "R1-F2 — high: Lua reports a definite interface after the returned table can be modified"
  - "R1-F3 — high: Lua constructor keys produce incorrect exported names and miss overwrites"
observations:
  - CLI help still promises re-exports in declaration filters
  - Regression fixtures are coupled to the active feature directory
human_review: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-11T09:19:45-07:00"
spec: 2026-10-11-reexports-all-languages/spec.md
implemented: true
implemented_by: claude/opus
log: tree-hugger/features/2026-10-11-reexports-all-languages/log.md
description: "A **feature** review of `2026-10-11-reexports-all-languages/spec.md`"
feature: 2026-10-11-reexports-all-languages/review-1.md
next: 2026-10-11-reexports-all-languages/review-2.md
---

# Review 1: Re-exports across all supported languages

The feature is **not ready**. Three defect classes affect the newly added extraction. The API, query planner, cache, and CLI consistently preserve these results; consistency does not make the underlying results correct. No human decision is needed to address the findings.

This is the first implementation review. There are no previous implementation findings to carry forward or partially repaired findings to reconcile. The earlier specification review and the placeholder removal are reflected in the implementation: `reexports()` replaces the removed method, and every language has an explicit report contract.

## Verification

Reviewed the finalized specification, plan, audit notes and samples, implementation log, language extractors, report schema, capability prerequisites, cache readers, import conversion, diagnostics, projection, and corresponding tests. The implementation uses schema 5.2 and extractor revision 53; the additional bump after the initial revision 52 is appropriate for the later extraction changes.

Checks run from `tree-hugger/`:

- `just test`: **2,381 passed**, zero skipped, 102.516 seconds; nextest reported 16 slow tests.
- `just lint`: passed for the library and CLI.
- `just test reexport`: **292 passed**, 2,089 excluded by the filter, 5.757 seconds. The complete-report cache matrix took 3.415 seconds in this focused run.

From the repository root, `just check-tier-coverage tree-hugger` found **zero stranded tests**. The library layout test also passed. New extraction tests are declared by the consolidated L1 target and appear in the executed nextest listing.

| Specification requirement | Appropriate verification | Evidence reviewed / result |
| --- | --- | --- |
| Dedicated and bounded convention extraction in all 16 languages; names, forms, qualifiers, restrictions, scopes and negatives | L1 public API and static fixtures | `symbols_reexports_*` tests pass; additional public reproductions expose the findings below. |
| Parse recovery and honest coverage | L1 pinned grammar and public report | Existing recovery tests pass. Every reproduction below also produces zero `syntax_diagnostics()` results; these failures are not grammar recovery. |
| Direct API, complete/selective query, filtering, cache transitions and CLI agree | L1 library and hermetic CLI | Existing equivalence tests pass. Additional probes verified the same reproduced reports through direct API, selective and complete queries, cache widening, cache hits, and a function-kind projection. |
| Cache rejects stale and malformed reports | L1 real persisted JSON entries edited through the shipped CLI | Upgrade, report-field matrix, limited-report matrix and consistency tests pass. |
| Only proven forwards suppress unused-import diagnostics; Rust and prelude behavior stay intact | L1 public diagnostics and CLI | Consumer and prelude tests pass; incorrect import links in R1-F1 still create false exemptions. |
| Shared extraction and no unrequested references, semantics, diagnostics or target-file reads | L1 work counters and isolated files | Existing work-count and absent-target tests pass. |
| Full-index JSON exposes the report; declaration listings remain separate | L1 shipped CLI | JSON and conversion tests pass. No new styled output or keyboard interaction is specified, so L2/L3 are not required. |

The input robustness matrix applies to the **persisted JSON cache**, not to source-language grammars. Reviewed the plan's field matrix against `reexport_columns`, `rx_span_columns`, the limited-report columns and the strict reader. It covers report/coverage containers, state and limits, entry collection, form/evidence, exported/original/local/source/scope/restriction roles, excluded names, type-only/conditional flags, caveats, linked and null import spans, both entry spans and their coordinates, import markers, and completed capabilities. The tests exercise control, absent, null, wrong whole-field type, wrong one/all element types where applicable, empty, duplicate-key and trailing-content shapes through cache-served or recomputed CLI output. The unedited positive fixture establishes report entries and forwarded/unforwarded markers. No additional cache-reader finding emerged.

## Findings

### R1-F1 — high: Same-file binding checks confuse replaced or shadowed names with imported bindings

**Authority — spec:** “A same-file binding may establish that relationship; it must not be inferred solely from matching text when a binding is shadowed or reassigned.” The consumer contract also says an “unrelated import or a guessed public surface must not suppress a warning.” Conversely, every agreed static construct must remain extractable: an unrelated local assignment must not erase a proven forward.

**Defect class:** name-based binding tracking ignores replacement or lexical scope, producing both false import provenance and missing required forwards.

In the tree-hugger library, Python's [binding selection](../../lib/src/analysis/reexports/python.rs:82) only checks non-import rebinding; CommonJS [whole/property forwarding](../../lib/src/analysis/reexports/ecma/cjs.rs) resolves names from a program-wide table without checking the value's nearer scope; Scala's [link_import](../../lib/src/analysis/reexports/scala.rs:225), which associates export selectors with imports, searches the entire import list by spelling. Lua's [collect_reassignments](../../lib/src/analysis/reexports/lua.rs), which determines whether a required module is still usable, treats an assignment to an unrelated local of the same spelling as a write to the imported local.

Minimal Python reproduction:

```python
from old import read
from new import read
__all__ = ["read"]
```

The report claims two unconditional declared forwards, from `old` and `new`, marks both imports `reexported: true`, and emits no unused-import diagnostic. The final `read` binding provably comes from `new`. Retain that result, exclude the replaced `old` binding from forwarding, and leave its marker false. A blanket empty fallback would also lose a required result. Preserve the separate valid case of mutually exclusive `try`/`except` imports: both conditional sites are required there.

Minimal CommonJS and Lua controls exposing the scope error:

```javascript
const m = require("m");
{ const m = {}; module.exports.read = m.read; }
```

```lua
local m = require("m")
local function unrelated() local m; m = {} end
return m
```

CommonJS falsely attributes `read` to the required module; Lua falsely reports `limited/rebound_binding` and loses the whole-module forward. The nearer CommonJS local and the unrelated Lua local must be distinguished from the imported binding.

The sweep covered every changed extractor that associates an indirect export with a binding, and every CommonJS consumer of that association:

| Child ID / site | Shape tested | Observed result | Independently expected result | Verification evidence |
| --- | --- | --- | --- | --- |
| R1-F1.1 — Python named imports | Add `from n import a` after the import in `python/pos-all-list.py`; also test sequential namespace aliases | Both sources are declared and both markers true | Only the final imported binding is forwarded; the overwritten import is not exempt | Copied fixture plus one inserted import; public API and CLI; named and namespace probes |
| R1-F1.2 — JS and TS CommonJS whole-module binding | `const m = require("m"); { const m = {}; module.exports = m; }` | A module forward from `m`, complete | No required-module forward from the shadowing local; report the ambiguity under the shadowing contract | Public API / CLI on both extensions |
| R1-F1.3 — JS and TS property forwarding | Shadow a namespace or destructured binding before `module.exports.read = m.read` or `exports.read = read`; also catch-parameter and loop-variable shadowing | Named forward from the outer required module, complete | No forward attributed to the outer import | Both extensions; member, bare-value, catch and loop probes |
| R1-F1.4 — JS and TS object-literal forwarding | Wrap `module.exports = { a }` in `{ const a = 1; ... }` in `js/pos-cjs-destructure.js`; repeat as TS | Still reports `a` from `m` | Withhold that false forward, preserving other proven fixture sites | Copied fixture edit, object shorthand and property consumer probes |
| R1-F1.5 — JS and TS ES import consumed by CommonJS | `import {read} from "m"; { const read = 1; exports.read = read; }` | Import link and marker true | Outer import is not forwarded here; marker false | Both extensions; full-index imports plus direct import API |
| R1-F1.6 — JS/TS loader and interface globals; TS `export =` | Insert `require = replacement` in `js/pos-cjs-module-exports.js` and `ts/pos-export-equals-require.ts`; also assign `module` or `exports` | Definite forward, complete | The replaced global is not proven to be the original loader/interface; limited, no definite forward | Copied fixtures plus one assignment; all three globals and TS consumer checked |
| R1-F1.7 — Scala selector import links | In `scala/pos-import-then-export.scala`, put the first export in an object with `val Thing = new Impl`; separately place an equal-spelling import in a different object | Selectors still link the hidden or inaccessible import and mark it forwarded | Keep the valid object-member export; leave `local_name`/`import_span` unlinked and unrelated import marker false | Copied fixture edit plus separate-scope probe; direct imports and full-index output |
| R1-F1.8 — Lua whole-module, returned constructor and assembled-table consumers | Insert `local function unrelated() local m; m = {} end` into `pos-local-return.lua`, `pos-table-ctor.lua`, and `pos-table-field-local.lua` | All lose their required entries with `rebound_binding` | Preserve each original complete forward: the function's local is a separate binding | Three copied fixtures, same inserted declaration; direct/query/cache/CLI |
| Clean — Python assignment, definition, walrus, `with` and exception-target rebinding | Replace the imported local through the indicated binding site | Limited/rebound, affected entry and marker removed | Those sites really replace the imported local | Public probes and existing definition tests |
| Clean — Python mutually exclusive fallback imports | `try: from old import read` / `except ImportError: from new import read` | Two conditional declared entries | Both possible conditional forwards remain visible | Public probe, both `conditional` flags asserted |
| Clean — ECMAScript ordinary rebinding and unrelated function locals | Reassign `let m` at module scope; separately write an unrelated function-local `read` before an ES export | Rebound import withheld; unrelated local does not erase ES forward | Distinguish the two bindings | Public probes and existing ECMAScript tests |
| Clean — declared/shadowing loader globals | `const require = custom`, block-local `require`, or imported `module` | Limited/shadowed, no false forward | These are not proven CommonJS globals | Public probes |
| Clean — Perl | Imported sub followed by local sub definition; imports in separate packages | Rebound entry withheld; packages kept separate | No imported-sub provenance after replacement or across package boundaries | Public probe and `symbols_reexports_python_perl` package tests |
| Clean by invariant — Go, Rust, Swift, C++, C# | Corresponding positive fixtures and excluded local/private constructs | Expected entries and exclusions | Go scans package declarations; the others report written syntax rather than resolving these indirect locals | Public controls plus each extractor's bounded traversal; Rust `pub(self)` marker false |
| Outside this binding class — Java, C, Bash, Zsh, PHP | No applicable forwarding contract | Not applicable | No indirect binding decision exists | Exhaustive language dispatch and passing negative tests |

**Downstream trace.** The erroneous entries survive source-order assembly and whole-file query projection. [mark_forwarded_imports](../../lib/src/analysis/reexports/mod.rs:63) correctly marks every linked syntax/declared entry; it cannot repair a wrong link. The marker then reaches `ImportRecord`, direct `imported_symbols()`, CLI import conversion and the [unused-import exemption](../../lib/src/file/tree_file.rs:1510). The Python probe demonstrates the false exemption. ECMAScript and Scala's wrong markers remain observable even where existing reference-name handling independently prevents a warning; this finding does not request a repair of that older reference heuristic. Lua has no import records, so its damage is the report itself. Cache validation accepts these internally consistent reports and a hit preserves them; filtering to function declarations also preserves them. Declaration listings and Rust prelude selection use their existing separate contracts and are not additional consumers requiring a behavior change.

**Repair scope.** Make these bounded same-file checks respect binding scope and replacement, using the existing parent/scope information where suitable. Do not introduce cross-file resolution or a general interpreter. Add positive controls that retain a provable final import or an unaffected outer binding, alongside the shadowing negatives.

### R1-F2 — high: Lua reports a definite interface after the returned table can be modified

**Authority — spec:** Lua support explicitly adopts “U4–U7 rules.” The accompanying audit's **Minimal same-file facts** requires the returned `M` to have only chunk-level field assignments, to remain unreassigned, and to be neither passed as an argument nor used as a metatable target before return. The coverage goal also requires incomplete evidence to be distinguishable from verified absence. These are bounded safety checks, not a promise to evaluate runtime code.

**Defect class:** a mutable interface is reported from an earlier snapshot without accounting for later writes or escape to code that can change it.

The tree-hugger library's Lua [top-level scan and dynamic checks](../../lib/src/analysis/reexports/lua.rs) retain the fields collected by `assign`/`constructor_fields`. They skip function writes and do not invalidate a returned table passed to an ordinary call. Conditional and computed writes can also leave an earlier field reported unconditionally.

Start with the actual `lua/pos-table-field-local.lua` fixture:

```lua
local m = require("m")
local M = {}
M.foo = m.foo
return M
```

Insert `if enabled then M.foo = 1 end` before the return. The result remains a complete, unconditional declared `foo` forward. The bounded contract no longer establishes that interface. No evaluation of `enabled` is necessary to recognize the loss of certainty.

| Child ID / site | Shape tested | Observed result | Expected result | Verification evidence |
| --- | --- | --- | --- | --- |
| R1-F2.1 — field table / conditional write | Insert `if enabled then M.foo = 1 end` | Complete; earlier `foo` forward remains unconditional | Limited; do not claim the affected field as a definite forward | Copied fixture, one inserted statement; public API and CLI |
| R1-F2.2 — field table / function write | Insert `local function replace() M.foo = 1 end` and call `replace()` | Complete; stale `foo` remains | The forbidden non-chunk write prevents a definite table-interface claim | Copied fixture, inserted function/call; public/query/cache/CLI |
| R1-F2.3 — field table / argument escape | Insert `configure(M)` | Complete; `foo` remains | Limited under the explicit no-argument-escape rule | Copied fixture, one inserted call; public/query/cache/CLI |
| R1-F2.4 — constructor-initialized local table / argument escape | `local M = {read = m.read}; configure(M); return M` | Complete; `read` remains | Same limited result as the assembled table | Public probe exercises the other table-initialization path |
| R1-F2.5 — field table / unknown later key | Insert `M[key] = 1` | Complete; `foo` remains | Limited: the later write may replace `foo` | Copied fixture, one inserted assignment; public/query/cache/CLI |
| Clean — field table / definite overwrite | Insert `M.foo = 1` or `M["foo"] = 1` | Complete, no `foo` entry | The final known value is not import-derived | Public dot and bracket overwrite probes |
| Clean — table identifier reassignment | Insert `M = {}` | Limited/rebound, no entry | No safe forward through the original table | Public probe |
| Clean — metatable forwarding | `setmetatable(M, {__index=m})` | Limited/dynamic forwarding | The interface cannot be enumerated statically | Public probe and existing metatable tests |
| Clean — direct returned constructor | `return {read = m.read}` | Complete, correct entry | No intermediate table can escape | Public control and existing constructor fixture |

The changed JavaScript/TypeScript merge consumers also report `dynamic_forwarding` for `Object.assign` and spread instead of enumerating them; their passing fixture tests provide the corresponding bounded-merge controls. Python/Perl export-list edits are a different mechanism and were separately inspected. The remaining languages do not add a mutable returned-table convention.

**Downstream trace and repair.** These false complete reports survive direct, full/selective, filtered and cached requests; Lua has no import markers to provide a second check. Track the disqualifying uses of the actual returned table and emit an existing limit reason, withholding affected definite entries. Recognizing a write or argument escape is sufficient; analyzing called function bodies or adding an interprocedural protection layer would be disproportionate and is not requested.

### R1-F3 — high: Lua constructor keys produce incorrect exported names and miss overwrites

**Authority — spec:** `exported_name` is “The name callers import,” and the Lua contract reports imported values in returned table constructors under the U4–U7 rules. The adopted minimal facts require the last field write to be import-derived. Reporting the spelling of a key expression as a resolved name violates the report's own field contract.

**Defect class:** constructor key expressions are treated as resolved field names, so equivalent keys fail to share an identity and computed keys become fabricated names.

In tree-hugger, [constructor_fields](../../lib/src/analysis/reexports/lua.rs:303), which gathers named fields for both direct and local-table returns, takes the raw text of every `name` node. The assignment path's `field_target` already distinguishes dot names and literal bracket keys, so the two paths disagree.

Copy `lua/pos-table-ctor.lua` and change only `x = m.x` to `["x"] = m.x`. The report says the exported name is **`"x"` including quote characters**, rather than `x`. Change it to `x = m.x, ["x"] = 1`: the report still forwards `x`, although the later literal key overwrites that exact field.

| Child ID / site | Shape tested | Observed result | Expected result | Verification evidence |
| --- | --- | --- | --- | --- |
| R1-F3.1 — directly returned constructor | `["x"] = m.x` | Exported name includes quotation marks | Name `x`, or explicitly decline the spelling; never a fabricated quoted name | Copied `pos-table-ctor.lua`, one key edit; public/query/cache/CLI |
| R1-F3.2 — local-table constructor returned by identifier | `local M = {["read"] = m.read}; return M` | Same quoted-name error | Same key interpretation as direct return | Public probe exercises the second constructor consumer |
| R1-F3.3 — both constructor consumers / computed key | `[key] = m.read` | Complete named forward with exported name `key` | Do not invent the evaluated key; limit unsupported dynamic forwarding | Copied constructor key edit and local-table probe |
| R1-F3.4 — constructor last-write identity | `x = m.x, ["x"] = 1` | Still forwards `x` | No `x` forward: both literal keys name the same field | Copied fixture, one field-list edit; public/query/cache/CLI |
| R1-F3.5 — constructor numeric key | `[1] = m.read` | Fabricated named export `1` | No string-named forwarding entry for this unsupported key form | Public probe |
| Clean — Lua field assignment | `M.read = m.read; M["read"] = 1; return M` | No entry | Same literal key overwrites the earlier field | Public probe |
| Clean — JS and TS object property consumers | `module.exports = {"read": require("m").read}` | Name `read`, without quotes | Correct literal key name | Both extensions through shipped CLI and public surfaces |

The same constructor helper feeds both returned constructors and local interface tables, so both require regression tests. Identifier keys remain correct in the positive fixture; positional fields are deliberately unreported by the current bounded contract. Scala backtick names and ECMAScript string-named specifiers have explicit name normalization rather than this raw constructor-key conversion; their existing alias/name tests passed.

**Downstream trace and repair.** Field identity is decided before `upsert`, so the mismatch prevents last-write replacement and then reaches exported-name output unchanged. Sorting, kind projection, report validation and cache hits cannot correct it. Reuse consistent literal-key handling across Lua constructors and assignments; reject or limit unsupported expressions before creating a named entry. A new general escaping/evaluation layer is unnecessary. Escaped exotic literals can remain a documented limitation rather than enlarging this repair.

## Reproduction method

For each listed fixture edit, copied the audit sample to a temporary directory, applied the stated single edit, and ran the current built CLI from the repository root:

```sh
cargo run --color=never -q -p tree-hugger-cli -- \
  symbols --file /absolute/path/to/copied-sample.lua --json-full --no-cache
```

The unedited sample was run as a control. Additional short snippets in the instance tables cover sibling consumers and both ECMAScript extensions. A temporary Rust example checked `TreeFile::reexports()`, `imported_symbols()`, `syntax_diagnostics()`, full/selective queries, persistent-cache widening and hits, and a function-kind projection against the same files. All reproduced files parsed without syntax diagnostics; reports agreed across those surfaces. Expected outcomes in the tables are independently based on the quoted contract and source binding/field identities. The temporary example was removed; no production source or tests were changed by this review.

## Observations

### CLI help still promises re-exports in declaration filters

The existing `--exported` help comments in tree-hugger-cli [declaration arguments](../../cli/src/main.rs:254) and [class arguments](../../cli/src/main.rs:319) still say “re-exports included.” The current CLI guide correctly describes declaration-only results. This is inherited help-text drift, not another blocking requirement for this review; a later cleanup can align the help with the guide.

### Regression fixtures are coupled to the active feature directory

The new L1 tests embed audit samples from the active feature directory. Moving the feature after author review will require updating those paths or relocating samples to stable test fixtures. The samples themselves are static and useful; there is no present test-discovery defect. Account for that dependency during the author's eventual lifecycle move.
