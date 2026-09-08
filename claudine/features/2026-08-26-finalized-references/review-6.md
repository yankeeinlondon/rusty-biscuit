---
$schema: feature-review.yaml
ready: false
agent: codex/gpt-5.6-sol
created: 2026-09-08T07:34:14-07:00
spec: 2026-08-26-finalized-references/spec.md
implemented: false
description: A **feature** review of `2026-08-26-finalized-references/spec.md`
feature: 2026-08-26-finalized-references/review-6.md
previous: 2026-08-26-finalized-references/review-5.md
---

# Review 6: Finalized References

## Verdict

The implementation is not ready for production. Review 5's named re-export,
local constructor-binding, and unbounded-glob cases are now covered and the
22-test Level 1 spawn-inventory suite passes. However, AC6 requires the guard
to fail for every executable process-command construction, and the retained
syntax analyzer still silently loses constructor identity when it crosses a
function boundary or is introduced by a macro expansion. AC10 also remains
unmet: Windows and WSL have no Level 2 execution, the current macOS Claudine
Level 2 run is red in WezTerm, and no hosted run verifies the final uncommitted
tree.

## Findings

### High — AC6's syntax inventory still has silent construction bypasses

The new resolver closes all three forms named by Review 5, but its documented
analysis boundary is weaker than AC6's invariant. The module documentation in
`claudine/cli/tests/spawn_inventory.rs` lines 56–58 explicitly says a
constructor function item is followed only when a local binding is called in
the same function. `FunctionAnalyzer::argument` at lines 1070–1080 evaluates a
constructor path as an ordinary expression; it does not pass constructor
identity to the callee. The callee's parameters are likewise absent from
`FlowState::constructors`.

Consequently, this valid Rust 2021 pattern executes a child while contributing
no command construction to the inventory:

```rust
use std::process::Command;

fn invoke<F>(construct: F)
where
    F: Fn(&'static str) -> Command,
{
    construct("true").status().unwrap();
}

fn launch() {
    invoke(Command::new::<&'static str>);
}
```

The caller contains only a function-item argument and the callee contains only
a call through an unclassified parameter. A compiler probe confirmed this
program is accepted. The same scanner also does not expand macros:
`FunctionAnalyzer::macro_tokens` at lines 1091–1101 parses only invocation
arguments. A zero-argument `macro_rules!` expansion containing
`std::process::Command::new("true").status()` is valid, executes a child, and
has no expression for the census to observe.

These are silent false negatives, not conservative `UNCONTROLLED` results.
There is no evidence that current production source uses either form, but AC6
defines a future regression barrier and requires any production construction
to fail closed. Documenting a bypass does not satisfy that contract.

Required change: use a semantic/compiler-expanded inventory, or extend the
analyzer so constructor identity cannot disappear through function-item
handoffs and macro expansion. Add non-vacuous Level 1 fixtures whose governed
and deliberately ungoverned variants produce distinct census results for both
forms. If the intended contract excludes generated or cross-function
constructions, ratify that narrower contract in the specification rather than
leaving the guard described as exhaustive.

Verification level: Level 1 is appropriate for this source-inventory
invariant. The gap is semantic coverage, not terminal behavior.

### High — AC10's required final platform matrix remains incomplete

AC10 explicitly requires `just test`, `just test-l2`, and `just lint` in the
biscuit-file, Darkmatter, and Claudine package areas on macOS, native Linux,
WSL, and native Windows. Review-cycle 5 refreshed useful evidence, but the
criterion is still not satisfied:

- `.github/workflows/_package-ci.yml` lines 573–576 still states that Windows
  and WSL are absent from the Level 2 matrix by construction.
- `.github/ci/environments.json` still records no provisionable Windows Level
  2 terminal backend and no WSL tmux/broker execution. A policy-gap record is
  not test execution.
- The current macOS Claudine Level 2 run is 241/242. The failing WezTerm case is
  blocked by the host's Atuin `?` binding consuming the `$?` exit-marker input.
  That is a host/harness condition rather than a file-reference defect, but a
  required red gate is not passing evidence.
- The reviewed tree remains uncommitted, so the latest green hosted CI run does
  not verify the spawn-inventory implementation reviewed here.

Required change: provision real Level 2 execution on Windows and WSL, remove
the WezTerm startup interference without taking focus, and record the complete
green matrix against the final tree. Backend installation or a zero-test job
does not satisfy the criterion; backend-proof must record actual execution.

Verification level: AC10 requires Level 1 and Level 2 evidence. Level 3 is not
applicable because finalized references do not depend on the terminal input
encoder or OS keyboard/mouse injection.

## Requirement Verification Matrix

| Requirement | Strongest evidence reviewed | Assessment |
|---|---|---|
| AC1 — grammar and parser contract | Level 1 parser, resolver, and CLI fixtures | Appropriate and recorded green. |
| AC2 — implicit reference precedence | Level 1 collision fixtures plus recorded Level 2 compose/proxy coverage | Appropriate and recorded green. |
| AC3 — remove legacy `!` grammar | Level 1 grammar, compiler, and inventory checks | Appropriate and recorded green. |
| AC4 — reference-owned scopes and topology | Level 1 catalog/topology/work-counter checks plus recorded Level 2 nested composition | Appropriate and recorded green. |
| AC5 — materialization and provenance | Level 1 schema/orchestration matrix plus recorded Level 2 proxy/sequence coverage | Appropriate and recorded green. |
| AC6 — `ctx.cwd` and `AGENT_CWD` | Level 1 context/subprocess tests and 22 passing spawn-inventory tests | Correct tier, but the guard silently omits valid cross-function and macro-generated constructions; see the first finding. |
| AC7 — magic conventions preserved | Level 1 collision/deduplication fixtures plus recorded Level 2 compose checks | Appropriate and recorded green. |
| AC8 — completion/execution parity | Level 1 completion/execution tests plus recorded Level 2 magic-reference checks | Appropriate; no input-encoder behavior is asserted. |
| AC9 — cross-platform syntax | Level 1 host-independent parser checks and native filesystem fixtures, including recorded Windows junction execution | Appropriate; final platform closure remains governed by AC10. |
| AC10 — final quality gates | Green Level 1/lint evidence on the named platforms from earlier trees; green Linux/macOS Level 2 evidence; current macOS Claudine L2 241/242; no Windows/WSL L2 or final-tree CI | Not satisfied; see the second finding. |
| AC11 — containment and junction safety | Level 1 lexical, symlink, deepest-ancestor, completion, and native Windows junction tests | Appropriate and recorded green. |
| AC12 — passive/public contracts and docs | Level 1 passive/public/corpus checks, real CLI paths, and documentation review | Appropriate and recorded green. |
| AC13 — reserved syntax | Level 1 grammar checks and design-document review | Appropriate and recorded green. |

No requirement asserts modifier-press visibility, hotkey activation, paste,
IME, mouse behavior, or another encoder-sensitive interaction requiring Level
3 OS input injection.

## Verification Performed

- `cargo nextest run -p claudine-cli --test spawn_inventory --color=never`
  passed 22/22 tests on macOS, including Review 5's four new fixtures and the
  production inventory scan.
- Rust 2021 compiler probes accepted both a cross-function, explicitly
  instantiated `Command::new` handoff and a zero-argument macro expansion that
  constructs and executes a command.
- The analyzer's expression, argument, local-binding, and macro handling was
  inspected directly. Both counterexamples disappear before a `CommandRecord`
  can be created.
- AC10 was checked against the implementation log, current workflow matrix,
  environment capability catalog, and recorded macOS Level 2 result. Earlier
  green runs are treated as historical evidence, not verification of the final
  uncommitted tree.
