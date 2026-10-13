---
$schema: feature-review.yaml
ready: false
findings:
  - title: A sole same-named type overrides explicit owner qualification
    priority: high
  - title: Reused query results retain obsolete file-wide test flags
    priority: medium
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: "2026-10-05T13:27:54-07:00"
spec: 2026-10-03-symbol-filters-and-reports/spec.md
implemented: true
implemented_by: claude/opus
log: tree-hugger/features/2026-10-03-symbol-filters-and-reports/log.md
description: "A **feature** review of `2026-10-03-symbol-filters-and-reports/spec.md`"
feature: 2026-10-03-symbol-filters-and-reports/review-2.md
previous: 2026-10-03-symbol-filters-and-reports/review-1.md
next: 2026-10-03-symbol-filters-and-reports/review-3.md
---

The feature is **not production ready**. The six original reproductions are repaired, but two sibling cases remain in the ownership and test-classification repairs. Both can be fixed without a human design decision. This review changes review documents and metadata only.

## Previous review follow-through

The preceding review has a single Findings section rather than separate Unblocked Findings and Blocked Findings sections. All six findings were unblocked; its frontmatter confirms there were no blocked findings. There are no newly unblocked items to reconcile.

| Earlier finding | Verification of the repair | Remaining class issue |
|---|---|---|
| Signature filters discard valid declared parameters | Pattern-bound Rust/JS/TS parameters, all Scala parameter lists, receiver exclusions, unnamed parameters and CLI count/type selectors pass the updated metrics, signatures and selection tests. | None found in this sweep. |
| Owner identity is lost in qualified names and containment | Repeated owners, qualified Rust impls, Swift extensions and stacked Scala packages pass the new owner-identity tests. Separate public CLI controls confirm correct links for unqualified, crate-qualified and repeated same-file Rust owners. | A single unrelated same-named Rust type is still chosen when the explicit owner is elsewhere; first finding below. |
| Marker detection loses meaningful source boundaries | Attribute arguments now follow syntax boundaries, Rust comments do not hide `cfg(test)`, and Go block comments retain paragraph breaks. Marker matrices and CLI regressions pass. | None found in the specified marker rules. |
| Python documentation coverage mistakes string expressions for docstrings | Function and class forms distinguish ordinary/concatenated literals from bytes/formatted literals. The documentation metrics and CLI coverage regressions pass. Additional public probes with leading comments and parenthesized comments preserve documentation presence. | None found within the documented source-text, non-evaluating rule. |
| Cached Rust test flags survive changes to package boundaries | Stored entries now retain source-derived flags and fresh cache lookups reapply file-wide flags. Warm-cache manifest-transition regressions pass. | Reusing an already finished result through the library's narrowing API retains the old file-wide flag; second finding below. |
| New terminal views lack real-terminal verification | Shared terminal checks now invoke outline, stats, full/names-only lists and tables, and class tables. Exact rows, indentation, wrapping, contextual styles and applicable links are asserted. | None found; tmux and WezTerm executed these checks successfully. |

The previous review's `implemented` property is set to true as requested: the repair cycle ran and addressed its named examples. That property does not establish readiness or closure of the defect classes.

## Scope and verification

Read the specification, plan, implementation and repair logs, previous review, extraction readers, ownership resolution, common filters, query finishing/narrowing, cache projection, class presentation, report-mode resolution, stats, terminal checks, current topic documentation and test registration. Used the tree-hugger and rust-testing skills.

| Check | Result |
|---|---|
| `just test` in `tree-hugger/` | 949 Level-1 tests passed; zero skipped. |
| `just lint` in `tree-hugger/` | Library and CLI passed. |
| `just test-l2` in `tree-hugger/` | Six selected test entry points passed. Declaration and god-files captures executed in tmux and WezTerm; two Kitty entry points returned through their unavailable-backend skip path. |
| Root `just check-tier-coverage tree-hugger` | Zero stranded tests. |
| Public CLI probes | Current debug `hug`, isolated source directories, explicit file selection and `--no-cache`; copied primary fixtures plus focused source controls. |
| Public library probes | Temporary Rust programs linked against the current tree-hugger library; compared fresh queries, parsed-file queries and narrowed results over copied primary fixtures for all 16 languages. |
| Rust ownership input validity | The focused two-file reproduction compiled with `rustc --edition=2024 --crate-type lib`. |

The library's consolidated Level-1 target declares its new modules, including owner identity. The CLI's symbols target declares the package-boundary cache tests. The terminal targets declare their required feature; the live Level-2 recipe and CI feature metadata enable it. No implementation tests were added or renamed by this review. Cross-OS evidence is left to CI and does not affect this readiness decision.

### Requirement-to-verification assessment

| User-facing requirement | Appropriate level | Strongest evidence reviewed |
|---|---|---|
| Scope, async, names, parameter/doc metrics, markers and selector composition | Level 1 | Per-language public-result matrices and CLI selections. Ownership and reused test flags have the defects below. |
| JSON, JSON Lines, count, stats JSON, deterministic ordering, errors and broken pipes | Level 1 | CLI report tests, goldens and subprocess behavior checks. |
| Cache capability selection, unchanged identities and post-lookup prefixes/classification | Level 1 | Cache/query tests and public probes; fresh cache classifications pass, narrowing misses the negative transitions below. |
| Styled lists, grouping, class sections and compact signatures | Level 2 | Shared declaration checks executed in tmux and WezTerm. |
| Outline indentation/context styling, sorting, grouping and narrow wrapping | Level 2 | Newly added scoped real-terminal captures with exact rows and styles. |
| Stats table layout and coverage line at wide/narrow widths | Level 2 | Newly added real-terminal captures. |
| Full/names-only lists and tables, long headers, class tables | Level 2 | Newly added real-terminal captures, wrapping and applicable link checks. |
| Plain/redirected text and literal control escaping | Level 1 | Byte/text contracts in CLI tests. |
| Keyboard, mouse, paste and input-method behavior | Not applicable | These commands introduce no interactive input behavior; Level 3 is unnecessary. |

### Input robustness matrix

The load-bearing structured reader affected by this feature is the persistent JSON cache. The existing [cache robustness matrix](../../cli/tests/symbols/cache_robustness.rs:141) covers the feature's scope, qualified name, namespace-rooting, async modifier, test/deprecation flags, metrics facet, parameter count and documentation-presence columns. Its public-output checks exercise the positive control, absence, explicit null, wrong whole-field type, one wrong element, every wrong element, empty value, duplicate key and trailing invalid content, with inapplicable shapes treated explicitly. It passed in the Level-1 run.

The syntax readers introduce no new JSON/YAML/TOML configuration format. Rust package detection checks manifest existence; it does not parse manifest fields. The second finding concerns changing external filesystem state after a successful query, rather than a malformed cache field. The cache-format matrix therefore remains necessary but cannot establish correctness of result reuse.

## Unblocked Findings

### High — A sole same-named type overrides explicit owner qualification

**Defect class:** an ownership fallback treats a unique bare-name candidate as sufficient even when the child's explicit qualified name identifies a different owner.

In the tree-hugger library, [find_container_symbol](../../lib/src/analysis/mod.rs:534) first tries enclosing spans and qualified names, then accepts the sole candidate of the recorded kind. Rust's container name is deliberately bare (`W`), so the final `bare == container_name` condition bypasses qualification validation. This silently attaches an out-of-body method to an unrelated local type. Correct qualified display text does not prevent the wrong relation from driving outlines and class reports.

Create these two files in a scratch directory:

```rust
// main.rs
pub struct W;
mod other;
impl other::W { pub fn probe(&self) {} }
```

```rust
// other.rs
pub struct W;
```

This is valid Rust. Run from that directory:

```sh
hug symbols --file main.rs --tree --format plain --no-cache
hug classes --file main.rs --name 'W!' --json --no-cache
hug classes --file main.rs --name 'W!' --jsonl --no-cache
```

The outline nests `probe` beneath the local `W`, and both class machine formats include it in that local type's member list. The complete index names the method `main.rs#other::W::probe`, but its parent reference names the local `W`. Because the owner is outside this file and cross-file resolution is intentionally unavailable, the method must retain its qualified name and appear with an unresolved/contextual owner; it must not become a member of the unrelated local `W`.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Rust bare-name fallback | Sole local `W`, explicit `impl other::W`, owner in another file | Links method to local `W` | No same-file owner link. |
| Rust copied primary fixture | Added sole `UniqueOwner` plus `impl external::UniqueOwner` | Same wrong local owner link | No link to local `UniqueOwner`. |
| Rust direct control | Sole `W`, `impl W` | Links to `W` | Clean. |
| Rust rooted control | Sole top-level `W`, nested `impl crate::W` | Name `W::probe`, link to `W` | Clean. |
| Rust repeated-owner control | `left::W`, `right::W`, `impl right::W` | Links to `right::W` | Clean. |
| Swift extension reader/fallback | Copied fixture, sole local `UniqueOwner`, `extension External.UniqueOwner` | Qualified name retained; no local parent link | Clean. |
| Lua table reader/fallback | Copied fixture, sole local `UniqueOwner`, `External.UniqueOwner.review_probe`; added before final return | Qualified name retained; no local parent link | Clean. |
| Go receiver reader | Same-file pointer receiver in owner matrix | Links to its same-file type | Clean. Go does not permit defining a method on another package's type, so the foreign-owner case is inapplicable. |
| C++ out-of-body reader | Copied fixture plus `External::UniqueOwner::review_probe` definition | No method record captured | Existing extraction limit; cannot prove a qualified fallback with this uncaptured shape. |
| C++, C#, Java, Python, TypeScript, Swift, Scala, PHP direct owners | Repeated named owners in distinct namespaces/outer types, owner-identity and containment matrices | Members retain their own owner's ID | Clean direct-owner paths. |
| JavaScript, C direct owners | Primary qualified-name/containment matrices | Extracted class/object or named aggregate members retain their recorded owners | Clean within accepted extraction set. |
| Perl, Bash, Zsh | Primary name/scope/containment matrices | No equivalent out-of-body type-member construct in the accepted extraction set | Not applicable to this fallback case. |
| Outline, class text, class JSON, class JSON Lines | Same Rust negative case; direct command checks for outline/JSON/JSON Lines, shared class summarizer inspected for text | Consume the wrong local relation | All presentations must keep the external member out of the local class. |

Repair the shared owner decision so uniqueness cannot override conflicting explicit qualification. Preserve the passing unqualified and rooted same-file controls, and preserve unresolved members in the outline. Add absent-owner cases alongside the existing repeated-owner matrix and assert parent IDs, class membership and contextual outline output. Update the ownership contract in comments/current docs and bump the extractor revision when changing stored relations.

### Medium — Reused query results retain obsolete file-wide test flags

**Defect class:** a reusable result mixes source-derived classification with a mutable filesystem-derived flag, then reapplies only the positive filesystem decision when producing another result.

In the tree-hugger library, [SymbolQueryResult::finish](../../lib/src/file/query.rs:474) sets every record's test flag when the file is currently test-only. [SymbolQueryResult::narrowed](../../lib/src/file/query.rs:520), the public API for answering another query without reparsing, removes the naming prefix and projects the already finished records before calling `finish` again. If the package layout now makes the file ordinary source, the previous `true` remains. Source-only stored cache entries are repaired, but this public reuse path is not.

Public API reproduction, with a scratch package containing `tests/probe.rs`:

```rust
use tree_hugger::{Capability, SourceFile, SymbolQuery};

// Write tests/probe.rs: pub fn helper() {} #[test] fn marked() {}
// Write Cargo.toml in the enclosing scratch package.
let query = SymbolQuery::new().capabilities([Capability::Flags]);
let result = SourceFile::read(&path)?.query(&query)?;
std::fs::remove_file(package_root.join("Cargo.toml"))?;
let reused = result.narrowed(&query).expect("covered query");
let fresh = SourceFile::read(&path)?.query(&query)?;
```

`reused` reports `helper.is_test == true`; `fresh` reports false. Both correctly retain true for the genuinely attributed `marked`. Public selector probes return two tests instead of one for `tests_only(true)`, and zero ordinary declarations instead of one for `exclude_tests(true)`. The same problem occurs when adding a nearer manifest, making `tests/` a package root rather than an integration-test directory. This is a library correctness issue; independent shipped CLI cache lookups pass the repaired transition tests.

The public sweep copied each of the 16 primary language fixtures under a scratch `tests/` directory, made one manifest change between the wide and narrowed queries, and compared every record's ID and test flag with a fresh result. Fresh `SourceFile` and `TreeFile` queries agreed in every cell.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Rust narrowed result | No manifest → add outer manifest | Zero differences from fresh result | Clean positive transition. |
| Rust narrowed result | Outer manifest → remove it | 103 of 107 fixture records retain obsolete true flags | Zero differences. |
| Rust narrowed result | Outer manifest → add nearer `tests/Cargo.toml` | 103 of 107 fixture records retain obsolete true flags | Zero differences. |
| Rust narrowed result | Nearer manifest → remove it | Zero differences | Clean positive transition. |
| Rust source-attribute control | Remove outer manifest around helper plus `#[test]` function | Helper stale; attributed function true in both results | Helper false; attributed function stays true. |
| Fresh `SourceFile::query`, `TreeFile::query` | All four transitions over every language fixture | Results agree | Clean. |
| Persistent `SymbolCache::query`, CLI JSON/test selectors | Outer and nearer manifests added/removed, existing warm-cache regressions | Hit and no-cache classifications agree | Clean; repaired storage path. |
| TypeScript, JavaScript, Go, Python narrowed results | All four manifest transitions, each primary fixture | Zero differences | Clean: these file-wide rules depend on file name, not Cargo manifests. |
| Java, C# narrowed results | All four transitions, attribute-bearing fixtures | Zero differences | Clean: source-derived test markers remain source facts. |
| C, C++, Swift, Scala, PHP, Perl, Bash, Zsh, Lua narrowed results | All four transitions, each primary fixture | Zero differences | Clean: no Cargo-dependent test-file rule. |
| Naming-prefix removal and coverage projection | Narrowing a finished result with Flags still requested | Retain merged test flags; `finish` cannot distinguish their origin | Reuse source-only facts or decline reuse when their external classification cannot be refreshed correctly. |

Keep the source-derived test facts available when reusing a result, or detect an external-classification change and return `None` so callers obtain a fresh result. Do not blanket-clear all test flags: attributes and Rust `cfg(test)` must survive. Extend the existing manifest-transition table through the public narrowing API, including both selectors, an unchanged-layout control, source-marked controls and the positive/negative transitions. Update narrowing/finishing docs to describe the chosen reuse contract.

## Blocked Findings

None. Neither finding requires a new design ruling, human interaction, foreground terminal testing or cross-file name resolution.

## Recurrence

- **A sole same-named type overrides explicit owner qualification** repeats the class of review-1's **Owner identity is lost in qualified names and containment**. That repair swept repeated owners and qualified matching owners, but missed the shared fallback when only one unrelated bare-name candidate exists. The sweep needed both matching-owner and absent-qualified-owner controls for Rust impls, Swift extensions and Lua table members, plus the downstream outline/class projections. This review carries those sites and records the inapplicable Go and uncaptured C++ shapes explicitly.
- **Reused query results retain obsolete file-wide test flags** repeats the class of review-1's **Cached Rust test flags survive changes to package boundaries**. That repair covered fresh analysis and persistent-cache lookup, but missed reuse through `SymbolQueryResult::narrowed`, its coverage projection and the second call to `finish`. This review sweeps all four manifest transitions over all 16 language fixtures and identifies both failing transitions, rather than reporting only manifest removal.

The earlier review is the only previous review in this feature directory. Its terminal-verification finding is now closed; this iteration finds no new mismatch between a terminal requirement and its verification level.

## Closure

Recorded two unblocked findings, linked this iteration from the previous review and advanced the spec's review count to two. The spec is not marked completed. No implementation code was changed, no commit was made, and the feature remains in its current lifecycle directory.
