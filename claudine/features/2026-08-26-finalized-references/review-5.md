---
$schema: feature-review.yaml
ready: false
agent: codex/gpt-5.6-sol
created: 2026-09-08T06:27:23-07:00
spec: 2026-08-26-finalized-references/spec.md
implemented: false
description: A **feature** review of `2026-08-26-finalized-references/spec.md`
feature: 2026-08-26-finalized-references/review-5.md
previous: 2026-08-26-finalized-references/review-4.md
---

# Review 5: Finalized References

## Verdict

The implementation is not ready for production. Review 4's three named
spawn-inventory bypasses are fixed and their Level 1 regression fixtures pass,
but AC6's guard still fails open for other valid Rust alias forms. AC10 also
remains unmet: native Windows and WSL have no Level 2 execution, the local
macOS Claudine Level 2 gate is red because a host startup prompt captures its
WezTerm pane, and the final uncommitted tree has no CI run.

## Findings

### High — AC6's spawn inventory still misses valid aliased constructions

AC6 requires the guard to fail on any production `std::process::Command` or
`tokio::process::Command` construction whose child can execute without the
shared environment helper, including aliased constructions. The review-4
implementation fixes `cfg` truth evaluation, direct type aliases, and local
module shadowing, but the analyzer still identifies constructors from a closed
set of source spellings rather than resolved Rust items:

- `collect_use_tree` and `record_alias` in
  `claudine/cli/tests/spawn_inventory.rs` lines 501–535 recognize only imports
  whose written path directly names `std` or `tokio`. A named re-export such as
  `mod process_api { pub use std::process::Command as ProcessCommand; }`,
  followed by `use process_api::ProcessCommand`, contributes no command alias.
  `ProcessCommand::new("x").status()` is therefore absent from the census.
- `FunctionAnalyzer::call` at lines 782–815 recognizes construction only when
  the called path itself ends in a type path accepted by
  `kind_for_constructor`. The valid function-item alias
  `let construct = std::process::Command::new; let mut command =
  construct("x"); command.status();` creates and executes a child without
  contributing a census entry.
- The bounded glob resolver has a hard depth of three at lines 320–370. A
  longer valid chain of local glob re-exports can therefore lose the same
  constructor identity and omit a bare `Command::new` site.

Both of the first two counterexamples compile as Rust 2021. These are silent
false negatives, not conservative `UNCONTROLLED` results. The current generated
inventory remains fully governed, so this does not demonstrate an ungoverned
production child today; it demonstrates that the required regression barrier
can still be bypassed by ordinary future Rust code.

Required change: make constructor/helper identity resolution semantic rather
than continuing to enumerate syntax spellings, for example with a compiler/HIR
or Clippy-based check. If the syntax analyzer is retained, it must at least
resolve named re-export chains and constructor function-item aliases and remove
the depth-based false-negative boundary. Add non-vacuous Level 1 fixtures in
which governed and deliberately ungoverned variants produce different census
results for each form.

Verification level: Level 1 is appropriate for this source-inventory
invariant. The gap is incomplete semantic coverage, not missing terminal
emulation.

### High — AC10's final platform matrix remains incomplete

AC10 explicitly requires `just test`, `just test-l2`, and `just lint` in
biscuit-file, Darkmatter, and Claudine on local macOS, native Linux, WSL, and
native Windows. The recorded evidence is materially strong but still does not
satisfy that contract:

- CI run `34192299897` on `ebc28e107` is green for Level 1 and lint on all four
  named platforms and for Level 2 on Linux and macOS. Native Windows and WSL
  Level 2 remain absent by construction in
  `.github/workflows/_package-ci.yml` lines 573–576. The capability records in
  `.github/ci/environments.json` explicitly describe both as policy gaps, and
  Ken's 2026-09-08 ruling says those records are not exclusions from AC10.
- The final reviewed tree contains the uncommitted spawn-inventory changes, so
  no hosted CI run verifies that exact tree.
- The latest recorded local macOS Claudine Level 2 run still has one failure:
  `level2_initialize_proxy_block_auto_detects_osc8_in_wezterm`. The captured
  pane shows an Atuin first-run prompt consuming the injected shell input. This
  is a host/harness condition rather than a finalized-reference defect, but a
  red or blocked required gate is not passing evidence.

Required change: provision genuine Level 2 execution on native Windows and
WSL, remove the local WezTerm startup interference without taking focus, and
record the complete green matrix against the final tree. Backend installation
or a zero-test job is not evidence; the Level 2 backend-proof mechanism must
record actual execution.

Verification level: AC10 itself requires Level 1 and Level 2 evidence. Level 3
is not applicable because no requirement depends on the terminal's input
encoder or OS keyboard/mouse injection.

## Requirement Verification Matrix

| Requirement | Strongest evidence reviewed | Assessment |
|---|---|---|
| AC1 — grammar and parser contract | Level 1 parser, resolver, and CLI fixtures | Appropriate and recorded green. |
| AC2 — implicit reference precedence | Level 1 collision fixtures plus recorded Level 2 compose/proxy coverage | Appropriate and recorded green. |
| AC3 — remove legacy `!` grammar | Level 1 grammar, compiler, and inventory checks | Appropriate and recorded green. |
| AC4 — reference-owned scopes and topology | Level 1 catalog/topology/work-counter checks plus recorded Level 2 nested composition | Appropriate and recorded green. |
| AC5 — materialization and provenance | Level 1 schema/orchestration matrix plus recorded Level 2 proxy/sequence coverage | Appropriate and recorded green. |
| AC6 — `ctx.cwd` and `AGENT_CWD` | Level 1 context/subprocess tests and 18 passing spawn-inventory tests | Correct tier, but the guard still omits valid aliased constructions; see the first finding. |
| AC7 — magic conventions preserved | Level 1 collision/deduplication fixtures plus recorded Level 2 compose checks | Appropriate and recorded green. |
| AC8 — completion/execution parity | Level 1 completion/execution tests plus recorded Level 2 magic-reference checks | Appropriate; no input-encoder behavior is asserted. |
| AC9 — cross-platform syntax | Level 1 host-independent parser checks and native filesystem fixtures, including recorded Windows junction execution | Appropriate; final platform closure remains governed by AC10. |
| AC10 — final quality gates | Green Level 1/lint on four platforms at `ebc28e107`; green Linux/macOS Level 2; red local macOS Level 2 and no Windows/WSL Level 2 or final-tree CI | Not satisfied; see the second finding. |
| AC11 — containment and junction safety | Level 1 lexical, symlink, deepest-ancestor, completion, and native Windows junction tests | Appropriate and recorded green. |
| AC12 — passive/public contracts and docs | Level 1 passive/public/corpus checks, real CLI paths, and manual documentation review | Appropriate and recorded green. |
| AC13 — reserved syntax | Level 1 grammar checks and design-document review | Appropriate and recorded green. |

No requirement asserts modifier-press visibility, hotkey activation, paste,
IME, mouse behavior, or another encoder-sensitive interaction requiring Level
3 OS input injection.

## Verification Performed

- `cargo nextest run -p claudine-cli --test spawn_inventory --color=never`
  passed 18/18 tests on macOS. This proves the analyzer handles its current
  fixtures and the current production source; it does not exercise the alias
  paths in the first finding.
- A Rust 2021 compiler check accepted both a constructor imported through a
  named local re-export and a constructor stored in a local function item,
  confirming that the omitted forms are valid Rust rather than hypothetical
  syntax.
- The analyzer's alias collection, glob-depth boundary, constructor matching,
  and generated inventory were inspected directly. Review 4's three requested
  regression fixtures are present and non-vacuous.
- AC10 was compared with the implementation log, the CI environment capability
  catalog, and the workflow's Level 2 matrix construction. Earlier green runs
  are treated as recorded evidence, not as verification of the final
  uncommitted tree.

