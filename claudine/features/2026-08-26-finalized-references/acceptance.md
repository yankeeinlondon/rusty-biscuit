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
| AC9 — cross-platform | Blocked by validation infrastructure | Host-independent Windows grammar fixtures pass on macOS and the completed WSL gates. Native-Windows junction coverage is `repository_containment_rejects_an_external_junction`, but `build-win-native` has only 4.8 GiB free on C: and 8.8 GiB on W:, below the non-bypassable 50 GiB preflight. Linux and the remaining WSL row were also interrupted by builder saturation as recorded below. |
| AC10 — validation | Satisfied locally; remote matrix blocked | All targeted regressions and every macOS area gate pass. WSL biscuit-file and Darkmatter pass after correcting the source snapshot to carry the real Git index. Linux and WSL Claudine did not return a result because their builders stopped accepting SSH under cold-build load. |
| AC11 — containment | Satisfied | Direct, recursive, completion, symlink, lazy deepest-ancestor, in-repository-link, and Windows junction fixtures cover lexical and canonical escape rejection. Public docs state the TOCTOU limitation and non-sandbox boundary. |
| AC12 — passive/public contracts | Satisfied | Passive validation tests, public exhaustiveness gates, shipped schema/prompt corpus tests, and real CLI routes pass. Raw overrides remain unchanged while effective values materialize through Darkmatter's schema stage. |
| AC13 — ratification | Satisfied | Commit `aa38252c8`'s encounter-versus-accepted wording remains unchanged. The exact removed-`!` diagnostic test passes and names `^`; reserved URI/device/drive-relative grammar tests match the ratified document. |

## Final validation matrix

This section is populated by the Phase 8 final gate run. A gate is never marked
green from an older snapshot.

| Environment | biscuit-file | Darkmatter | Claudine | Notes |
|---|---|---|---|---|
| macOS host | Passed | Passed | Passed | `just test`, `just test-l2`, and `just lint` passed in all three areas. L2 used background tmux/WezTerm/Apple Terminal harnesses and took no focus. |
| native Linux (`build-linux`) | Blocked | Blocked | Blocked | Exact snapshot `/tmp/rb-finalized-phase8.zKdINf` began a cold biscuit-file build, then the builder stopped producing output or accepting a second SSH transfer; the attached command was terminated after repeated recovery windows. No gate is inferred green. |
| WSL (`build-win`) | Passed | Passed | Blocked | Exact snapshot `/home/ken/rb-finalized-phase8.EAIMjS`: biscuit-file test/L2/lint passed; Darkmatter 7,560 L1, 18+69+3 L2, and lint passed. Claudine reached final package linking, then the saturated host dropped SSH before tests began. |
| native Windows (`build-win-native`) | Blocked | Blocked | Blocked | Capacity preflight failed: 4.8 GiB free on C: and 8.8 GiB on W: versus the required 50 GiB; the junction/reparse-point fixture could not run. |

No acceptance criterion conflicts with the ratified design-intent document.
AC9 and the remote portion of AC10 remain escalated until the three blocked
environment rows can be rerun on healthy builders.
