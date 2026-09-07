# Finalized References Acceptance Matrix

Evidence date: 2026-08-27

| Criterion | Verdict | Evidence |
|---|---|---|
| AC1 — catalog behavior | Satisfied | `reference_grammar`, `finalized_reference_resolution`, `detailed_resolution`, and remote-reference tests cover every kind, malformed/rooted payloads, candidate order, clean misses, and typed remote outcomes. |
| AC2 — implicit order flip | Satisfied | `precedence_flip` plus Darkmatter/Claudine conflict fixtures prove authoring-base first and caller launch-base first. Superseded repository-first expectations were inverted. |
| AC3 — `!` removal | Satisfied | `removed_package_sigil_has_a_migration_diagnostic` passes; the Phase 8 repository proof finds no removed enum/provenance/error vocabulary or shipped `!` file reference. |
| AC4 — scope derivation | Satisfied | Repository-scope catalog projection, source derivation, standalone parity, work-counter, second-repository, scaffolded-area, and symlink-equivalent fixtures pass. The repository proof confirms Darkmatter owns the sole Sniff observation adapter and Claudine calls it. |
| AC5 — materialization and provenance | Satisfied | Darkmatter's scalar/array/union/lazy/eager/remote/idempotence matrix and Claudine's direct/proxy/retry/resume/loop/sequence/harness/system-prompt/overlay route matrix pass. Phase 8 adds mixed per-property origins and a real sequence `task-spec.md` L2 regression. |
| AC6 — `ctx.cwd` / `AGENT_CWD` | Satisfied | Invocation-context route matrices and the non-vacuous spawn-seam inventory guard cover every specified compose and child-process surface, including missing/non-absolute inherited values. |
| AC7 — magic conventions | Satisfied | Magic-root collision, skill lookup, prompt convention, deduplication, and nested-source fixtures pass; completion documentation records Claudine's complete effective order. |
| AC8 — completion parity | Satisfied | Biscuit-file completion round trips and Claudine completion parity tests cover `@`, `&`, `^`, implicit, malformed rooted payloads, and unsupported recursive completion. Phase 8 executes a real `__complete` candidate unchanged through `compose`. |
| AC9 — cross-platform | Blocked by validation infrastructure | Host-independent Windows grammar fixtures, including bare `C:`, pass on macOS; the earlier grammar suite passed on WSL before that final regression case was added. Native-Windows junction coverage is `repository_containment_rejects_an_external_junction`, but `build-win-native` has only 20.9 GiB free on C: and 8.7 GiB on W:, below the non-bypassable 50 GiB preflight. Linux and the final WSL rerun were interrupted by builder availability/capacity as recorded below. |
| AC10 — validation | Satisfied locally; remote matrix blocked | All targeted regressions and every current-tree macOS area gate pass. WSL biscuit-file and Darkmatter pass after correcting the source snapshot to carry the real Git index. Linux accepted and hash-verified the final changed files but stopped accepting SSH before a gate could start; WSL Claudine exhausted the filesystem during test compilation. |
| AC11 — containment | Satisfied | Direct, recursive, completion, symlink, lazy deepest-ancestor, in-repository-link, and Windows junction fixtures cover lexical and canonical escape rejection. Public docs state the TOCTOU limitation and non-sandbox boundary. |
| AC12 — passive/public contracts | Satisfied | Passive validation tests, public exhaustiveness gates, shipped schema/prompt corpus tests, and real CLI routes pass. Raw overrides remain unchanged while effective values materialize through Darkmatter's schema stage. |
| AC13 — ratification | Satisfied | Commit `aa38252c8`'s encounter-versus-accepted wording remains unchanged. The exact removed-`!` diagnostic test passes and names `^`; reserved URI/device/drive-relative grammar tests reject both bare `C:` and `C:path` while retaining drive-absolute forms. |

## Final validation matrix

This section is populated by the Phase 8 final gate run. A gate is never marked
green from an older snapshot.

| Environment | biscuit-file | Darkmatter | Claudine | Notes |
|---|---|---|---|---|
| macOS host | Passed | Passed | Passed | Current tree: biscuit-file 813 L1 + 6 no-default-feature tests and lint passed (L2 not applicable); Darkmatter 7,561 L1, 18+69+3 L2, and lint passed; Claudine 6,680 L1, 234+3 L2, and lint passed. Managed L2 harnesses took no focus. |
| native Linux (`build-linux`) | Blocked | Blocked | Blocked | Snapshot `/tmp/rb-finalized-phase8.zKdINf` was synchronized with the five final changed files and their SHA-256 hashes matched the macOS source. The host then refused repeated bounded batch-only SSH attempts before `just test` could start. No gate is inferred green. |
| WSL (`build-win`) | Passed | Passed | Blocked | Snapshot `/home/ken/rb-finalized-phase8.EAIMjS` was synchronized with the final changed files. Prior biscuit-file test/L2/lint and Darkmatter 7,560 L1, 18+69+3 L2/lint results remain green; the final Claudine retry reached real compilation through native Windows → WSL but failed with `No space left on device` before tests started. |
| native Windows (`build-win-native`) | Blocked | Blocked | Blocked | Final capacity preflight found 20.9 GiB free on C: and 8.7 GiB on W: versus the required 50 GiB; the junction/reparse-point fixture could not run. |

No acceptance criterion conflicts with the ratified design-intent document.
AC9 and the remote portion of AC10 remain escalated until the three blocked
environment rows can be rerun on healthy builders.
