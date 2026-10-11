---
kind: fix
name: docs-audit-code-defects
area: tree-hugger
status: draft-spec
date: 2026-10-10
origin: the 2026-10-09 restructure of `tree-hugger/docs/`, which checked every page against the code and found these six defects in the code itself
packages:
  - tree-hugger
  - tree-hugger-cli
needs_rulings: true
---

# Six code defects found while auditing the Tree Hugger docs

## Summary

Rewriting the Tree Hugger documentation meant checking each documented
claim against the code. Six places where the code is inconsistent with
itself, or with what its own API promises, turned up. Each was confirmed
on 2026-10-09 against the working tree. The docs currently describe the
code as it is, defects included; each fix below must update them.

| ID | Defect | Package | Ruling needed |
|----|--------|---------|---------------|
| F1 | `undefined-symbol` is an `Error` in the library but a `Warning` in the rule registry | tree-hugger | yes |
| F2 | Three registered rules are never emitted, and two rules' language lists disagree with the queries | tree-hugger | yes |
| F3 | `TreeFile::reexported_symbols()` always returns an empty list | tree-hugger | yes |
| F4 | Tree Hugger's own queries are labeled `MIT`; the repository is `AGPL-3.0-only` | tree-hugger | no |
| F5 | Query drift checking defines five kinds of drift but only ever detects one | tree-hugger | yes |
| F6 | A command-level error is printed as a Rust `Debug` dump (`Error: NoSourceFiles { … }`) | tree-hugger-cli | no |

The findings are independent. They can be fixed in any order, in one
change or several.

## Background, for a reader new to this area

- **Diagnostics and the rule registry.** `hug lint` and the library report
  *diagnostics*. Each has a rule ID such as `unwrap-call`. Pattern rules come
  from per-language `lint.scm` Tree-sitter queries (a capture named
  `@diagnostic.<rule-id>`); semantic rules (`undefined-symbol`,
  `unused-import`, …) are emitted by Rust code in `lib/src/file/tree_file.rs`.
  `RuleRegistry::builtin()` (`lib/src/rule_registry.rs`) holds every rule's
  metadata: title, category, default severity, languages, and whether it is
  on by default. The CLI applies that metadata as policy (`--warn`, `--deny`,
  `--allow`, `--strict`); library callers apply it themselves (see
  [Diagnostics](../../docs/library/diagnostics.md)).
- **Query provenance.** Every query file has a `QueryProvenance` record
  (`lib/src/queries/provenance.rs`): where it came from, at which upstream
  revision, under which license, and how Tree Hugger changed it. Queries
  vendored from nvim-treesitter are Apache-2.0; queries Tree Hugger writes
  itself carry the repository's license. See
  [Tree-sitter Queries](../../docs/queries.md).
- **Drift.** `check_query_drift` / `check_all_drift`
  (`lib/src/queries/drift.rs`) compare a vendored query with upstream so a
  maintainer knows when to re-vendor.

## F1. `undefined-symbol` has two different default severities

### Evidence

- The registry says **Warning**, off by default, experimental
  (`lib/src/rule_registry.rs:448-456`):
  `default_severity: DiagnosticSeverity::Warning`, `enabled_by_default: false`,
  `requires_experimental_semantics: true`.
- The library stamps diagnostics with **Error**: `tree_file.rs:1429` uses
  `severity_for_rule("undefined-symbol")`, and `severity_for_rule`
  (`lib/src/queries/mod.rs:366-369`) maps `undefined-symbol` (and
  `undefined-variable`, `unreachable-code`, `invalid-syntax`) to `Error`.

So a library caller with `experimental_semantics = true` receives
`undefined-symbol` as `Error`, while `hug lint --experimental-semantics`,
which applies registry policy, reports it as a warning. The same file can
therefore pass `hug lint` and fail a library-based check.

The root cause is two severity tables: `severity_for_rule` and the
registry's `default_severity`. `severity_for_rule` also names rules the
registry does not have (`undefined-variable`, `unreachable-code`).

### Ruling needed

What should `undefined-symbol` default to?

- **Warning, from the registry (recommended).** The rule is low-confidence
  and experimental; an error-by-default experimental rule would fail builds
  on false positives. The registry already says this and the CLI already
  behaves this way.
- **Error.** Change the registry to match the library. This changes
  `hug lint --experimental-semantics` exit status.

### Fix

Make the registry the single source of default severity: the library stamps
each diagnostic with its rule's `RuleRegistry` default, and
`severity_for_rule` is removed (or reduced to a fallback for unregistered
rule IDs, if any remain after F2). Remove the entries for rule IDs that
nothing registers or emits.

### Acceptance

- For every registered rule, the severity on a diagnostic returned by the
  library equals the registry's `default_severity`. Add an L1 test that
  iterates `RuleRegistry::builtin()` rather than listing rules by hand.
- `hug lint --experimental-semantics` and the library agree on the severity
  of `undefined-symbol` for the same file.

## F2. Registered rules that never fire, and language lists that disagree

### Evidence

Compare the registry with what is actually emitted:

- **Registered, never emitted.** `unused-variable`, `shadowed-variable` and
  `deprecated-syntax` are registered, but no `lint.scm` captures them and no
  Rust code emits them. The only other mentions are their titles and
  severities in `queries/mod.rs:372-408`. `hug lint --list-rules` therefore
  lists three rules that can never fire, and `--warn unused-variable` is
  silently a no-op.
- **`eval-call`.** The registry lists JavaScript, TypeScript and PHP
  (`rule_registry.rs:295-299`), but `python/lint.scm` also captures
  `@diagnostic.eval-call`. Python `eval()` is reported under a rule whose
  metadata says it does not apply to Python.
- **`exec-call`.** The registry lists Python and PHP
  (`rule_registry.rs:320`), but only `python/lint.scm` captures it. PHP
  `exec()` is never reported.

Every other pattern rule's registry language list matches the query files
exactly (checked for `dbg-macro`, `debugger-statement`, `breakpoint-call`,
`console-log`, `print-call`, `fmt-println`, `unwrap-call`, `expect-call`).

### Ruling needed

For the three never-emitted rules:

- **Remove them from the registry (recommended).** No user can depend on a
  rule that never fires. Re-add each one when it is implemented.
- **Keep them, marked as not implemented** in their metadata and in
  `--list-rules` output.
- **Implement them.** Larger: each needs a design; not a fix.

For `exec-call` and PHP: add a PHP `exec()` pattern, or drop PHP from the
registry entry (recommended: drop it, since PHP `exec` runs a shell command
and is a different hazard from Python's `exec`).

### Fix

- Apply the rulings above.
- Add Python to `eval-call`'s registry languages.
- Add an L1 consistency test, so this cannot recur: for every language,
  the set of `@diagnostic.<id>` captures in its `lint.scm` equals the set of
  pattern rules whose registry `languages` include it, and every registered
  rule is either emitted by a query or in an explicit list of rules emitted
  by Rust code.

### Acceptance

- The consistency test passes, and fails if a rule is added to a query or the
  registry without the other.
- `hug lint --list-rules` lists only rules that can fire.

## F3. `TreeFile::reexported_symbols()` always returns nothing

### Evidence

```rust
// lib/src/file/tree_file.rs:1093
pub fn reexported_symbols(&self) -> Result<Vec<ImportSymbol>, TreeHuggerError> {
    Ok(Vec::new())
}
```

The method is public and documented as providing re-exports, but it is a
stub. Re-exports are in fact detected: `ImportSymbol::is_reexport` is set by
`import_is_reexport` (`tree_file.rs:244`, `:581`) for a Rust `pub use`, and
used at `tree_file.rs:1509`. A caller who uses the obvious method gets an
empty list and no error.

### Ruling needed

- **Implement it (recommended):** return the file's imports with
  `is_reexport == true`. This is a one-line filter over
  `imported_symbols()`, matches the method's documentation, and costs no new
  analysis.
- **Remove it:** callers filter `imported_symbols()` themselves. The repo has
  no external users, so removal needs no deprecation.

Either way, document which languages produce re-exports (today only Rust
`pub use`).

### Acceptance

- An L1 test: a Rust file with `pub use a::B;` and `use c::D;` returns
  exactly `B` from `reexported_symbols()` (if kept).

## F4. Tree Hugger's own queries are labeled MIT

### Evidence

- `overlay_query_provenance` (`lib/src/queries/provenance.rs:217`) hard-codes
  `license: "MIT"` for every query Tree Hugger writes itself (`lint.scm`,
  `references.scm`, `comments.scm`, and overlay `locals.scm`).
- The same file defines `REPOSITORY_LICENSE = "AGPL-3.0-only"` (line 115),
  documented as "the license of queries Tree Hugger writes itself, which is
  the repository's license", and uses it for Perl's original `locals.scm`
  (line 82). The repository's `LICENSE` file is the GNU AGPL v3.

So two code paths disagree about the license of the same category of file,
and the `MIT` value contradicts the repository license.

### Fix

Use `REPOSITORY_LICENSE` at line 217. No ruling is needed: the constant's
own documentation states the intent. Extend the existing provenance test
(which asserts `REPOSITORY_LICENSE` for Perl at line 294) to every
`TranslationStatus::Original` record.

### Acceptance

- Every provenance record with `TranslationStatus::Original` carries
  `AGPL-3.0-only`; every vendored record carries its upstream license.

## F5. Drift checking only compares revision strings

### Evidence

`DriftItem` (`lib/src/queries/drift.rs:24-38`) defines five kinds:
`NewerUpstreamRevision`, `ContentMismatch`, `UpstreamPathMissing`,
`CaptureDivergence` and `PredicateAdded`. `check_query_drift` (lines 80-110)
only ever produces `NewerUpstreamRevision`, by comparing the recorded
revision string with the one the caller passes in. The other four variants
appear only in their own `Display` implementation; nothing constructs them.
The library has no access to upstream query text, so it could not detect
them as written.

The public API therefore advertises checks it never performs, and a report
saying "no drift" does not mean the vendored content matches upstream.

### Ruling needed

- **Remove the four unproduced variants (recommended)** and document drift as
  "recorded revision versus a revision you supply". Content comparison, if
  wanted later, belongs in a maintainer tool that fetches upstream, not in
  the library.
- **Implement content and capture checks** against an upstream checkout path
  supplied by the caller. Larger; this is a feature, not a fix.

### Acceptance

- Every `DriftItem` variant is produced by some code path, and has a test
  that produces it.
- [Tree-sitter Queries](../../docs/queries.md#drift) describes exactly what
  is checked.

## F6. Errors that end a command are printed as a Rust `Debug` dump

### Evidence

`main` in `cli/src/main.rs:887` is `fn main() -> Result<(), TreeHuggerError>`.
When an error escapes it, the Rust runtime prints `Error: {error:?}` (the
`Debug` form) and exits `1`. Reproduced from an empty scratch Git repository:

```text
$ hug symbols 'missing/**'
Error: NoSourceFiles { path: "/private/tmp/…/refsamb" }

$ hug god-files nope-dir
Error: Io { path: "nope-dir", source: Custom { kind: Other, error: "god-files scan directory does not exist" } }
```

The same happens for `refs`, `lint` and `imports`. Everywhere else the CLI
prints `error: <message>` using the error's `Display` text (for example
`main.rs:1017`, `:1167`, `:1192`, and the `refs` ambiguity message at
`cli/src/refs.rs:400`). `TreeHuggerError` already has a readable `Display`
(`lib/src/error.rs`, via `thiserror`: "No supported source files found in
`{path}`").

This is very likely what one docs agent saw as an occasional `Error:` prefix
from `hug refs`: the ambiguity message itself is always `error:`, but a run
whose filters matched no files fails through this path.

### Fix

Handle the error in `main` instead of returning it: print
`error: {error}` (Display) to stderr, through the same neutralizing helper
the other error paths use (`render::neutralized`), and exit `1`. Keep exit
codes unchanged (`1` for these failures; `2` stays clap's usage errors).

### Acceptance

- An L1 CLI test for each of `symbols`, `refs`, `lint`, `imports` and
  `god-files` with an input that fails this way asserts stderr starts with
  `error: `, contains the `Display` message, contains no `{` struct dump, and
  the exit status is `1`.

## Documentation to update with each fix

The docs describe today's behavior; each fix must correct them in the same
change:

- F1, F2: [Diagnostics](../../docs/library/diagnostics.md) (the rule
  table at lines ~80-93, and the paragraph at line ~148 that explains the
  `undefined-symbol` severity mismatch).
- F3: [Using the Library](../../docs/library.md) (line ~282 says
  `reexported_symbols()` always returns an empty list).
- F4, F5: [Tree-sitter Queries](../../docs/queries.md) (the license wording
  and the "Drift" section, lines ~258-276).
- F6: [Using the `hug` CLI](../../docs/cli.md) exit-status section, if it
  shows error text.
- The agent skill's `diagnostics.md` and `query-system.md`
  (`.claude/skills/tree-hugger/`) where they describe the registry, drift or
  provenance.

## Investigated and not a defect

- **No cache-upgrade row for extractor revision 49.**
  `cli/tests/symbols/cache_upgrade.rs` has stale-revision rows up to 48 but
  none for 49. Committed history moved `EXTRACTOR_REVISION` from 43 to 46
  (`580342c73`) and from 46 to 50 (`2807ee765`), so no build or user cache
  ever carried revision 49. That bump also moved the record schema from 5.0
  to 5.1, and schema-5.0 entries are covered by
  `incompatible_record_schema_versions_are_misses_never_served_or_widened`
  (`lib/tests/l1/symbols_cache.rs`).
