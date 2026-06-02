---
created: "2026-06-02T13:10:35"
agent: "codex"
model: ""
yolo: ""
duration: "PT10M"
ready: false
---

### 1. Executive Summary

The `tree-hugger` area is a focused Rust library plus CLI for Tree-sitter based symbol extraction, imports, diagnostics, and schema-v2 indexing across many languages. Overall risk level: `medium`. The biggest strengths are broad fixture coverage, a clear CLI surface, green package tests, green clippy, cached query compilation, and a mostly clean boundary between CLI rendering and library extraction. The most important concern is that some library diagnostic paths intentionally swallow query/extraction failures and return partial or empty results, which can make a broken analyzer look clean. A second concern is API drift: `TreeFile::dead_code()` still returns an empty list while lint diagnostics already implement dead-code detection. The implementation is also concentrated in very large modules, especially `tree_file.rs` and `cli/src/main.rs`, which makes language-specific regressions harder to isolate. There is no unsafe code in the reviewed source. The area appears usable and actively tested, but I would classify it as pre-production rather than production-ready until silent failure paths and stale public APIs are tightened.

### 2. Key Findings

#### [Severity: High] Lint diagnostics silently disappear when query or extraction steps fail

- **Location:** `tree-hugger/lib/src/file/tree_file.rs::run_pattern_diagnostics`, `run_semantic_diagnostics`
- **Why it matters:** A static-analysis tool should fail loudly or report an internal diagnostic when its analyzer cannot run. Returning an empty diagnostic list on query compilation or extraction failure makes the CLI and library report a clean file when the analyzer may actually be broken.
- **Evidence:** `run_pattern_diagnostics` catches any `query_for(self.language, QueryKind::Lint)` error and returns `Vec::new()` at lines 954-957. `run_semantic_diagnostics` does the same for `symbols`, `imported_symbols`, `exported_symbols`, and `referenced_symbols` at lines 1006-1023.
- **Recommendation:** Add a fallible API such as `try_lint_diagnostics() -> Result<Vec<LintDiagnostic>, TreeHuggerError>` and have the CLI use it. Keep the current infallible method only as a compatibility wrapper if needed, but do not let the CLI silently hide analyzer failures.
- **Confidence:** high

#### [Severity: Medium] `TreeFile::dead_code()` is a stale public API that always returns empty results

- **Location:** `tree-hugger/lib/src/file/tree_file.rs::dead_code`, `check_dead_code`
- **Why it matters:** Consumers calling the documented public dead-code API get no results even though the same file already implements dead-code detection for lint diagnostics. This is observable incorrect behavior and creates two inconsistent ways to ask the same question.
- **Evidence:** `dead_code()` unconditionally returns `Vec::new()` at lines 1488-1493. The actual dead-code implementation exists separately in `check_dead_code()` at lines 1066-1138 and is only surfaced through lint diagnostics.
- **Recommendation:** Either implement `dead_code()` by reusing the existing terminal-statement traversal and returning `CodeBlock` values, or remove/deprecate the method until the API can be made truthful. Update rustdoc in the same change.
- **Confidence:** high

#### [Severity: Medium] `--language` is treated as both parser override and extension filter

- **Location:** `tree-hugger/cli/src/main.rs::collect_files`, `TreeFile::with_language`
- **Why it matters:** The CLI help says `--language` forces a language, but the scanner still rejects files whose extension does not map to that same language. This prevents forced-language parsing for extensionless scripts, unconventional file names, and ambiguous extensions.
- **Evidence:** When `language` is `Some(lang)`, `collect_files` skips any file where `ProgrammingLanguage::from_path(entry.path()) != Some(lang)` at lines 1527-1529. Later, matching files are parsed with `TreeFile::with_language(&file, language)` at lines 777-779 and 850-852.
- **Recommendation:** Separate scan filtering from parse-language override. If explicit file inputs are provided with `--language`, include those files even when extension detection fails or maps differently. For directory scans, consider using `lang.extensions()` as the default candidate set but document that behavior.
- **Confidence:** high

#### [Severity: Medium] Forced TypeScript parsing loses TSX grammar selection

- **Location:** `tree-hugger/lib/src/shared/symbol.rs`, `tree-hugger/lib/src/file/tree_file.rs::with_language`
- **Why it matters:** TSX requires `tree_sitter_typescript::LANGUAGE_TSX`, but a `Some(ProgrammingLanguage::TypeScript)` override always uses the plain TypeScript grammar. A user can trigger this through `hug --language typescript ...tsx`, producing syntax errors or degraded symbol extraction for valid TSX.
- **Evidence:** Extension-aware detection selects TSX grammar for `tsx` at `symbol.rs` lines 154-163. The override path in `TreeFile::with_language` bypasses that and uses `language.tree_sitter_language()` at `tree_file.rs` lines 59-63.
- **Recommendation:** Preserve extension-specific grammar selection even with a forced language. For `ProgrammingLanguage::TypeScript`, call `tree_sitter_language_for_extension` when the path extension is TSX/MTS/CTS/TS, falling back to `language.tree_sitter_language()` only when no better grammar exists.
- **Confidence:** high

#### [Severity: Medium] `TreePackage` models a package as single-language and drops supported files in mixed-language packages

- **Location:** `tree-hugger/lib/src/package/tree_package.rs::with_config`, `detect_primary_language`, `collect_files`
- **Why it matters:** The rest of the project presents tree-hugger as multi-language tooling, and the CLI scans all supported languages by default. The package API instead chooses one primary language and then collects only that language's extensions, which is surprising for polyglot repositories and can hide files without warning.
- **Evidence:** `with_config` chooses one `language` at lines 61-64, then calls `collect_files(&root_dir, language.extensions(), ...)` at line 66. `detect_primary_language` counts all supported files but returns only the max-count language at lines 193-218.
- **Recommendation:** Either rename/document `TreePackage` as a primary-language package abstraction, or change it to store per-file languages and collect all supported source files when no language override is provided. Match the CLI's all-supported-language default unless there is a strong API reason not to.
- **Confidence:** medium

#### [Severity: Low] Package-root detection uses string containment instead of structured manifest parsing

- **Location:** `tree-hugger/lib/src/package/tree_package.rs::is_cargo_package`, `is_node_package`; duplicated in `tree-hugger/cli/src/main.rs::has_package_manifest`
- **Why it matters:** `contents.contains("[package]")` and `!contents.contains("\"workspaces\"")` are brittle. Comments, nested metadata, formatting, or unrelated strings can change package-root selection and scan scope.
- **Evidence:** The library checks Cargo and Node manifests with substring matches at `tree_package.rs` lines 179-190. The CLI repeats equivalent logic at `main.rs` lines 1189-1205.
- **Recommendation:** Use TOML and JSON parsers for manifest inspection, and share the package-manifest helper between library and CLI instead of duplicating it.
- **Confidence:** high

### 3. Rust-Idiomaticity Notes

- `TreeFile` is doing too much: parsing, imports, references, linting, dead-code detection, doc extraction, v2 schema conversion, and many language-specific AST helpers are all in one 5.8k-line module. Split by concern or language family when touching this code next.
- `cli/src/main.rs` has a similar concentration of parsing, scanning, prelude resolution, rendering, and completion behavior. Extracting scanner and prelude modules would make CLI regressions easier to test.
- The `expect("ordinal entry must exist")` in `symbol_records()` is logically justified, but `entry(...).and_modify(...)` or holding the entry reference would avoid a needless invariant assertion in normal library code.
- Several comments in the extraction code narrate obvious AST-walking mechanics. Keep comments that describe grammar-specific surprises, but trim routine "find ancestor" comments during behavior-touching edits.

### 4. Testing Gaps

- Add a regression test for `hug --language rust symbols path/to/extensionless_file` and for forced-language parsing of a file whose extension does not map to the forced language.
- Add a regression test for `hug --language typescript symbols fixture.tsx` containing JSX syntax so the TSX grammar path cannot regress.
- Add a direct test for `TreeFile::dead_code()` once it is implemented or deprecated.
- Add a test that injects a failing lint/reference query path, if feasible, and asserts the fallible diagnostics API returns an error instead of an empty list.
- Add mixed-language `TreePackage` tests that clarify whether the intended behavior is primary-language-only or all-supported-language collection.

### 5. Unsafe Code Review

No unsafe usage was found in the reviewed `tree-hugger/lib/src` or `tree-hugger/cli/src` Rust source. The review searched for `unsafe` along with panic and synchronization markers; the only source hit was builtin symbol data, not an unsafe block. There are no unsafe invariants to document or minimize in the current area.

### 6. Prioritized Next Steps

1. Add a fallible diagnostics API and make the CLI surface analyzer/query failures instead of silently returning clean output.
2. Fix or deprecate `TreeFile::dead_code()` so the public API matches implemented behavior.
3. Split forced-language parsing from extension-based scan filtering, including TSX grammar preservation.
4. Decide whether `TreePackage` is intentionally primary-language-only; then encode that decision in API names, docs, and tests.
5. Replace manifest substring checks with structured TOML/JSON parsing and share the helper between library and CLI.
6. Start carving `tree_file.rs` and `cli/src/main.rs` into smaller modules around existing responsibilities, without changing behavior.

### Verification

Reviewed source and tests in `tree-hugger/lib` and `tree-hugger/cli`.

- `cargo test -p tree-hugger --color=never` passed: 227 tests plus doctests, with one ignored doctest.
- `cargo test -p tree-hugger-cli --color=never` passed: 64 CLI tests.
- `cargo clippy -p tree-hugger -p tree-hugger-cli --all-targets --color=never -- -D warnings` passed.
