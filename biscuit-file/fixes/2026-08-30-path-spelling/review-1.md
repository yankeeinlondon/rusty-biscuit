---
$schema: feature-review.yaml
ready: false
findings:
  - title: Import resolution silently hides guarded calls and unresolved aliases
    priority: medium
  - title: Manifest interpretation can omit compiled targets and accept malformed fields
    priority: medium
  - title: Source discovery excludes production binaries and external modules
    priority: medium
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-03T13:32:42-07:00"
spec: "2026-08-30-path-spelling/spec.md"
implemented: true
next: "2026-08-30-path-spelling/review-2.md"
description: "A **fix** review of `2026-08-30-path-spelling/spec.md`"
fix: "2026-08-30-path-spelling/review-1.md"
---

The fix is **not production ready**. The path-identity, canonicalization-helper, and home round-trip tests passed, but the required source guard has three reproducible gaps. It can report success while missing a forbidden call, a compiled target, or a production module. These are implementation and regression-test defects, not requests for human approval or additional cross-OS evidence.

This review read the specification and implementation log, inspected the shared identity and ordering implementation, resolver containment, canonicalization replacements and exceptions, home lookup and consumers, documentation, manifests, test registration, and scanner tests. Only this review and the spec's review counter were changed. The earlier policy choices were retained; no new design decision requires human review.

## Findings

### Medium — Import resolution silently hides guarded calls and unresolved aliases

**Defect class:** the scanner loses import provenance or applies one scope's imports to another scope, allowing guarded operations and unresolved candidates to disappear without review.

In the `darkmatter-cli` package, [imports and scan_target](../../../darkmatter/cli/tests/common/path_lookup_guard.rs:320) identify forbidden canonicalization and home reads for the shared guard. `imports` records module aliases, but resolving an imported function only checks the original module names. An alias imported through a module alias therefore falls into the branch that discards unknown aliases. A file-wide `bare_helper` flag also suppresses unrelated unresolved home calls in other modules.

For example, this production function returns a raw canonical path, yet the engine's `scan` finds zero sites and `check` returns zero problems:

```rust
use std::fs as filesystem;
use filesystem::canonicalize as canon;

pub fn escaped(path: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
    canon(path)
}
```

The same failure occurs with `use std::env as environment; use environment::home_dir as home;` followed by `home()`. These are same-file imports, not the documented limitation concerning cross-module re-exports or macro expansion.

The sweep invoked the actual shared engine with a source fixture for each row, testing both the chained call and a function reference (`let _ = invoke`). Qualified references and directly imported aliases were controls.

| Site/provider | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| `std::fs::canonicalize` | Qualified/direct alias; chained alias call/reference | Controls: one site; chained forms: zero sites | Every form reported |
| `tokio::fs::canonicalize` | Same four forms | Controls: one site; chained forms: zero sites | Every form reported |
| `dunce::canonicalize` | Same four forms | Controls: one site; chained forms: zero sites | Every form reported |
| `std::path::Path::canonicalize` | Same four forms | Controls: one site; chained forms: zero sites | Every form reported |
| `std::path::PathBuf::canonicalize` | Same four forms | Controls: one site; chained forms: zero sites | Every form reported |
| `std::env::home_dir` | Same four forms | Controls: one site; chained forms: zero sites | Every form reported |
| `dirs::home_dir` | Same four forms | Controls: one site; chained forms: zero sites | Every form reported |
| `home::home_dir` | Same four forms | Controls: one site; chained forms: zero sites | Every form reported |
| Unknown imported `canonicalize` / `home_dir` | `use other::… as invoke; invoke(...)` | Zero sites for both rules | Unresolved candidate requiring explicit review |
| Home lookup in separate inline modules | One imports `biscuit_file::home_dir`; another imports `unknown::home_dir` and calls it | Zero sites | Helper allowed; unrelated call reported as unresolved |
| Existing method, helper, and lookalike controls | Zero-argument path method; approved helper; style normalizer; comments/strings; test-only code | Existing scanner tests passed | Retain these distinctions |

**Required change:** retain and resolve import aliases within their scopes, or conservatively report the unresolved use for review. Do not discard an alias merely because its source cannot be attributed. Add these cases to the declared Level 1 scanner target, including call and reference forms and imports in separate modules. The existing direct-alias tests do not exercise this failure.

### Medium — Manifest interpretation can omit compiled targets and accept malformed fields

**Defect class:** a line-based configuration reader treats unsupported valid syntax and invalid field types as missing configuration, producing an incomplete source inventory.

In `darkmatter-cli`, [declared_target_roots, build_script, and string_value](../../../darkmatter/cli/tests/common/path_lookup_guard.rs:255) decide which files the guard examines. They recognize exact table headers and double-quoted strings only. Cargo accepts single-quoted TOML paths, whitespace in table headers, and an inline `lib` table; the guard misses the resulting production source.

The sweep used a Cargo-generated library manifest, a safe `src/lib.rs`, a safe default `build.rs`, and `production/main.rs` containing an unapproved `std::fs::canonicalize` call. Each matrix cell changed one field. Calling the actual engine's `problems` tested its public guard result, rather than a parser return. `cargo metadata --no-deps --offline` independently confirmed that the valid alternate spellings select the omitted targets.

| Reader/site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Library target reader | `[lib]` with double-quoted `path` | Forbidden call rejected | Reject the call; control passed |
| Binary target reader | `[[bin]]` with double-quoted `path` | Forbidden call rejected | Reject the call; control passed |
| Build-script reader | Double-quoted `package.build` | Forbidden call rejected | Reject the call; control passed |
| All three readers | Single-quoted path to the same file | Guard passed | Scan the file and reject the call |
| Library target reader | `[ lib ]` or top-level `lib = { path = "production/main.rs" }` | Guard passed | Scan the Cargo-declared file and reject the call |

The new reader's load-bearing fields are `[lib].path`, `[[bin]].path`, and `[package].build`. TOML is its only format. The complete robustness sweep below records each requested shape; “passes” means the guard returned no problems.

| Shape | `[lib].path` | `[[bin]].path` | `[package].build` | Expected contract |
| --- | --- | --- | --- | --- |
| Control: double-quoted file containing forbidden call | Rejects call | Rejects call | Rejects call | Discover and reject the call |
| Absent | Passes with safe default library | Passes despite no inferred binary source | Passes with safe default build script | Follow Cargo defaults; report a missing required source |
| Explicit null (`null`) | Passes | Passes | Passes | TOML has no null value: reject invalid document |
| Wrong whole-field type (`123`) | Passes | Passes | Passes | Reject invalid field type |
| One wrong element (`["production/main.rs", 123]`) | Passes | Passes | Passes | Reject invalid field type |
| Every element wrong (`[123]`) | Passes | Passes | Passes | Reject invalid field type |
| Empty collection (`[]`) | Passes | Passes | Passes | Reject invalid field type |
| Empty string (`""`) | Reports missing root | Reports missing root | Reports missing build script | Explicit failure; do not treat as absent |
| Duplicate key | Rejects forbidden call from first spelling | Same | Same | Reject duplicate TOML key, independently of source contents |
| Valid document plus garbage | Rejects forbidden call | Same | Same | Reject malformed document, independently of source contents |

The last two rows find the retained forbidden call; that is not evidence of syntax validation. The functions never parse or validate TOML, and their diagnostics concern canonicalization rather than malformed configuration. The valid single-quoted control demonstrates a practical coverage failure even when Cargo has already validated the manifest.

**Required change:** use a TOML reader for target paths and the build-script setting, preserve Cargo's defaults and valid boolean build settings, and fail explicitly on malformed inputs or undiscoverable required roots. Add one Level 1 manifest matrix covering all three fields through the guard result. The current manifest test covers only the double-quoted control.

### Medium — Source discovery excludes production binaries and external modules

**Defect class:** source discovery infers test-only status from a directory name and stops at directory boundaries instead of following compiled production modules.

In `darkmatter-cli`, [package_sources](../../../darkmatter/cli/tests/common/path_lookup_guard.rs:199) unconditionally skips every manifest-declared root beginning with `tests/`. Cargo permits an ordinary shipped `[[bin]]` there. Separately, [production_sources](../../../darkmatter/cli/tests/common/source_scan.rs:178) walks a directory and excludes test modules, but does not follow production `#[path]` modules outside it. Either case leaves compiled Rust unexamined while other files keep the scan nonempty.

The sweep used the same safe default library and forbidden-call file as the manifest sweep. For the external-module case, the declared library root was conventional and used `#[path = "../production/main.rs"] mod external;`. An offline `cargo check` also compiled that fixture successfully. Thus the omitted file was compiled production code, not an orphan Rust file.

| Discovery site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Manifest-declared binary | `path = "production/main.rs"` | Call rejected | Scan compiled binary; clean control |
| Manifest-declared binary | Same call in `tests/bin/app.rs`, no test-only feature requirement | Guard passed; Cargo metadata lists the binary | Scan compiled binary and reject call |
| Library module discovery | Production `#[path]` module outside `src/` | Guard passed | Follow the production module and reject call |
| Directory discovery | Ordinary production files inside scanned directories | Existing guards reject unlisted calls | Retain coverage |
| Test exclusion | Inline and separate `#[cfg(test)]` modules | Existing scanner tests passed | Exclude genuinely test-only code |
| Required-root validation | Missing double-quoted declared root or empty source inventory | Existing scanner tests passed | Explicit failure |

**Required change:** distinguish explicitly identified fixture targets from production binaries; `tests/` alone is insufficient. Follow production module declarations, including `#[path]` destinations outside a root's directory, and fail on unreadable required production modules. Keep the documented macro-expansion and `include!` limitations separate. Add executable Level 1 discovery fixtures for both missed cases and retain the test-only controls.

## Shared guard consumers

All three findings affect one shared implementation. Its complete consumer inventory is:

| Package | Guard registration | Rules enabled | Current-source verification |
| --- | --- | --- | --- |
| `biscuit-file` | [Library guard](../../lib/tests/l1/path_lookup_guard.rs) | Canonicalization | Passed in full area run |
| `biscuit-file-cli` | [CLI tests](../../cli/tests/cli_tests.rs) | Canonicalization | Passed in full area run |
| `claudine` | [Library guard](../../../claudine/lib/tests/l1/path_lookup_guard.rs) | Canonicalization and home lookup | Passed targeted run |
| `claudine-cli` | [CLI guard](../../../claudine/cli/tests/l1/path_lookup_guard.rs) | Canonicalization and home lookup | Passed targeted run |
| `darkmatter` | [Library guard](../../../darkmatter/lib/tests/l1/path_lookup_guard.rs) | Canonicalization | Passed targeted run |
| `darkmatter-cli` | [CLI guard and scanner fixtures](../../../darkmatter/cli/tests/l1/path_lookup_guard.rs) | Canonicalization; fixtures also exercise home lookup | Passed targeted run |

Passing current-source checks establish that today's scanned calls match the exceptions. The counterexamples establish that these checks cannot yet enforce the intended inventory. No additional independent scanner implementation exists among these consumers.

## Requirements and verification levels

| User-facing or prevention requirement | Strongest relevant test present | Review result |
| --- | --- | --- |
| Windows lexical equality and inequality; first-seen candidate order and provenance | Level 1 production Windows parser and shared ordering helper, both input orders | Passed on this host; no second normalizer used |
| Root validation distinct from document containment | Level 1 catalog public-result tests, plus native Windows variants | Host tests passed; Windows-specific tests remain CI evidence |
| Symlink/junction escapes, including missing descendants | Level 1 filesystem resolver tests; native Windows junction tests | Existing containment checks preserved |
| Ordinary canonical path is absolute, usable for I/O, portable, and errors for missing paths | Level 1 helper tests, including minimal-feature run | Passed |
| Safe/unsafe Windows rendering, verbatim canonicalization, actual short-name aliases | Level 1 rendering tests and native Windows filesystem tests | Appropriate level; native execution not claimed here |
| Home variable precedence, relative-home rejection, config save and reload | Level 1 child-process `bf` and Claudine CLI tests | Passed |
| Explicit injected/cleared homes and request-scoped home use | Level 1 context and captured-home tests | Existing coverage inspected; no new failure found |
| Complete source guard with aliases, unknown candidates, roots, and exclusions | Level 1 scanner and package fixtures | Incomplete: findings above |

These contracts concern path results and headless filesystem/configuration behavior. They introduce no terminal rendering, keyboard, mouse, or focus contract requiring Level 2 or Level 3. Cross-OS result collection does not determine readiness in this review. The implementation log records missing native Windows execution; this review does not turn that evidence gap into a finding.

Validation performed on macOS:

- `just test` in `biscuit-file`: **1,087 nextest tests passed**, plus **9 minimal-feature path tests passed** through the area's existing recipe.
- `just test path_lookup_guard` in `darkmatter`: **18 tests passed**.
- `just test home_lookup_round_trip` in `claudine`: **3 tests passed**.
- `just test path_lookup_guard` in `claudine`: **2 tests passed**.
- `just check-tier-coverage` for `biscuit-file`, `claudine`, and `darkmatter`: **zero stranded tests** in each area. Relevant consolidated modules and cross-package source-input declarations were inspected.
- A standalone Rust probe imported the unchanged shared guard engine and exercised the tables above; Cargo metadata confirmed the valid alternate manifests. Fixtures and probe files were under `/tmp`, outside the repository. No production code was edited, no terminal/browser window was opened, and no lint or broad unrelated suite was required for these review-document changes.

The spec's review counter is now 1. Completion remains pending the three guard fixes and their regression tests.
