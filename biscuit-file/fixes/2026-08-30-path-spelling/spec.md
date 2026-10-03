---
status: draft-spec
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
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-03
review_iterations: 0
created: 2026-08-30
area: biscuit-file
packages:
  - biscuit-file
  - claudine
  - claudine-cli
  - darkmatter
  - darkmatter-cli
human_review: false
message_to_agent: |-
  Phase 1 changed no source. Read the implementation log's "Existing guard that overlaps Phases 3 and 4" section before starting Phase 3 or 4: `darkmatter/cli/tests/common/context_guard.rs` already counts every `dirs::home_dir` / `std::env::home_dir` / `biscuit_file::home_dir` read per file in Claudine and Darkmatter (`context_construction_guard.rs` allowlists in claudine/{lib,cli}/tests/l1 and darkmatter/cli/tests/l1). Phase 4 edits must update those allowlists in the same change, and Phase 3's home-lookup rule should tighten that gate rather than build a second scanner for the same identifiers. The canonicalize audit table (convert/except/ignore per call line) and the home-lookup inventory, including two Claudine sites the plan omits (`linking/paths.rs:55`, `messaging/resolve.rs:161`), are in the log. `mcp/state.rs:218` writes a canonical path into persisted MCP state; converting it changes stored keys on Windows. Phase 4 should add `file-reference` explicitly to claudine/lib's biscuit-file features instead of relying on defaults.

  Baseline: biscuit-file and claudine `just test`/`just lint` pass. darkmatter `just test` has 2 PRE-EXISTING failures (current_root_documentation_contract, current_root_migration_guard) caused by HEAD commit 92c562dcc splitting `.claude/skills/claudine/SKILL.md` into `cli-commands.md`; they are unrelated to path code and must not be attributed to this fix. Phase 6 (skills) is the natural place to fix them; until then darkmatter's suite is red at baseline.

  Phase 2 (done): candidate and `@` root dedupe in biscuit-file now go through `first_seen_by_identity` (crate-internal, `file_reference/portable/path_identity.rs`), keyed on `PathIdentity` instead of `normalize_components`. On Windows this also folds unreducible (long) verbatim paths and verbatim UNC paths onto their legacy spellings; Unix behavior is unchanged. Six new Windows-gated tests (`a_real_short_name_alias_is_one_directory_but_two_identities`, two in `tests/l1/magic_local_roots.rs`, two in `tests/l1/repository_scope_catalog.rs`) compiled on build-win-native but have NOT run: the cross-check consume step hit the 50 GiB storage preflight (47.8 GiB free; the sweep freed nothing). When a later phase cross-checks biscuit-file on Windows, confirm those six pass. `just check-tier-coverage` is a repo-root recipe, not a package-area one.
---

# Test path spelling on every host and centralize filesystem path lookup

## Summary

Windows can spell a directory with a drive path (`C:\Users\ken`), a
verbatim prefix (`\\?\C:\Users\ken`), a short name (`C:\Users\RUNNER~1`), or
mixed separators (`C:\temp\docs/file.md`). Different spellings can break
candidate deduplication, search order, and repository containment when a
producer and its consumer use different rules.

This fix has three goals:

1. Test Windows lexical comparison rules (comparison without consulting the
   filesystem) on every host through the production implementation. Test filesystem aliases separately:
   text alone cannot establish that `RUNNER~1` names `Runner Administrator`.
2. Route filesystem canonicalization that leaves a private comparison through
   biscuit-file's [canonicalize_simplified](../../lib/src/path_text.rs), which
   resolves an existing path and removes a Windows prefix only when safe.
   Guard legitimate exceptions with a source inventory.
3. Make Claudine's home-based reads and writes honor the same home choice.
   The public home lookup already exists; the author must choose whether its
   Windows behavior changes before this part is implemented.

**Reader's note from the 2026-10-03 review:** the incident below is historical.
Biscuit-file now has [PathIdentity](../../lib/src/file_reference/portable/path_identity.rs),
a lossless lexical comparison type, and
[Windows text tests](../../lib/src/file_reference/portable/path_identity/tests.rs)
that run on every host. Its [home_dir](../../lib/src/file_reference/context.rs)
helper still uses the Windows known-folder lookup, while Darkmatter's
[RequestSnapshot::from_process](../../../darkmatter/lib/src/markdown/compose/context/request.rs)
captures an environment-selected home. Implementation must extend these
existing pieces rather than create a second path parser or duplicate public
home helper. The [file-reference topic](../../docs/topics/file-references.md)
is the current contract.

## Background — the PR #66 incident

The evidence base for this spec is the 2026-08-30 fix session on
`feat/finalized-references`. Every failure below shipped past a green macOS
`just test` run and surfaced only on Windows CI:

| Failure | Mechanism |
|---|---|
| darkmatter-cli `schema_triggers` ×5: `md schema validate` reported "no schema definition, valid by default"; `md schema triggers` errored "document is outside discovery boundary" | `file.canonicalize()` produced `\\?\C:\...`; lexical segmentation turned the prefix into an extra `?` segment, so the gix-derived legacy boundary never prefix-matched and trigger discovery silently found nothing |
| claudine `sequence` ×4: "invalid file reference `\\?\C:\...`: Windows device-prefix paths are not supported" | `preflight::canonical` (raw canonicalize) fed its result back into `FileReference::new`, which correctly rejects verbatim spellings — production manufactured the very spelling its own grammar forbids |
| claudine-cli `propagated_context`: child process panic `RootNotNormalized { path: "\\?\..." }` through an `.expect` | `LaunchContext` stored raw-canonicalized roots; the discovered `system-prompt.md` path inherited the verbatim prefix, gix then discovered a verbatim workdir, and `RepositoryScopeCatalog::new` refused it |
| biscuit-file `precedence_flip` verbatim-dedupe test asserted `Repository` provenance where its own same-spelling sibling asserts `Source` | the test was `#[cfg(windows)]`-only, written on macOS, and had never executed on the host where it was authored |
| claudine-cli `agent_cwd` ×2: hermetic config never loaded, hook action never fired | `dirs::home_dir()` (dirs 6) resolves via the known-folder API on Windows and ignores `%USERPROFILE%`/`HOME`, so the fixture home was invisible and claudine read the machine's real `~/.claudine` |
| 5 collateral unit tests flipped when the seam fix landed | their expectations were built with raw `fs::canonicalize` and encoded the verbatim spelling as correct |

Two aggravators make this class CI-only in practice: GitHub's Windows
runners hand out an 8.3 short-name `TEMP` (`RUNNER~1`) that the local
Windows build host does not reproduce, and `Path::join("a/b")` preserves the
literal `/`, so needle strings built from fixture paths carry mixed
separators that match nothing.

## Remaining defects and scope

The incident exposed two gaps: tests did not exercise the Windows spelling
rules on development hosts, and callers could create a spelling their next
consumer rejected. Existing identity tests now cover much of the first gap;
what remains is proving candidate ordering and the consumers' boundaries
with those rules. Do not reproduce already-covered parser cases as a second
implementation.

Home lookup remains inconsistent. Claudine's config loader honors process
overrides, but many config writers, provider paths, logs, and caches still use
the OS profile. A fixture can therefore read one home and write another.
Changing that behavior is intentional, but affects real users who override
`USERPROFILE`, including users of shells on Windows. Do not assume all users'
environment values equal the OS profile.

The implementation scope is the five packages in frontmatter. Audit raw
filesystem canonicalization throughout the workspace to identify escaping
results; remediation and the executable guard cover these five packages.
Record findings in other packages in the implementation log for separate
work. Expanding remediation requires the author decision under Open questions.
A workspace-wide scan does not authorize new dependencies across every area.

## Required behavior

### Lexical comparison, candidate order, and containment

Use biscuit-file's [PathIdentity](../../lib/src/file_reference/portable/path_identity.rs)
for comparison rules. Its public constructor interprets paths using the
host's grammar: passing `C:\repo` to a Unix `Path` does not simulate Windows.
Extend the existing test-only Windows text entry point to exercise the same
production Windows parser. Where necessary, extract the smallest internal
ordering or comparison function that accepts parsed identities; keep the
public API and host-native interpretation unchanged.

Tests must reach production comparison and ordering code, not a test-only
reimplementation. Cover these cases:

| Input or operation | Required result |
| --- | --- |
| Safe verbatim drive spelling and its legacy spelling | Same lexical identity; duplicate candidates retain the first occurrence and its provenance |
| Ordinary `C:\repo\docs/file.md` and `C:\repo\docs\file.md` | Same identity under Windows grammar |
| Verbatim paths containing `/`, `.` or `..` | Preserve Windows literal-name rules; do not apply ordinary-path normalization |
| Legacy UNC and corresponding verbatim UNC | Equivalent identity when their names are equal; different servers or shares remain distinct |
| `C:\repo` and `D:\repo`, or `C:repo` and `C:\repo` | Distinct identities |
| `C:\Repo` and `C:\repo` | Distinct names; only drive-letter case is folded |
| Boundary `C:\repo` and document `C:\repo-old\x.md` | Outside: containment uses whole components |
| Short and long names, or symlink aliases | No lexical unification; they require filesystem evidence |
| Non-Unicode native names | Preserve distinct raw names; do not turn lossy display text into identity |

Test candidate counts and first-seen provenance with distinct tags, including
reversing input order. Pin emitted path spelling separately from identity:
remove a verbatim prefix only when the reduction preserves meaning, and
preserve the spelling when it cannot be safely reduced. Identity equivalence
does not promise a portable rendering.

For biscuit-file's [RepositoryScopeCatalog](../../lib/src/file_reference/context.rs),
which validates repository and package roots, preserve its requirement that
roots arrive absolute and normalized. A reducible verbatim root is rejected
as unnormalized until the caller simplifies it; this is an explicit error,
not an empty search result. Tests must distinguish constructor validation
from document containment against an already-valid catalog.

Lexical containment is only the first check. Keep canonical checks that reject
symlinks or junctions escaping the repository or file-tree boundary, including
checks through the nearest existing ancestor for missing targets. Do not
replace them with lexical tests. Keep native Windows tests for actual 8.3
aliases, junctions, and platform conversion; document why those require the
Windows filesystem. A fixture must not assume 8.3 generation is enabled.

### Canonicalization audit and source guard

Audit filesystem calls written as `std::fs::canonicalize`, imported or aliased
`fs::canonicalize`, path-method `.canonicalize()`, and direct
`dunce::canonicalize`. Classify each production call by where its result goes:

- **Private comparison:** both operands receive identical canonicalization;
  the result stays inside that comparison. Keep a narrowly identified
  exception with a reason naming this invariant.
- **Result passed onward:** a returned, stored, serialized, displayed, or
  reparsed path, or a path compared against another producer's spelling, uses
  biscuit-file's [canonicalize_simplified](../../lib/src/path_text.rs).
  A hash or cache key is private only when its entire lifetime and every
  producer follow the same spelling contract.

Preserve each call's error handling and canonicalization frequency. Do not
add existence requirements to paths that currently support missing files.
Do not silently convert errors to empty search results or add panicking
fallbacks. The helper itself is the allowed direct `dunce` call. Any other
exception needs a concrete invariant; being inside biscuit-file is not an
exception by itself.

Add a source guard using the existing Claudine
[dispatch inventory](../../../claudine/cli/tests/l1/dispatch_inventory.rs)
as a model. Register it in an existing Level 1 test target. Its inventory must:

- Discover production Rust source for both library and CLI in each of the
  five packages, including module files outside conventional `src` folders
  when manifests declare them. Inspect all target-specific source branches.
- Exclude inline and separate test-only modules, comments, and string
  literals. Do not mistake Darkmatter's style-name normalization function for
  filesystem canonicalization.
- Recognize qualified, imported, aliased, and path-method forms above. Where
  a lightweight scanner cannot resolve a receiver, report the candidate for
  explicit review instead of silently ignoring it. Document macro-expansion
  limitations; this is a source guard, not proof about compiled code.
- Identify exceptions by repository-relative path and enclosing item plus
  operation, with exact occurrence counts and a reason. Line numbers belong
  in diagnostics, not identity. Fail on new calls and unused exceptions.
- Reject direct `dirs::home_dir` and `std::env::home_dir` in Claudine after
  home lookup is unified, except at an approved process-capture boundary.
  Recognize imported aliases and function references as well as calls.
- Test the scanner with small source fixtures: new call, alias, allowed call,
  moved line, deleted exception, inline test, and string/comment lookalikes.
  Fail if required source roots are missing; an empty scan cannot pass.

Declare repository source reads in forms the
[rust-testing skill](../../../.claude/skills/rust-testing/SKILL.md) can index.
For cross-package reads, declare the guard's source inputs in package
metadata as required by [test-input rules](../../../docs/cicd/test-inputs.md).
An archive run must have the inputs available at runtime through the existing
test harness. Do not use compile-time absolute checkout paths or skip the
guard when inputs are absent.

### Consistent home lookup

Resolve the first Open question before implementing home lookup. The
requirements below describe the recommended option; choosing another option
requires revising these requirements to match. Reuse biscuit-file's existing [home_dir](../../lib/src/file_reference/context.rs), which supplies
the default captured home for file resolution, rather than adding a helper
with the same purpose under a new name.

If the recommended option is accepted, use the pinned toolchain's
`std::env::home_dir()` result filtered to absolute paths: `HOME` on POSIX and
`USERPROFILE` on native Windows, with the standard library's platform fallback.
Do not add a Windows `HOME`-over-`USERPROFILE` rule. Preserve `Option<PathBuf>`,
non-Unicode native path support, and the `file-reference` feature boundary.
Do not canonicalize the home, require it to exist, or rebase a relative home.
A relative selected home yields `None`, without a second lookup of the real
profile that could defeat an override.

Route Claudine's home-based config load and save, backups, logs, cache,
provider config, and fallback home uses through the chosen shared reader.
Reuse an authoritative captured home in request-scoped code; do not reread
the environment midway through a request. Preserve explicitly configured
provider roots, storage overrides, and injected context homes. An explicit
cleared home stays cleared. Permission and policy paths using home must use
that same captured value; document that process environment is an input, not
proof of a trusted filesystem boundary.

If the shared helper changes, replace Darkmatter's duplicate process home
reader with it and revise its now-obsolete explanation of the two readers.
Update the file-reference topic, affected READMEs and Claudine topic pages,
and skill guidance describing lookup behavior. Document that native Windows
users relocate home with `USERPROFILE`; setting only `HOME` is insufficient.
Audit `dirs::config_dir`, `cache_dir`, and `data_dir` in the affected home-based
flows: replacing a fallback home must not leave the primary write destination
outside the fixture. Preserve intentional OS-folder policies and seed their
fixture directories explicitly rather than claiming every storage path is
home-based.

### Canonicalization helper contract

Biscuit-file's [canonicalize_simplified](../../lib/src/path_text.rs) remains
available without `file-reference`. For an existing ordinary temporary
directory, prove the result is absolute, usable for I/O, and accepted by
[try_portable_string](../../lib/src/path_text.rs), the rendering helper that
reports when portable spelling is unavailable. Compare against the real
canonical target: macOS `/var` and `/private/var` need not keep the authored
spelling. Missing paths return an I/O error.

On native Windows, canonicalizing a safe verbatim disk path and its legacy
form yields the same result. Where available, an actual short-name alias
must reach the same target. Do not promise that every canonical result loses
its prefix: long paths, reserved names, trailing dots or spaces, and UNC paths
must follow the existing safe-reduction and native-rendering contracts.
Use existing rendering tests for unsafe cases rather than requiring unusual
filesystem names on every host. Refeeding an unsimplifiable result into the
file-reference grammar must produce the existing explicit error, not panic
or change the grammar's device-prefix prohibition.

## Design decisions

- **Reuse the existing identity implementation.** Tests that run everywhere
  must invoke the production Windows parser; a second string normalizer
  could pass while the resolver remains broken.
- **Keep identity, I/O paths, and presentation separate.** A prefix can be
  ignored for comparison while remaining necessary to open the file. Safe
  simplification must never rename a verbatim-only path.
- **Guard direct `dunce` calls too.** Otherwise a caller can bypass the shared
  canonicalization function while appearing compliant. Keep reviewed
  exceptions for private comparisons rather than banning valid raw paths.
- **Audit widely, change the named packages.** The incident spans those
  consumers. A wider migration is a separate scope decision, not an
  incidental dependency expansion during this fix.
- **No performance spike is needed.** The planned work replaces lookups at
  existing sites and adds test-only checks. Preserve the number of filesystem
  calls; propose measurement only if implementation introduces a new known
  runtime cost.

## Open questions

### Should the existing public home lookup honor environment overrides on Windows?

This is a public behavior choice: other biscuit-file callers currently get
the OS profile even when `USERPROFILE` points elsewhere. Claudine and
Darkmatter must converge on a documented policy so fixtures and real launches
cannot read one home and write another.

1. **Change the existing biscuit-file helper to the environment-first policy
   above (recommended).** Pros: one reader for defaults and process snapshots;
   consistent config reads and writes; no duplicate API. Cons: changes native
   Windows behavior for every default caller with an overridden
   `USERPROFILE`, including home-based search and permission paths. Mitigate
   by auditing affected callers, documenting the change, and preserving
   explicit captured homes and provider overrides. This best achieves the
   single-reader goal and matches the two applications' process behavior.
2. **Keep the OS-profile helper and add an explicitly named process-home
   reader.** Pros: preserves existing defaults and allows controlled launches
   to relocate home. Cons: two public policies remain; each caller must choose
   correctly, and the source guard must enforce the choice. Requires separate
   documentation and tests for both policies.
3. **Keep OS lookup and inject homes into every application boundary.**
   Pros: no shared public behavior change; explicit dependencies. Cons:
   larger application plumbing across config, reporting, and providers;
   ambient convenience calls still risk inconsistent lookup. This is useful
   for request internals but does not alone fix all default reads and writes.

### Should remediation and the guard extend beyond the five named packages?

1. **Keep the five-package implementation and record outside findings
   (recommended).** Pros: bounded dependencies and review; addresses the
   demonstrated failures. Cons: remaining callers need separate fixes.
   This is proportional to the evidence while preserving the workspace audit.
2. **Convert and guard every workspace package now.** Pros: broader protection
   against recurrence. Cons: expands dependencies, callers, documentation,
   and verification beyond the listed scope. The author must approve the
   additional package inventory before implementation.

## Verification

- Run affected Level 1 suites with `just test` from each touched package area
  (nextest). The lexical ordering, identity, and scanner tests execute on
  macOS, Linux, native Windows, and WSL2. Ordinary filesystem tests remain
  portable; native Windows evidence covers actual aliases and junctions.
- A development-host run catches wrong Windows identity rules and ordering.
  Do not claim it proves Windows I/O, prefix reduction by the platform
  library, or real junction behavior. For newly extracted comparison code,
  temporarily break one relevant comparison or ordering rule and confirm the
  targeted test fails; restore the edit before completing verification.
- Source-guard fixtures prove that an unlisted filesystem call or home
  lookup fails with `file:line`, and that stale exceptions fail too.
- Use the monorepo's [test toolkit](../../../tools/test-toolkit/README.md)
  and each area's existing child-process fixtures for home round trips.
  Set `HOME` and `USERPROFILE` in the child's environment, use a temporary
  home, and invoke production config load and save. Assert the written file
  is under the fixture and reload it there. Do not read or write the real
  profile. Test conflicting overrides to pin the platform's chosen variable,
  relative and missing home inputs, explicit home injection, and unchanged
  captured home after the process environment changes. Isolate environment
  changes in child processes rather than mutating a multithreaded test's
  process environment.
- Run `just lint` for changed package areas. Use `just test-l2` only for
  affected existing Level 2 tests; headless child-process config tests belong
  in Level 1. No terminal or browser window may gain focus.
- Reuse qualifying passing evidence for unchanged required cells; obtain
  missing behavioral evidence on each required environment using the
  repository's existing cross-check procedures. Report any environment gap
  explicitly. No new CI matrix or gate is part of this fix.

## Out of scope

- CI scheduling or preflight changes, tracked separately as
  `ci-preflight-local-parity`.
- Changing output contracts: context path values and Markdown presentation
  remain portable, while eager file-expression identity retains native
  spelling. See the [OS guidance](../../../.claude/skills/os/SKILL.md) and
  current package topic pages; do not normalize all displayed paths as part
  of the audit.
- Lexical short-name/long-name unification, filesystem name case folding,
  or equating symlink aliases without filesystem lookup. As-launched paths
  retain their existing reporting policy.
- Glob grammar changes, a new virtual filesystem, and unrelated source or
  comment cleanup. Maintain comments and docs whose behavior this fix changes.
