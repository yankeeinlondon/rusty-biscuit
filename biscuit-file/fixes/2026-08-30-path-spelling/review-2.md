---
$schema: feature-review.yaml
ready: false
findings:
  - title: Import resolution still drops guarded uses across block scopes and alternate identifiers
    priority: medium
  - title: Module discovery still misses valid Rust path attributes
    priority: medium
  - title: Manifest validation still accepts malformed inventory fields
    priority: medium
human_review: true
human_review_items:
  - |-
      Inspect the source guard's completeness plan before another automated fix cycle. All three defect classes from review #1 recur at sibling sites even though its original examples now pass. The findings below provide reproducible cases and complete provider and target tables; no home-directory policy or package-scope decision needs to be made again.
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: "2026-10-03T14:46:40-07:00"
spec: "2026-08-30-path-spelling/spec.md"
implemented: false
description: "A **fix** review of `2026-08-30-path-spelling/spec.md`"
fix: "2026-08-30-path-spelling/review-2.md"
previous: "2026-08-30-path-spelling/review-1.md"
---

The fix is **not production ready**. The original review examples are addressed, but three recurring source-guard defects still allow a passing check to miss forbidden production code or accept malformed inventory configuration. These are actionable implementation and test gaps. Human inspection is requested because the recurrence rule stops the automated loop; it does not block implementing the findings.

## Previous review findings

[Review #1](review-1.md) contains a single `Findings` section with three actionable findings, rather than separate unblocked and blocked sections. Its frontmatter marks none as blocked. There were no blocked findings to unblock before implementation.

| Previous finding | What the implementation addressed | Closure assessment |
| --- | --- | --- |
| Import resolution silently hides guarded calls and unresolved aliases | Chained imports, unknown imported aliases, inline-module and function scopes, grouped imports, globs, and same-file re-exports now have executable regression tests | Original examples fixed; class remains incomplete at block scopes, raw identifiers, and the alias-depth limit |
| Manifest interpretation can omit compiled targets and accept malformed fields | A TOML parser replaces the line reader; alternate target syntax, path validation, defaults, boolean build settings, duplicates, and invalid documents are tested through the guard result | Original three-field matrix fixed; newly read fixture-exclusion fields are not fully validated |
| Source discovery excludes production binaries and external modules | Binaries under `tests/` are scanned unless explicitly listed as gated fixtures; production module trees and ordinary external `#[path]` modules are followed | Original examples fixed; valid alternate attribute strings and conditional inline-module paths are missed |

The implementation log for this iteration is [log.md](log.md). No production changes were made during this review.

## Unblocked Findings

### Medium — Import resolution still drops guarded uses across block scopes and alternate identifiers

**Defect class:** incomplete scope and identifier resolution loses guarded import provenance and silently treats some function uses as safe.

In `darkmatter-cli`, the shared [Resolver](../../../darkmatter/cli/tests/common/path_lookup_guard.rs:711) identifies forbidden filesystem canonicalization and home lookup for all six guard consumers. Its scope inventory covers functions and inline modules, but ordinary blocks and closures share the enclosing function's scope. The first import or definition found there can therefore decide a use in a different block. The [use_bindings](../../../darkmatter/cli/tests/common/path_lookup_guard.rs:888) reader also abandons an entire import when it meets a raw identifier such as `r#invoke`. Finally, the alias-depth fallback leaves an intermediate alias whose last segment is not the guarded function name; `Provider::Other` then discards it rather than reporting uncertainty.

This valid Rust fixture returns no guard problems, although its second block uses raw filesystem canonicalization:

```rust
mod style {
    pub fn canonicalize() {}
}

pub fn probe() {
    { use self::style::canonicalize; }
    {
        use std::fs::canonicalize;
        let _ = canonicalize(".");
    }
}
```

The same failure occurs when the first import is inside a closure. Reversing the blocks reports both uses as guarded, instead of distinguishing the forbidden call from the harmless style function. This matters to exact-count exceptions: the scanner attributes calls to the wrong operation as well as losing calls entirely.

Two independent same-file reproductions also pass incorrectly:

```rust
use std::fs::canonicalize as r#invoke;
pub fn probe() {
    let _ = r#invoke(".");
    let _ = r#invoke::<&str>;
}
```

An eight-import chain (`use std::fs::canonicalize as a0; use a0 as a1; … use a6 as a7;`) followed by `a7(".")` and a function reference also produces zero problems. The module documentation explicitly promises that a chain too deep to resolve remains an unresolved candidate; the implementation does not keep that promise.

The sweep exercised the actual `engine::problems` result for every provider below. “Missed” means an empty problem list, with no exception. Alias tests include both a call and a function reference; generic function references specify their type argument.

| Provider/site | Qualified control | Separate blocks / closure import | Reversed blocks | Raw function alias (`r#invoke`) | Alias chain of 1 / 4 / 8 / 12 imports | Expected result |
| --- | --- | --- | --- | --- | --- | --- |
| `std::fs::canonicalize` | Rejected | Missed / missed | Both uses attributed to guarded import | Both uses missed | Rejected / rejected / missed / missed | Report guarded uses; distinguish block bindings; deep chains must at least be unresolved |
| `tokio::fs::canonicalize` | Rejected | Missed / missed | Both uses attributed to guarded import | Both uses missed | Rejected / rejected / missed / missed | Same |
| `dunce::canonicalize` | Rejected | Missed / missed | Both uses attributed to guarded import | Both uses missed | Rejected / rejected / missed / missed | Same |
| `std::path::Path::canonicalize` | Rejected | Missed / missed | Both uses attributed to guarded import | Both uses missed | Rejected / rejected / missed / missed | Same |
| `std::path::PathBuf::canonicalize` | Rejected | Missed / missed | Both uses attributed to guarded import | Both uses missed | Rejected / rejected / missed / missed | Same |
| `std::env::home_dir` | Rejected | Missed / missed | Both uses attributed to guarded import | Both uses missed | Rejected / rejected / missed / missed | Same |
| `dirs::home_dir` | Rejected | Missed / missed | Both uses attributed to guarded import | Both uses missed | Rejected / rejected / missed / missed | Same |
| `home::home_dir` | Rejected | Missed / missed | Both uses attributed to guarded import | Both uses missed | Rejected / rejected / missed / missed | Same |
| All eight providers, raw **module** alias | `use <provider> as r#provider; provider::<name>(…)` | Reported as unresolved | — | — | — | Clean conservative fallback: review required |
| All eight providers, existing import-form matrix | Qualified references, direct aliases, module aliases, chained aliases, grouped/nested imports, and provider globs | Existing matrix passes | — | — | — | Retain these controls |
| Both rules, existing separate-module and separate-function fixtures | Approved helper, unknown import, unimported name, function-local import | Existing tests pass | — | — | — | Retain scope isolation |
| Canonicalization method / style controls | Zero-argument path method; method with arguments; same-file style function; comments, strings, test-only code | Existing tests pass | — | — | — | Retain distinctions |

**Required change:** represent ordinary lexical blocks and closures, recognize Rust raw identifiers, and retain target provenance when resolution reaches its depth limit. A conservative unresolved candidate is sufficient where attribution is unavailable. Extend the existing provider matrix to include these shapes and both block orders, through the public guard result. Update the scope and depth-limit documentation to match the corrected behavior.

### Medium — Module discovery still misses valid Rust path attributes

**Defect class:** a module-source reader interprets only a subset of valid Rust attribute syntax and can scan a harmless fallback instead of compiled production code.

In `darkmatter-cli`, [attribute_paths](../../../darkmatter/cli/tests/common/source_scan.rs:550) extracts module paths by accepting an opening double quote and taking bytes through the next quote. It does not decode Rust string literals, and ignores raw strings entirely. [module_declarations](../../../darkmatter/cli/tests/common/source_scan.rs:438) uses a plain `#[path]` for an inline module's directory but ignores that inline module's `cfg_attr` path alternatives. The same shared reader handles library, binary, and build-script module trees.

A compact reproduction has a root outside `src/` containing:

```rust
#[path = r"../external/file.rs"]
mod file;
fn main() {}
```

Place `pub fn f() { let _ = std::fs::canonicalize("."); }` in `external/file.rs` and an empty file in `tools/file.rs`, with the root at `tools/main.rs`. Cargo compiles the external module. The guard ignores the raw-string path, scans `tools/file.rs`, and passes. Without the harmless fallback it reports a missing module; that still misreads valid production code, but the fallback proves this can be a silent omission.

For `#[path = "../external/\x66ile.rs"]`, Rust loads `external/file.rs`; the scanner invents `external/x66ile.rs`. An empty file at the invented location makes the guard pass. For an inline module, `#[cfg_attr(unix, path = "../external")] mod outer { mod file; }` similarly selects the external child on this host, while the scanner reads `tools/outer/file.rs`.

The sweep used the same forbidden external file and harmless alternate files, changing one declaration per fixture. All 24 target/shape fixtures below compiled with `cargo check --offline` on macOS. Each was checked through `engine::problems` without exceptions.

| Source-tree entry site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Declared library root | Plain double-quoted `#[path]` | Forbidden call rejected | Reject; clean control |
| Declared binary root | Same | Forbidden call rejected | Reject; clean control |
| Build-script root | Same | Forbidden call rejected | Reject; clean control |
| Declared library root | Raw `r"…"` and hash-delimited `r##"…"##` `#[path]` | Both pass | Follow external file and reject |
| Declared binary root | Same | Both pass | Same |
| Build-script root | Same | Both pass | Same |
| Declared library root | Escaped `\x66` inside `#[path]` | Passes, reads invented safe filename | Decode literal and reject |
| Declared binary root | Same | Same | Same |
| Build-script root | Same | Same | Same |
| Declared library root | Ordinary `cfg_attr(unix, path = "…")` on an out-of-line module | Forbidden call rejected | Reject; clean control |
| Declared binary root | Same | Forbidden call rejected | Reject; clean control |
| Build-script root | Same | Forbidden call rejected | Reject; clean control |
| Declared library root | Raw-string path inside out-of-line `cfg_attr` | Passes | Follow external file and reject |
| Declared binary root | Same | Passes | Same |
| Build-script root | Same | Passes | Same |
| Declared library root | Plain `#[path]` on inline module | Forbidden call rejected | Reject; clean control |
| Declared binary root | Same | Forbidden call rejected | Reject; clean control |
| Build-script root | Same | Forbidden call rejected | Reject; clean control |
| Declared library root | `cfg_attr` path on inline module | Passes | Follow conditional directory and reject |
| Declared binary root | Same | Passes | Same |
| Build-script root | Same | Passes | Same |
| Existing inferred library, binary-under-`tests/`, nested `a.rs`/`mod.rs`, out-of-line conditional branches, orphan-source, missing-source, and test-only fixtures | Existing discovery matrix | Passes its assertions | Retain these controls |

These are ordinary Rust declarations, not the documented exclusions for macro expansion or `include!`.

**Required change:** read path values as Rust string literals and propagate every conditional inline-module directory to its children. Preserve the requirement to scan target-specific branches and to fail on unreadable required modules. Add the target-by-attribute matrix above, retaining the safe fallback files so the regression tests prove omission rather than merely detecting a missing file. Correct the topic page and scanner documentation that currently promise every conditional path branch is followed.

### Medium — Manifest validation still accepts malformed inventory fields

**Defect class:** a configuration reader validates some inventory fields but coerces or incompletely validates sibling fields that determine target identity and exclusion.

In `darkmatter-cli`, [manifest_targets](../../../darkmatter/cli/tests/common/path_lookup_guard.rs:417) now correctly validates the three path/build fields from review #1. However, it reads `[[bin]].required-features` with `as_array().is_some_and(|features| !features.is_empty())`, without checking element types. A listed fixture with `required-features = [123]` or `["fixtures", 123]` is therefore accepted as feature-gated and excluded from scanning. With the forbidden call in `tests/fake.rs`, `engine::problems_with_fixtures` returns no problems. Wrong whole-field types become “no required-features,” rather than explicit malformed-field errors. Without a fixture listing, the forbidden call is reported, but the malformed field is still never diagnosed.

The sibling `[package].name` lookup uses `and_then(as_str)` and accepts absence, numbers, arrays, tables, and an empty string. `[[bin]].name` validates a present value and correctly permits absence when a path is supplied. Cargo will reject the malformed manifests before normal package tests run; this limits the runtime impact, but does not satisfy this reader's explicit robustness contract or its synthetic fixture validation. The fixture-exclusion decision must never accept a wrong-type element as a feature name.

The following TOML matrix combines the executed existing three-field matrix with additional one-edit fixtures for the newly inspected fields. “Field error” means a diagnostic naming the invalid field; “syntax error” means TOML rejection. For the first three columns, the selected source contains the forbidden call. For `required-features`, the bin is explicitly listed as a fixture and contains that same call. Name-only probes use safe sources; their string controls pass.

| Shape | `[lib].path` | `[[bin]].path` | `[package].build` | `[package].name` | `[[bin]].name` | `[[bin]].required-features` |
| --- | --- | --- | --- | --- | --- | --- |
| Valid control | Rejects call | Rejects call | Rejects call | Passes valid string | Passes valid string | Valid nonempty string list permits listed-fixture exclusion; unlisted fixture rejects call |
| Absent | Scans default library and rejects call | Reports missing inferred source in existing fixture | Scans default build script and rejects call | **Passes without required package name** | Passes with explicit path, as Cargo permits | Rejects listed fixture as ungated; unlisted bin remains scanned |
| Explicit `null` | Syntax error | Syntax error | Syntax error | Syntax error | Syntax error | Syntax error |
| Wrong whole-field type (`123`) | Field error | Field error | Field error | **Passes** | Field error | Rejects listing as ungated; **does not diagnose wrong type** |
| One wrong element | Field error | Field error | Field error | **Passes** | Field error | **Passes and excludes bin** |
| Every element wrong | Field error | Field error | Field error | **Passes** | Field error | **Passes and excludes bin** |
| Empty array | Field error | Field error | Field error | **Passes** | Field error | Rejects listed fixture as ungated; empty list is valid and means no gating |
| Empty string | Field error | Field error | Field error | **Passes** | Field error | Rejects listing as ungated; **does not diagnose wrong type** |
| Table instead of required type | Reader rejects nonstring path | Reader rejects nonstring path | Reader rejects unsupported type | **Passes** | Field error | Rejects listing as ungated; **does not diagnose wrong type** |
| Duplicate key | Syntax error | Syntax error | Syntax error | TOML parser rejects duplicate keys | TOML parser rejects duplicate keys | Syntax error |
| Trailing/invalid content | Syntax error | Syntax error | Syntax error | TOML parser rejects invalid document | TOML parser rejects invalid document | Syntax error |

For `required-features`, the sweep also ran every invalid/absent/empty cell without a fixture listing. Except for invalid TOML, the reader reported the forbidden call and no malformed-feature diagnostic. This is the clean coverage direction for unlisted targets, but not field validation. The existing library/binary table-shape readers and present binary-name validator use explicit type matches; the permissive sibling sites are the package-name lookup and feature-array lookup identified above. Build booleans and alternate TOML table/string spellings remain covered by the passing existing matrix.

**Required change:** validate `required-features` as an array of strings before making any exclusion decision, distinguish absent from invalid types, and validate the package name consumed for inference. Extend the single manifest matrix to these fields and both listed/unlisted target states. Invalid cells must assert a malformed-input diagnostic, independently of whether source happens to contain a forbidden call. No change to the agreed explicit fixture-listing policy is needed.

## Blocked Findings

None. All three findings have concrete reproductions and can be implemented without another policy decision.

## Recurrence

Every finding repeats a class in [review #1](review-1.md), so `recurrence: true` stops the automated review loop.

| Earlier finding | Sibling sites that its fix should have swept | This review's completed sweep |
| --- | --- | --- |
| Import resolution silently hides guarded calls and unresolved aliases | Ordinary block and closure scopes alongside function/module scopes; raw identifiers alongside ordinary aliases; the depth-limit fallback alongside short alias chains | Every canonicalization and home provider, both block orders, closure imports, raw function/module aliases, and short/deep chains; original forms retained as controls |
| Manifest interpretation can omit compiled targets and accept malformed fields | Every field deciding inventory and exclusion, especially the newly consumed `required-features` elements and the package-name lookup | Three original path/build fields plus both name fields and fixture-gating array, including listed/unlisted targets and all robustness shapes |
| Source discovery excludes production binaries and external modules | Rust literal decoding for both direct and conditional paths; inline-module conditional directories alongside out-of-line conditional paths | Library, binary, and build-script roots across eight attribute shapes, with actual compilation and guard results for each |

The previous implementation performed useful sweeps, but its log overstates two outcomes: block scopes are not represented by the resolver, and `required-features` is now read even though the manifest-sweep entry says it is not. The recurring findings concern those missed behaviors, not an unchanged policy requiring renewed approval.

## Shared consumers and test registration

All findings affect the single shared implementation in `darkmatter-cli`; there is no separate path-lookup scanner among these consumers.

| Package | Guard registration | Enabled rules | Current-source result |
| --- | --- | --- | --- |
| `biscuit-file` | [Library guard](../../lib/tests/l1/path_lookup_guard.rs) | Canonicalization | Passed |
| `biscuit-file-cli` | [CLI tests](../../cli/tests/cli_tests.rs) | Canonicalization | Passed |
| `claudine` | [Library guard](../../../claudine/lib/tests/l1/path_lookup_guard.rs) | Canonicalization and home lookup | Passed |
| `claudine-cli` | [CLI guard](../../../claudine/cli/tests/l1/path_lookup_guard.rs) | Canonicalization and home lookup | Passed |
| `darkmatter` | [Library guard](../../../darkmatter/lib/tests/l1/path_lookup_guard.rs) | Canonicalization | Passed |
| `darkmatter-cli` | [CLI guard and scanner fixtures](../../../darkmatter/cli/tests/l1/path_lookup_guard.rs) | Canonicalization; fixtures also exercise home lookup | Passed |

The new regression tests are declared by the existing `l1` target's `main.rs`, have no higher-tier name marker, and were selected by nextest. Cross-package consumers retain source-input declarations for both shared engine files. The added TOML dev-dependencies supply the parser in the including test binaries. Tier coverage reports no stranded tests. Passing current-source checks do not resolve the counterexamples above.

## Requirements and verification levels

| Requirement | Strongest relevant verification | Assessment |
| --- | --- | --- |
| Windows lexical equivalence, distinct identities, first-seen order and provenance in either order | Level 1 production Windows parser and shared ordering helper | Present; passing biscuit-file suite |
| Catalog root normalization distinct from document containment | Level 1 public catalog-result tests and native Windows variants | Present; applicable host tests pass |
| Symlink/junction escape rejection, including missing descendants | Level 1 filesystem resolver tests and native Windows junction tests | Present; applicable host tests pass |
| Canonical result absolute, usable for I/O, safely rendered, and missing-path error | Level 1 helper tests, including minimal-feature execution | Passed |
| Safe/unsafe Windows rendering, actual verbatim conversion and optional short-name alias | Level 1 parser/rendering tests and native Windows filesystem tests | Appropriate test level; native Windows execution belongs to CI evidence |
| Platform home precedence, relative-home rejection, config save/reload, explicit and captured homes | Level 1 context and hermetic child-process tests | Applicable host tests pass |
| Complete source inventory, imported calls/references, conservative uncertainty, malformed-input rejection | Level 1 source-guard fixtures | Incomplete: three findings above |

These requirements describe paths, filesystem behavior, and headless configuration. They add no terminal rendering or keyboard/mouse behavior requiring Level 2 or Level 3. There is no verification-level mismatch finding. Cross-OS evidence and human review are not used as reasons for `ready: false`.

Validation performed on macOS:

- `just test` in biscuit-file: **1,087 nextest tests passed**; the recipe's existing minimal-feature step also passed **9 path tests**.
- `just test path_lookup_guard` in darkmatter: **24 tests passed**, including the new provider, manifest, and discovery matrices.
- `just test path_lookup_guard` in claudine: **2 tests passed**.
- `just test home_lookup_round_trip` in claudine: **3 tests passed**.
- `just check-tier-coverage biscuit-file claudine darkmatter`: **zero stranded tests**.
- An external temporary Rust probe imported the unchanged shared engine and called its guard-result entry points for the instance tables. Offline Cargo checks confirmed the standard-library import counterexamples and all **24** module target/attribute fixtures compile. External-provider import shapes were scanner fixtures, not claims that dependency-free packages compile with tokio/dirs/home/dunce installed.

No production or test source was edited, no terminal/browser window was opened, and no commit was created. This review, the previous review's requested linkage/status fields, and the spec's review counter are the only review-authored repository edits. The spec remains incomplete.
