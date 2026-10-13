---
kind: fix
name: docs-audit-code-defects
area: tree-hugger
status: planned
date: 2026-10-10
clarified: true
clarified_by: claude/sonnet
reviewed: true
review_note: the clarification process served as a review
origin: the 2026-10-09 restructure of `tree-hugger/docs/`, which checked every page against the code and found six defects in the code itself (a seventh item, F7, was added by ruling)
packages:
  - tree-hugger
  - tree-hugger-cli
needs_rulings: false
review_iterations: 2
completed: true
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
implemented: true
implemented_by: "claude/opus"
human_review: false
message_to_agent: |-
    All four phases are implemented. Phase 4 is uncommitted in the working
    tree (this pipeline commits separately). Phase 4 added cli/tests/command_feedback.rs and updated three
    older CLI assertions that matched the Debug variant name `NoSourceFiles`.
    Host trap: CDPATH puts the main checkout first, so always `cd` to the
    absolute worktree path before running `just` gates.
---

# Six code defects found while auditing the Tree Hugger docs

## Summary

Rewriting the Tree Hugger documentation meant checking each documented
claim against the code. Six places where the code is inconsistent with
itself, or with what its own API promises, turned up. Each was confirmed
on 2026-10-09 against the working tree. The docs currently describe the
code as it is, defects included; each fix below must update them.

Two packages are involved: `tree-hugger` (the library, in `lib/`) and
`tree-hugger-cli` (the `hug` command, in `cli/`).

| ID | Defect | Package | Change |
|----|--------|---------|--------|
| F1 | `undefined-symbol` is an `Error` in the library but a `Warning` in the rule registry | tree-hugger | 1 |
| F2 | Three registered rules are never emitted; two rules' language lists disagree with the queries | tree-hugger | 1 |
| F3 | `TreeFile::reexported_symbols()` always returns an empty list | tree-hugger | 2 |
| F4 | Tree Hugger's own queries are labeled `MIT`; the repository is `AGPL-3.0-only` | tree-hugger | 3 |
| F5 | Query drift checking defines six kinds of drift but only ever detects one | tree-hugger | 2 |
| F6 | A command-level error is printed as a Rust `Debug` dump (`Error: NoSourceFiles { … }`) | tree-hugger-cli | 3 |
| F7 | (added by ruling) `hug` silently accepts rule names it does not know | tree-hugger-cli | 3 |

All rulings are recorded below; none is open. The
[risk review](#risk-review) resolved the questions that could be settled by
reading code. What remains are
[checks to make during implementation](#verify-during-implementation), which
are facts to look up, not decisions.

One change goes beyond the six defects as originally listed: F2(e) makes the
PHP `eval` lint pattern case-insensitive, which produces new findings in PHP
files that call `EVAL(...)`. It is a small, deliberate behavior change and is
called out in the release notes.

## Delivery: three changes, in this order

1. **Change 1: F1 and F2 together.** They edit the same severity and message
   tables in `lib/src/queries/mod.rs`, the same rule registry and the same
   tests. They must not be split.
2. **Change 2: F3 and F5 together** ("remove unused public API": the empty
   `reexported_symbols()` method and the drift module).
3. **Change 3: F4, F6 and F7** (small, CLI-and-provenance work, independent of
   changes 1 and 2).

Each change is one local commit (see Execution).

## Execution

The human gives design feedback on this spec once. Agents then carry all
three changes through implementation, tests, lint, docs and agent-skill
updates start to finish, with no human review between stages.

- The terminal state is **"implementation complete, ready for review"**. By
  repository rule an agent never moves a spec to `_completed` and never runs
  `just complete`.
- If an acceptance criterion cannot be met, the agent **stops and reports**.
  It does not weaken the criterion, delete the test that checks it, or skip a
  gate.

### Pre-authorizations granted

These are authorized, verbatim; anything not listed is not authorized.

- (a) Build, test and lint with `just test`, `just test-l2` and `just lint`
  for `tree-hugger` and `tree-hugger-cli`, repeated until green.
- (b) Create throwaway Git repositories in the session scratch directory and
  run `hug` there to reproduce and verify F6. Nothing outside the scratch
  directory is touched.
- (c) Edit agent skill files under `.claude/skills/tree-hugger/` and the
  docs, and delete tests that only exercise removed code (the drift tests in
  F5).
- (d) One signed local commit per change (three in all), with the
  repository's configured author identity and no agent attribution or
  co-author trailers (the repository rule overrides any harness suggestion).
  Verify each commit with `git verify-commit HEAD`. **No push and no pull
  request.**

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
  (`lib/src/queries/drift.rs`) were meant to tell a maintainer when a vendored
  query is behind upstream and should be re-vendored. In practice they only
  compare two revision strings (see F5).
- **The persistent cache.** The library can store a file's analysis results,
  diagnostics included, on disk (`lib/src/cache/symbol_cache.rs`) and replay
  them later. A stored entry is reused only if its fingerprint still matches:
  the crate version plus `EXTRACTOR_REVISION` (`lib/src/file/mod.rs`,
  currently `"50"`), the query text, and a digest of the registry's rule
  metadata. See [Cache design](../../docs/cache-design.md).

## F1. `undefined-symbol` has two different default severities

### What `undefined-symbol` flags

`undefined-symbol` is an experimental rule in the `tree-hugger` library. It
reports a name that is used in a file but is not defined in it, not imported
by it, and not a language builtin. For example, in a Python file that has no
`def compute_total` and no import of it:

```python
total = compute_total(items)   # flagged: `compute_total` is not defined or imported
```

Not flagged: a name the file defines or imports; a qualified name such as
`os.path` (the qualifier is checked by the separate `undefined-module`
rule); a builtin such as `len`; a single-character name; a name that starts
with `_`.

It is low confidence because the check sees one file at a time. Known false
positives come from wildcard imports (`from x import *`), names created by
macros or dynamically at run time, and gaps in the per-language builtin list.

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
registry's `default_severity`.

Findings from checking every entry of `severity_for_rule` against the
registry (2026-10-10):

- `severity_for_rule` has six call sites, all in `lib/src/file/tree_file.rs`
  (about lines 1231, 1313, 1429, 1481, 1539 and 1616). Line 1231 is the
  generic loop over a query's `@diagnostic.<rule-id>` captures; the others
  are the five semantic rules `dead-code`, `undefined-symbol`,
  `unused-symbol`, `unused-import` and `undefined-module`.
- For every rule that can fire, the function agrees with the registry
  except `undefined-symbol`.
- Three of its entries are dead:
  - `undefined-variable` is not registered and nothing emits it.
  - `unreachable-code` is only an alias of `dead-code` in the registry, and
    nothing emits it directly.
  - `invalid-syntax` is registered as `Error` and is emitted (about
    `tree_file.rs:1694`) with a hard-coded `Error`, not through the function.
- The CLI's `apply_lint_policy` (`cli/src/main.rs`, about line 1946) already
  recomputes each diagnostic's severity from the registry, so only callers of
  the library see the difference.

A comment in the same code is also wrong. `check_undefined_symbols`
(`tree_file.rs:1375`) says that identifiers starting with an uppercase
letter are skipped, but the code skips only single-character names and names
starting with `_`. Per repository rule the code is right and the comment is
wrong.

### The cache can keep serving the old severity (resolved by reading the code, 2026-10-10)

**Resolved fact:** stored diagnostics ARE replayed when the fingerprint
matches. A cache entry holds the file's `diagnostics`, and `symbol_cache.rs`
keeps them whenever the entry's stored diagnostics fingerprint matches (it
clears them only on a mismatch, about line 358). The fingerprint covers the
crate version, `EXTRACTOR_REVISION`, the lint and comment query text, and a
digest of each
registered rule's `id`, `version`, `category`, `default_severity`,
`confidence`, `languages` and two enablement flags (`rule_metadata_digest`).
F1 changes how severity is stamped but changes no registry value, so on its
own it would leave every fingerprint unchanged, and an entry written by an
older build could keep serving `undefined-symbol` as `Error`. Change 1 also
edits the registry (F2), which would happen to change the digest, but F1 must
not depend on that side effect.

### Ruling

**Decided.** The rule registry (`RuleRegistry::builtin`) is the single source
of default severity. `undefined-symbol` is **Warning**. Library diagnostics
take their severity from the registry; `severity_for_rule` is deleted; a rule
ID the registry does not know falls back to `Info`.

- Rejected: **Error, by changing the registry to match the library.** It
  would change the `hug lint --experimental-semantics` exit status, and an
  error-by-default rule that is low confidence would fail builds on false
  positives.
- Rejected: **keep `severity_for_rule` as a second table.** Two tables are
  how this defect arose.

Why: the registry already says Warning, the CLI already behaves that way,
and one table cannot disagree with itself.

Two points of meaning:

- The `Info` fallback is not new behavior. It is the old function's own
  `_ => Info` arm, kept.
- A registered rule that is disabled (off by default, or experimental and not
  enabled) keeps its default severity. Whether a rule is enabled never changes
  its severity.

### Fix

- Stamp each diagnostic with its rule's registry default severity, at all six
  call sites. An unregistered ID gets `Info`.
- Delete `severity_for_rule` and its re-export, along with the dead entries
  above.
- Correct the comment in `check_undefined_symbols` to list what the code
  skips (single-character names and names starting with `_`).
- Bump `EXTRACTOR_REVISION` from `"50"` to `"51"` and add a cache-upgrade row
  (see the first acceptance item). As ordinary test-first practice, the first
  task is a test, written against the unmodified code, proving that an entry
  stored by the current build is replayed with `Error` severity; it then
  fails or is rewritten once the fix lands, and the revision bump is what
  makes the new behavior safe.

### Acceptance

- **Cache.** A cache entry written by the previous build (revision `"50"`,
  `undefined-symbol` stored as `Error`) is a miss under the new build, never
  served. Concrete shape: add a `"50"` stale-revision row to
  `cli/tests/symbols/cache_upgrade.rs`, which seeds stale entries through
  `FingerprintInputs` and `canonical_digest` with a test-owned `TMPDIR`; the
  row confirms that entries stamped with revision 50 are treated as misses
  after the bump to 51. Add a library-level check in
  `lib/tests/l1/symbols_cache.rs` if the CLI row cannot observe severity.
- For every registered rule, the severity on a diagnostic returned by the
  library equals the registry's `default_severity`. Add an L1 test that
  iterates `RuleRegistry::builtin()` rather than listing rules by hand, in
  `lib/tests/l1/lint_severity_registry.rs`.
- The library and `hug lint --experimental-semantics` agree on the severity of
  `undefined-symbol` for the same file. Both sides read one shared fixture:
  the fixture lives under `lib/tests/fixtures/`, the library side is
  `lib/tests/l1/lint_severity_registry.rs` (next to `lint_diagnostics.rs`), and
  the CLI side is `cli/tests/lint_severity_agreement.rs`. The mechanism for
  the cross-package read, per the forms the `rust-testing` skill lists and
  `docs/cicd/test-inputs.md` at the repository root:
  - add a `source-inputs` entry for the fixture path in
    `tree-hugger/cli/Cargo.toml`, under the existing
    `[package.metadata.ci.tests]`;
  - have `cli/tests/lint_severity_agreement.rs` spell that path through
    `manifest_dir!()` (for example
    `.parent().unwrap().join("lib/tests/fixtures/…")`) or a repo-root-anchored
    join.

  Without the entry, a fixture-only edit would schedule no CLI cell; without
  the spelled path, `RealWorkspaceTestInputTests` would fail on a declared
  path no test spells. Fixture files with code suffixes count as source of
  the library package.
- `severity_for_rule` no longer exists, and a diagnostic for an unregistered
  rule ID has severity `Info`.
- `format_rule_message` (`lib/src/queries/mod.rs`, the message table) has no
  entry for an unregistered rule. The test resolves IDs the way the registry
  does, so the entry for the `unreachable-code` alias, which still resolves,
  does not fail it.

## F2. Registered rules that never fire, and language lists that disagree

### Evidence

Compare the registry with what is actually emitted:

- **Registered, never emitted.** `unused-variable`, `shadowed-variable` and
  `deprecated-syntax` are registered (`rule_registry.rs:567-599`), but no
  `lint.scm` captures them and no Rust code emits them. The only other
  mentions are their lines in the severity and message tables in
  `queries/mod.rs`. `hug lint --list-rules` therefore lists three rules that
  can never fire. The registry already labels `unused-variable` deprecated
  ("use unused-symbol instead"), and `unused-symbol` already flags unused
  locals, so a separate rule would double-report.
- **`eval-call`.** The registry lists JavaScript, TypeScript and PHP
  (`rule_registry.rs:295-299`), but `lib/queries/python/lint.scm` also
  captures `@diagnostic.eval-call`. Python `eval()` is reported under a rule
  whose metadata says it does not apply to Python.
- **`exec-call`.** The registry lists Python and PHP
  (`rule_registry.rs:320`), but only `python/lint.scm` captures it.
  `lib/queries/php/lint.scm` has no `exec` pattern (its only pattern is
  `eval`), so PHP `exec()` is never reported.
- **An existing PHP gap.** PHP function names are case-insensitive, but
  the PHP `eval` pattern compares with the case-sensitive `#eq?`, so
  `EVAL(...)` is missed. The PHP grammar treats `eval` as a plain `name`, so
  `EVAL(...)` has the same node shape as `eval(...)` (see
  [Risk review](#risk-review)). Python's patterns use `#eq?` too, which is
  correct there because Python is case-sensitive.

Every other pattern rule's registry language list matches the query files
exactly (checked for `dbg-macro`, `debugger-statement`, `breakpoint-call`,
`console-log`, `print-call`, `fmt-println`, `unwrap-call`, `expect-call`).

### Ruling

**Decided**, in five parts. Parts (c) and (e) carry two human-confirmed
rulings added after the first draft of this spec.

**(a) Remove the three never-emitted rules.** Delete `unused-variable`,
`shadowed-variable` and `deprecated-syntax` from the registry, and their
lines in the message table in `queries/mod.rs` (the severity-table lines go
with `severity_for_rule` in F1).

- Rejected: **make them aliases** of `unused-symbol` (or anything else).
- Rejected: **implement them.** Larger than a fix; each needs a design.
- Rejected: **keep them, marked as not implemented.**

Why: no user can depend on a rule that never fires, and `unused-symbol`
already covers the unused case. Implementing `shadowed-variable` or
`deprecated-syntax` later is out of scope here; each must be re-added to the
registry, with its own design, by the change that implements it.

What `--warn unused-variable` does afterward: it prints a warning (see F7).
`RuleSelector::parse` in the library stays non-validating.

**(b) Add Python to `eval-call`'s registry languages.** The query already
fires for Python; the metadata catches up with it. Add a Python example
(`eval('1 + 1')`) beside the JavaScript one, and bump the rule's `version`
from `1.0.0` to `1.1.0`. (The version feeds the cache fingerprint; the
languages list does too.)

**(c) `exec-call` and PHP: PHP `exec()` only.** Add a PHP pattern for the
function `exec()` to `lib/queries/php/lint.scm` and keep PHP in the registry
entry.

- Rejected: **drop PHP from the registry.** PHP `exec` is a different hazard
  from Python's `exec`, but the human preferred to cover PHP than to shrink
  the rule.
- Rejected: **also flag `shell_exec`, `system`, `passthru`, `proc_open`,
  `popen` and the backtick operator.** Each is a different grammar node, so it
  would multiply the query and turn a metadata fix into a new rule family.
  A broader PHP shell-execution rule may come later (see Not in scope).

Why: it closes the gap the audit found at the smallest size, and it keeps
`exec-call` meaning one thing per language: "the call named `exec`".

Details:

- **Case-insensitive.** The pattern matches with `#match?` and an anchored
  `(?i)` regular expression (for example `"(?i)^exec$"`), because PHP function
  names are case-insensitive: `EXEC('ls')` is the same call.
- **Which spellings are flagged (ruling R1).** The pattern flags `exec(...)`
  and `\exec(...)` only, case-insensitively. `\exec(...)` is the global
  function written with a bare leading backslash, so it is the same function
  as `exec(...)`. A call in another namespace such as `Foo\exec()` is **not**
  flagged: it is a different function that happens to share a name. The
  pattern must therefore require the bare leading-backslash form for the
  qualified case; matching any qualified name that ends in `exec` is wrong.
  - Rejected: **flag every qualified call whose last segment is `exec`**
    (`Foo\exec()` included). It would report calls to user-defined functions
    as the shell-command hazard.
  - Rejected: **flag unqualified `exec(...)` only.** `\exec(...)` calls the
    same global function and is a common way to write it inside a namespace.
  - Why: the rule means "the call named `exec`" for the PHP built-in, and the
    leading backslash is the only qualified spelling that still names it.
  - The grammar reading says `\exec` parses as a `qualified_name` whose prefix
    is a bare backslash, distinct from `Foo\exec` (a `qualified_name` with a
    namespace prefix) and from `namespace\exec` (a `relative_name`). The
    implementer confirms this with a sample parse before writing the pattern;
    see [Verify during implementation](#verify-during-implementation).
  - Tests, in the PHP lint tests: `exec('ls')`, `EXEC('ls')` and `\exec('ls')`
    each produce an `exec-call` diagnostic; `Foo\exec('ls')` produces none.
- **Title and wording.** The rule title becomes **"Dynamic code or command
  execution (exec)"**. In plain terms: in Python, `exec()` runs a string as
  code; in PHP, `exec()` runs a shell command. The registry entry has no
  separate description field, so this wording goes in the title, a new PHP
  example (`exec('ls -l');`), and the caveats. The message in
  `format_rule_message` is reworded to match the title (a consequence of the
  title change, not a separate ruling).
- **Caveat.** The registry caveats name the PHP functions the rule does *not*
  flag: `shell_exec`, `system`, `passthru`, `proc_open`, `popen` and the
  backtick operator.
- **Version.** `exec-call` `version` goes from `1.0.0` to `1.1.0`. The
  registry's `version` feeds the cache fingerprint, so this retires stored
  results for the rule.
- **Users.** `--deny exec-call` now also affects PHP files. The release note
  says so.

**(d) Keep the consistency test** described in the Fix section, so the
registry and the query files cannot drift apart again.

**(e) Make the PHP `eval` pattern case-insensitive, unconditionally (ruling
R2).** Change the `eval` pattern in `lib/queries/php/lint.scm` from `#eq?` to
`#match?` with an anchored `(?i)` expression, and add a test that
`EVAL('1 + 1')` produces an `eval-call` diagnostic. The risk review confirmed
that the PHP grammar treats `eval` as a plain `name`, so `EVAL(...)` has the
same node shape as `eval(...)`.

- Rejected: **leave the pattern case-sensitive and record the gap.** PHP
  function names are case-insensitive, so `EVAL(...)` is the same call and
  stays unreported for no reason once the shape is known to match.
- Rejected: **fix it only if a sample parse shows the same shape.** The shape
  is already established by reading the grammar, so the condition adds a
  decision point with only one outcome.
- This is a small behavior change beyond the six originally listed defects:
  PHP files that call `eval` in uppercase or mixed case gain new findings.
  The release note says so (see Documentation to update).

### Fix

- Apply the rulings above.
- Add an L1 consistency test in
  `lib/tests/l1/rule_registry_consistency.rs`: for every language, the set of
  `@diagnostic.<id>` captures in its `lint.scm` equals the set of pattern
  rules whose registry `languages` include it, and every registered rule is
  either emitted by a query or in an explicit list of rules emitted by Rust
  code. The test enumerates canonical rule IDs through `rule_ids()`, so the
  `unreachable-code` alias is not treated as a rule.
- That list is a public constant beside the semantic checks in
  `lib/src/file/tree_file.rs` (`SEMANTIC_RULE_IDS`), so adding a semantic
  check puts the new ID next to the code that emits it. Its complete set,
  established by the risk review, is `dead-code`, `undefined-symbol`,
  `unused-symbol`, `unused-import`, `undefined-module` and `invalid-syntax`.

### Acceptance

- The consistency test passes, and fails if a rule is added to a query or the
  registry without the other.
- `hug lint --list-rules` lists only rules that can fire, and no longer lists
  `unused-variable`, `shadowed-variable` or `deprecated-syntax`.
- A PHP file calling `exec(...)`, `EXEC(...)` or `\exec(...)` produces an
  `exec-call` diagnostic; a PHP file calling `Foo\exec(...)` or
  `shell_exec(...)` produces none.
- A PHP file calling `EVAL(...)` produces an `eval-call` diagnostic, as
  `eval(...)` already does.
- A Python file calling `eval(...)` is reported under a rule whose registry
  languages include Python; the registry's `eval-call` and `exec-call`
  `version` are both `1.1.0`.

## F3. `TreeFile::reexported_symbols()` always returns nothing

### Evidence

```rust
// lib/src/file/tree_file.rs:1093
pub fn reexported_symbols(&self) -> Result<Vec<ImportSymbol>, TreeHuggerError> {
    Ok(Vec::new())
}
```

The method is public, and its own documentation says it "returns an empty
list until re-export capture support is added". Nothing in the workspace
calls it. It is listed in [Using the Library](../../docs/library.md) and in
the agent skill's `api-reference.md`. A caller who uses the obvious method
gets an empty list and no error.

What does exist: `ImportSymbol::is_reexport` is set by `import_is_reexport`
(`tree_file.rs:244`, `:581`) for a Rust `pub use` only, and is read at
`tree_file.rs:1509`. The symbol-record field `visibility.is_reexported` is
never filled by extraction (the CLI sets it only on synthetic prelude
records).

### Ruling

**Decided.** Remove `TreeFile::reexported_symbols()`. Re-export detection for
all supported languages becomes a separate feature, stubbed as
`2026-10-11-reexports-all-languages` (an active feature of this package,
queued behind the three changes in this spec).

- Rejected: **implement the method** as a filter over `imported_symbols()` on
  `is_reexport`. It would work for Rust only and present a language-neutral
  API for a one-language capability.
- Rejected: **implement re-exports for a subset of languages now.** The
  reviewer ruled that adding a feature for some languages is poor practice;
  the capability must be designed for all 16 supported languages.

Why: a public method that answers "nothing" is worse than no method, and the
repository has no external users, so removal needs no deprecation. Until the
feature lands, the only re-export signal is the `is_reexport` flag on a
Rust import, and the docs say so.

### Fix

- Delete the method (and any now-unused import it leaves).
- Update the docs listed below to say that re-exports are detected for Rust
  `pub use` only, through `ImportSymbol::is_reexport`.
- Add one line to `docs/release-notes.md` saying the method was removed.

### Acceptance

- A test (add one if none exists) over a Rust file containing `pub use a::B;`
  and `use c::D;`: through `imported_symbols()`, `is_reexport == true` on `B`
  and only on `B`.
- Searching by symbol name, `reexported_symbols` appears nowhere in
  `tree-hugger/lib/`, `tree-hugger/cli/`, `tree-hugger/docs/` or
  `.claude/skills/tree-hugger/`, except in the removed-API line of
  `tree-hugger/docs/release-notes.md`. (The search does not use the word
  "re-export", which appears legitimately.) The re-exports feature spec
  and the historical cross-check text files under `_completed` are
  intentionally excluded and must not be touched.

## F4. Tree Hugger's own queries are labeled MIT

### Evidence

- `overlay_query_provenance` (`lib/src/queries/provenance.rs:217`) hard-codes
  `license: "MIT"` for every query Tree Hugger writes itself (`lint.scm`,
  `references.scm`, `comments.scm`, and overlay `locals.scm`). The same record
  sets `upstream_revision` from `env!("CARGO_PKG_VERSION")` (line 214).
- The same file defines `REPOSITORY_LICENSE = "AGPL-3.0-only"` (line 115),
  documented as "the license of queries Tree Hugger writes itself, which is
  the repository's license", and uses it for Perl's original `locals.scm`
  (line 82, with `upstream_revision` from `CARGO_PKG_VERSION` at line 79).
  The repository's `LICENSE` file is the GNU AGPL v3.

So two code paths disagree about the license of the same category of file,
and the `MIT` value contradicts the repository license.

Findings (2026-10-10):

- No other hard-coded `MIT` label exists in library source or `.scm` files.
  The only other hit in `lib/src` is the example text in a doc comment in
  `lib/src/corpus/manifest.rs` (`"MIT", "Apache-2.0"`); `lib/queries` has none.
- The `MIT` strings in `lib/tests/fixtures/corpus_manifest.yaml` and
  `lib/tests/l1/corpus_tests.rs` describe third-party corpus samples and
  **must stay**.
- Crate manifests: a search of the workspace `Cargo.toml` and the `lib/` and
  `cli/` manifests found no `license` key. See
  [Verify during implementation](#verify-during-implementation).

### Ruling

**Decided (closing an earlier open question).** A vendored query that Tree
Hugger has modified keeps its **upstream license** (Apache-2.0 for the
nvim-treesitter queries); only records whose status is
`TranslationStatus::Original` carry `REPOSITORY_LICENSE`. This already holds
for vendored records today.

- Rejected: **relicense modified vendored queries** to the repository's
  license. A modification does not remove the upstream license's terms.

### Fix

Use `REPOSITORY_LICENSE` at line 217. The constant's own documentation states
the intent. Extend the existing provenance test (which asserts
`REPOSITORY_LICENSE` for Perl at line 294) to every
`TranslationStatus::Original` record, including the overlay records.

### Acceptance

- Every provenance record with `TranslationStatus::Original` carries
  `AGPL-3.0-only`. That covers **both** the overlay record (line 217) and the
  Perl original (line 82); the test iterates all records and does not name
  them.
- Every vendored record keeps its upstream license (`Apache-2.0`).

## F5. Drift checking only compares revision strings

### Evidence

`DriftItem` (`lib/src/queries/drift.rs:24-38`) defines **six** kinds:
`NewerUpstreamRevision`, `ContentMismatch`, `UpstreamPathMissing`,
`CaptureDivergence`, `PredicateAdded` and `PredicateRemoved`.
`check_query_drift` (lines 80-110) only ever produces
`NewerUpstreamRevision`, by comparing the recorded revision string with the
one the caller passes in. The other five variants appear only in their own
`Display` implementation; nothing constructs them. The library has no access
to upstream query text, so it could not detect them as written.

Further findings (2026-10-10):

- `check_all_drift` iterates only `QueryKind::Locals`, the one vendored kind.
- Nothing in the `hug` CLI, the CI workflows or the `just` recipes calls any
  drift function. The only callers are the module's own unit tests, the
  integration test file `lib/tests/l1/phase6_neovim_query_reuse.rs`, and
  docs.
- The check needs the caller to already know the latest upstream revision, so
  it adds nothing over comparing two version strings by hand.
- The public re-exports in `lib/src/lib.rs` are `DriftItem`, `DriftReport`,
  `check_all_drift`, `check_query_drift` and `summarize_drift`.
  `expected_upstream_revisions()` is public in the module but not re-exported
  at the crate root. It returns a hard-coded `nvim-treesitter` => `0.10.4`
  and is used only by the module's own test and by `docs/queries.md`. No
  revision constant is shared: the recorded revision `0.10.4` is a separate
  string literal in `lib/src/queries/provenance.rs` (about line 100), which
  stays.

### Ruling

**Decided.** Remove the whole drift module (`lib/src/queries/drift.rs`):
`check_query_drift`, `check_all_drift`, `summarize_drift`,
`expected_upstream_revisions`, `DriftReport` and `DriftItem`, with their
re-exports in `lib/src/lib.rs` and the `pub mod drift;` line in
`lib/src/queries/mod.rs`. Keep the upstream revision recorded in each
query's provenance record (`QueryProvenance`).

- Rejected: **keep the module and remove only the five unproduced variants.**
  What remains is a string comparison nobody calls.
- Rejected: **implement content and capture checks** against an upstream
  checkout. A feature, not a fix, for a capability the human judged to be of
  little value.

Why: the feature is of little value, nothing uses it, and it advertises
checks it never performs.

### Fix

- Delete `lib/src/queries/drift.rs` and its re-exports.
- In `lib/tests/l1/phase6_neovim_query_reuse.rs`, delete the "Upstream Drift
  Detection" tests and the drift import. The test
  `full_query_audit_workflow` also runs a drift step (step 3); remove that
  step and keep the provenance and compatibility steps. Update the file's
  header comment and the "Provenance + Compatibility + Drift" banner.
- Remove the "Drift" section of [Tree-sitter Queries](../../docs/queries.md),
  keeping its refresh guidance in a short form: to refresh a vendored query,
  compare it with the upstream file at the revision recorded in its
  provenance, carry over Tree Hugger's changes (the provenance `notes` list
  them), and update the recorded revision.
- In the agent skill, remove the "Upstream Drift Detection" section of
  `.claude/skills/tree-hugger/query-system.md` (about line 453) and the
  phrase "so drift checks skip it" near line 415.
- Add one line to `docs/release-notes.md` saying the module was removed.

### Acceptance

- The drift module and its public exports no longer exist, and the workspace
  builds.
- The provenance tests still pass (every query still has a provenance record
  with a non-empty upstream revision).
- Searching by symbol name, none of `check_query_drift`, `check_all_drift`,
  `summarize_drift`, `expected_upstream_revisions`, `DriftItem` or
  `DriftReport` appears in `tree-hugger/lib/`, `tree-hugger/cli/`,
  `tree-hugger/docs/` (other than the removed-API line in
  `tree-hugger/docs/release-notes.md`) or `.claude/skills/tree-hugger/`.
  (Search by these names, not the word "drift", which appears legitimately
  elsewhere.) The re-exports feature spec, the historical
  cross-check text files under `_completed`, and `claudine/**/drift.rs` (an
  unrelated module) are intentionally excluded and must not be touched.
- The docs and the agent skill mention drift only to say how a maintainer
  refreshes a vendored query using the recorded revision.

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

The same happens for `refs`, `lint` and `imports`, and for **any** error that
reaches `main`, not only those commands. Everywhere else the CLI prints
`error: <message>` using the error's `Display` text (for example
`main.rs:1017`, `:1167`, `:1192`, and the `refs` ambiguity message at
`cli/src/refs.rs:400`). `TreeHuggerError` already has a readable `Display`
(`lib/src/error.rs`, via `thiserror`: "No supported source files found in
`{path}`"). The `Io` variant's `Display` already includes its source
("Failed to read `{path}`: {source}").

This is very likely what one docs agent saw as an occasional `Error:` prefix
from `hug refs`: the ambiguity message itself is always `error:`, but a run
whose filters matched no files fails through this path.

Other places that produce errors which reach `main` (checked 2026-10-10), all
covered by the same fix because they travel the same path:

- `current_dir()` (`cli/src/main.rs`, about line 1396) builds an `Io` error
  with path `.`.
- The external-lint adapter call in `add_external_lint_diagnostics`
  (about line 1615) builds an `Io` error whose path is the analyzed file, so
  a file name with a control character reaches the message. (A reviewer cited
  line 1596; in the current tree the call is at about line 1615.)
- `render_rule_list` (about line 1810) builds an `Io` error with path
  `<stdout>` when JSON output fails.
- Two more `Io` errors built in `main` itself (about lines 923 and 945: a
  failure reason and a JSON serialization failure) take the same path.

### Ruling

**Decided.** `main` catches every error that reaches it, prints
`error: <Display message>` to stderr with control characters escaped by the
neutralizing helper (`render::neutralized`, `cli/src/render.rs:1383`), and
exits `1`. Clap's usage-error status `2` is unchanged.

- Rejected: **fix only the five commands that were reproduced.** Every other
  path through `main` has the same defect.
- Rejected: **print the `source()` chain** after the message. The `Io`
  variant's `Display` already includes its source, so the chain would print
  it twice.

### Fix

Rename the current body of `main` to `run()` (returning the same
`Result<(), TreeHuggerError>`) and make `main` a thin wrapper: on `Err`, print
`error: <neutralized Display>` to stderr and exit `1`. Only `Err` returns are
affected. Clap usage errors already exit `2` through clap's own path, and the
other paths in `main` already call `process::exit` themselves, so neither
changes.

For example, the `god-files` case above becomes:

```text
$ hug god-files nope-dir
error: Failed to read `nope-dir`: god-files scan directory does not exist
```

(The `Io` variant's `Display` is "Failed to read `{path}`: {source}".)

### Acceptance

- A table-driven L1 CLI test over `symbols`, `refs`, `lint`, `imports` and
  `god-files`, each with an input that fails this way, asserts that stderr
  starts with `error: `, contains the `Display` message, contains no `{`
  struct dump, and that the exit status is `1`.
- Existing tests in `cli/tests/cli.rs` (about lines 2028 and 2043) that assert
  stderr contains "does not exist" and "not a directory" keep passing
  unchanged; they are part of the regression evidence.
- One more test (`#[cfg(unix)]` only, because Windows file names cannot hold
  control characters) passes a file name containing a control character and
  asserts the control character appears escaped (`\u{…}`), not raw, in stderr.

## F7. Unknown rule names are accepted silently (added by ruling)

### Evidence

`--warn`, `--deny` and `--allow` values go through `parse_selectors`
(`cli/src/main.rs`, about line 671), which calls `RuleSelector::parse`
(`lib/src/rule_registry.rs`). That function treats any string that is not
`all` or `category:<name>` as a rule ID and never checks it against the
registry. A typo such as `--deny unwrap-cal` is accepted, matches nothing,
and the rule you meant to enforce stays unenforced. After F2(a), the removed
rule names behave the same way.

### Ruling

**Decided.** `tree-hugger-cli` prints `warning: unknown rule '<name>'` to
stderr, with the name escaped by `render::neutralized`, and continues, for
each rule name given to `--warn`, `--deny` or `--allow` that the registry does
not know. A name the registry resolves as an alias is known. The exit status
is unchanged. The library's `RuleSelector::parse` stays non-validating: it is
public API, and this is no behavior change for library callers. The warning is
printed once per distinct name per run.

- Rejected: **reject the name as an error** (exit non-zero). It would break
  existing scripts that pass the removed rule names.
- Rejected: **validate inside `RuleSelector::parse`.** It changes public
  library behavior and needs a registry the function does not have.

Why: it turns a silent no-op into a visible one at the least cost, and covers
typos and the F2(a) removals alike.

### Fix

Run the unknown-rule check **once**, at the place where the CLI builds the
rule selectors for a command, not inside `parse_selectors` on each call. The
same lint arguments feed both the lint command construction and
`AnalysisOptions::from_command`, so a check inside `parse_selectors` would
print the warning twice. Dedupe per distinct name per run. Use
`RuleRegistry::is_known`, which exists and counts aliases as known. `all` and
`category:` selectors are not rule names and are not checked.

Before finishing F7, the implementer greps the three `symbols` export tests
that assert empty stderr (`cli/tests/symbols/export_controls.rs`,
`export_ctags.rs` and `export_ctags_paths.rs`) for `--warn|--deny|--allow`, to
be sure none passes rule flags that would now print a warning.

### Acceptance

- An L1 CLI test: `--warn unused-variable` and `--deny unwrap-cal` each print
  the warning exactly once (the test asserts the count, so a double print
  fails it) and exit with the status they had before; a known rule (for
  example `--deny unwrap-call`) prints none.
- Documented in [Using the `hug` CLI](../../docs/cli.md) and
  [Diagnostics](../../docs/library/diagnostics.md), plus a one-line
  `docs/release-notes.md` entry that removed rule names are accepted with a
  warning.

## Documentation to update with each fix

The docs describe today's behavior; each fix must correct them in the same
change. A docs page never names a feature or fix.

- F1, F2: [Diagnostics](../../docs/library/diagnostics.md) (the rule
  table at lines ~80-93, and the paragraph at line ~148 that says the
  library returns `undefined-symbol` as `Error`: after F1 the library
  severity equals the registry default, so the paragraph goes). Add the
  plain-language `undefined-symbol` description above if the page lacks one.
  Remove the sentence near line 115 saying the registry also holds
  `unused-variable`, `shadowed-variable` and `deprecated-syntax`. Update the
  `exec-call` and `eval-call` rows (languages, wording; `eval-call` now
  matches PHP `eval` case-insensitively). Note that `--deny exec-call` now
  also affects PHP, and that `exec-call` flags `exec(...)` and `\exec(...)`
  but not a call in another namespace such as `Foo\exec(...)`.
- F3: [Using the Library](../../docs/library.md) (line ~282 says
  `reexported_symbols()` always returns an empty list: replace it with the
  statement that re-exports are detected for Rust `pub use` only, via
  `ImportSymbol::is_reexport`). The agent skill's `api-reference.md` (line
  ~45) drops the method and says the same.
- F4: [Tree-sitter Queries](../../docs/queries.md) (the license wording).
- F5: [Tree-sitter Queries](../../docs/queries.md): remove the "Drift"
  section (lines ~258-285), keeping the refresh-a-vendored-query guidance.
- F6: [Using the `hug` CLI](../../docs/cli.md) exit-status section, if it
  shows error text.
- F7: [Using the `hug` CLI](../../docs/cli.md) and
  [Diagnostics](../../docs/library/diagnostics.md).
- Release notes (`docs/release-notes.md`): one line per user-visible change
  above (removed methods and module, removed rule names accepted with a
  warning, PHP `exec()` now reported, PHP `eval` now matched
  case-insensitively so `EVAL(...)` and similar spellings gain new findings).
- The agent skill (`.claude/skills/tree-hugger/`): `diagnostics.md` (the
  "add severity mapping" step near line 266 no longer exists: a new rule's
  severity is its registry entry), and `query-system.md` (the "Severity
  Mapping" section near line 223 that quotes `severity_for_rule()`, the
  `exec-call` and `eval-call` rows of the rule table, the drift section and
  the drift phrase near line 415).

## Not in scope

- A broader PHP shell-execution rule (for example `shell-exec-call`, covering
  `shell_exec`, `system`, `passthru`, `proc_open`, `popen` and backticks) is a
  possible later addition. This fix flags PHP `exec()` only.
- Implementing `shadowed-variable` or `deprecated-syntax`.
- Re-export detection for languages other than Rust (the active feature
  `2026-10-11-reexports-all-languages`).
- Content or capture drift checks against an upstream checkout.
- Validating selectors in the library's `RuleSelector::parse`. (Its handling
  of a malformed `category:` selector, which is dropped silently, is also
  unchanged.)

## Risk review

No spikes were needed. These were resolved by reading code and are recorded
as facts, not to-dos:

- **Cache replay.** Stored diagnostics are replayed when the fingerprint
  matches; the fingerprint covers crate version, `EXTRACTOR_REVISION`, query
  text and the rule-metadata digest (see F1).
- **PHP grammar shapes.** A bare `exec` is a `name`; `Foo\exec` is a
  `qualified_name` with a namespace prefix; `namespace\exec` is a
  `relative_name`; `\exec` is a `qualified_name` with a bare backslash prefix.
  `eval` is a plain `name`, so `EVAL(...)` has the same shape as `eval(...)`.
- **Rust-emitted rule IDs.** The complete set is `dead-code`,
  `undefined-symbol`, `unused-symbol`, `unused-import`, `undefined-module` and
  `invalid-syntax`. `unreachable-code` is an alias, not a rule, and the
  consistency test enumerates canonical IDs through `rule_ids()`.
- **Removed rules.** No snapshot, golden or completion file enumerates
  `unused-variable`, `shadowed-variable` or `deprecated-syntax`.
- **Registry consumers.** Outside `rule_registry.rs` and `symbol_cache.rs`,
  the only consumers are the `--list-rules` rendering, whose two tests assert
  on `unwrap-call` only.
- **`severity_for_rule`** is used only in `tree_file.rs`.
- **Dependents.** Only `tree-hugger/cli` and `tree-hugger/lib` depend on the
  `tree-hugger` package.
- **Drift API users** are limited to the module, the `lib.rs` re-exports, the
  phase6 test, `docs/queries.md` and the skill's `query-system.md`.
- **Broken pipe** is handled elsewhere and is unaffected by the F6 change.
- **`RuleRegistry::is_known`** exists, counts aliases, and is what F7 uses.

## Verify during implementation

These are the facts that can only be settled while implementing. They are
checks, not decisions. Record each result in the implementation log.

1. **`\exec()` parse shape (F2).** Parse a sample containing `\exec('ls');`,
   `exec('ls');` and `Foo\exec('ls');` with the PHP grammar. The grammar
   reading says `\exec` is a `qualified_name` with a bare backslash prefix;
   confirm it and write the pattern to match.
2. **New `#match?` query compiles (F2).** Confirm that the `#match?` patterns
   for `exec` and `eval` compile, through the existing query-compile test.
3. **Manifest license key (F4).** None is present in the workspace or crate
   manifests today; report what is found. No change is implied.
4. **Export-test grep (F7).** The 10-second grep named in F7 over the three
   `symbols` export tests.

## Investigated and not a defect

- **No cache-upgrade row for extractor revision 49.**
  `cli/tests/symbols/cache_upgrade.rs` has stale-revision rows up to 48 but
  none for 49. Committed history moved `EXTRACTOR_REVISION` from 43 to 46
  (`580342c73`) and from 46 to 50 (`2807ee765`), so no build or user cache
  ever carried revision 49. That bump also moved the record schema from 5.0
  to 5.1, and schema-5.0 entries are covered by
  `incompatible_record_schema_versions_are_misses_never_served_or_widened`
  (`lib/tests/l1/symbols_cache.rs`). F1 adds the row for `"50"`.
