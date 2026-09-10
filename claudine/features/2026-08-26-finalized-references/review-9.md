---
$schema: feature-review.yaml
ready: false
agent: codex/gpt-5.6-sol
created: 2026-09-08T12:10:30-07:00
spec: 2026-08-26-finalized-references/spec.md
log: claudine/features/2026-08-26-finalized-references/log.md
implemented: true
implemented_by: codex/gpt-5.6-sol
description: A **feature** review of `2026-08-26-finalized-references/spec.md`
feature: 2026-08-26-finalized-references/review-9.md
previous: 2026-08-26-finalized-references/review-8.md
next: 2026-08-26-finalized-references/review-10.md
---

# Review 9: Finalized References

## Verdict

The implementation is not ready for production. Review 8's direct
`#[macro_export]` cross-crate case is now detected, `$crate` keeps its
definition-crate identity, all 30 focused Level 1 spawn-inventory tests pass,
and the previously red generated-provider checks are clean. However, the
inventory recognizes only a literal `#[macro_export]`: a macro exported in a
production build through `#[cfg_attr(not(test), macro_export)]` is still
censused only against its definition crate and can reproduce the cross-crate
false negative. AC10 also remains unmet because the required native Linux,
native Windows, and WSL final-tree executions are absent, while Windows and
WSL Level 2 remain explicit CI policy gaps.

## Findings

### High — AC6's cross-crate macro census ignores conditional `macro_export`

`visit_item_macro` in `claudine/cli/tests/spawn_inventory.rs` lines 1538–1546
sets `exported` only when an attribute's direct path is `macro_export`. Rust
also permits the attribute to be applied through `cfg_attr`; for example,
`#[cfg_attr(not(test), macro_export)]` exports the macro in every production
build, but the scanner classifies it as non-exported.

That classification restores review 8's missed construction. A conditionally
exported macro in `claudine` whose transcriber contains
`Command::new(...).status()` can resolve `Command` from a `claudine-cli`
invocation module. The scanner gives that transcriber only the library's
`local_expansion_contexts`, so it records no site when the definition crate
does not bind `Command`, even though the production expansion executes a
child. This remains inside D8.7's source-authored boundary and violates AC6's
requirement that the guard fail on any production construction that can
execute without the shared environment helper.

Required change: classify a macro as exported whenever a production-reachable
`cfg_attr` can apply `macro_export`; unknown feature or platform predicates
must be handled conservatively. Extend the distinct-crate Level 1 regression
with `#[cfg_attr(not(test), macro_export)]`, showing that definition-only
contexts produce no record and the production census produces one
uncontrolled record.

Verification level: Level 1 is appropriate for this static source-inventory
invariant. The current 30/30 passing focused suite does not contain the
conditional-export case.

### High — AC10's required final platform matrix remains incomplete

AC10 requires `just test`, `just test-l2`, and `just lint` in biscuit-file,
Darkmatter, and Claudine on local macOS, `build-linux`, `build-win`, and
`build-win-native` before hosted CI. The implementation log now records a
fully green macOS matrix, including 6,871/6,871 Claudine Level 1 tests and
242/242 Claudine Level 2 tests, and the generated-provider drift that made
review 8's Level 1 gate red has been repaired.

The remaining cells are not passing evidence:

- no current-tree native Linux, native Windows, or WSL gate execution is
  recorded;
- `.github/workflows/_package-ci.yml` lines 573–576 excludes Windows and WSL
  from Level 2 by construction;
- `.github/ci/environments.json` lines 34–42 records no native-Windows Level 2
  backend, and lines 107–115 records that WSL lacks the archived broker and
  running terminal server; and
- no hosted-CI run exists for the uncommitted reviewed tree.

Required change: execute and record the complete green AC10 matrix for the
final tree, including non-vacuous Windows and WSL Level 2 coverage. If those
platform cells are no longer requirements, amend AC10 explicitly; unavailable
infrastructure and read-only reachability probes do not satisfy the current
acceptance criterion.

Verification level: AC10 explicitly requires Level 1 and Level 2 platform
evidence. Level 3 is not applicable because this feature asserts no keyboard,
mouse, paste, IME, or terminal-input encoder behavior.

## Requirement Verification Matrix

| Requirement | Strongest evidence reviewed | Assessment |
|---|---|---|
| AC1 — grammar and parser contract | Level 1 parser, resolver, and CLI fixtures | Appropriate and previously recorded green. |
| AC2 — implicit reference precedence | Level 1 collision fixtures plus recorded Level 2 compose/proxy coverage | Appropriate and previously recorded green. |
| AC3 — remove legacy `!` grammar | Level 1 grammar, compiler, and inventory checks | Appropriate and previously recorded green. |
| AC4 — reference-owned scopes and topology | Level 1 catalog/topology/work-counter checks plus recorded Level 2 nested composition | Appropriate and previously recorded green. |
| AC5 — materialization and provenance | Level 1 schema/orchestration matrix plus recorded Level 2 proxy/sequence coverage | Appropriate and previously recorded green. |
| AC6 — `ctx.cwd` and `AGENT_CWD` | Level 1 context/subprocess coverage and 30 passing spawn-inventory tests | Correct tier, but the inventory misses production-reachable conditional macro exports; see the first finding. |
| AC7 — magic conventions preserved | Level 1 collision/deduplication fixtures plus recorded Level 2 compose checks | Appropriate and previously recorded green. |
| AC8 — completion/execution parity | Level 1 completion/execution tests plus recorded Level 2 magic-reference checks | Appropriate; no terminal input encoder behavior is asserted. |
| AC9 — cross-platform syntax | Level 1 host-independent parser checks and native filesystem fixtures, including recorded Windows junction execution | Appropriate; final platform closure remains governed by AC10. |
| AC10 — final quality gates | Recorded green macOS L1/L2/lint matrix; no final-tree Linux/Windows/WSL matrix and no Windows/WSL L2 provisioning | Not satisfied; see the second finding. |
| AC11 — containment and junction safety | Level 1 lexical, symlink, deepest-ancestor, completion, and native Windows junction tests | Appropriate and previously recorded green. |
| AC12 — passive/public contracts and docs | Level 1 passive/public/corpus checks, real CLI paths, and documentation review | Appropriate and previously recorded green. |
| AC13 — reserved syntax | Level 1 grammar checks and design-document review | Appropriate and previously recorded green. |

No requirement asserts modifier-press visibility, hotkey activation, paste,
IME, mouse behavior, or another encoder-sensitive interaction requiring Level
3 OS input injection.

## Verification Performed

- `cargo nextest run -p claudine-cli --test spawn_inventory --color=never
  --no-fail-fast` passed 30/30 tests on macOS, including the direct
  cross-crate export and `$crate` regressions.
- Source tracing confirmed that exported transcribers now receive contexts
  from both guarded crates, but export detection is the direct-path check at
  `spawn_inventory.rs:1545`; neither implementation nor regression coverage
  handles `cfg_attr(..., macro_export)`.
- `cargo run -q -p claudine-gen -- check` reported all ten provider fragments,
  the catalog, signals, stream vocabulary, and families clean. It emitted only
  the existing stale models-catalog warning.
- The implementation log records green current-tree macOS gates for all three
  package areas. The remaining AC10 platform gap was checked against the CI
  workflow and environment capability catalog; Windows and WSL Level 2 are
  still explicitly unprovisioned.
- `git diff --check` over review 8's implementation files passed.
