---
ready: false
agent: codex/default
created: 2026-07-10T14:29:09
---

# Review 2 — Godless Beauty

## Verdict

Not ready for production. The behavioral fixes reviewed here pass focused tests, but Improvement 6
does not implement the required context-capture decomposition. The implementation and its own plan
and closeout explicitly leave that work unfinished.

## Findings

### High — Improvement 6 leaves the god-file in place behind placeholder modules

The specification requires capture orchestration and population to move into the domain modules,
with `capture/mod.rs` reduced to the facade and sequencing layer. Instead,
`capture/mod.rs` remains 2,026 lines and still owns `ContextGroup`, demand scanning,
`ContextCapture`, `ContextCapture::new`, every `populate_*` implementation, test constructors, and
most capture tests. The requested destination modules are mostly key-list placeholders:
`agent.rs`, `docs.rs`, and `snapshot.rs` contain one line each; the remaining population-domain
files contain only 3–11 lines, apart from `groups.rs`.

This is not merely a preferred follow-up refactor. The central goal of this specification is to
remove god-files and establish one obvious authority per behavior. Leaving the behavior in
`mod.rs` means the capture god-file, its mixed dependency ownership, and its inline test ballast
remain. It also contradicts the definition of done that no new production file merely replace a
god-file. The execution plan accurately leaves the relevant Phase 5 tasks and validation checkpoint
unchecked, and the closeout acknowledges the deferred structure, but Phase 7 is nevertheless marked
complete.

Complete the specified moves before closing the fix:

- move `ContextGroup`, demand scanning, and key lookup into `groups.rs`;
- move `ContextCapture` and probe orchestration into `snapshot.rs`;
- move each population implementation and its tests into its owning domain module;
- retain only the public(crate) facade and explicit population sequencing in `mod.rs`; and
- compare the 15-test pre-move capture inventory with the post-move inventory, then run the focused
  nextest suite and the Phase 5 check/test/lint checkpoint.

## Requirement-to-verification assessment

| Requirement | Strongest evidence | Assessment |
|---|---|---|
| UTF-8-safe shared link/image parsing | L1 unit regressions for UTF-8 titles, ASCII-case-insensitive attributes, nesting, escapes, and malformed input | Appropriate; focused review run passed |
| GPU-only `ctx.gpu` population without hardware capture | L1 injected-capture regression | Appropriate; focused review run passed |
| No relevant `ctx.*` performs datetime-only work | L1 in-process regression | Appropriate; focused review run passed |
| Preserve terminal rendering bytes and real-terminal behavior | L2 render-tree target, reported passing at closeout with its 20-test inventory preserved | Appropriate for rendering; no physical-keyboard behavior is specified, so L3 is not applicable |
| Mechanical test relocation preserves inventory and gates | L1 inventories plus L2 target inventory recorded in phase artifacts | Appropriate for the completed relocations |
| Split context capture into domain-owned modules and move owning tests | Source inspection; Phase 5 plan tasks remain unchecked | Failed implementation requirement; no test level can substitute for the missing structure |

## Verification performed for this review

- Focused nextest selection: 8 passed, covering shared reference scanners, UTF-8 preservation,
  HTML attribute casing, GPU-only capture, datetime-only demand behavior, and capture ownership
  invariants.
- `cargo check -p darkmatter -p darkmatter-cli -p dmls -p claudine -p claudine-cli`: passed.
- Source-layout inspection confirmed the incomplete Improvement 6 split described above.

