---
$schema: feature-review.yaml
ready: true
agent: codex/default
created: 2026-07-31T22:11:24-07:00
spec: 2026-07-31-portable-strings/spec.md
implemented: true
description: "A third **feature** review of `2026-07-31-portable-strings/spec.md`"
feature: 2026-07-31-portable-strings/review-3.md
previous: 2026-07-31-portable-strings/review-2.md
---

# Review 3

## Verdict

Ready for production. No implementation or test-coverage findings remain in
the portable-strings scope. All four Review 2 findings are resolved, and the
focused native-Windows suites and affected package-area lint gates pass.

Per the direction carried forward from Review 2, the separately specified
file-transclusion error policy and visible failure notice remain intentional
and are not reviewed as portable-strings findings.

## Findings

None.

## Prior Findings Rechecked

- **Resolved — WSL archive-mode gate:** `test` now forwards its arguments to
  `test-minimal`, and `test-minimal` skips only when it sees `--archive-file` in
  either separated or `--archive-file=...` form. Direct recipe probes confirmed
  both spellings skip without invoking Cargo, including a filterset-like extra
  argument. Ordinary native execution still runs the dedicated
  `--no-default-features` Cargo command.
- **Resolved — non-Windows lint:** `OsStr` is imported only under
  `#[cfg(windows)]`, while the rustdoc reference uses the fully qualified name.
  Both Biscuit-file and Darkmatter area lint recipes pass with their `-D
  warnings` policy.
- **Resolved — UTF-16 component length:** `survives_namespace_removal` now
  measures `name.encode_utf16().count()` and rejects values above 255 units.
  The boundary matrix distinguishes ASCII, multi-byte BMP, and astral text at
  255/256 units, including the 128-emoji regression that the former scalar-value
  count accepted incorrectly.
- **Resolved — anchored unsafe-category coverage:** repository, home, and
  environment anchor tests drive every document-reachable unsafe category
  through `normalize_links` and assert byte-identical preservation plus a
  warning. Each arm also has an over-`MAX_PATH` success control, demonstrating
  that the guard is not a blanket rejection of declined paths. The non-Unicode
  case remains correctly helper-level because authored document text cannot
  contain an unpaired surrogate.

Review 1's structured, non-lossy comparison-key corrections also remain in
place. The key retains raw `OsString` components, does not re-enter `PathBuf`
for prefix comparison, keeps distinct unpaired-surrogate identities distinct,
and exercises legacy/verbatim UNC equivalence through `starts_with`,
`strip_prefix`, and relative computation. The durable minimal-feature gate,
consumer-level completion test, and full transclusion regression remain present.

## Requirement Verification

| Requirement | Strongest verification reviewed | Assessment |
| --- | --- | --- |
| Public unfeatured path-to-text API and unconditional `dunce` | No-default-features unit suite | Passing |
| Preserve declined Windows namespace spellings | Direct renderer matrix | Passing |
| Non-lossy Windows comparison identity | Native-Windows key and surrogate tests | Passing |
| Safe anchoring of declined descendants | Repository/home/environment end-to-end matrices | Passing |
| UTF-16 component-limit enforcement | 255/256-unit ASCII, BMP, and astral matrix | Passing |
| Finalization preserve-and-warn behavior | `normalize_links` consumer tests | Passing |
| Inline-Pre and `link()` decline errors | Transclusion integration and expression-function suites | Passing |
| Completion portable/native separator consistency | Enumerating completion entry-point test | Passing |
| `bf reference` portable stdout | CLI integration tests | Passing |
| Affected package-area lint policy | Biscuit-file and Darkmatter `just lint` | Passing |

## Verification Performed

- `cargo test -p biscuit-file --no-default-features --lib path_text --color=never`
  — 10 passed.
- `cargo test -p darkmatter --lib markdown::compose::link_normalization --color=never -j 1`
  — 23 passed.
- `cargo test -p darkmatter --test declined_path_transclusion --color=never -j 1`
  — 3 passed.
- `cargo test -p darkmatter-cli --lib args::completion --color=never -j 1`
  — 8 passed.
- `cargo test -p darkmatter --lib markdown::compose::expression::functions::tests --color=never -j 1`
  — 153 passed.
- `cargo test -p biscuit-file-cli --test cli_tests reference_ --color=never -j 1`
  — 9 passed.
- `just lint` in `biscuit-file/` — passed for the library and CLI.
- `just lint` in `darkmatter/` — passed for the library, CLI, and DMLS.
- Direct `just test-minimal` archive-mode probes for both supported flag
  spellings — passed.
- `git diff --check 74cd95e59..HEAD` and worktree `git diff --check` — passed.

GitNexus was refreshed successfully to the reviewed `HEAD` (149,859 symbols,
308,905 relationships, 300 flows). Its full-text extension remained
unavailable, and the MCP transport closed after the refresh; the CLI
`detect-changes` compare fallback also exited without a report. The review
therefore used the focused implementation history from `f2d85e5f0` through
`299e62c44`, exact source inspection, direct call-site inventory, and the tests
above rather than claiming a successful changed-flow report.

WSL, Linux, and macOS runtime suites were not executed on this Windows-only
host. The prior static blockers on those paths are corrected, and the durable
native/WSL gate split is now encoded in the area recipe; the normal
cross-platform CI matrix remains the final environment-specific confirmation.

## Production Readiness

Ready for production. The previous correctness, CI, lint, and coverage gaps are
closed, and no new portable-strings regression was found in this pass.
