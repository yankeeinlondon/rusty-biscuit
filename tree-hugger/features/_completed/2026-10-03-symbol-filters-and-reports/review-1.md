---
$schema: feature-review.yaml
ready: false
findings:
  - title: Signature filters discard valid declared parameters
    priority: high
  - title: Owner identity is lost in qualified names and containment
    priority: high
  - title: Marker detection loses meaningful source boundaries
    priority: high
  - title: Python documentation coverage mistakes string expressions for docstrings
    priority: medium
  - title: Cached Rust test flags survive changes to package boundaries
    priority: high
  - title: New terminal views lack real-terminal verification
    priority: high
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: "2026-10-05T12:23:59-07:00"
spec: 2026-10-03-symbol-filters-and-reports/spec.md
implemented: true
implemented_by: claude/opus
log: tree-hugger/features/2026-10-03-symbol-filters-and-reports/log.md
description: "A **feature** review of `2026-10-03-symbol-filters-and-reports/spec.md`"
feature: 2026-10-03-symbol-filters-and-reports/review-1.md
next: 2026-10-03-symbol-filters-and-reports/review-2.md
---

The feature is **not production ready**. Six grouped findings remain. None is blocked or needs a human design decision. This review records defects and updates the specification's review count; it does not repair implementation code.

## Scope and verification

Reviewed the specification, implementation log, declaration extraction, query predicates, cache coverage and projection, class presentation, report writers, renderer, current documentation, and test registration. Used the tree-hugger and rust-testing skills. Existing documented extraction limits were treated as limits, rather than requests to introduce new symbol kinds or resolve cross-file references.

| Check | Result |
|---|---|
| `just test` in `tree-hugger/` | 929 Level-1 tests passed; 0 skipped |
| `just lint` in `tree-hugger/` | Passed for library and CLI |
| `just test-l2` in `tree-hugger/` | 6 selected tests passed; declaration captures ran in tmux and WezTerm |
| Root `just check-tier-coverage tree-hugger` | 0 stranded tests |
| Public reproductions | Built `target/debug/hug`, isolated temporary source trees, explicit file selection, and `--no-cache` except cache probes |

The library's consolidated Level-1 target declares the new matrix modules. Cargo discovers the CLI `symbols` test target and its declared modules. Both terminal targets require `terminal-tests`; the live `test-l2` recipe and CI feature metadata enable them. There is no keyboard interaction in this feature, so Level 3 is unnecessary. A passing test count does not establish coverage of terminal modes the tests never invoke.

For language sweeps, primary fixtures from `lib/tests/fixtures/symbols/` were copied into scratch directories, retaining their existing controls, and a focused declaration was added. Lua additions were placed before the final `return`, and PHP additions retained the original opening tag. Small standalone source pairs additionally isolated the failing shapes below. Checks used `--json-full symbols` for public metrics and relations, then ordinary filtered commands, counts, stats, outlines, or class reports to establish the user-visible consequence.

**Input robustness matrix:** the persistent reader is relevant because scope, namespace-rooting, test flags, async modifiers, and metrics now affect cached reports. The existing [cache robustness matrix](../../cli/tests/symbols/cache_robustness.rs) exercises control, absent, null, wrong whole-field type, one wrong element, every wrong element, empty, duplicate-key, and trailing-invalid shapes through complete CLI output. Its feature columns distinguish absent metrics from explicitly unsupported values and reject missing required scope/flag/rooting fields. These tests passed. The new source readers read syntax trees, rather than introducing another JSON/YAML/TOML configuration parser. The cache finding below concerns an unrecorded filesystem input, which malformed-entry tests cannot catch.

**Recurrence:** the final finding repeats the class of insufficient real-terminal verification reported in the [earlier symbol-improvements review](../2026-10-03-symbol-improvements/review-1.md). That earlier gap was repaired for the default declaration layouts; this feature introduces additional layouts without equivalent terminal checks.

## Findings

### High — Signature filters discard valid declared parameters

**Defect class:** parameter measurement and signature predicates consume a partial display-signature list, silently losing valid parameter patterns or later parameter lists.

In the tree-hugger library, [symbol_metrics](../../lib/src/file/extract/metrics.rs:15) counts the list returned by [declared_parameters](../../lib/src/file/extract/signatures.rs:48), except for the separate C/C++/Go counter. The Rust and JavaScript/TypeScript readers require particular identifier shapes; Scala's [parameter_list_node](../../lib/src/file/extract/signatures.rs:104) takes only the first list. The tree-hugger library's [RecordFilter](../../lib/src/file/filter.rs) also uses this partial signature for `--param-type`. Consequently, valid functions disappear from selections, while reporting a measured zero or smaller count rather than an unsupported metric.

Representative source:

```rust
pub fn patterned((a, b): (u32, u32), _: String) {}
```

```typescript
export function patterned({x}: {x:number}, [a,b]: number[]) {}
```

```scala
def patterned(a: Int)(b: String): Int = a
```

For each copied file, `hug functions --file FILE --name patterned --min-params 2 --count --no-cache` prints `0`; it must print `1`. Type predicates for `(u32,u32)`, `number[]`, and `String`, respectively, also print `0` instead of `1`.

| Reader/site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Rust | Tuple pattern plus `_` parameter | `parameters: 0`; count selector drops function; declared tuple type does not match | Two parameters; function selected; declared type available |
| TypeScript | Typed object and array patterns | `parameters: 0`; count and declared array-type selectors drop function | Two parameters; both selectors select function |
| JavaScript | Object and array patterns | `parameters: 0`; count selector drops function | Two parameters; function selected |
| Scala | Two curried lists, second containing `String` | `parameters: 1`; count and second-list type selectors drop function | Two parameters; both selectors select function |
| C | Two unnamed prototype parameters | Count is 2; minimum-count selection succeeds | Clean for count; unnamed display parameters remain a documented extraction limit |
| C++ | Two unnamed prototype parameters | Count is 2; minimum-count selection succeeds | Clean for count; same documented signature limit |
| Go | Two unnamed parameters | Count is 2; minimum-count selection succeeds | Clean for count; no named signature entries were synthesized |
| Python | Positional-only parameter plus required keyword-only parameter | Count is 2; minimum-count and `str` selectors succeed | Clean |
| Java | Two typed method parameters | Count is 2; minimum-count and `String` selectors succeed | Clean |
| C# | Two typed method parameters | Count is 2; minimum-count and `string` selectors succeed | Clean |
| PHP | Two typed parameters | Count is 2; minimum-count and `string` selectors succeed | Clean |
| Swift | Unnamed external label plus named second parameter | Count is 2; minimum-count and `String` selectors succeed | Clean |
| Lua | Two named parameters | Count is 2 | Clean; no declared type text |
| Perl, Bash, Zsh | Ordinary function declaration | Parameter metric is null; minimum-count selector excludes it | Clean: documented unsupported parameter extraction |

Measure syntactic parameters independently of whether they have a simple display name, and preserve the declaration's pattern/type when producing signature entries. Include all Scala parameter lists, while retaining existing receiver, variadic, known-empty, and unsupported-value rules. Extend the language metrics and selection matrices with these forms and CLI count/type-filter regressions. A fix to the extraction facts must bump `EXTRACTOR_REVISION`.

### High — Owner identity is lost in qualified names and containment

**Defect class:** owner resolution discards explicit qualification or selects a same-named owner without preserving its identity, producing misleading names and incorrect member grouping.

In the tree-hugger library, [find_container_symbol](../../lib/src/analysis/mod.rs:517) first looks for an enclosing span, then chooses the first candidate with the same bare name and kind. That fallback cannot distinguish out-of-body Rust methods on same-named types. The name builder also reuses [impl_self_type_name](../../lib/src/file/extract/rust_impl.rs:18), whose documented purpose is to return only the last path segment, so an explicitly qualified Rust self type loses its qualifier. [file_namespace_before](../../lib/src/file/extract/qualified_name.rs:269) keeps only the nearest Scala package clause, although consecutive Scala package clauses nest. Swift extension members preserve a qualified name but cannot link it to a same-file nested type named only by its final segment.

```rust
mod left { pub struct Boxed; impl Boxed { pub fn work(&self) {} } }
mod right { pub struct Boxed; impl Boxed { pub fn work(&self) {} } }
```

`hug symbols --file repeat.rs --tree --format plain --no-cache` places both `work` methods beneath `left::Boxed`, leaving `right::Boxed` empty. `hug classes --file repeat.rs --in '*right::Boxed' --json --no-cache` selects the right container but returns an empty member list. These are same-file declarations, so cross-file resolution limits do not explain the failure.

| Reader/projection | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Rust containment fallback | Same-named types in two inline modules; separate impls | Right method links to left type; outline/class grouping is wrong | Each method links to its own type ID |
| Rust qualified-name builder | `impl right::Repeated` at file level | Name ends in `Repeated::second`; relation links to first `Repeated` | Name retains `right::Repeated::second`; relation identifies right type |
| Swift extension containment | `extension Left.Repeated` and `extension Right.Repeated`, both types in the file | Qualified method names are correct, but neither has a container relation; selected class displays no method | Link each extension method to the same-file nested type |
| Scala namespace builder | `package outer` followed by `package inner` | `inner.PackageProbe.work`; `outer.inner.*` matches nothing | `outer.inner.PackageProbe.work` |
| C++ / C# | Nested namespaces and repeated nested type names | Full namespace names and correct enclosing owner IDs | Clean |
| Java | Repeated nested type names under different classes | Correct enclosing owner IDs and package-qualified names | Clean |
| Python | Repeated nested class names under different outer classes | Correct enclosing owner IDs and file-qualified names | Clean |
| JS / TS | Repeated local or namespace-contained class names | Correct enclosing owner IDs and qualified names | Clean |
| Swift, in-body declarations | Repeated nested types under different outer classes | Correct enclosing owner IDs | Clean; extension path is the failing sibling |
| Scala, in-body declarations | Repeated nested types under different objects | Correct enclosing owner IDs | Clean; sequential package clauses are the failing sibling |
| Go | Pointer receiver methods on a same-file type | Correct type ID and package-qualified names | Clean; two same-named top-level types are not a legal Go control |
| PHP / Perl | Consecutive namespace/package declarations | Later namespace replaces earlier one | Clean: these declarations do not nest as Scala's do |
| PHP | Methods on two differently named classes | Correct class IDs | Clean |
| Lua | Methods on two named tables | Correct table IDs | Clean |
| C | Primary fixture's named struct/union fields and enum members | Existing containment matrix passes | Clean; no out-of-body methods |
| Perl / Bash / Zsh | Primary fixture package/function names | Existing qualified-name and containment matrices pass or mark member cases inapplicable | Clean within documented extraction set |
| Outline / class terminal / class JSON / class JSON Lines | Rust repeated-owner case | Consume the wrong or missing relations; class `members` omits the right method | All presentations agree on the correct owner |

Retain declaration-node identity for direct owners and enough explicit qualification for out-of-body owners. Resolve within the source's namespace/module context, and avoid choosing the first unrelated same-named type. Keep qualification intended for display separate from the bare-name helper. Extend containment/name matrices with duplicate owner names, qualified impls/extensions, and stacked Scala package clauses; assert relations by owner ID and qualified name, not merely by bare container name. Update the relevant docs/comments alongside the repair and bump the extractor revision.

### High — Marker detection loses meaningful source boundaries

**Defect class:** deprecation and test detectors flatten syntax or comment structure, causing quoted marker text to count and genuine markers separated by legal trivia or paragraphs to disappear.

In the tree-hugger library, [is_swift_unconditional_deprecation](../../lib/src/file/extract/flags.rs:179) splits raw arguments on every comma, including commas inside a quoted message. [is_cfg_test](../../lib/src/file/extract/flags.rs:115) removes whitespace but does not ignore Rust comments. Go's [has_deprecated_paragraph](../../lib/src/file/extract/flags.rs:205) needs blank lines, but [clean_block_comment](../../lib/src/file/extract/doc_comments.rs:187) deletes them before the detector runs.

```swift
@available(*, message: "hello, deprecated, world")
func decoy() {}
```

`hug functions --file markers.swift --deprecated --name decoy --count --no-cache` prints `1`, although the declaration is not deprecated.

```rust
#[cfg(/* rationale */ test)]
mod checks { pub fn helper() {} }
```

`hug symbols --file markers.rs --tests-only --name checks --count --no-cache` prints `0`, although the module has the same predicate as `#[cfg(test)]`.

```go
/* OldBlock documents OldBlock.

Deprecated: use New.
*/
func OldBlock() {}
```

`hug functions --file probe.go --deprecated --name OldBlock --count --no-cache` prints `0`, although the deprecation paragraph is valid. The equivalent `//` comment group prints `1`.

| Detector/site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Swift availability | Marker word is a comma-separated fragment inside `message` | `is_deprecated: true`; decoy selected | False; no selection |
| Swift availability control | Real `deprecated` argument with comma-containing message | True | Clean |
| Rust cfg predicate | Comment before the `test` token | Module and descendants have `is_test: false` | True for both |
| Rust cfg controls | Plain `cfg(test)`; fixture's `cfg(any(test, …))` | True / false respectively | Clean |
| Go block-doc paragraph | Introductory paragraph, blank line, `Deprecated:` paragraph | False | True |
| Go controls | First-paragraph block marker; equivalent line-comment paragraphs; marker mid-paragraph | True / true / false | Clean |
| Rust deprecation/test attributes | Real attributes; marker word inside unrelated `doc` argument; qualified test attribute | Correct exact-name decisions | Clean |
| Java | `@Deprecated`; unrelated annotation argument saying `Deprecated`; qualified test attributes | Correct flags | Clean |
| C# | `[Obsolete]`; description argument saying `Obsolete`; supported test attributes | Correct flags | Clean |
| C++ | `[[deprecated("old, deprecated")]]`; `[[nodiscard("deprecated")]]` | True / false | Clean |
| Scala | `@deprecated` versus `@nowarn("deprecated")` | True / false | Clean |
| Python | `warnings.deprecated` versus unrelated decorator with marker argument | True / false | Clean |
| Java / JS / TS / PHP doc tags | Actual `@deprecated` tag versus ordinary prose containing the word | Correct flags in tag controls and primary matrices | Clean |
| C / Perl / Lua / Bash / Zsh | Primary fixtures and marker prose; C Doxygen tag | False | Clean: no supported deprecation marker; no test-attribute rule |

Read Swift argument boundaries and Rust cfg tokens from syntax rather than splitting opaque strings; preserve blank paragraph separators when cleaning Go block docs. Keep the explicit non-test compound-cfg rule. Expand the flags matrix with quoted commas, comments inside predicates, and both Go comment styles, and assert the filtered CLI selection as well as the flags. Bump the extractor revision.

### Medium — Python documentation coverage mistakes string expressions for docstrings

**Defect class:** documentation presence is inferred from the shape of one string node rather than whether the first statement is an actual nonempty docstring.

In the tree-hugger library, [python_docstring](../../lib/src/file/extract/doc_comments.rs:143) accepts any first `string` expression and rejects `concatenated_string`. Python bytes literals and formatted strings are not docstrings; adjacent ordinary string literals are. This changes `--undocumented` results and the published documentation coverage ratio.

```python
def plain():
    "docs"
def formatted():
    f"docs"
def bytestring():
    b"docs"
def concatenated():
    "doc" "umented"
def escaped():
    "\u0041"
```

`hug functions --file docs.py --undocumented --no-cache` selects only `concatenated`. It must select `formatted` and `bytestring`. The corresponding `--stats --json` reports 4/5 documented (0.8), instead of 3/5 (0.6).

| Reader/site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Python function docstring | First statement `f"docs"` | Documented | Undocumented |
| Python function docstring | First statement `b"docs"` | Documented | Undocumented |
| Python function docstring | First statement `"doc" "umented"` | Undocumented | Documented |
| Python function controls | Ordinary plain literal and nonempty escaped literal | Documented | Clean |
| Python class reader | Same first-statement string forms on four classes | Plain/formatted/bytes classes documented; concatenated-literal class undocumented | Plain and concatenated documented; formatted and bytes undocumented |
| Rust / TS / JS / Java / C# / C / C++ / Swift / Scala / PHP | Documenting comment before a function and ordinary-comment decoy | `has_docs: true` / false | Clean |
| Go | Attached comment versus comment detached by a blank line | True / false | Clean for presence; paragraph preservation is covered above |
| Lua / Perl / Bash / Zsh | Primary fixtures | `has_docs: null`; excluded by `--undocumented` | Clean: unsupported documentation detection |

Recognize ordinary Unicode string literals and their legal adjacent concatenation; exclude bytes and formatted literals. Apply the nonempty rule consistently. Extend the documentation metric forms for both functions and classes, then assert `--undocumented` membership and stats totals. Avoid evaluating arbitrary source to obtain this result. Bump the extractor revision.

### High — Cached Rust test flags survive changes to package boundaries

**Defect class:** a cached classification depends on external filesystem state that the cache neither records nor rechecks, so a cache hit changes the result relative to fresh analysis.

In the tree-hugger library, [is_in_cargo_tests_dir](../../lib/src/file/extract/flags.rs:153) locates the nearest ancestor containing `Cargo.toml`. The tree-hugger library's [SymbolCache::query](../../lib/src/cache/symbol_cache.rs:317) serves previously computed flags on a hit, while its key covers source identity/content and extraction inputs rather than that package-boundary state. Identical source at an identical path can therefore have a stale test flag.

Reproduction with a new scratch cache:

1. Write `tests/sample.rs` containing `pub fn helper() {}` in a directory with no ancestor Cargo manifest.
2. Run `hug functions --file tests/sample.rs --tests-only --count --verbose`: `0`, one cache miss.
3. Add `Cargo.toml` in the scratch root, without touching the source.
4. Run the same command: `0`, one cache hit.
5. Add `--no-cache`: `1`.

An extractor-revision bump would clear today's entries once, but would not fix the continuing mismatch when manifests are added, removed, or moved.

| Classifier/site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Rust package-test detection | Add root `Cargo.toml` around an unchanged `tests/` file | Warm count 0; fresh count 1 | Both 1 |
| Rust nearest-package detection | Seed root-positive cache, then add `tests/Cargo.toml` | Hit retains outer-package test flags; fresh analysis recognizes the new nearest package | Warm and fresh flags agree |
| Rust primary-fixture sweep | Same two manifest changes, all declarations | 103 flags differ between warm and fresh reports in each direction | No differences |
| Go / Python / JS / TS | Add Cargo manifest around unchanged primary-fixture copies | Cache hit and fresh test flags agree | Clean: their rules depend on file name, not this manifest |
| Java / C# | Same change, attribute-bearing primary fixtures | Cache hit and fresh test flags agree | Clean: attributes are source-keyed |
| C / C++ / Swift / Scala / PHP / Perl / Lua / Bash / Zsh | Same change, primary-fixture copies | Cache hit and fresh test flags agree | Clean: no package-based test-file rule |
| `--tests-only`, `--exclude-tests`, JSON flags | Rust cached versus fresh classification | Consume the stale flag | All must consume the current classification |

Make this external dependency explicit: recheck/reapply the file-wide test classification outside the stored source facts, or validate the recorded package boundary before serving cached flags. Preserve genuine source-derived test attributes and cfg markers. Add warm-cache regressions for adding/removing the outer manifest and introducing/removing a nearer manifest, with explicit cache-hit assertions and fresh-result comparison. Document the chosen cache behavior.

### High — New terminal views lack real-terminal verification

**Defect class:** tests verify rendered bytes and plain goldens for new terminal layouts without exercising those layouts through a terminal's rendering behavior.

In tree-hugger-cli, [level2_declarations](../../cli/tests/level2_declarations.rs:164) invokes default lists, grouping, classes, wrapping, and the short-signature table. Its tmux/WezTerm/Kitty entry points all call the same checks. None invokes `--tree`, `--stats`, `--signatures full`, or `--signatures none`. The corresponding [reports tests](../../cli/tests/symbols/reports.rs) and goldens are Level 1. The live terminal recipe therefore does not prove the new outline indentation/context styling, stats tables, or alternate signature rendering.

| User-facing requirement / report path | Strongest verification present | Observed test coverage | Expected verification |
|---|---|---|---|
| Styled default lists, file/module grouping, class sections | Level 2 | Scoped pane captures assert order, styles, locations and links | Appropriate; clean |
| Compact-signature prose wrapping | Level 2 | Pane-width and injected-width captures | Appropriate; clean |
| Short-signature declaration table | Level 2 | Columns, wrapping, continuation cells and links captured | Appropriate; clean |
| Outline with selected/contextual parents, grouping and sorting | Level 1 | Plain rows and golden; no real-terminal invocation | Level 2 for visible indentation, contextual styling and wrapped members |
| Terminal stats tables and coverage line | Level 1 | Plain golden and captured-pipe assertions | Level 2 for actual table layout and coverage line at representative widths |
| Full and names-only signatures in styled lists/tables | Level 1 | Plain output and forced-color bytes; terminal checks use the default short mode | Level 2 for the alternate rendering paths, including a long full header |
| Plain/redirected output, literal control escaping | Level 1 | Byte and text assertions | Appropriate for the byte contract; clean |
| JSON, JSON Lines, count, filters, metadata, cache behavior | Level 1 | Public-result tests, matrices and subprocess assertions | Appropriate; clean aside from the functional findings above |
| Keyboard, paste, mouse, IME | Not applicable | Feature reads no such events | No Level-3 requirement |

Extend the shared real-terminal declaration checks with these new views, using scoped capture markers, isolated fixtures, controlled widths, and existing background harness panes. Assert exact rows and indentation, relevant cell styles, and narrow-width behavior. Keep plain/machine assertions at Level 1, and keep all additions in the declared terminal target and live Level-2 recipe. No foreground focus changes or human interaction are needed.

## Repair scope

No additional dependency, general visibility ordering, or project-wide resolver is needed to address these findings. Preserve the specification's supported/unsupported distinctions, update behavior comments and current topic docs when repairing extraction, and rerun the affected language matrices and CLI checks through nextest-backed recipes. Leave the feature in its current lifecycle directory for author review.
