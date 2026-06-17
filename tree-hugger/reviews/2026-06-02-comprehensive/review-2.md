---
created: "2026-06-02T21:50:18Z"
agent: "codex"
model: ""
ready: true
---

### 1. Executive Summary

The follow-up implementation addresses the major findings from the first review: the CLI now reaches the fallible diagnostics path through `symbol_index_v2()`, `TreeFile::dead_code()` returns real blocks, TSX grammar selection is preserved for forced TypeScript parsing, `TreePackage` is explicitly polyglot by default, and Cargo/Node package manifest checks now use structured parsing via `biscuit-file`.

Overall risk level is now `low-to-medium`. Tests and clippy are green. I found one user-visible regression in the new scanner path: bare extensionless file names such as `myscript` are still classified as symbol filters before the forced-language explicit-file resolver can see them. I also found a related exclude handling gap for explicit files.

### 2. Findings

#### [Severity: Medium] Bare extensionless files still fail with `--language`

- **Location:** `tree-hugger/cli/src/scanner.rs:25-43`, `tree-hugger/cli/src/scanner.rs:173-180`
- **Why it matters:** The original review called out forced-language parsing for extensionless scripts. The fix works for paths containing a slash, such as `./myscript` or `/tmp/myscript`, but the natural command from the file's directory still fails: `hug symbols myscript --language rust`.
- **Evidence:** `classify_filters()` sends symbol commands through `is_file_filter_token()`. A token with no slash and no recognized extension is treated as a symbol glob, so `myscript` never reaches `resolve_explicit_file()`. I confirmed this command returns `NoSourceFiles`, while `./myscript` succeeds.
- **Recommendation:** When `--language` is present, treat an existing positional token as a file filter before falling back to symbol-glob classification. The check can reuse `resolve_explicit_file()` or a root-relative equivalent.
- **Confidence:** high

#### [Severity: Low] Explicit forced-language files bypass `--exclude-files`

- **Location:** `tree-hugger/cli/src/scanner.rs:173-180`, `tree-hugger/cli/src/scanner.rs:186-192`
- **Why it matters:** Files resolved by the explicit-file fast path are pushed directly into `files` before the ignore override builder is applied. This changes previous behavior for explicit inputs and makes `--exclude-files` inconsistent between `./myscript` and glob/directory scans.
- **Evidence:** `hug symbols ./myscript --language rust --exclude-files './myscript' --json` still analyzes `./myscript`. The exclusion patterns are only added to the walker overrides, not applied to the pre-resolved explicit files.
- **Recommendation:** Apply the same exclude glob logic to explicit files before sorting/deduping, or document that `--exclude-files` only applies to directory/glob scans.
- **Confidence:** medium

### 3. Resolved Items

- Fallible diagnostics are wired into the v2 analysis path via `tree_file.try_diagnostics()?`, so CLI analyzer/query failures no longer silently become clean output.
- `TreeFile::dead_code()` now reuses the dead-code range traversal and returns ordered `CodeBlock` values with snippets.
- Forced TypeScript parsing of `.tsx` files now stores the concrete grammar and compiles queries against the TSX grammar variant.
- `TreePackage` now collects all supported source files without an override and exposes `language_of()` / `languages()` for per-file language data.
- Cargo and Node package-root detection now uses structured TOML/JSON parsing instead of substring checks.

### 4. Verification

- `cargo test -p tree-hugger --color=never` passed: 50 unit tests, 56 lint diagnostic tests, 33 query compile tests, 89 tree file tests, 7 tree package tests, and doctests with one ignored doctest.
- `cargo test -p tree-hugger-cli --color=never` passed: 66 CLI tests.
- `cargo clippy -p tree-hugger -p tree-hugger-cli --all-targets --color=never -- -D warnings` passed.
- Manual regression checks:
  - `hug symbols myscript --language rust --json` from the file's directory returns `NoSourceFiles`.
  - `hug symbols ./myscript --language rust --json` succeeds.
  - `hug symbols ./myscript --language rust --exclude-files './myscript' --json` still analyzes the file.
