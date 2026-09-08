---
$schema: feature-review.yaml
ready: false
agent: codex/gpt-5.6-sol
created: 2026-09-08T11:16:35-07:00
spec: 2026-08-26-finalized-references/spec.md
implemented: false
description: A **feature** review of `2026-08-26-finalized-references/spec.md`
feature: 2026-08-26-finalized-references/review-8.md
previous: 2026-08-26-finalized-references/review-7.md
---

# Review 8: Finalized References

## Verdict

The implementation is not ready for production. Review 7's same-crate
invocation-scope macro case is now detected and all 28 focused Level 1
spawn-inventory tests pass. However, the inventory still partitions expansion
contexts by scanned crate, so an exported macro authored in `claudine` and
invoked in `claudine-cli` can resolve a process constructor from the CLI call
site without being censused against that context. AC10 also remains unmet:
the canonical Claudine Level 1 gate is red, and the required Windows and WSL
Level 2 environments remain unprovisioned.

## Findings

### High — AC6's macro inventory does not model cross-crate invocation contexts

`generate_inventory` in `claudine/cli/tests/spawn_inventory.rs` lines
1719–1744 creates a fresh `ModuleResolver` and `contexts` collection inside
the loop over `SCANNED_ROOTS`. A macro transcriber found under
`claudine/lib/src` is consequently evaluated only against library module
contexts; the CLI contexts are collected later and used only while scanning
`claudine/cli/src`.

That partition leaves a valid source-authored construction invisible. For
example, a `#[macro_export] macro_rules! launch` in the library can expand
`Command::new("true").status()` while relying on
`use std::process::Command` in a CLI invocation module. Rust resolves that
ordinary item name at the invocation site, but the scanner never evaluates the
library transcriber with the CLI binding. If no library module binds
`Command`, the transcriber contributes no inventory record even though the CLI
expansion executes a child.

This is within the deliberately narrowed D8.7 boundary: the generated tokens
come from a `macro_rules!` transcriber authored in one of the two guarded
source roots, not from an external macro. It also contradicts AC6's requirement
to check locally authored transcribers against every possible invocation-module
binding. The current regression proves only a definition module and invocation
module within one synthetic crate.

Required change: build an invocation-context set that covers both guarded
crates for exported local macros, while retaining each context's own resolver
and preserving `$crate` as the definition crate. Add a Level 1 fixture with the
macro definition and invocation in distinct synthetic crates; its pre-fix
result should be no record and its required result should be one uncontrolled
site.

Verification level: Level 1 is appropriate for this static source-inventory
invariant. The gap is semantic reachability, not terminal behavior.

### High — AC10's required final platform matrix and green gates are incomplete

AC10 requires `just test`, `just test-l2`, and `just lint` in biscuit-file,
Darkmatter, and Claudine on local macOS, `build-linux`, `build-win`, and
`build-win-native` before hosted CI. The reviewed tree still does not meet that
contract:

- A fresh `just test --no-fail-fast` in `claudine/` ran all 6,869 selected
  Level 1 tests: 6,862 passed and 7 `claudine-gen` drift/generation tests
  failed. The failures may be unrelated to finalized references, but AC10
  requires a green gate.
- The implementation log records a green macOS Claudine Level 2 run of
  242/242 after setting `ATUIN_AI__ENABLED=false` for the managed process. This
  closes the prior host-specific WezTerm failure without changing user config.
- `.github/workflows/_package-ci.yml` lines 573–576 still excludes Windows and
  WSL Level 2 by construction. `.github/ci/environments.json` lines 34–42 and
  107–115 still records no provisioned native-Windows backend and no usable WSL
  tmux/broker environment. Those are explicit policy gaps, not passing Level 2
  executions.
- No final-tree native Linux, native Windows, WSL, or hosted-CI evidence exists
  for the uncommitted reviewed tree.

Required change: restore the canonical Claudine Level 1 gate, provision and
execute non-vacuous Level 2 coverage on Windows and WSL, and record the full
green AC10 matrix for the final tree. If that platform requirement is no longer
intended, amend AC10 explicitly; absent such an amendment, unavailable
infrastructure cannot count as passing evidence.

Verification level: AC10 requires Level 1 and Level 2 evidence. Level 3 is not
applicable because this feature does not assert terminal keyboard, mouse,
paste, or IME encoder behavior.

## Requirement Verification Matrix

| Requirement | Strongest evidence reviewed | Assessment |
|---|---|---|
| AC1 — grammar and parser contract | Level 1 parser, resolver, and CLI fixtures | Appropriate and previously recorded green. |
| AC2 — implicit reference precedence | Level 1 collision fixtures plus recorded Level 2 compose/proxy coverage | Appropriate and previously recorded green. |
| AC3 — remove legacy `!` grammar | Level 1 grammar, compiler, and inventory checks | Appropriate and previously recorded green. |
| AC4 — reference-owned scopes and topology | Level 1 catalog/topology/work-counter checks plus recorded Level 2 nested composition | Appropriate and previously recorded green. |
| AC5 — materialization and provenance | Level 1 schema/orchestration matrix plus recorded Level 2 proxy/sequence coverage | Appropriate and previously recorded green. |
| AC6 — `ctx.cwd` and `AGENT_CWD` | Level 1 context/subprocess coverage and 28 passing spawn-inventory tests | Correct tier, but the inventory omits cross-crate invocation contexts for locally authored exported macros; see the first finding. |
| AC7 — magic conventions preserved | Level 1 collision/deduplication fixtures plus recorded Level 2 compose checks | Appropriate and previously recorded green. |
| AC8 — completion/execution parity | Level 1 completion/execution tests plus recorded Level 2 magic-reference checks | Appropriate; no terminal input encoder behavior is asserted. |
| AC9 — cross-platform syntax | Level 1 host-independent parser checks and native filesystem fixtures, including recorded Windows junction execution | Appropriate; final platform closure remains governed by AC10. |
| AC10 — final quality gates | Fresh macOS Claudine L1 at 6,862/6,869; recorded macOS L2 at 242/242 and lint green; no Windows/WSL L2 or final-tree remote/hosted matrix | Not satisfied; see the second finding. |
| AC11 — containment and junction safety | Level 1 lexical, symlink, deepest-ancestor, completion, and native Windows junction tests | Appropriate and previously recorded green. |
| AC12 — passive/public contracts and docs | Level 1 passive/public/corpus checks, real CLI paths, and documentation review | Appropriate and previously recorded green. |
| AC13 — reserved syntax | Level 1 grammar checks and design-document review | Appropriate and previously recorded green. |

No requirement asserts modifier-press visibility, hotkey activation, paste,
IME, mouse behavior, or another encoder-sensitive interaction requiring Level
3 OS input injection.

## Verification Performed

- `cargo nextest run -p claudine-cli --test spawn_inventory --color=never`
  passed 28/28 tests on macOS, including the new same-crate invocation-scope
  fixture and `production_spawn_inventory_is_complete_and_governed`.
- Source tracing confirmed that `generate_inventory` collects and consumes
  macro expansion contexts independently for each entry in `SCANNED_ROOTS`,
  leaving no library-definition-to-CLI-invocation pairing.
- `just test --no-fail-fast` in `claudine/` completed with 6,862 passed, 7
  failed, and 11 skipped. All 28 spawn-inventory tests and the repaired
  spawn-site isolation test passed; the seven failures were the recorded
  `claudine-gen` drift/generation failures.
- AC10's remaining platform gap was checked against the current CI workflow,
  environment capability catalog, and implementation log. The log records
  macOS Claudine lint green and Level 2 green at 242/242, but no Windows/WSL
  Level 2 or final-tree remote/hosted execution.
- GitNexus `detect_changes` reports 26 changed indexed symbols in five files,
  no affected execution processes, and low indexed risk. Its symbol index does
  not establish the source scanner's macro-expansion completeness.
