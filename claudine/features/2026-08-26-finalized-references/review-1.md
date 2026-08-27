---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-08-27T09:23:50+01:00
spec: 2026-08-26-finalized-references/spec.md
implemented: false
description: A **feature** review of `2026-08-26-finalized-references/spec.md`
feature: 2026-08-26-finalized-references/review-1.md
---

# Review 1: Finalized References

## Verdict

The feature is **not ready for production**.

The implementation covers most of the finalized reference grammar and the
Claudine/Darkmatter integration surfaces, including real-terminal compose,
proxy, sequence, and completion round trips. One production blocker and one
documentation issue remain: the parser deliberately accepts the Windows drive-relative form `C:` as an
absolute path despite the specification's rejection rule, and the harness API
documentation describes an obsolete `@` root order. The incomplete
Linux/Windows validation matrix (Finding 2) is a non-blocker: it will be
covered by CI/CD.

## Findings

### 1. High — Bare `C:` is accepted as absolute instead of rejected as drive-relative

`is_absolute_reference` contains a special case that classifies a two-byte
drive designator as absolute
(`biscuit-file/lib/src/file_reference/parse.rs:214-218`). The feature's public
grammar test explicitly locks that behavior in
(`biscuit-file/lib/tests/reference_grammar.rs:142-148`). This conflicts with
D9/D10 and the Non-goals section, which require unsupported Windows
drive-relative forms to be rejected rather than acquiring process-dependent
meaning.

Bare `C:` is not a drive-absolute Windows path; it refers to the current
directory on drive C. Treating it as `FileReferenceKind::Absolute` therefore
reintroduces the mutable process-working-directory dependency that the feature's
CWD model is intended to eliminate. It can also bypass the unsupported-scheme
diagnostic applied to `C:path`, even though both forms belong to the same
drive-relative namespace.

Required change: remove the two-byte `C:` absolute special case, classify it
through the unsupported-scheme guard, and invert the test to require
`FileReferenceError::UnsupportedScheme { scheme: "C", .. }`. Keep only
`C:/...` and `C:\\...` as drive-absolute forms. Add `C:` to the host-independent
drive-relative matrix so the same grammar is proved on macOS, Linux, and
Windows.

Verification level: Level 1 parser coverage is appropriate because this is a
host-independent lexical contract. The current Level 1 test is present but
asserts the wrong behavior.

### 2. Non-blocker — The specification's required cross-platform gate is incomplete

**Status: non-blocking.** The remaining Linux, WSL, and native-Windows rows
will be exercised by the CI/CD pipeline rather than by a manual pre-merge run.

AC9 and AC10 require the complete `just test`, `just test-l2`, and `just lint`
matrix for biscuit-file, Darkmatter, and Claudine on macOS, Linux, WSL, and
native Windows. The implementation's own acceptance record reports:

- no completed area on `build-linux`;
- biscuit-file and Darkmatter green on WSL, but no completed Claudine test run;
- no completed native-Windows area because the host failed its free-space
  preflight; and
- no native-Windows execution of
  `repository_containment_rejects_an_external_junction`.

These are environmental blockers, not evidence of a functional failure, but
the acceptance criterion is explicit and the native-Windows junction/reparse
behavior cannot be inferred from portable parser tests. The feature therefore
cannot be marked ready until the exact final tree completes the missing rows.

Required change: rerun all three package-area gates on a healthy Linux builder,
finish Claudine on WSL, and run the full matrix on a native-Windows host that
passes the repository capacity preflight. Record the exact final commit/tree
and executed native-Windows junction test.

Verification level: Level 1 native-filesystem coverage is appropriate for
junction containment; Level 2 is appropriate for the specified real compose
and sequence surfaces. Level 3 is not applicable because the feature makes no
OS keyboard, mouse, paste, IME, or terminal-input-encoder claim. The gap is
missing execution evidence at the required platforms, not a request for Level
3 coverage.

### 3. Medium — Harness rustdoc describes the obsolete `@` candidate order

`resolve_harness_path` documents `@foo` as searching “repository root,
configured roots, home”
(`claudine/lib/src/harness/resolve.rs:38-45`). The finalized implementation
actually uses registered prepends, then intrinsic package root, package-area
root, repository root, home, and registered appends. The module was modified by
this feature, so leaving the prior description violates the repository's
behavior-change documentation discipline and can send future callers toward
the wrong collision assumptions.

Required change: describe the effective D6 order, including the distinction
between registered prepend/append roots and intrinsic roots. Review the nearby
system-prompt and lifecycle rustdoc for the same terminology; their current
short “magic-root search” wording is not incorrect, but a link to the shared
ordering authority would reduce future drift.

Verification level: documentation/source review is sufficient; no runtime tier
applies.

## Requirement Verification Levels

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| AC1 grammar, candidate order, misses, remote typing, and typed parse errors | Level 1 biscuit-file unit/integration tests | Broadly appropriate, but `C:` asserts behavior contrary to D9/D10 (Finding 1). |
| AC2 implicit CWD-first behavior for documents and caller parameters | Level 1 conflict fixtures plus Level 2 tmux compose/proxy capture | Appropriate on the macOS host. |
| AC3 removed `!` sigil and provenance enum migration | Level 1 parser and source-inventory tests | Appropriate. |
| AC4 source-scope derivation, topology projection, and no ambient discovery | Level 1 topology/work-counter/parity tests plus Level 2 compose coverage | Appropriate; full OS matrix remains incomplete under Finding 2. |
| AC5 caller materialization across direct/proxy/re-entry/loop/sequence routes | Level 1 schema/orchestration matrices plus Level 2 proxy and sequence captures | Appropriate. |
| AC6 `ctx.cwd` and `AGENT_CWD` propagation | Level 1 process/integration tests and a non-vacuous spawn inventory | Appropriate; terminal encoding is not involved. |
| AC7 magic conventions and effective collision order | Level 1 collision/deduplication tests plus Level 2 completion execution | Runtime coverage is appropriate; rustdoc is stale (Finding 3). |
| AC8 completion/execution parity | Level 1 completion round trips plus Level 2 tmux completion-to-compose execution | Appropriate. |
| AC9 Windows/UNC/junction/reparse behavior | Host-independent Level 1 parser tests; native-Windows junction test exists but was not executed | **Gap:** required native verification is absent (Finding 2). |
| AC10 package-area validation matrix | macOS L1/L2/lint reported green; WSL partial; Linux and native Windows incomplete | **Gap:** acceptance criterion not satisfied (Finding 2). |
| AC11 repository containment | Level 1 lexical, symlink, deepest-ancestor, and platform-gated junction tests | Correct level, but native-Windows evidence is missing. |
| AC12 passive/public contracts | Level 1 validation, corpus, exhaustiveness, and CLI integration tests | Appropriate. |
| AC13 ratification/document alignment | Level 1 grammar diagnostics and document review | Appropriate except for the `C:` contradiction. |
| OS keyboard/mouse/paste/IME/hotkey behavior | No Level 3 tests | Not applicable; no such behavior is specified. |

## Verification Performed

- Read the full specification, acceptance matrix, consumer audit, four feature
  commits, and the principal biscuit-file, Darkmatter, and Claudine
  implementation/test surfaces.
- Used GitNexus concept search and the feature's recorded upstream impact audit
  to trace resolution, source derivation, materialization, and child-environment
  consumers. The worktree-local symbol context index was unavailable, so source
  inspection and compiler/source inventories were used for those details.
- Ran the focused Level 1 test
  `supported_absolute_and_explicit_filename_escape_hatches_are_preserved` with
  Nextest: 1 passed. Its pass confirms that the current suite intentionally
  accepts `C:` and therefore does not satisfy the specification.
- Did not rerun Level 2 windows during review. The committed acceptance record
  reports macOS background-terminal success and the incomplete remote matrix
  described in Finding 2.
- Preserved the pre-existing unrelated modification to
  `.claudine/memory/commits.md`. No formatting or Git commit was performed.

## Production Readiness Closure

Production readiness requires rejecting bare `C:` consistently with the final
grammar and correcting the stale harness ordering documentation. The Linux,
WSL Claudine, and native-Windows validation rows (Finding 2) are deferred to
CI/CD and do not block. No Level 3 test is required for this feature.
