---
$schema: feature-review.yaml
ready: false
agent: codex/gpt-5.6-sol
created: 2026-09-08T15:43:59-07:00
spec: 2026-08-26-finalized-references/spec.md
log: claudine/features/2026-08-26-finalized-references/log.md
implemented: false
description: A **feature** review of `2026-08-26-finalized-references/spec.md`
feature: 2026-08-26-finalized-references/review-10.md
previous: 2026-08-26-finalized-references/review-09.md
---

# Review 10: Finalized References

## Verdict

The implementation is not ready for production. Review 9's conditional
`macro_export` gap is fixed: production-reachable `cfg_attr` attributes are
recognized conservatively, nested attributes are covered, the cross-crate
regression now uses the conditional form, the generated inventory matches the
live census, and all 31 focused Level 1 inventory tests pass. No additional
functional, ergonomic, or performance defect was found in the finalized
file-reference implementation.

AC10 nevertheless remains unsatisfied. The final tree is green on macOS,
native Linux, and Windows-hosted WSL2 for the recorded applicable package-area
gates, but native Windows could not execute because the builder failed the
repository's mandatory storage preflight. Native Windows also has no supported
live terminal backend, so its required Level 2 cell cannot produce non-vacuous
evidence. An unavailable or skipped platform cell is not passing evidence.

## Findings

### High — AC10 still lacks the required native-Windows final-tree matrix

AC10 requires `just test`, `just test-l2`, and `just lint` in biscuit-file,
Darkmatter, and Claudine on macOS, native Linux, WSL, and native Windows. The
implementation log records current-tree green results on macOS, native Linux,
and WSL2. It also records that native Windows stopped before compilation:
`W:` had 27,724,689,408 bytes free and `C:` had 27,421,388,800 bytes free,
below the repository's 50 GiB safety threshold.

The missing Level 2 evidence is structural as well as operational.
`.github/ci/environments.json` still declares that native Windows has no
provisionable terminal backend, and `.github/workflows/_package-ci.yml`
excludes Windows from the Level 2 matrix by construction. Consequently, a
future green hosted run would still not verify the native-Windows Level 2 cell
required by the specification.

Required change: provide an isolated native-Windows builder with at least
50 GiB of free space and a supported live terminal backend, then execute and
record the full biscuit-file, Darkmatter, and Claudine AC10 matrix against the
same final tree. If native-Windows Level 2 is no longer intended, amend AC10;
do not treat a skipped recipe as equivalent evidence.

Verification level: AC10 explicitly requires Level 1, Level 2, and lint
evidence on native Windows. None is present for the final tree. Level 3 is not
applicable because this feature specifies no OS-keyboard, mouse, paste, IME,
or terminal-input-encoder behavior.

## Requirement Verification Matrix

| Requirement | Strongest evidence reviewed | Assessment |
|---|---|---|
| AC1 — grammar and parser contract | Level 1 parser, resolver, and CLI fixtures | Appropriate and previously recorded green. |
| AC2 — implicit reference precedence | Level 1 collision fixtures and Level 2 compose/proxy execution | Appropriate and recorded green on the available final-tree environments. |
| AC3 — remove legacy `!` grammar | Level 1 grammar, compiler, and inventory checks | Appropriate and previously recorded green. |
| AC4 — reference-owned scopes and topology | Level 1 catalog/topology/work-counter checks and Level 2 nested composition | Appropriate and previously recorded green. |
| AC5 — materialization and provenance | Level 1 schema/orchestration matrix and Level 2 proxy/sequence execution | Appropriate and previously recorded green. |
| AC6 — `ctx.cwd` and `AGENT_CWD` | Level 1 context/subprocess coverage, Level 2 composition coverage, and 31/31 focused spawn-inventory tests | Appropriate. Review 9's conditional-export hole is closed. |
| AC7 — magic conventions preserved | Level 1 collision/deduplication fixtures and Level 2 compose checks | Appropriate and previously recorded green. |
| AC8 — completion/execution parity | Level 1 completion round trips and Level 2 real compose execution | Appropriate; no terminal input encoder behavior is asserted. |
| AC9 — cross-platform syntax and filesystem behavior | Cross-platform Level 1 parser fixtures plus native Unix filesystem coverage and an earlier native-Windows junction fixture | The test design is appropriate, but the final-tree native-Windows closure is missing under AC10. |
| AC10 — final quality gates | Green macOS, native-Linux, and WSL final-tree evidence; no native-Windows execution or terminal backend | Not satisfied; see the finding. |
| AC11 — repository containment | Level 1 lexical, symlink, deepest-ancestor, completion, and native junction tests | Appropriate and previously recorded green. |
| AC12 — passive/public contracts and CLI paths | Level 1 passive/public/corpus checks and Level 2 real CLI surfaces | Appropriate and previously recorded green. |
| AC13 — reserved syntax | Level 1 host-independent grammar and diagnostic checks | Appropriate and previously recorded green. |

No requirement asserts modifier-press visibility, hotkey activation, paste,
IME, mouse behavior, or another encoder-sensitive interaction requiring Level
3 OS input injection.

## Verification Performed

- `cargo nextest run -p claudine-cli --test spawn_inventory --color=never
  --no-fail-fast` passed 31/31 tests on macOS. This includes the conditional
  cross-crate export regression, unknown feature/platform predicates, nested
  `cfg_attr`, `$crate` identity, and the production inventory artifact gate.
- Source review confirmed `production_macro_export` treats predicates proven
  test-only as absent and conservatively includes unknown production feature
  and platform configurations.
- The implementation log records green final-tree Level 1 for Claudine on
  native Linux and WSL after the inventory artifact correction; the applicable
  Level 2 and lint gates had already passed on those exact source snapshots.
- CI policy and capability metadata still exclude native Windows from Level 2
  and identify the missing backend explicitly.
- `git diff --check` passed for review 9's scanner, generated inventory,
  review-chain update, and implementation log before this review was written.
