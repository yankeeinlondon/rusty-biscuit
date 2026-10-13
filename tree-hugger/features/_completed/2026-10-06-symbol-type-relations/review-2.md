---
$schema: feature-review.yaml
ready: false
findings:
  - title: Python nested class scopes still assign incorrect base identities
    priority: high
  - title: Generic and base fields lack a permanent cache robustness matrix
    priority: medium
  - title: Three languages still omit required base relations
    priority: high
human_review: true
human_review_items:
  - |-
    Choose how C#, Swift, and Scala should record lists that mix inheritance
    with interface, protocol, or trait conformance. This is the unchanged,
    unresolved choice from review-1; no accepted choice was found.

    - Recommended: preserve written entries in `extends`, moving entries to
      `implements` only when same-file declarations prove conformance.
      Attach Swift extension conformances only to a uniquely identified
      same-file type; document the loss for external extension owners.
    - Add a separate list for bases whose category cannot be established.
      This requires a schema and output change.
    - Preserve only entries whose category can be established. This requires
      explicitly accepting the loss of external base facts.

    Select the meaning, including the treatment of Swift extensions, so the
    remaining readers and their acceptance tests can be implemented.
has_blocked_findings: true
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-10-05T20:25:48-07:00
spec: 2026-10-06-symbol-type-relations/spec.md
implemented: true
implemented_by: codex/gpt-6.1-sol
log: 2026-10-06-symbol-type-relations/log.md
description: "A **feature** review of `2026-10-06-symbol-type-relations/spec.md`"
feature: 2026-10-06-symbol-type-relations/review-2.md
previous: 2026-10-06-symbol-type-relations/review-1.md
next: 2026-10-06-symbol-type-relations/review-3.md
---

# Review of Symbol Type Relations — Iteration 2

The feature is **not production ready**. The earlier reproductions now pass,
but the repaired resolver still treats an outer Python class as a lexical
scope for a nested class body. That can assign a base relation to the wrong
type. Three required language readers remain blocked on the existing design
choice, and several newly populated cache fields lack lasting robustness tests.

## Earlier findings and blocked work

Review-1 uses a single Findings section and marks its last finding blocked.
The following checks distinguish that blocked work from the three actionable
repairs. No recorded author acceptance was found in the specification, plan,
original implementation log, or repair log.

| Review-1 finding | Repair assessment |
|---|---|
| Relation IDs are assigned without proving path meaning | The listed reproductions are repaired. The 30-row path matrix, Rust member-only queries, older-model container kinds, and cache widening tests pass. The class sweep remains incomplete: nested Python class scopes are covered below. |
| Template-template parameter defaults are dropped | Repaired for struct, function, alias, method, and constructor owners. Ordinary type/non-type defaults remain intact. Complete, declarations-only, warm-cache, and projection assertions pass. The additional Swift pack/inside-list requirement and Rust attributed-parameter position repairs are exercised too. |
| Callable requirements disappear without a direct template wrapper | Repaired for C++ member declarations, definitions, constructors, friends, and out-of-class definitions, with wrapped controls. The additional C# list-less requirement cases pass. Owned parameter lists remain empty where appropriate. |
| Three languages still omit required base relations | Still blocked, never unblocked before this repair. C#, Swift, and Scala rows remain Pending. Evaluating this finding was appropriate; claiming its implementation would not be. |

## Verification and scope

Reviewed the specification, both implementation logs, repaired readers and
their siblings, record construction, containment, query/cache projection,
schema decoding, outline rendering, current documentation, and test placement.
No production source changes or permanent tests were added during this review.

- `just test`: **1,079 passed, zero skipped**.
- `just lint`: passed for the library and CLI.
- `just test-l2`: six selected entry points passed. The tmux and WezTerm
  declaration/report tests executed; Kitty's two entry points returned early
  because that backend was unavailable. Those returns are not Kitty evidence.
- `just check-tier-coverage tree-hugger`: zero stranded tests.
- Supplemental checks copied library fixtures into temporary directories,
  appended isolated declarations, and ran the built `hug` CLI. Python checks
  compared complete uncached indexes, type-only uncached output, and actual
  warm hits. Each warm check reported one hit and zero parses/captures.
- A separate edited-cache sweep checked 13 columns × nine shapes through
  complete CLI output. A renamed sentinel distinguished served entries from
  recomputation. The persisted-format results are listed below.

The existing matrices run through declared L1 targets. The terminal targets
have a live L2 recipe and the `terminal-tests` feature enabled by that recipe
and CI metadata. Cross-OS receipts are left to CI and do not affect readiness.

## Unblocked Findings

### High — Python nested class scopes still assign incorrect base identities

**Defect class:** enclosing declaration scopes are treated as lexical name
lookup scopes even when the language excludes those scopes from lookup.

In the tree-hugger library,
[binding_scope](../../lib/src/file/tree_file/relations.rs) skips a Python class
only after passing a callable. It never records that lookup has already left
another class body. Thus a base inside `Outer.Middle` can incorrectly resolve
through `Outer` instead of continuing to an enclosing function or the module.
[class_scope_is_lexical](../../lib/src/file/tree_file/relations.rs) encodes the
same incomplete rule. This affects both bare and qualified base expressions.

Minimal reproduction:

```python
class Base: pass
class Outer:
    class Base: pass
    class Middle:
        class Leaf(Base): pass
```

Python evaluates `Outer.Middle.Leaf.__bases__[0] is Base` as true. Tree-hugger
instead gives Leaf's `extends` target the ID and qualified name of
`Outer.Base`. The result contradicts Python's class-block lookup rules; see
the [Python execution model](https://docs.python.org/3.14/reference/executionmodel.html).
The reproduction was also evaluated directly with the host Python interpreter.

To reproduce through the shipped interface, save the snippet as `probe.py`
and run these commands from its directory:

```sh
hug --json-full symbols --no-cache probe.py
hug --json types --all-scopes --no-cache probe.py
hug --verbose --json-full symbols probe.py
hug --verbose --json-full symbols probe.py
```

The wrong ID appears on both uncached paths and survives the final warm hit.
The ID identifies an unrelated base, so downstream inheritance graphs and
cached reports become incorrect even though the written `name` remains useful.

The scope sweep covers every implemented relation reader, the Rust ownership
consumers, and the pending/absent language branches. Names below omit report
file prefixes. Each Python row was checked on all three CLI paths above.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Python nested class, bare base | Module `Base`, `Outer.Base`, then `Outer.Middle.Leaf(Base)` | Resolves to `Outer.Base` | Resolve to module `Base` |
| Python nested class, qualified base | Module `NS.B`, `Outer.NS.B`, then `Outer.Middle.Leaf(NS.B)` | Resolves to `Outer.NS.B` | Resolve to module `NS.B` |
| Python nested class, outer variable | Module `Base`; `Outer.Base = object`; Leaf inside Middle | Text only: the outer variable blocks lookup | Resolve to module `Base` |
| Python nested class, outer import | Module `Base`; Outer imports `object as Base`; Leaf inside Middle | Text only: the outer import blocks lookup | Resolve to module `Base` |
| Python nested class, missing module binding | Only `Outer.Base` exists; Leaf inside Middle | Fabricates an ID for `Outer.Base` | Text only; runtime lookup would fail |
| Python nested class inside a function | Function-local `Base`, competing `Outer.Base`, then Middle.Leaf | Resolves to `factory.Outer.Base` | Resolve to `factory.Base` |
| Python immediate class body | Middle itself declares Base before Leaf | Resolves to `Outer.Middle.Base` | Same; clean |
| Python direct nested declaration header | `Outer.Leaf(Base)` evaluated directly in Outer's body | Resolves to `Outer.Base` | Same; clean |
| Python method body | Outer has Base; a method defines Leaf(Base); module Base exists | Resolves to module `Base` | Same; clean |
| TypeScript class member scope | Class property named Base; a method declares Leaf extending module Base | Resolves to module `Base` | Same; clean |
| JavaScript class member scope | Same shape as TypeScript | Resolves to module `Base` | Same; clean |
| Java nested class | Outer declares Base; Middle declares Leaf extending Base | Resolves to `Outer.Base` | Same; Java permits this lookup |
| C++ nested class | Outer declares Base; Middle declares Leaf deriving from Base | Resolves to `Outer::Base` | Same; C++ permits this lookup |
| Rust nested modules | Middle's trait writes `super::Base` | Resolves to outer module's Base | Same; clean |
| Rust impl ownership | Earlier absolute-root, enclosing-generic, method-only, and cache-widening rows | Correct textual/resolved containers and conformance | Same; existing regressions pass |
| Go interface embed | Package Base embedded in Leaf | Resolves to package Base | Same; no nested class construct |
| PHP qualified base in a function | Function declares a class extending `\Base` | Resolves to global Base | Same; PHP has no directly nested class-body analogue |
| C#, Swift, Scala | Local base/conformance declarations and owners | Empty relations | Separate blocked finding below |
| C, Perl, Bash, Zsh, Lua | Existing negative relation matrix rows | No invented relations | Same; no applicable required base construct |

**Required change:** model the distinction between the class body currently
executing a base expression and class bodies enclosing it. Skip outer class
bindings, variables, and imports after leaving the immediate body, while
retaining enclosing function/module lookup and explicit generic-parameter
evidence. Preserve the clean direct-header and own-body cases. Extend the
shared path matrix with all nine Python rows, then check complete/selective
results and warm cache hits. Update the current relation documentation and
the reader's comments, which currently describe only the method exception.
Bump the extractor revision when the extraction behavior changes.

### Medium — Generic and base fields lack a permanent cache robustness matrix

**Defect class:** populated persisted fields have positive round-trip tests
but no permanent malformed-input matrix asserting their public outcomes.

The tree-hugger CLI's
[cache robustness matrix](../../cli/tests/symbols/cache_robustness.rs) covers
`relations.implements`, metrics, attributes, and modifiers. It has no columns
for `type_info.generic_params`, declaration constraints, `relations.extends`,
the six nested generic fields, or the three nested relation-target fields.
Review-1 explicitly recommended retaining its supplemental sweep as permanent
tests, but the repair added positive cache checks rather than those columns.

This is a testing gap, not an observed decoder coercion. These fields carry
the feature's new facts; a future permissive deserializer could silently erase
them while all positive extraction/cache-equivalence tests still pass.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Generic parameter list | All nine matrix shapes below | Supplemental probes pass; no permanent column | Permanent public-output matrix |
| Declaration constraints | All nine shapes | Supplemental probes pass; no permanent column | Same |
| Extends list | All nine shapes | Supplemental probes pass; no permanent column | Same |
| Implements list | All nine shapes | Supplemental and permanent checks pass | Keep the permanent control |
| Generic name, kind, constraints, default, variance, position | Each field × all nine shapes | Supplemental probes pass; no permanent columns | Permanent columns for every field |
| Relation target name, ID, qualified name | Each field × all nine shapes | Supplemental probes pass; no permanent columns | Permanent columns; shared reader covers both relation lists |
| Metrics parameters, has_docs, lines | Existing public-output shape matrix, including invalid line counts | Permanent checks pass | Keep |
| Attributes and modifiers | Existing public-output shape matrices | Permanent checks pass | Keep |
| Other formats | YAML/TOML manifests, lockfiles, configuration | No reader added or changed by this feature | Not applicable |

**Required change:** extend the existing format-wide test with the missing
columns, retaining its real CLI-written fixture, positive controls, sentinel,
and one edit per cell. Assert the resulting public values as well as rejection
or service. The decoder need not change: the following observed contract is
appropriate. Keep these tests in the already declared L1 target.

## Blocked Findings

### High — Three languages still omit required base relations

**Defect class:** required source forms remain unimplemented while their
classification decision is unresolved and tests assert empty output.

**Blocked on the unchanged author choice in frontmatter.** No acceptance of
the specification's recommendation was found. This does not block repairing
the Python defect or completing the cache tests.

In the tree-hugger library,
[declared_bases](../../lib/src/file/extract/relations.rs) still has no C#,
Swift, or Scala reader. Their
[relation matrix rows](../../lib/tests/l1/symbols_relations.rs) remain Pending;
the current docs correctly mark these behaviors planned.

| Site | Shape tested | Observed result | Expected result after the choice |
|---|---|---|---|
| C# class | Local class Base and interface I; `class Child : Base, I` | Empty lists | Preserve/classify both targets |
| Swift class | Local class Base and protocol P; `class Child: Base, P` | Empty lists | Preserve inheritance and conformance |
| Swift struct | `struct S: P` | Empty lists | Preserve conformance |
| Swift extension | `extension Root: P`, with local Root | Nothing attached to Root | Apply accepted local-owner attachment rule |
| Scala class | `class Child extends Base with T`, with local class and trait | Empty lists | Preserve/classify both targets |
| Other eight implemented languages | Existing positive base and impl rows | Written lists populated | Retain, with Python repair above |
| C, Perl, Bash, Zsh, Lua | Existing negative rows | No invented relations | Retain |

Record the accepted meaning before implementing these readers, replace the
Pending rows with supported/negative/limitation cases, and verify query/cache
and outline behavior. If the author instead accepts reduced scope, record that
explicitly; an unresolved recommendation or planned label is not completion.

## Recurrence

- **Review-1, “Relation IDs are assigned without proving path meaning”:** the
  Python finding repeats its language-specific lexical-scope defect class.
  The repair extended the sweep to methods, imports, inherited names, and
  runtime owners, but missed nested class bodies and the corresponding
  exclusion of outer class variables/imports. It should have swept the
  immediate class body, outer class bodies, method bodies, and enclosing
  function scopes using both bare and qualified bases. This review carries
  all nine Python rows and controls for every sibling relation reader.
- **Review-1, “Three languages still omit required base relations”:** carried
  forward unchanged because its prerequisite choice was never accepted.
  This is outstanding blocked work, not a missed repair after unblocking.
- The cache-test gap was called out as a recommendation in review-1's Input
  robustness sweep, rather than as one of its titled findings.

## Input robustness matrix

JSON cache entries are the only affected persisted format. Each supplemental
cell started with a cache written from the real Rust generic fixture plus
populated relation declarations. The variance control was populated in the
stored record to exercise that optional field. A sentinel name on a separate
record proved service rather than equal output after recomputation.

**Serve** means the edited entry reached the complete public report;
**recompute** means rejection and fresh analysis. Absent collections mean
empty by this schema's omitted-empty encoding. Optional scalar absence and
null intentionally both mean no value; metrics keys must remain present.
For scalars, element-error shapes are arrays containing the control plus a
wrong value, or only a wrong value.

| Shape | generic_params | Declaration constraints | extends | implements |
|---|---|---|---|---|
| Populated control | Serve | Serve | Serve | Serve |
| Absent | Serve empty | Serve empty | Serve empty | Serve empty |
| Null | Recompute | Recompute | Recompute | Recompute |
| Wrong whole-field type | Recompute | Recompute | Recompute | Recompute |
| One wrong element | Recompute | Recompute | Recompute | Recompute |
| Every element wrong | Recompute | Recompute | Recompute | Recompute |
| Empty array | Serve empty | Serve empty | Serve empty | Serve empty |
| Duplicate key | Recompute | Recompute | Recompute | Recompute |
| Trailing invalid content | Recompute | Recompute | Recompute | Recompute |

| Shape | Parameter name | kind | constraints | default | variance | position |
|---|---|---|---|---|---|---|
| Populated control | Serve | Serve | Serve | Serve | Serve | Serve |
| Absent | Recompute | Recompute | Serve empty | Serve none | Serve none | Recompute |
| Null | Recompute | Recompute | Recompute | Serve none | Serve none | Recompute |
| Wrong whole-field type | Recompute | Recompute | Recompute | Recompute | Recompute | Recompute |
| One wrong element | Recompute | Recompute | Recompute | Recompute | Recompute | Recompute |
| Every element wrong | Recompute | Recompute | Recompute | Recompute | Recompute | Recompute |
| Empty | Serve empty string | Recompute | Serve empty | Serve empty string | Serve empty string | Serve zero |
| Duplicate key | Recompute | Recompute | Recompute | Recompute | Recompute | Recompute |
| Trailing invalid content | Recompute | Recompute | Recompute | Recompute | Recompute | Recompute |

| Shape | Target name | Target ID | Target qualified_name | metrics.lines |
|---|---|---|---|---|
| Populated control | Serve | Serve | Serve | Serve count |
| Absent | Recompute | Serve none | Serve none | Recompute |
| Null | Recompute | Serve none | Serve none | Serve null |
| Wrong whole-field type | Recompute | Recompute | Recompute | Recompute |
| One wrong element | Recompute | Recompute | Recompute | Recompute |
| Every element wrong | Recompute | Recompute | Recompute | Recompute |
| Empty | Serve empty string | Serve empty string | Serve empty string | Serve zero |
| Duplicate key | Recompute | Recompute | Recompute | Recompute |
| Trailing invalid content | Recompute | Recompute | Recompute | Recompute |

The first thirteen columns were exercised by the supplemental sweep in this
review. The metrics column is established by the passing permanent CLI
matrix; fractional, negative, and overflowing line counts are rejected too.
Attributes/modifiers and the other metric columns retain their permanent
shape tests. No malformed-value coercion was observed. The missing permanent
columns are the medium finding above.

## Requirement-to-verification assessment

| Requirement | Strongest relevant verification | Assessment |
|---|---|---|
| Generic ownership, bounds, defaults, variance, packs, positions, overflow | Level 1 complete/selective matrices and warm hits/projections | Earlier C++ defects repaired; new sibling repairs tested |
| Declared bases and conservative target IDs | Level 1 complete/selective matrices plus shipped-CLI probes | Python lexical-scope recurrence; three pending language readers |
| Rust positive/negative/empty impls, aggregation, self types and containment | Level 1 member-only, complete, older-model, and cache-widening tests | Earlier reproductions repaired |
| Written modifiers and raw attributes, order and ownership | Level 1 sixteen-language matrices and cache checks | No additional defect found |
| Callable own-body spans/lines, expressions, absent bodies, LF/CRLF and half-open spans | Level 1 body matrix and formula assertions | Appropriate verification; no additional defect found |
| One capture, optional-pass avoidance, bounded lookup work | Level 1 counters and growth checks | Appropriate verification; no new performance finding |
| Schema 4.1, revision invalidation, exact-version cache reads and projection | Level 1 public-output and warm-hit tests | Passing; malformed generic/target fields need permanent columns |
| JSON/JSONL/full indexes, counts, selectors, sorting and signature modes | Level 1 shipped-CLI tests | Appropriate level; erroneous extracted facts propagate |
| Styled outlines, relation headings and wrapping | Level 2 pane capture in tmux/WezTerm plus Level 1 output checks | Appropriate level present; Kitty not exercised |
| Keyboard, mouse, paste or terminal input encoding | No such behavior added | Level 3 not required |
| Older-model adapter losses and current docs | Level 1 adapter checks and document review | Losses documented; correct the incomplete Python scope explanation with its repair |

No additional implementation or performance recommendation is required beyond
the findings. Passing suites do not establish readiness for a source shape
they omit, and the remaining blocked language scope cannot be considered done.
