---
$schema: feature-review.yaml
ready: false
findings:
  - title: Declaration metadata repeatedly searches shared file and module context
    priority: medium
  - title: Large valid Zsh files abort the process in the existing grammar
    priority: high
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-05T17:05:08-07:00"
spec: 2026-10-03-symbol-filters-and-reports/spec.md
implemented: true
implemented_by: claude/opus
log: tree-hugger/features/2026-10-03-symbol-filters-and-reports/log.md
description: "A **feature** review of `2026-10-03-symbol-filters-and-reports/spec.md`"
feature: 2026-10-03-symbol-filters-and-reports/review-8.md
previous: 2026-10-03-symbol-filters-and-reports/review-7.md
next: 2026-10-03-symbol-filters-and-reports/review-9.md
---

The feature is **not production ready**. The previous review's missing performance record is repaired. The resulting investigation establishes an avoidable performance defect: declarations repeatedly rediscover shared namespace and test context. A separate high-priority finding records a pre-existing Zsh grammar crash, outside this feature's implementation. No human design decision is required.

## Previous review follow-through

Review-7 had one unblocked finding and no blocked findings. Reviews 1–6 also contain no blocked findings, so there is no newly unblocked requirement to reconcile.

| Previous requirement | Verification | Assessment |
|---|---|---|
| Record the post-feature performance sample | [implementation-log.md](./implementation-log.md) now contains the restored Phase 7 sample and a new review-cycle-7 sample. Both identify build/input/cache state and report elapsed time, output bytes and peak resident memory. | Implemented. |
| Compare cold and warm functions, lint and imports | The new sample compares retained release binaries on a frozen 162-file input. Warm functions explicitly report 162 hits and zero parses/captures/diagnostics. Lint/imports control output is identical. | Implemented; regressions are stated rather than concealed. |
| Measure serialized size and repository-wide memory | Baseline-tree JSON reproduces the prerequisite's exact byte count. Repository measurements distinguish default scope from `--all-scopes` and attribute the memory reduction to streaming. | Implemented. The recorded 361-line difference on the live repository was not investigated; it is not presented as an exact-output control. |
| Restore the missing completion narrative | Phase 6's requirement-to-test table and Phase 7's documentation, measurement and close-out sections are restored. | Implemented. |
| Preserve earlier functional repairs | Registered tests for patterned/curried parameters, owner identity, cache/package transitions, query reuse, docstrings, fence boundaries, Go directives/attachment precedence and literal stars passed in this review's feature run. | No recurrence established. |

The performance requirement has **no numerical gate**. A slower measured sample alone would not reopen it. The first finding below instead identifies repeated work with a small, semantics-preserving alternative, which the caller explicitly asks this review to assess.

## Scope and verification

Used the tree-hugger, rust-testing, Rust and biscuit-test-harness skills. Read the specification, prior reviews, implementation records, metadata extraction, shared predicates, name construction, query finishing/reuse, cache reader/projection, class selection, report modes/writers, renderer, current topic documentation and test registration. Existing extraction limits were treated as limits.

The workspace also contains concurrent symbol-type-relations changes, including schema 4.1 and extractor revision 18. They were preserved. Current-build checks below include those working-tree changes; timing comparisons use the retained release binaries from the previous repair's sample. The namespace/scope implementation and Rust inner-test-context loop being investigated are present in both that feature build and the current source. This review changes review documents and metadata only.

| Check | Result |
|---|---|
| `cargo build -p tree-hugger-cli --color=never` | Passed; fresh debug CLI used for public comparisons and crash confirmation. |
| Area `just test --profile ci -j 2 symbols_` | 247 passed; 780 tests outside the filter. Includes the all-language public-result matrices and prior repair regressions. |
| Root `just test tree-hugger-cli --profile ci -j 1` | All 283 CLI tests passed, zero skipped. Includes report goldens, malformed-cache matrix, revision upgrades, filter intersections, failure handling and output-mode conflicts. |
| Area `just lint` | Library and CLI passed. |
| Area `just test-l2` | Six selected entry points passed. tmux and WezTerm executed declaration and god-files captures; Kitty entry points followed their unavailable-backend paths. 283 tests were outside the tier. |
| Root `just check-tier-coverage tree-hugger` | Zero stranded tests. |
| Independent current-build CLI comparisons | 160 comparisons passed: ten per primary language fixture, copied into a new isolated Git repository with test-owned home/config/temp directories. |
| Independent release-build scaling probes | Copied fixtures with additional simple declarations; results and limits are detailed below. |

The 160 comparisons check default scopes, all scopes, local-only scope, deprecated/test/ordinary/async/undocumented/minimum-parameter selections against complete public records; count and all three stats maps agree. Selected JSON records equal JSON Lines records in every language. These verify report wiring; the registered extraction matrices independently pin the underlying expected facts. A full library suite was not rerun, and the filtered run is not represented as one.

### Requirement-to-verification assessment

| User-facing requirement | Appropriate level | Present verification |
|---|---|---|
| Scope, qualified names, async, owner identity and containment | Level 1 | All-language matrices; repeated/absent-owner and naming-context regressions; selected tests passed. |
| Test/deprecation flags, documentation/parameter metrics and unsupported values | Level 1 | Source-shape matrices, manifest-transition/cache/reuse tests and CLI selectors/coverage; selected tests passed. |
| Smart case, exact/type/container/file selectors, intersections, imports and argument conflicts | Level 1 | Public query and subprocess tests; all CLI tests passed. |
| JSON/JSON Lines envelopes, ordering, count/stats, empty selections, partial failures, broken pipes and cache equivalence | Level 1 | Structural assertions, goldens, revision/robustness tests and independent mode comparisons. |
| Styled lists/classes, outline/context headings, stats tables, signature modes, wrapping and links | Level 2 | Shared declaration scenarios executed in tmux and WezTerm, including every new view. |
| Plain/redirected text and literal control-sequence escaping | Level 1 | Byte/text assertions and goldens in the passing CLI run. |
| Keyboard, mouse, paste or input-method behavior | Not applicable | These commands introduce no interactive input behavior; Level 3 is unnecessary. |
| Work proportional to declarations when deriving shared source context | Level 1 | No stable work-count regression for repeated namespace/inner-attribute scans; the scaling reproduction below exposes the gap. |

The library's consolidated `l1` target declares the symbol matrices. The CLI `symbols` target declares its filter/report/cache modules. Both terminal targets require `terminal-tests`, enabled by the live L2 recipe and package CI metadata. No test was added or renamed. No terminal-verification-level mismatch was found. Cross-OS evidence does not determine readiness here.

### Input robustness matrix

The persistent JSON cache is the relevant structured reader. Its [field-by-shape contract](../../docs/cache-design.md) and [public-output matrix](../../cli/tests/symbols/cache_robustness.rs) cover entry identity/coverage and record scope, qualified name, namespace rooting, async/static modifiers, test/deprecation flags, metrics-facet presence and nullable parameter/documentation values. Control, absent, explicit null, wrong whole-field type, one/all wrong elements, empty, duplicate key and trailing-invalid outcomes are specified, including inapplicable cells. Required nullable metrics distinguish missing fields from unsupported values. A sentinel proves that an accepted entry was served, rather than silently recomputed.

The matrix passed in the complete CLI run. Required entry fields have no permissive defaults; record collection defaults follow the documented schema contract. The current concurrent schema's version/lines checks also passed, but those additions are not requirements of this feature. Rust package-test detection checks manifest existence rather than reading a manifest format. Source readers consume syntax trees, so neither finding below is a JSON/YAML/TOML configuration-field defect.

## Unblocked Findings

### Medium — Declaration metadata repeatedly searches shared file and module context

**Defect class:** metadata extraction repeatedly scans the same file/module context for individual declarations, making cold analysis grow much faster than the number of declarations.

In the tree-hugger library, [file_namespace_before](../../lib/src/file/extract/qualified_name.rs:330), which establishes a declaration's file-level namespace, walks all preceding siblings until it finds a namespace, or reaches the beginning. [Builder::prefix](../../lib/src/file/extract/qualified_name.rs:186), which chooses a declaration's enclosing naming context, invokes it separately whenever a symbol reaches the file root. The memo stores each symbol's finished name; it does not store the namespace in force at a top-level node. With N flat declarations and no namespace, the helper makes at least N(N−1)/2 sibling steps. Rust, Python, JS/TS, C/C++, Swift, Lua, Bash and Zsh pay this search even though `file_level_namespace` has no matching statement branch for them. Java, Go, C#, PHP and Perl repeatedly traverse the intervening declarations to a preceding statement. Scala repeats the traversal of every preceding package clause and declaration.

The same class exists in [is_in_rust_cfg_test](../../lib/src/file/extract/flags.rs:79), the tree-hugger library's source test-context detector. For each unmarked declaration it reaches the same root/module and calls [has_inner_cfg_test](../../lib/src/file/extract/flags.rs:101), enumerating all children again. Files/modules with no inner attribute are the expensive negative case. Counting only two attributed test functions out of thousands still performs the shared-context work for the other declarations.

This affects normal cold runs and cache misses, not just a benchmark mode. A 2,000-function Rust file took 2.675 seconds in the feature build against 0.104 seconds in the retained prerequisite build; TypeScript took 5.175 against 0.144 seconds. The existing log's repository slowdown is consistent with this defect, but it is not the only evidence. A one-pass index of file-level namespace state and memoized inner-test state removes the repeated scans without changing the name/test contracts, requiring another crate, or imposing a numerical performance gate.

#### Complete language instance sweep

Each primary fixture was copied, retaining its existing controls, and extended with 500, 1,000 and 2,000 uniquely named simple declarations. Functions were used except Java/C#, which used empty classes and `types`. Lua additions were placed before its final return for the corrected probe. Both retained release binaries ran the same relative-file selection, `--no-cache --plain`, with one Rayon worker and captured stdout. Values below are elapsed seconds, the smaller of two runs except the corrected Lua single runs. They are diagnostic measurements under concurrent host load, not thresholds. Every reported declaration count agrees between the two builds.

| Site | Shape tested | Observed result: feature build at 500 / 1,000 / 2,000; prerequisite at 2,000 | Expected result |
|---|---|---|---|
| Rust | Flat free functions, no file-level namespace | 0.178 / 0.577 / 2.675; 0.104 | Do not search preceding declarations for a nonexistent file namespace. |
| TypeScript | Flat free functions after fixture controls | 0.196 / 0.719 / 5.175; 0.144 | Same; in-body namespaces retain their existing names. |
| JavaScript | Flat free functions | 0.166 / 0.839 / 4.218; 0.101 | Same. |
| Python | Flat functions | 0.179 / 0.540 / 2.575; 0.530 | Same; preserve file-qualified names. |
| C | Flat `void` functions | 0.304 / 0.780 / 1.629; 0.938 | No file-statement scan; earlier extraction costs also exist. |
| C++ | Flat functions after namespace fixture controls | 0.308 / 1.967 / 6.925; 3.410 | No file-statement scan; preserve namespace-body ownership. Baseline already grows superlinearly. |
| Swift | Flat functions after fixture controls | 0.214 / 0.620 / 1.711; 0.175 | No file-statement scan. |
| Lua | Flat functions before final return | 0.077 / 0.252 / 0.968; 0.051 | Same; all added functions are extracted. |
| Bash | Flat functions | 0.072 / 0.247 / 0.962; 0.045 | No file-statement scan. |
| Zsh | 50 / 100 / 200 flat functions; larger case blocked by the separate crash below | Feature 0.027 / 0.030 / 0.037; counts correct. Shared helper still runs. | Skip the inapplicable namespace search. No 500–2,000 timing claim for this grammar. |
| Java | File-level package followed by empty classes | 0.150 / 0.578 / 2.773; 0.092 | Determine the package once, preserving its prefix. |
| Go | File-level package followed by functions | 0.150 / 0.729 / 2.585; 0.090 | Same. |
| C# | File-scoped namespace followed by classes | 0.268 / 1.037 / 3.169; 1.838 | Index namespace state once; baseline also has growth. |
| PHP | Fixture namespace followed by functions | 0.111 / 0.304 / 1.062; 0.077 | Index the namespace in force at each statement; later namespaces replace earlier ones. |
| Perl | Fixture package followed by subs | 0.157 / 0.500 / 1.958; 1.086 | Same replacement rule; baseline also has growth. |
| Scala | Fixture package clauses followed by functions | 0.128 / 0.318 / 1.332; 0.077 | Accumulate preceding clauses once; preserve their nesting. |

#### Shared-helper and consumer sweep

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| `file_level_namespace` dispatch | All 16 language copies above | Correct matching branches, but unsupported languages still reach the full backward walk | Clean language semantics; eliminate the inapplicable work. |
| `Builder::qualified` memo | All-language names/owner matrices and scaled copies | Correct names; each distinct flat symbol still discovers its own root context | Keep symbol memoization and share root context too. |
| `Builder::prefix`, `rust_impl_prefix`, `symbol_scope` ancestor walks | Flat probes; registered nested/qualified-owner/scope cases | Correct classifications; ancestry is walked separately per record | Include these callers when designing the shared context index; preserve nearest-owner and `crate`/`self`/`super` semantics. No separately quantified depth-only regression is claimed. |
| Rust inner-attribute detector | Same Rust fixture with 500 / 1,000 / 2,000 additions; `functions --count` | No flags: 0.089 / 0.265 / 1.005 s. `--tests-only`: 0.115 / 0.349 / 1.322 s, always count 2. `--exclude-tests` always count N+33. | Correct selections are clean controls; discover root/module inner-test state once. |
| Rust outer-attribute detector; other-language source test/deprecation detectors | Registered source-boundary matrices and independent selectors | Correct source markers; no equivalent shared root/module inner-attribute scan in other languages | Clean; retain attribute-derived flags. |
| `SymbolQueryResult::finish`, naming prefix and package test rule | Public namespace/scope queries, manifest-transition/reuse tests, all-language CLI comparisons | Prefix and file-wide test rule decided once per result; current flags and names correct | Clean; keep mutable filesystem decisions outside cached source context. |
| Cache hit/widening/projection | Passing cache comparisons and genuine-hit/counter regressions | Hits avoid parsing; source work recurs on misses or widening requiring a parse | Clean hits; do not change cache identity to conceal extraction cost. |
| Count, stats, JSON, JSON Lines and terminal reports | Count probe and 160 public mode/filter comparisons | Correct selections/output; even count pays extraction cost | Share corrected metadata; no report-specific workaround. |

Repair the two repeated shared-context scans together. Prefer a bounded per-file traversal/index or memo for namespace and root/module inner-attribute state; an early language guard alone leaves the five namespace-replacement languages, Scala and Rust inner-test scan unfixed. Preserve all existing name/owner/test matrices, including stacked Scala packages, later PHP/Perl namespaces, Rust inner/outer `cfg(test)` and negative controls. Add a stable work-count regression for the growing flat-file negative case and repeated module descendants, compiled in the existing L1 target. Re-measure the same bounded sample and record the results. Timing assertions, new performance CI cells, a new dependency and changes to report ordering are unnecessary.

### High — Large valid Zsh files abort the process in the existing grammar

**Defect class:** a source grammar serializes parser state inconsistently and aborts in its native reader, bypassing the library's error return and terminating every report that parses the file.

**This is pre-existing and outside the changes required by this specification.** Both retained prerequisite and feature binaries abort on the same input, as does the freshly built current CLI. It is recorded because the caller asks for glaring defects observed outside the feature to be stated explicitly. The feature's readiness decision already follows from the preceding finding.

Copy `lib/tests/fixtures/symbols/zsh.zsh` into an isolated Git repository and append 500 uniquely named lines of this form:

```zsh
scale_0() { :; }
scale_1() { :; }
# Continue through scale_499.
```

`/bin/zsh -n zsh.zsh` succeeds. `hug functions zsh.zsh --no-cache --plain` aborts with `Assertion failed: (size == length), function deserialize, file scanner.c, line 446.` The subprocess return code is −6 (SIGABRT), rather than a successful report or an ordinary reported analysis failure.

The dependency is `tree-sitter-zsh` 0.56.0 in [tree-hugger's manifest](../../lib/Cargo.toml:42). Its external scanner stores `context_stack.size` in one byte but serializes every stack element; its deserializer reads that one-byte count and asserts that the consumed length equals the supplied length. The observed failure is at that assertion. In the tree-hugger library, [SourceFile::parse](../../lib/src/file/source_file.rs:142), used to turn source text into a syntax tree, cannot translate a native abort into its promised error result. Catching a Rust panic around this call would not catch SIGABRT.

#### Complete parser and caller instance sweep

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Zsh grammar, unedited primary fixture | Original controls | Both retained builds and current public comparisons succeed | Clean positive control. |
| Zsh grammar, growing copied fixture | 50, 100, 200, then 500 valid flat functions | Both retained builds select all added functions at the smaller sizes; both abort at 500 | Valid source must not terminate the process. |
| Rust, TypeScript, JavaScript, Go, Python, Java, C#, C, C++, Swift, Scala, PHP, Perl, Lua and Bash grammars | Same copied-fixture growth, using each language's simple function/class syntax; Lua before final return | All 15 parse/report 500, 1,000 and 2,000 additions without a process abort | Clean controls; performance costs are the separate finding above. |
| Current CLI `functions`, `types`, `symbols`, `classes` | Exact 500-function Zsh file, relative file selection and `--no-cache --plain` | All four abort at the same scanner assertion | Complete analysis or report a normal file failure. |
| Current CLI `imports`, `lint` | Same file and flags | Both abort at the same assertion | Same; these commands are outside this feature's report changes. |
| Current CLI `god-files` | Isolated directory containing only the same file; command has no `--no-cache` option | Aborts at the same assertion, with no stdout | Same; its independent analysis also uses the grammar. |
| `SourceFile::parse` / `TreeFile::new` entry points | Shared parser path inspected; declaration and god-files executions above exercise their callers | Both paths enter the same native grammar | One scanner repair should protect all callers. No separate direct-library reproduction is claimed. |
| Declaration error envelope/partial-output handling | Existing passing read/analysis-failure and partial-output tests | Handles returned errors, but cannot handle the demonstrated native abort | Clean normal-error controls; do not confuse them with coverage of grammar-state overflow. |

Handle this in a separate grammar fix: correct or replace the scanner's state encoding while preserving Zsh support, and add a subprocess regression so a native abort fails a test without killing the whole runner. Retain the original fixture, smaller-size controls, and the six report commands plus god-files in the caller sweep. Do not silently exclude larger Zsh files or try to convert this abort with `catch_unwind`. The grammar source is a dependency rather than a new reader introduced by this feature; a dependency change belongs to that separate fix and its documentation.

## Blocked Findings

None. The findings need no human design decision or foreground input testing. The Zsh issue is explicitly assigned to a separate fix because it predates this feature, rather than described as an unimplemented symbol-report requirement.

## Recurrence

Compared both defect classes with every finding in reviews 1–7. Neither repeats an earlier finding. Review-7 identified a missing measurement record; this review confirms that record is repaired and identifies repeated runtime context discovery. Reviews 1–6 concern parameter loss, owner identity, stale test classification, documentation/marker semantics and terminal verification; their repaired regressions pass. The native Zsh scanner abort was not reported there. `recurrence: false`.

## Closure

Saved review #8, linked review-7 forward and marked it implemented, and set the specification's review count to eight. The specification remains incomplete. No implementation repair, test edit, formatting run, commit or lifecycle-directory move was made.
