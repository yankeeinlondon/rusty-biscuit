---
status: draft
created: 2026-08-14
reviewed: false
area: claudine
depends-on: ../2026-08-13-finalize/spec.md
evidence: ../2026-08-13-finalize/failing.md
packages:
    - darkmatter
    - darkmatter-cli
    - claudine-cli
    - biscuit-terminal-cli
---

# Attribute and clear the 24 failing cells of run 31753281913

## Summary

CI run `31753281913` on `fix/ctx-launch-anchor` at commit `a00ea7c08` failed 24
of 601 jobs, carrying 51 distinct failing test identities. The full catalog is
[`failing.md`](../2026-08-13-finalize/failing.md).

The launch-anchor branch is otherwise finished: its own Level-1 suites pass on a
native Windows 11 host (6050/6050 for Claudine, 7542/7542 for Darkmatter), on
macOS, and on a local WSL2 host, and the composition latency work has two
consecutive two-core Linux proofs. That evidence is **Level 1 only**, and six of
the failing jobs are Level 2. The gap between "green locally" and this run is
the subject of this fix.

Three populations are mixed together in those 24 cells:

- **P1 — branch-owned regressions.** Darkmatter on `wsl2-ubuntu` fails six
  tests clustered on interpolation and frontmatter shell expansion, including
  one this branch introduced. Darkmatter-cli on `windows-latest` fails a
  byte-identical output baseline. Both subsystems were changed by the
  launch-anchor work.
- **P2 — unattributed Level-2 rendering failures.** Eight identities across
  `claudine-cli`, `darkmatter-cli`, and `biscuit-terminal-cli` on macOS and
  Ubuntu. Terminal rendering is out of scope for the launch-anchor fix and the
  code-block colour-mode contract on CI tmux is already a recorded main-side
  follow-up, but neither claim has been checked against this run.
- **P3 — verified main drift.** `sniff`, `sniff-cli`, `rendezvous-daemon`,
  `unchained-ai`, `biscuit-tui-cli`, `dmls`, `messenger`, `model_id`,
  `biscuit-speaks-cli`, and `claudine` on `wsl2-ubuntu`.

> **Reader's note.** An earlier analysis in `ci-baseline-evidence.md` concluded
> that no proposed baseline cell hid a branch regression. That analysis diffed
> run `31651014023`, which predates every fix on this branch, and
> `darkmatter/wsl2-ubuntu` was never among its candidate cells. Its conclusion
> must not be reused without re-running the diff against `a00ea7c08` or later.

## Scope

**In scope:**

1. Attribute every P1 and P2 identity to a cause, on evidence rather than on
   the argument that a subsystem is out of scope.
2. Fix the P1 regressions this branch caused.
3. For P2, either fix, or demonstrate the identity is red on `main` and record
   the main-side handoff.

**Out of scope:** implementing P3 product fixes. `sniff` hermeticity and lints,
Neovim provisioning, the daemon security-descriptor assertions, the ConPTY
shutdown ordering, and the WSL2 environment assumptions remain main-side work.
This fix may record handoffs and may disposition those cells through the
baseline, but must not absorb their implementations.

## Fix specification

### F1 — Reproduce and attribute the Darkmatter WSL2 cluster

Six identities on `darkmatter/wsl2-ubuntu`:

- `interpolation_literal_pipeline::frontmatter_literal_survives_shell_bracketed_interpolation_passes`
- `markdown::compose::frontmatter_shell_expansion::tests::detects_no_cache_suffix`
- `markdown::compose::frontmatter_shell_expansion::tests::no_cache_combines_with_timeout_either_order`
- `markdown::compose::frontmatter_shell_expansion::tests::no_cache_defaults_false_without_suffix`
- `shell_expansion_coordinates::shell_block_execution_failed_renders_inner_diagnostic`
- `shell_expansion_coordinates::shell_block_origin_counts_lines_not_bytes_with_crlf`

The first is a test this branch added. This cell has **no prior evidence of
being red on main**, and Darkmatter was never run on the WSL2 host during the
launch-anchor work — only Claudine was.

Reproduce on the `build-win` WSL2 host, which reaches this configuration in
minutes rather than through a CI round trip. Then establish attribution by
running the same suite at `origin/main` on that host. A cluster that reproduces
on main is drift; one that does not is a regression this branch owns.

Three of the six concern `no_cache` suffix parsing and two concern shell-block
source coordinates including CRLF handling. Determine whether they share a
cause with the interpolation test or are independent before fixing anything.

**Acceptance.** Every identity is attributed with a recorded main-versus-branch
result. Branch-owned failures pass on the WSL2 host, and the fix does not
regress the native Windows, macOS, or Linux Level-1 suites.

### F2 — Resolve the Darkmatter-CLI Windows baseline contradiction

`schema_validate_baseline::schema_validate_legacy_pretty_output_is_byte_identical`
fails on `windows-latest`, while the same package passed 655/655 on a native
Windows 11 host at this commit.

One of those two observations does not mean what it appears to. Resolve the
contradiction before changing any code: confirm whether the local run actually
executed this test, whether the CI job differs in tier, feature flags, or
environment, and whether the baseline is environment-sensitive.

A byte-identical output baseline is precisely what a change to interpolation
escaping would move, so treat a genuine regression as the leading hypothesis
until the evidence rules it out.

**Acceptance.** The contradiction is explained in writing. If the baseline
legitimately changed, it is re-derived through its documented review workflow
rather than blessed. If the local run did not cover the test, the gap in local
verification is recorded so it is not repeated.

### F3 — Attribute the Level-2 rendering failures

Eight identities, none exercised by any local run because `just test` is
Level 1 only:

| Cell | Identity |
| --- | --- |
| `claudine-cli` macOS + Ubuntu | `level2_context_capture::level2_context_default_at_140_fills_cap_in_tmux` |
| `claudine-cli` macOS + Ubuntu | `level2_context_capture::level2_context_default_caps_at_140_in_wide_tmux` |
| `claudine-cli` macOS + Ubuntu | `level2_context_capture::level2_context_default_narrow_preserves_type_and_wraps_in_tmux` |
| `claudine-cli` Ubuntu | `level2_context_capture::level2_context_default_preserves_columns_at_min_width_in_tmux` |
| `darkmatter-cli` macOS | `level2_code_block_styling::level2_code_block_clears_inherited_dim_before_theme_colors` |
| `darkmatter-cli` Ubuntu | `level2_schema_about::level2_schema_about_light_terminal_uses_dark_code_theme` |
| `biscuit-terminal-cli` Ubuntu | `level2_diagrams::level2_diagram_fallback_when_no_image_protocol` |
| `biscuit-terminal-cli` macOS | `level2_apple_terminal_prose::level2_apple_terminal_double_underline_plain_text_visible` |

The context-capture group failing on **both** operating systems is a weak fit
for flake and must not be dismissed as one. Run the Level-2 suites locally for
the affected packages, then attribute each identity against `main`.

Level 2 must never focus or open a host terminal window.

**Acceptance.** Each identity is either fixed or shown red on `main` with a
recorded handoff. The reason Level 2 was not run during the launch-anchor work
is recorded, and the verification matrix of the parent fix is corrected so a
future branch does not repeat the omission.

### F4 — Re-run the identity diff before any baseline entry

The drafted baseline entries in `ci-baseline-evidence.md` rest on run
`31651014023`. Every failure that run recorded for the branch has since been
fixed, so the entries are stale.

Re-run the exact identity diff against the newest completed branch run at or
after `a00ea7c08` and the current `main` baseline run. Re-verify that each
proposed cell carries identical failing identities on both sides, and that no
new branch identity has appeared inside a cell that is already baselined.

The two lint cells remain a special case: a lint cell carries zero test
identities, so once baselined, a new lint error there is invisible to the gate
forever. Their main-side fixes are small.

**Acceptance.** No baseline entry is written from evidence predating
`a00ea7c08`. Every entry is paired with a fresh identity diff, and no cell is
accepted whose branch identities are a superset of main's.

## Phases

1. **Phase 1 — Attribution.** F1 and F3 reproduction on the WSL2 and Linux
   hosts, plus the F2 contradiction. Fix nothing yet; produce the
   main-versus-branch result for every P1 and P2 identity.
2. **Phase 2 — Branch-owned fixes.** Repair whatever Phase 1 attributes to this
   branch, re-verifying the parent fix's suites on native Windows, macOS,
   Linux, and WSL2 so a repair does not reopen a closed regression.
3. **Phase 3 — Handoffs.** Record a main-side handoff for every identity shown
   red on `main`.
4. **Phase 4 — Gate policy.** F4, then the full verdict and identity-aware diff.

## Verification matrix

- `build-win` (WSL2) — Darkmatter Level 1, at branch and at `main`.
- `build-win-native` (Windows 11) — Darkmatter and Claudine Level 1.
- `build-linux` — Level 1 and Level 2 for the affected packages. Note this host
  has at least two pre-existing terminal-rendering failures unrelated to any
  branch (`horizontal_rule_integration::test_custom_weight_thick_differs_from_thin`
  and the `group_framing` group), both confirmed red on `main`; attribute
  against `main` on the same host rather than against a green expectation.
- macOS — Level 2 for `claudine-cli`, `darkmatter-cli`, `biscuit-terminal-cli`.
- One full CI run, used as authoritative proof rather than as an iteration loop.

## Success criteria

1. Every one of the 51 identities in [`failing.md`](../2026-08-13-finalize/failing.md)
   is attributed to this branch or to `main`, with the evidence recorded.
2. No identity this branch owns remains red on any host.
3. The Level-1-only gap in the parent fix's verification is documented and its
   matrix corrected.
4. Any baseline entry rests on an identity diff at or after `a00ea7c08`.
5. No new red identity appears relative to `main`, including inside cells
   accepted by a baseline entry.

## Open questions

### 1. Does the Darkmatter WSL2 cluster share one cause?

Three `no_cache` suffix tests, two shell-block coordinate tests, and one
frontmatter-literal interpolation test fail together. They may share a root
cause in frontmatter shell expansion, or the interpolation test may be
independent and merely adjacent. Phase 1 must answer this before Phase 2 begins,
because a single shared fix and six separate ones carry very different risk.

### 2. Should Level 2 join the parent fix's acceptance gate?

The launch-anchor specification argued Level 2 adds no relevant assurance
because no requirement depends on terminal rendering. This run shows Level-2
cells failing on a branch that changed how values are escaped for display. If
the F3 attribution finds any Level-2 failure is branch-caused, that argument was
wrong and the gate should be widened.
