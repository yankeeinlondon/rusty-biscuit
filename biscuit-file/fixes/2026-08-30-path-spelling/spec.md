---
status: draft
created: 2026-08-30
area: biscuit-file
packages:
  - biscuit-file
  - claudine
  - claudine-cli
  - darkmatter
  - darkmatter-cli
---

# Path spelling is one seam, and it must be testable on every host

## Summary

Windows names one directory many ways: verbatim (`\\?\C:\Users\ken\...`),
legacy (`C:\Users\ken\...`), 8.3 short-name (`C:\Users\RUNNER~1\...`), and
mixed-separator (`C:\temp\docs/file.md`). The reference-resolution stack
treats spelling equivalence as a correctness question — dedupe, precedence,
containment, and the finalized grammar all depend on it — yet the logic that
answers it is exercised almost entirely through real Windows filesystem
paths, and the seam that *produces* spellings (`canonicalize`, home-dir
resolution) is scattered and unguarded.

PR #66 paid the bill for both gaps at once: 23 Windows-only test failures
across five packages plus one production panic, none reproducible on the
macOS development host, all traceable to two mechanical causes. This spec
makes the two causes structurally impossible to reintroduce:

1. **Spelling logic becomes host-independent testable.** The layers that
   decide "same directory or not" gain pure string-level tests that run on
   every OS, in the style the `reference_grammar` suite already proves out.
2. **Spelling production becomes one guarded seam.**
   `biscuit_file::canonicalize_simplified` (added in PR #66) becomes the
   only sanctioned canonicalization whose result may cross a comparison or
   grammar boundary; raw `std::fs::canonicalize` in production code is
   audited down to an allowlist and held there by a drift guard. Home-dir
   resolution joins the seam because `dirs::home_dir()` on Windows ignores
   environment overrides, which silently breaks hermetic test homes.

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

## The defect

### Defect one — spelling equivalence is only tested where it can't be run

`file_reference/resolve.rs` dedupes candidates by `normalize_components`,
resolves precedence by first-seen order, and validates containment against
repository roots. All of that is lexical — no filesystem required — yet its
Windows-spelling behavior is asserted only in `#[cfg(windows)]` tests
against `TempDir` paths. A developer on macOS gets zero signal, and (as the
`precedence_flip` test proved) can ship an assertion that is simply wrong,
because nothing on their machine ever runs it.

The counter-example already exists in-tree: `reference_grammar` tests such
as `windows_absolute_and_unc_classify_absolute_on_any_host` classify Windows
spellings as pure strings, run on every OS, and stayed green throughout the
incident. `normalize_relative_path` in darkmatter's trigger discovery
documents the same principle ("testable on every platform"). The pattern is
proven; it just doesn't cover the layers that broke.

### Defect two — spelling production is scattered and unguarded

`std::fs::canonicalize` is the only std way to resolve symlinks, and its
Windows result is verbatim. That is fine inside a closed key-space (both
sides of every comparison canonicalized the same way — e.g.
`invocation_context::canonical_key`, `protect::path`) and wrong everywhere
else. Nothing distinguishes the two uses today except a comment, so each new
call site is a coin flip that only Windows CI ever calls. PR #66 fixed four
leaking sites one at a time; the class survives.

The home-dir variant: on Unix `dirs::home_dir()` reads `$HOME`, so hermetic
test fixtures work; on Windows it consults only the known-folder API, so the
same fixture pattern silently reads and could write the developer's real
`~/.claudine`. PR #66 patched exactly one call site
(`dispatch/loader.rs::user_config_path`, via `std::env::home_dir()`, which
is env-first on the pinned toolchain); roughly a dozen `dirs::home_dir()`
sites remain in claudine alone, and config *save* still resolves differently
from config *load*.

## Required behavior

### R1 — string-level spelling tests for dedupe, precedence, and containment

Table-driven tests over string spellings — verbatim, legacy, 8.3
short-name, UNC, and mixed-separator forms of one root — asserting, on every
OS with no `TempDir` and no `#[cfg(windows)]`:

- candidate count after dedupe (two spellings of one root → one candidate);
- surviving provenance (first-seen order; matches the same-spelling
  collapse);
- the reported path spelling (legacy, per `simplify_root`);
- containment verdicts for the repository-scope catalog and any lexical
  boundary check biscuit-file owns: a verbatim document against a legacy
  boundary must produce a deliberate, tested outcome — never a silent
  "no match".

Where the production entry point takes `&Path` values that in practice come
from `canonicalize`, introduce a thin inner function parameterized on the
already-spelled paths so the tables can drive it without touching the
filesystem. The public API does not change.

`#[cfg(windows)]` tests remain only for behavior that genuinely requires the
Windows filesystem (8.3 name generation, junction traversal); each must
state in its doc comment why a string-level test cannot cover it.

### R2 — canonicalize audit and drift guard

Audit every `std::fs::canonicalize` / `.canonicalize()` in production code
workspace-wide (tests excluded). Each site ends in one of two states:

- **Key-space (allowed):** the result never leaves a comparison space where
  both sides are canonicalized identically. The site gains a comment naming
  the invariant and joins the guard's allowlist.
- **Boundary-crossing (converted):** the site moves to
  `biscuit_file::canonicalize_simplified`.

Add a repo-lint test in the style of the existing dispatch-inventory drift
guard: it scans production sources for raw canonicalize, diffs against the
allowlist, and fails with the offending `file:line` when a new unlisted site
appears. The allowlist lives next to the guard so extending it is a
reviewed, deliberate act.

### R3 — one home-dir seam that honors overrides on every OS

Introduce a single home-resolution helper with `std::env::home_dir()`
semantics (`$HOME`/`%USERPROFILE%` first, platform API as fallback) and
route claudine's `dirs::home_dir()` call sites through it — config load
**and** save, backups, logs, model-catalog cache, and the per-provider
config paths, so no two surfaces can resolve home differently. The helper's
natural owner is biscuit-file next to `canonicalize_simplified` (it is the
same "one spelling seam" concern); claudine consumes it.

The guard from R2 also flags new direct `dirs::home_dir()` calls in the
swept packages.

### R4 — contract tests for `canonicalize_simplified`

On every host: for an existing plain directory, the result carries no
verbatim disk prefix and round-trips through `to_portable_string` without
falling back to native spelling. On Windows additionally: the result of
canonicalizing a verbatim spelling equals the result of canonicalizing its
legacy spelling.

## Design decisions

- **Extend the proven pattern, don't invent one.** R1 copies the
  `reference_grammar` string-classification style rather than introducing a
  mocking layer or a virtual filesystem. The logic under test is lexical;
  the tests should be too.
- **Guard by allowlist, not by ban.** Raw `canonicalize` has legitimate
  key-space uses; removing it entirely would force `canonicalize_simplified`
  into places where verbatim is actually safer (open-by-handle style
  operations). The guard makes the distinction visible and reviewed instead
  of implicit.
- **Home seam sweeps claudine only, defines in biscuit-file.** claudine is
  where the hermeticity hole bit and where the call sites cluster. Other
  areas adopt the helper opportunistically; forcing a workspace-wide sweep
  now would balloon the change for no demonstrated defect (Rule 3).
- **No behavior change for real Windows users.** They do not set `HOME`, and
  `%USERPROFILE%` env equals the known folder in practice; the env-first
  order only matters when someone deliberately overrides — which is the
  point.

## Open questions

1. Should the R2 guard also cover `dunce::canonicalize` called directly
   (bypassing the named seam), or is the seam function purely a
   documentation convenience? Leaning: guard it too — one name, one
   grep target.
2. `darkmatter` and `biscuit-*` areas have their own `dirs::home_dir()`
   uses. Sweep them in this fix or file follow-ups per area after the
   claudine sweep proves the helper's shape?

## Verification

- A macOS-only `just test` run fails if any PR #66 spelling bug is
  reintroduced: verbatim dedupe collapse, `?`-segment boundary mismatch,
  device-prefix rejection of a self-manufactured spelling, or a provenance
  flip in the precedence order. Prove non-vacuity by neutering each guard
  once and confirming red (per the established test discipline).
- The drift-guard test goes red when a raw `canonicalize` (or direct
  `dirs::home_dir` in swept packages) is added outside the allowlist, and
  its failure message names the file and line.
- On Windows, a test that sets `HOME`/`USERPROFILE` to a fixture home reads
  and writes claudine config only under that fixture — verified by a
  round-trip test that writes config through the production save path and
  observes the file under the fixture home.
- Full suites for the touched packages stay green on macOS, Linux, and
  native Windows (the `build-linux` / `build-win-native` hosts).

## Out of scope

- The CI-side hardening (running the scope self-test in `just ci-local`, a
  `just cross-check` recipe) — tracked separately in
  `fixes/_unscheduled/ci-preflight-local-parity.md`.
- Changing what spelling any production surface *emits*. The
  portable-vs-native contracts (portable `ctx.*` and Markdown presentation,
  native eager-`file()` identity) are ratified in
  `claudine/features/2026-08-26-finalized-references/spec.md` D8 and are not
  revisited here.
- Short-name (8.3) ↔ long-name unification. `canonicalize_simplified`
  resolves to long names when it runs; surfaces that deliberately report the
  as-launched spelling (e.g. `ctx.cwd`) keep doing so.
