---
$schema: feature-review.yaml
ready: false
agent: codex/gpt-5.6-sol
created: 2026-09-08T08:56:26-07:00
spec: 2026-08-26-finalized-references/spec.md
implemented: false
description: A **feature** review of `2026-08-26-finalized-references/spec.md`
feature: 2026-08-26-finalized-references/review-7.md
previous: 2026-08-26-finalized-references/review-6.md
---

# Review 7: Finalized References

## Verdict

The implementation is not ready for production. Review 6's constructor-value
handoff is now detected and all 26 Level 1 spawn-inventory tests pass, but the
new macro analysis still has silent false negatives. In particular, it resolves
literal names in a `macro_rules!` transcriber in the definition module even
when Rust resolves those names at the invocation site. AC10 also remains unmet:
the required Windows and WSL Level 2 cells do not exist, and the current macOS
Claudine Level 2 evidence is 241/242 rather than green.

## Findings

### High — AC6's macro inventory still silently misses executable constructions

`FileScanner::scan_macro_rules` in
`claudine/cli/tests/spawn_inventory.rs` lines 1374–1405 rewrites each
transcriber once and analyzes it using `self.location`, the module containing
the macro definition. The module contract at lines 47–54 makes the same
definition-site assumption. That is not Rust's resolution model for ordinary
non-local identifiers in a `macro_rules!` expansion: the invocation scope can
supply the item named by a literal transcriber token.

This valid Rust 2021 program executes a child:

```rust
macro_rules! launch {
    () => { Command::new("true").status().unwrap() };
}

mod call_site {
    use std::process::Command;

    pub fn run() {
        launch!();
    }
}

fn main() {
    call_site::run();
}
```

A review probe passed that source to the current `scan_source`. The analyzer
reported no sites at all: `(0 governed, 0 uncontrolled)`. Compiling and running
the same program with `rustc --edition=2021` succeeded and executed the child.
The existing macro fixture at lines 2829–2865 cannot expose this because it
imports `Command` into the same module that defines every macro. The documented
limit at lines 73–76 also leaves macro expansions from workspace dependencies
or third-party crates entirely invisible when their invocation arguments do not
themselves contain a constructor function item.

These are silent false negatives, so the inventory is not the fail-closed guard
required by D8.7 and AC6. The fix should inventory compiler-expanded or
semantically resolved code, or model each reachable invocation with Rust's
actual macro hygiene and expansion context. Add a Level 1 cross-module fixture
where only the invocation module imports `std::process::Command`, plus coverage
for externally defined declarative/procedural macros that can emit process
construction. If generated constructions are intentionally excluded, narrow
AC6 explicitly; the current specification says any production construction.

Verification level: Level 1 is appropriate for this source-inventory invariant.
The gap is semantic coverage, not terminal behavior.

### High — AC10's required final platform matrix remains incomplete

AC10 explicitly requires `just test`, `just test-l2`, and `just lint` in the
biscuit-file, Darkmatter, and Claudine package areas on local macOS,
`build-linux`, `build-win`, and `build-win-native` before hosted CI. The current
evidence still does not meet that contract:

- `.github/workflows/_package-ci.yml` lines 573–576 says Windows and
  `wsl2-ubuntu` are absent from the Level 2 matrix by construction.
- `.github/ci/environments.json` lines 34–42 and 107–115 still mark the native
  Windows and WSL tmux capabilities unavailable. A policy-gap record is not a
  real-terminal test execution.
- The latest final-tree attempt recorded in the implementation log reports the
  Claudine Level 2 suite at 241/242. Its WezTerm failure is attributed to the
  host Atuin `?` binding, but a required red gate is not passing evidence.
- The same log reports eight current Claudine Level 1 failures (one caused by a
  concurrent untracked test and seven by generated-provider drift) and no
  hosted CI run for the uncommitted reviewed tree. Those failures may be
  unrelated to finalized references, but AC10 requires green gates, not merely
  attribution.

Required change: provision and execute non-vacuous Level 2 coverage on Windows
and WSL, remove or isolate the WezTerm startup interference without taking
focus, restore all named Level 1 gates, and record a complete green matrix for
the final tree.

Verification level: AC10 requires Level 1 and Level 2 evidence. Level 3 is not
applicable because this feature does not depend on a terminal's keyboard,
mouse, paste, or IME encoder.

## Requirement Verification Matrix

| Requirement | Strongest evidence reviewed | Assessment |
|---|---|---|
| AC1 — grammar and parser contract | Level 1 parser, resolver, and CLI fixtures | Appropriate and recorded green. |
| AC2 — implicit reference precedence | Level 1 collision fixtures plus recorded Level 2 compose/proxy coverage | Appropriate and recorded green. |
| AC3 — remove legacy `!` grammar | Level 1 grammar, compiler, and inventory checks | Appropriate and recorded green. |
| AC4 — reference-owned scopes and topology | Level 1 catalog/topology/work-counter checks plus recorded Level 2 nested composition | Appropriate and recorded green. |
| AC5 — materialization and provenance | Level 1 schema/orchestration matrix plus recorded Level 2 proxy/sequence coverage | Appropriate and recorded green. |
| AC6 — `ctx.cwd` and `AGENT_CWD` | Level 1 context/subprocess tests and 26 passing spawn-inventory tests | Correct tier, but the guard silently omits valid invocation-scoped and external macro expansions; see the first finding. |
| AC7 — magic conventions preserved | Level 1 collision/deduplication fixtures plus recorded Level 2 compose checks | Appropriate and recorded green. |
| AC8 — completion/execution parity | Level 1 completion/execution tests plus recorded Level 2 magic-reference checks | Appropriate; no terminal input encoder behavior is asserted. |
| AC9 — cross-platform syntax | Level 1 host-independent parser checks and native filesystem fixtures, including recorded Windows junction execution | Appropriate; final platform closure remains governed by AC10. |
| AC10 — final quality gates | Historical green Level 1/lint evidence; Linux/macOS Level 2 evidence; current macOS Claudine L2 241/242; no Windows/WSL L2 or final-tree hosted CI | Not satisfied; see the second finding. |
| AC11 — containment and junction safety | Level 1 lexical, symlink, deepest-ancestor, completion, and native Windows junction tests | Appropriate and recorded green. |
| AC12 — passive/public contracts and docs | Level 1 passive/public/corpus checks, real CLI paths, and documentation review | Appropriate and recorded green. |
| AC13 — reserved syntax | Level 1 grammar checks and design-document review | Appropriate and recorded green. |

No requirement asserts modifier-press visibility, hotkey activation, paste,
IME, mouse behavior, or another encoder-sensitive interaction requiring Level
3 OS input injection.

## Verification Performed

- `cargo nextest run -p claudine-cli --test spawn_inventory --color=never`
  passed 26/26 tests on macOS, including Review 6's new handoff and local-macro
  fixtures and the production inventory scan.
- A temporary Level 1 analyzer probe for the invocation-scoped macro above
  failed as predicted: the current scanner returned no command record. The
  probe was removed after execution and did not alter the implementation diff.
- The identical Rust 2021 source compiled and ran successfully with `rustc`,
  confirming that the missed expansion executes `std::process::Command`.
- AC10 was checked against the current workflow matrix, environment capability
  catalog, and the implementation log's final-tree L1/L2/hosted-CI evidence.
