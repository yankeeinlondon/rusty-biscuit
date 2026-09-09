# Faster Sniff tests inventory

Phase 2 reconciles the source and runner populations at baseline revision
`c2dee9217f3e6be14d7a6adfeb2c90cd2cd31966`. The machine-readable
classification is [`families.json`](families.json);
[`family-members.json`](family-members.json) enumerates every member rather
than relying on a count or timing threshold. [`sources.json`](sources.json)
and [`sources.md`](sources.md) preserve the source-side scan.

## Coordination status

The baseline is **before** the `production-caching-2026-07-22` boundary. No
timing comparison may pair it with an `after` reading. If
`2026-07-22-inefficient-calling` lands, every affected cohort must be
re-baselined before comparison.

## Population and reconciliation

| Population | Count | Evidence |
|---|---:|---|
| Distinct runner identities across all six selections | 2,636 | `enumeration/captures.json` |
| Source test attributes | 2,677 | Final live reconciler source scan |
| Source-only platform exclusions on this macOS capture host | 41 | Declared below and in `families.json` |
| Runner-only identities | 0 | Reconciler source diff |
| Ignored subprocess helper entry points | 15 | Source scan; all have an owning parent test |
| Library doctests | 113 | `cargo test -p sniff --features remote --doc -- --list` |
| CLI doctests | 0 | `cargo test -p sniff-cli --features test-fixtures --doc -- --list` |

The source scanner inspected 216 files. Its five diagnostics are known
tree-sitter limitations at `&raw`, a `raw` identifier, and two GPU syntax
boundaries; the affected files still contribute their test items. No
`macro_rules!`-generated or attribute-macro-expanded tests were found.

Feature reachability is explicit: bare library lists 1,558 identities,
`network` lists 1,616, and `remote` lists 1,832. `remote` transitively enables
`network`; `network` alone leaves `focused_provider`, `remote_observation`,
and `remote_providers` as empty targets, so it is not evidence for remote
provider coverage. Bare CLI lists 802 identities and `test-fixtures` lists
804, adding the two L2 tests and their helper binaries.

## Review conventions for family rows

- **Deterministic** means assertions consume owned values, temp files/repos, local Wiremock endpoints, or CLI arguments. Existing CLI spawn families remain marked for isolation remediation where they can inherit ambient state.
- **Native-detector** means the real platform API or bounded subprocess is the subject. These remain cfg-gated L1 and assert portable invariants.
- **External-resource** means a real network endpoint or terminal is required. Unavailable resources are not passing evidence.
- “No semantic floor” means no required minimum elapsed time. Cost provenance is Phase 1 JUnit/listing evidence; per-family attribution and budgets belong to Phase 3.

## Reconciled family index

Every row has exact membership in `family-members.json`.

| Family | Identities | Purpose | Behavior and assertion quality | Boundary, ownership, waits, and timing floor | Tier, features, platforms, recipe, and observed cost | Disposition |
|---|---:|---|---|---|---|---|
| `lib-unit-credentials` | 1 | deterministic | credentials parsing, request, projection, and error contracts; typed values/errors asserted. | In-process or owned temp input; no live network/terminal; cleanup on drop; no semantic floor. | `test` L1; library `remote`; cfg matrix where declared; Phase 1 JUnit, family timing pending. | satisfactory |
| `lib-unit-error` | 9 | deterministic | error parsing, request, projection, and error contracts; typed values/errors asserted. | In-process or owned temp input; no live network/terminal; cleanup on drop; no semantic floor. | `test` L1; library `remote`; cfg matrix where declared; Phase 1 JUnit, family timing pending. | satisfactory |
| `lib-unit-executable-index` | 12 | deterministic | executable index parsing, request, projection, and error contracts; typed values/errors asserted. | In-process or owned temp input; no live network/terminal; cleanup on drop; no semantic floor. | `test` L1; library `remote`; cfg matrix where declared; Phase 1 JUnit, family timing pending. | satisfactory |
| `lib-unit-filesystem` | 588 | deterministic | filesystem parsing, request, projection, and error contracts; typed values/errors asserted. | In-process or owned temp input; no live network/terminal; cleanup on drop; no semantic floor. | `test` L1; library `remote`; cfg matrix where declared; Phase 1 JUnit, family timing pending. | satisfactory |
| `lib-unit-hardware` | 50 | deterministic | hardware parsing, request, projection, and error contracts; typed values/errors asserted. | In-process or owned temp input; no live network/terminal; cleanup on drop; no semantic floor. | `test` L1; library `remote`; cfg matrix where declared; Phase 1 JUnit, family timing pending. | satisfactory |
| `lib-unit-network` | 55 | deterministic | network parsing, request, projection, and error contracts; typed values/errors asserted. | In-process or owned temp input; no live network/terminal; cleanup on drop; no semantic floor. | `test` L1; library `remote`; cfg matrix where declared; Phase 1 JUnit, family timing pending. | satisfactory |
| `lib-unit-os` | 165 | deterministic | os parsing, request, projection, and error contracts; typed values/errors asserted. | In-process or owned temp input; no live network/terminal; cleanup on drop; no semantic floor. | `test` L1; library `remote`; cfg matrix where declared; Phase 1 JUnit, family timing pending. | satisfactory |
| `lib-unit-package` | 54 | deterministic | package parsing, request, projection, and error contracts; typed values/errors asserted. | In-process or owned temp input; no live network/terminal; cleanup on drop; no semantic floor. | `test` L1; library `remote`; cfg matrix where declared; Phase 1 JUnit, family timing pending. | satisfactory |
| `lib-unit-performance` | 6 | deterministic | performance parsing, request, projection, and error contracts; typed values/errors asserted. | In-process or owned temp input; no live network/terminal; cleanup on drop; no semantic floor. | `test` L1; library `remote`; cfg matrix where declared; Phase 1 JUnit, family timing pending. | satisfactory |
| `lib-unit-process` | 25 | deterministic | process parsing, request, projection, and error contracts; typed values/errors asserted. | Owned children with injected deadlines/output drainage; ignored helper modes owned by parents; timeout floors retained. | `test` L1; library `remote`; cfg matrix where declared; Phase 1 JUnit, family timing pending. | satisfactory |
| `lib-unit-programs` | 287 | deterministic | programs parsing, request, projection, and error contracts; typed values/errors asserted. | In-process or owned temp input; no live network/terminal; cleanup on drop; no semantic floor. | `test` L1; library `remote`; cfg matrix where declared; Phase 1 JUnit, family timing pending. | satisfactory |
| `lib-unit-remote` | 94 | deterministic | remote parsing, request, projection, and error contracts; typed values/errors asserted. | In-process or owned temp input; no live network/terminal; cleanup on drop; no semantic floor. | `test` L1; `remote`; all OSes; Phase 1 JUnit, family timing pending. | satisfactory |
| `lib-unit-request` | 25 | deterministic | request parsing, request, projection, and error contracts; typed values/errors asserted. | In-process or owned temp input; no live network/terminal; cleanup on drop; no semantic floor. | `test` L1; library `remote`; cfg matrix where declared; Phase 1 JUnit, family timing pending. | satisfactory |
| `lib-unit-services` | 46 | deterministic | services parsing, request, projection, and error contracts; typed values/errors asserted. | In-process or owned temp input; no live network/terminal; cleanup on drop; no semantic floor. | `test` L1; library `remote`; cfg matrix where declared; Phase 1 JUnit, family timing pending. | satisfactory |
| `lib-unit-test-helpers` | 4 | deterministic | test helpers parsing, request, projection, and error contracts; typed values/errors asserted. | In-process or owned temp input; no live network/terminal; cleanup on drop; no semantic floor. | `test` L1; library `remote`; cfg matrix where declared; Phase 1 JUnit, family timing pending. | satisfactory |
| `lib-unit-tests` | 13 | deterministic | tests parsing, request, projection, and error contracts; typed values/errors asserted. | In-process or owned temp input; no live network/terminal; cleanup on drop; no semantic floor. | `test` L1; library `remote`; cfg matrix where declared; Phase 1 JUnit, family timing pending. | satisfactory |
| `lib-real-network` | 8 | external-resource | Live Cargo/npm lookup, missing-package, and enrichment results/errors. | Live crates.io/npm; bounded clients; no owned server; variable network floor. | `test-real`; `network`; supported OSes when endpoints available; timing pending Phase 3. | satisfactory |
| `cli-unit-args` | 84 | deterministic | args argument, planning, and rendering contracts; parsed values/rendered output asserted. | In-process or owned temp input; no live network/terminal; cleanup on drop; no semantic floor. | `test` L1; bare local, fixtures in CI where applicable; all OSes; Phase 1 L1 evidence. | satisfactory |
| `cli-unit-commands` | 6 | deterministic | commands argument, planning, and rendering contracts; parsed values/rendered output asserted. | In-process or owned temp input; no live network/terminal; cleanup on drop; no semantic floor. | `test` L1; bare local, fixtures in CI where applicable; all OSes; Phase 1 L1 evidence. | satisfactory |
| `cli-unit-install` | 12 | deterministic | install argument, planning, and rendering contracts; parsed values/rendered output asserted. | In-process or owned temp input; no live network/terminal; cleanup on drop; no semantic floor. | `test` L1; bare local, fixtures in CI where applicable; all OSes; Phase 1 L1 evidence. | satisfactory |
| `cli-unit-install-plan-cmd` | 13 | deterministic | install plan cmd argument, planning, and rendering contracts; parsed values/rendered output asserted. | In-process or owned temp input; no live network/terminal; cleanup on drop; no semantic floor. | `test` L1; bare local, fixtures in CI where applicable; all OSes; Phase 1 L1 evidence. | satisfactory |
| `cli-unit-install-ui` | 5 | deterministic | install ui argument, planning, and rendering contracts; parsed values/rendered output asserted. | In-process or owned temp input; no live network/terminal; cleanup on drop; no semantic floor. | `test` L1; bare local, fixtures in CI where applicable; all OSes; Phase 1 L1 evidence. | satisfactory |
| `cli-unit-output` | 281 | deterministic | output argument, planning, and rendering contracts; parsed values/rendered output asserted. | In-process or owned temp input; no live network/terminal; cleanup on drop; no semantic floor. | `test` L1; bare local, fixtures in CI where applicable; all OSes; Phase 1 L1 evidence. | satisfactory |
| `cli-process-fixture` | 9 | deterministic | Shared CLI process policy, containment, PATH modes, hostile inherited inputs, and real-binary launch context. | Fixture-owned directories and subprocesses; bounded PATH and scrubbed environment; cleanup on drop. | `test` L1; bare local and fixtures in CI; all OSes; Phase 4 focused and broad evidence. | satisfactory |
| `cli-spawn-site-guard` | 4 | deterministic | Source-scan guard rejects raw spawns, stale exemptions, and detector weakening while accepting prose/string mentions. | In-process source scan plus fixture-owned artifact output; no live host resource. | `test` L1; bare local and fixtures in CI; all OSes; Phase 4/5 focused and broad evidence. | satisfactory |
| `lib-bench-fixtures` | 5 | deterministic | Benchmark fixture identity, plan, and stable work-count contracts detect catalog/request drift. | TempDir repo/fixture removed on drop; no readiness sleep; no semantic floor. | `test` L1; library `remote`; cfg matrix where declared; Phase 1 JUnit, family timing pending. | satisfactory |
| `lib-bench-ids-sync` | 3 | deterministic | Benchmark fixture identity, plan, and stable work-count contracts detect catalog/request drift. | In-process or owned temp input; no live network/terminal; cleanup on drop; no semantic floor. | `test` L1; library `remote`; cfg matrix where declared; Phase 1 JUnit, family timing pending. | satisfactory |
| `lib-bench-plans` | 3 | deterministic | Benchmark fixture identity, plan, and stable work-count contracts detect catalog/request drift. | In-process or owned temp input; no live network/terminal; cleanup on drop; no semantic floor. | `test` L1; library `remote`; cfg matrix where declared; Phase 1 JUnit, family timing pending. | satisfactory |
| `lib-benchmark-workloads` | 4 | deterministic | Benchmark fixture identity, plan, seeded-observation reuse, and stable work-count contracts detect catalog/request drift. | TempDir repo/fixture removed on drop; no readiness sleep; no semantic floor. | `test` L1; library `remote`; cfg matrix where declared; Phase 1 JUnit plus Phase 6 focused evidence, family timing pending. | satisfactory |
| `lib-focused-provider` | 51 | deterministic | Provider selection, pagination, typed failure, and projection; Wiremock request/response assertions distinguish failures. | Owned Wiremock server/port, bounded requests and shutdown; no live API; no semantic floor. | `test` L1; library `remote`; cfg matrix where declared; Phase 1 JUnit, family timing pending. | satisfactory |
| `lib-foo` | 0 | deterministic | Placeholder `bar` proved no product behavior and was removed in Phase 5. | No remaining target. | Explicitly empty family retained for baseline-to-candidate coverage mapping. | satisfactory |
| `lib-git-parity` | 85 | deterministic | Git parity/conflict results against owned repo states; dependent status and commit outputs asserted. | TempDir repo/fixture removed on drop; no readiness sleep; no semantic floor. | `test` L1; library `remote`; cfg matrix where declared; Phase 1 JUnit, family timing pending. | satisfactory |
| `lib-host-capability-cache` | 6 | native-detector | Public values and errors are asserted. | Real host APIs/tools, portable invariants; no semantic floor except named elapsed detector test. | `test` L1; library `remote`; cfg matrix where declared; Phase 1 JUnit, family timing pending. | satisfactory |
| `lib-integration` | 117 | native-detector | Public values and errors are asserted. | Real host APIs/tools, portable invariants; no semantic floor except named elapsed detector test. | `test` L1; library `remote`; cfg matrix where declared; Phase 1 JUnit, family timing pending. | satisfactory |
| `lib-merge-conflict-prediction` | 13 | deterministic | Git parity/conflict results against owned repo states; dependent status and commit outputs asserted. | In-process or owned temp input; no live network/terminal; cleanup on drop; no semantic floor. | `test` L1; library `remote`; cfg matrix where declared; Phase 1 JUnit, family timing pending. | satisfactory |
| `lib-program-installable` | 1 | deterministic | Public values and errors are asserted. | In-process or owned temp input; no live network/terminal; cleanup on drop; no semantic floor. | `test` L1; library `remote`; cfg matrix where declared; Phase 1 JUnit, family timing pending. | satisfactory |
| `lib-program-serialization` | 4 | deterministic | Public values and errors are asserted. | In-process or owned temp input; no live network/terminal; cleanup on drop; no semantic floor. | `test` L1; library `remote`; cfg matrix where declared; Phase 1 JUnit, family timing pending. | satisfactory |
| `lib-remote-observation` | 14 | deterministic | Provider selection, pagination, typed failure, and projection; Wiremock request/response assertions distinguish failures. | Owned Wiremock server/port, bounded requests and shutdown; no live API; no semantic floor. | `test` L1; `remote`; all OSes; Phase 1 JUnit, family timing pending. | satisfactory |
| `lib-remote-providers` | 70 | deterministic | Provider selection, pagination, typed failure, and projection; Wiremock request/response assertions distinguish failures. | Owned Wiremock server/port, bounded requests and shutdown; no live API; no semantic floor. | `test` L1; `remote`; all OSes; Phase 1 JUnit, family timing pending. | satisfactory |
| `lib-remote-resolution` | 7 | deterministic | Public values and errors are asserted. | In-process or owned temp input; no live network/terminal; cleanup on drop; no semantic floor. | `test` L1; `remote`; all OSes; Phase 1 JUnit, family timing pending. | satisfactory |
| `lib-uv-with-install-plan` | 6 | deterministic | Public values and errors are asserted. | In-process or owned temp input; no live network/terminal; cleanup on drop; no semantic floor. | `test` L1; library `remote`; cfg matrix where declared; Phase 1 JUnit, family timing pending. | satisfactory |
| `cli-install-interactive-pty` | 0 | external-resource | The unreachable bespoke PTY case duplicated the reachable install-interview behavior and was removed in Phase 5. | No remaining target. | Explicitly empty family retained for baseline-to-candidate coverage mapping. | satisfactory |
| `cli-install-interview-cli` | 1 | deterministic | install interview cli CLI contract; exit status and stdout/stderr or JSON shape distinguish failures. | CLI subprocess and temp repos where present; remaining raw spawns inherit ambient state; output drained; no semantic floor. | `test` L1; bare local, fixtures in CI where applicable; all OSes; Phase 1 L1 evidence. | remediation in this fix |
| `cli-install-plan` | 9 | deterministic | install plan CLI contract; exit status and stdout/stderr or JSON shape distinguish failures. | CLI subprocess and temp repos where present; remaining raw spawns inherit ambient state; output drained; no semantic floor. | `test` L1; bare local, fixtures in CI where applicable; all OSes; Phase 1 L1 evidence. | remediation in this fix |
| `cli-level2-cicd-styling` | 1 | external-resource | Real-terminal visible text plus SGR/link fallback behavior; raw and plain captures asserted. | Shared tmux + fixture binary; 200 ms readiness sleep is Phase 7 remediation; real-terminal floor. | `test-l2`; `test-fixtures`; macOS/Linux tmux; Phase 1 L2 total 1.410 s. | remediation in this fix |
| `cli-level2-git-status-styling` | 1 | external-resource | Real-terminal visible text plus SGR/link fallback behavior; raw and plain captures asserted. | Shared tmux + fixture binary; 200 ms readiness sleep is Phase 7 remediation; real-terminal floor. | `test-l2`; `test-fixtures`; macOS/Linux tmux; Phase 1 L2 total 1.410 s. | remediation in this fix |
| `cli-snapshots` | 14 | deterministic | snapshots CLI contract; exit status and stdout/stderr or JSON shape distinguish failures. | CLI subprocess and temp repos where present; remaining raw spawns inherit ambient state; output drained; no semantic floor. | `test` L1; bare local, fixtures in CI where applicable; all OSes; Phase 1 L1 evidence. | remediation in this fix |
| `cli-tty` | 1 | deterministic | tty CLI contract; exit status and stdout/stderr or JSON shape distinguish failures. | CLI subprocess and temp repos where present; remaining raw spawns inherit ambient state; output drained; no semantic floor. | `test` L1; bare local, fixtures in CI where applicable; all OSes; Phase 1 L1 evidence. | remediation in this fix |
| `cli-help-version` | 3 | deterministic | help version CLI contract; exit status and stdout/stderr or JSON shape distinguish failures. | CLI subprocess and temp repos where present; remaining raw spawns inherit ambient state; output drained; no semantic floor. | `test` L1; bare local, fixtures in CI where applicable; all OSes; Phase 1 L1 evidence. | remediation in this fix |
| `cli-completions` | 9 | deterministic | completions CLI contract; exit status and stdout/stderr or JSON shape distinguish failures. | CLI subprocess and temp repos where present; remaining raw spawns inherit ambient state; output drained; no semantic floor. | `test` L1; bare local, fixtures in CI where applicable; all OSes; Phase 1 L1 evidence. | remediation in this fix |
| `cli-output-mode` | 4 | deterministic | output mode CLI contract; exit status and stdout/stderr or JSON shape distinguish failures. | CLI subprocess and temp repos where present; remaining raw spawns inherit ambient state; output drained; no semantic floor. | `test` L1; bare local, fixtures in CI where applicable; all OSes; Phase 1 L1 evidence. | remediation in this fix |
| `cli-flag-position` | 8 | deterministic | flag position CLI contract; exit status and stdout/stderr or JSON shape distinguish failures. | CLI subprocess and temp repos where present; remaining raw spawns inherit ambient state; output drained; no semantic floor. | `test` L1; bare local, fixtures in CI where applicable; all OSes; Phase 1 L1 evidence. | remediation in this fix |
| `cli-repo-json-aggregate` | 16 | deterministic | repo json aggregate CLI contract; exit status and stdout/stderr or JSON shape distinguish failures. | CLI subprocess and temp repos where present; remaining raw spawns inherit ambient state; output drained; no semantic floor. | `test` L1; bare local, fixtures in CI where applicable; all OSes; Phase 1 L1 evidence. | remediation in this fix |
| `cli-repo-leaf-e2e` | 12 | deterministic | repo leaf e2e CLI contract; exit status and stdout/stderr or JSON shape distinguish failures. | CLI subprocess and temp repos where present; remaining raw spawns inherit ambient state; output drained; no semantic floor. | `test` L1; bare local, fixtures in CI where applicable; all OSes; Phase 1 L1 evidence. | remediation in this fix |
| `cli-terminal-subset` | 8 | native-detector | terminal subset CLI contract; exit status and stdout/stderr or JSON shape distinguish failures. | Real host APIs/tools, portable invariants; no semantic floor except named elapsed detector test. | `test` L1; bare local, fixtures in CI where applicable; all OSes; Phase 1 L1 evidence. | remediation in this fix |
| `cli-section-subcommands` | 11 | native-detector | section subcommands CLI contract; exit status and stdout/stderr or JSON shape distinguish failures. | Real host APIs/tools, portable invariants; no semantic floor except named elapsed detector test. | `test` L1; bare local, fixtures in CI where applicable; all OSes; Phase 1 L1 evidence. | remediation in this fix |
| `cli-hardware-detail` | 8 | native-detector | hardware detail CLI contract; exit status and stdout/stderr or JSON shape distinguish failures. | Real host APIs/tools, portable invariants; no semantic floor except named elapsed detector test. | `test` L1; bare local, fixtures in CI where applicable; all OSes; Phase 1 L1 evidence. | remediation in this fix |
| `cli-filesystem-detail` | 14 | deterministic | filesystem detail CLI contract; exit status and stdout/stderr or JSON shape distinguish failures. | CLI subprocess and temp repos where present; remaining raw spawns inherit ambient state; output drained; no semantic floor. | `test` L1; bare local, fixtures in CI where applicable; all OSes; Phase 1 L1 evidence. | remediation in this fix |
| `cli-software` | 21 | deterministic | software CLI contract; exit status and stdout/stderr or JSON shape distinguish failures. | CLI subprocess and temp repos where present; remaining raw spawns inherit ambient state; output drained; no semantic floor. | `test` L1; bare local, fixtures in CI where applicable; all OSes; Phase 1 L1 evidence. | remediation in this fix |
| `cli-test-runner` | 10 | deterministic | test runner CLI contract; exit status and stdout/stderr or JSON shape distinguish failures. | CLI subprocess and temp repos where present; remaining raw spawns inherit ambient state; output drained; no semantic floor. | `test` L1; bare local, fixtures in CI where applicable; all OSes; Phase 1 L1 evidence. | remediation in this fix |
| `cli-repo-version` | 22 | deterministic | repo version CLI contract; exit status and stdout/stderr or JSON shape distinguish failures. | CLI subprocess and temp repos where present; remaining raw spawns inherit ambient state; output drained; no semantic floor. | `test` L1; bare local, fixtures in CI where applicable; all OSes; Phase 1 L1 evidence. | remediation in this fix |
| `cli-negative-program-paths` | 10 | deterministic | negative program paths CLI contract; exit status and stdout/stderr or JSON shape distinguish failures. | CLI subprocess and temp repos where present; remaining raw spawns inherit ambient state; output drained; no semantic floor. | `test` L1; bare local, fixtures in CI where applicable; all OSes; Phase 1 L1 evidence. | remediation in this fix |
| `cli-services` | 5 | native-detector | services CLI contract; exit status and stdout/stderr or JSON shape distinguish failures. | Real host APIs/tools, portable invariants; no semantic floor except named elapsed detector test. | `test` L1; bare local, fixtures in CI where applicable; all OSes; Phase 1 L1 evidence. | remediation in this fix |
| `cli-scoped-enrichment` | 14 | deterministic | scoped enrichment CLI contract; exit status and stdout/stderr or JSON shape distinguish failures. | CLI subprocess and temp repos where present; remaining raw spawns inherit ambient state; output drained; no semantic floor. | `test` L1; bare local, fixtures in CI where applicable; all OSes; Phase 1 L1 evidence. | remediation in this fix |
| `cli-verbose` | 2 | deterministic | verbose CLI contract; exit status and stdout/stderr or JSON shape distinguish failures. | CLI subprocess and temp repos where present; remaining raw spawns inherit ambient state; output drained; no semantic floor. | `test` L1; bare local, fixtures in CI where applicable; all OSes; Phase 1 L1 evidence. | remediation in this fix |
| `cli-negative` | 2 | deterministic | negative CLI contract; exit status and stdout/stderr or JSON shape distinguish failures. | CLI subprocess and temp repos where present; remaining raw spawns inherit ambient state; output drained; no semantic floor. | `test` L1; bare local, fixtures in CI where applicable; all OSes; Phase 1 L1 evidence. | remediation in this fix |
| `cli-remote` | 2 | deterministic | remote CLI contract; exit status and stdout/stderr or JSON shape distinguish failures. | CLI subprocess and temp repos where present; remaining raw spawns inherit ambient state; output drained; no semantic floor. | `test` L1; bare local, fixtures in CI where applicable; all OSes; Phase 1 L1 evidence. | remediation in this fix |
| `cli-install` | 7 | deterministic | install CLI contract; exit status and stdout/stderr or JSON shape distinguish failures. | CLI subprocess and temp repos where present; remaining raw spawns inherit ambient state; output drained; no semantic floor. | `test` L1; bare local, fixtures in CI where applicable; all OSes; Phase 1 L1 evidence. | remediation in this fix |
| `cli-plain` | 2 | deterministic | plain CLI contract; exit status and stdout/stderr or JSON shape distinguish failures. | CLI subprocess and temp repos where present; remaining raw spawns inherit ambient state; output drained; no semantic floor. | `test` L1; bare local, fixtures in CI where applicable; all OSes; Phase 1 L1 evidence. | remediation in this fix |
| `cli-repo-command` | 2 | deterministic | repo command CLI contract; exit status and stdout/stderr or JSON shape distinguish failures. | CLI subprocess and temp repos where present; remaining raw spawns inherit ambient state; output drained; no semantic floor. | `test` L1; bare local, fixtures in CI where applicable; all OSes; Phase 1 L1 evidence. | remediation in this fix |
| `cli-blast-radius` | 33 | deterministic | blast radius CLI contract; exit status and stdout/stderr or JSON shape distinguish failures. | CLI subprocess and temp repos where present; remaining raw spawns inherit ambient state; output drained; no semantic floor. | `test` L1; bare local, fixtures in CI where applicable; all OSes; Phase 1 L1 evidence. | remediation in this fix |
| `cli-recent-commits-basic` | 17 | deterministic | recent commits basic CLI contract; exit status and stdout/stderr or JSON shape distinguish failures. | CLI subprocess and temp repos where present; remaining raw spawns inherit ambient state; output drained; no semantic floor. | `test` L1; bare local, fixtures in CI where applicable; all OSes; Phase 1 L1 evidence. | remediation in this fix |
| `cli-recent-commits-routing` | 9 | deterministic | recent commits routing CLI contract; exit status and stdout/stderr or JSON shape distinguish failures. | CLI subprocess and temp repos where present; remaining raw spawns inherit ambient state; output drained; no semantic floor. | `test` L1; bare local, fixtures in CI where applicable; all OSes; Phase 1 L1 evidence. | remediation in this fix |
| `cli-recent-commits-payload` | 5 | deterministic | recent commits payload CLI contract; exit status and stdout/stderr or JSON shape distinguish failures. | CLI subprocess and temp repos where present; remaining raw spawns inherit ambient state; output drained; no semantic floor. | `test` L1; bare local, fixtures in CI where applicable; all OSes; Phase 1 L1 evidence. | remediation in this fix |
| `cli-repo-packages` | 13 | deterministic | repo packages CLI contract; exit status and stdout/stderr or JSON shape distinguish failures. | CLI subprocess and temp repos where present; remaining raw spawns inherit ambient state; output drained; no semantic floor. | `test` L1; bare local, fixtures in CI where applicable; all OSes; Phase 1 L1 evidence. | remediation in this fix |
| `cli-package-areas` | 22 | deterministic | package areas CLI contract; exit status and stdout/stderr or JSON shape distinguish failures. | CLI subprocess and temp repos where present; remaining raw spawns inherit ambient state; output drained; no semantic floor. | `test` L1; bare local, fixtures in CI where applicable; all OSes; Phase 1 L1 evidence. | remediation in this fix |
| `cli-stable-json-shape` | 11 | deterministic | stable json shape CLI contract; exit status and stdout/stderr or JSON shape distinguish failures. | CLI subprocess and temp repos where present; remaining raw spawns inherit ambient state; output drained; no semantic floor. | `test` L1; bare local, fixtures in CI where applicable; all OSes; Phase 1 L1 evidence. | remediation in this fix |
| `cli-remote-pr` | 1 | deterministic | remote pr CLI contract; exit status and stdout/stderr or JSON shape distinguish failures. | CLI subprocess and temp repos where present; remaining raw spawns inherit ambient state; output drained; no semantic floor. | `test` L1; bare local, fixtures in CI where applicable; all OSes; Phase 1 L1 evidence. | remediation in this fix |
| `cli-locator-json` | 16 | deterministic | locator json CLI contract; exit status and stdout/stderr or JSON shape distinguish failures. | CLI subprocess and temp repos where present; remaining raw spawns inherit ambient state; output drained; no semantic floor. | `test` L1; bare local, fixtures in CI where applicable; all OSes; Phase 1 L1 evidence. | remediation in this fix |
| `cli-repo-worktree` | 12 | deterministic | repo worktree CLI contract; exit status and stdout/stderr or JSON shape distinguish failures. | CLI subprocess and temp repos where present; remaining raw spawns inherit ambient state; output drained; no semantic floor. | `test` L1; bare local, fixtures in CI where applicable; all OSes; Phase 1 L1 evidence. | remediation in this fix |
| `cli-repo-worktrees` | 13 | deterministic | repo worktrees CLI contract; exit status and stdout/stderr or JSON shape distinguish failures. | CLI subprocess and temp repos where present; remaining raw spawns inherit ambient state; output drained; no semantic floor. | `test` L1; bare local, fixtures in CI where applicable; all OSes; Phase 1 L1 evidence. | remediation in this fix |
| `cli-package-scope-flags` | 9 | deterministic | package scope flags CLI contract; exit status and stdout/stderr or JSON shape distinguish failures. | CLI subprocess and temp repos where present; remaining raw spawns inherit ambient state; output drained; no semantic floor. | `test` L1; bare local, fixtures in CI where applicable; all OSes; Phase 1 L1 evidence. | remediation in this fix |
| `cli-repo-area` | 10 | deterministic | repo area CLI contract; exit status and stdout/stderr or JSON shape distinguish failures. | CLI subprocess and temp repos where present; remaining raw spawns inherit ambient state; output drained; no semantic floor. | `test` L1; bare local, fixtures in CI where applicable; all OSes; Phase 1 L1 evidence. | remediation in this fix |

## Platform and feature exclusions

These 41 source tests are absent because the baseline listing host is macOS. Platform specificity does not change their tier: each remains L1 on its matrix leg. Windows authority is `windows-latest`; Sniff has no local MinGW recipe.

| Source test | Gate | Actual route | Disposition |
|---|---|---|---|
| `alsa_cards_parses_card_entries` | `#[cfg(target_os = "linux")]` | test-ci (ubuntu-latest or WSL2 as applicable) | satisfactory |
| `alsa_missing_pcm_falls_back_to_output_only` | `#[cfg(target_os = "linux")]` | test-ci (ubuntu-latest or WSL2 as applicable) | satisfactory |
| `alsa_pcm_capture_only_card` | `#[cfg(target_os = "linux")]` | test-ci (ubuntu-latest or WSL2 as applicable) | satisfactory |
| `alsa_pcm_detects_playback_and_capture` | `#[cfg(target_os = "linux")]` | test-ci (ubuntu-latest or WSL2 as applicable) | satisfactory |
| `classify_alsa_kind_matches_expected_categories` | `#[cfg(target_os = "linux")]` | test-ci (ubuntu-latest or WSL2 as applicable) | satisfactory |
| `current_user_id_returns_a_canonical_token_user_sid` | `#[cfg(windows)]` | test-ci (windows-latest) | satisfactory |
| `expand_env_vars_handles_empty_string` | `#[cfg(target_os = "windows")]` | test-ci (windows-latest) | satisfactory |
| `expand_env_vars_passes_through_plain_string` | `#[cfg(target_os = "windows")]` | test-ci (windows-latest) | satisfactory |
| `expand_env_vars_preserves_unknown_variable` | `#[cfg(target_os = "windows")]` | test-ci (windows-latest) | satisfactory |
| `expand_env_vars_resolves_system_root` | `#[cfg(target_os = "windows")]` | test-ci (windows-latest) | satisfactory |
| `extra_system_config_windows_does_not_panic_when_file_absent` | `#[cfg(target_os = "windows")]` | test-ci (windows-latest) | satisfactory |
| `install_root_dirs_reads_env_vars` | `#[cfg(target_os = "windows")]` | test-ci (windows-latest) | satisfactory |
| `orphaned_hkcu_entry_is_filtered` | `#[cfg(target_os = "windows")]` | test-ci (windows-latest) | satisfactory |
| `path_wins_over_fallbacks_for_cmd` | `#[cfg(target_os = "windows")]` | test-ci (windows-latest) | satisfactory |
| `phase5_non_utf8_ref_is_not_silently_dropped` | `#[cfg(target_os = "linux")]` | test-ci (ubuntu-latest or WSL2 as applicable) | satisfactory |
| `preserves_windows_drive_prefix_and_casing` | `#[cfg(windows)]` | test-ci (windows-latest) | satisfactory |
| `scan_app_paths_filters_orphaned_entries` | `#[cfg(target_os = "windows")]` | test-ci (windows-latest) | satisfactory |
| `scan_app_paths_honors_hkcu_precedence_over_hklm` | `#[cfg(target_os = "windows")]` | test-ci (windows-latest) | satisfactory |
| `test_detect_timezone_windows_populates_timezone_name` | `#[cfg(target_os = "windows")]` | test-ci (windows-latest) | satisfactory |
| `test_detect_windows_package_managers_on_windows` | `#[cfg(target_os = "windows")]` | test-ci (windows-latest) | satisfactory |
| `test_find_macos_app_bundle_all_known_apps_return_none` | `#[cfg(not(target_os = "macos"))]` | test-ci (platform matrix) | satisfactory |
| `test_find_macos_app_bundle_empty_returns_none` | `#[cfg(not(target_os = "macos"))]` | test-ci (platform matrix) | satisfactory |
| `test_find_macos_app_bundle_returns_none` | `#[cfg(not(target_os = "macos"))]` | test-ci (platform matrix) | satisfactory |
| `test_get_path_dirs_splits_semicolon_separated_entries` | `#[cfg(target_os = "windows")]` | test-ci (windows-latest) | satisfactory |
| `test_installable_false_for_builtin_tts` | `#[cfg(target_os = "windows")]` | test-ci (windows-latest) | satisfactory |
| `test_linux_package_managers_finds_at_least_one` | `#[cfg(target_os = "linux")]` | test-ci (ubuntu-latest or WSL2 as applicable) | satisfactory |
| `test_list_windows_scm_services_returns_real_services` | `#[cfg(target_os = "windows")]` | test-ci (windows-latest) | satisfactory |
| `test_parse_linux_proc_default_route_interface` | `#[cfg(target_os = "linux")]` | test-ci (ubuntu-latest or WSL2 as applicable) | satisfactory |
| `test_parse_linux_proc_default_route_interface_no_default_route` | `#[cfg(target_os = "linux")]` | test-ci (ubuntu-latest or WSL2 as applicable) | satisfactory |
| `test_services_detailed_running_filter_windows` | `#[cfg(target_os = "windows")]` | test-ci (windows-latest) | satisfactory |
| `test_services_detailed_stopped_filter_windows` | `#[cfg(target_os = "windows")]` | test-ci (windows-latest) | satisfactory |
| `walk_install_roots_first_write_wins_across_roots` | `#[cfg(target_os = "windows")]` | test-ci (windows-latest) | satisfactory |
| `walk_install_roots_ignores_nested_bin_directory` | `#[cfg(target_os = "windows")]` | test-ci (windows-latest) | satisfactory |
| `walk_install_roots_indexes_exe_at_child_root` | `#[cfg(target_os = "windows")]` | test-ci (windows-latest) | satisfactory |
| `walk_install_roots_skips_missing_root` | `#[cfg(target_os = "windows")]` | test-ci (windows-latest) | satisfactory |
| `windows_audio_probe_drains_large_stdout_and_stderr` | `#[cfg(target_os = "windows")]` | test-ci (windows-latest) | satisfactory |
| `windows_audio_probe_honors_injected_deadline` | `#[cfg(target_os = "windows")]` | test-ci (windows-latest) | satisfactory |
| `windows_default_route_probe_drains_large_output` | `#[cfg(target_os = "windows")]` | test-ci (windows-latest) | satisfactory |
| `windows_default_route_probe_honors_injected_deadline` | `#[cfg(target_os = "windows")]` | test-ci (windows-latest) | satisfactory |
| `windows_timezone_probe_drains_large_output` | `#[cfg(target_os = "windows")]` | test-ci (windows-latest) | satisfactory |
| `windows_timezone_probe_honors_injected_deadline` | `#[cfg(target_os = "windows")]` | test-ci (windows-latest) | satisfactory |

## Ignored helper entry points

All 15 ignored tests are executable child modes, not standalone assertions: 12 in `process.rs`, one remote-refresh timeout child, one BurntToast probe child, and one install-timeout child. Parent tests spawn the current test binary by exact ignored name, drain output, assert behavior, and own termination.

## Shared fixtures and build-only targets

| Fixture or target | Consumers and behavior | Inputs, ownership, cleanup, waits | Purpose / route / disposition |
|---|---|---|---|
| `cli.rs::run_isolated_software` | Seven software CLI contracts | Pins PATH and selected home/install roots, but not CWD, Git plumbing, cache/config, or rendering | deterministic; L1; Phase 4 remediation |
| `snapshots.rs::run_stdout` | Fourteen snapshots | Raw `cargo_bin`; inherited CWD/full env; assert_cmd drains output | deterministic; L1; Phase 5 remediation |
| `lib/tests/fixtures.rs` | Repository/topology builders | TempDir Git/manifest trees; drop cleanup; no waits | deterministic; L1; satisfactory, reuse review Phase 6 |
| `lib/tests/fixtures/remote/` | Provider payloads | Static owned data; no live endpoint | deterministic; `remote` L1; satisfactory |
| `lib/benches/support/builder.rs` | Git parity, fixture tests, workloads, Criterion | Deterministic temp repos/topologies/commits | deterministic; test/bench; satisfactory shared seam |
| `lib/benches/support/fixtures.rs` | Criterion/fixture tests | Reusable descriptions and owned temp state | deterministic; test/bench; satisfactory |
| `lib/benches/support/plans.rs` | Plan/benchmark cohorts | Explicit request shapes | deterministic; test/bench; satisfactory |
| `lib/benches/support/network_fixture.rs` | Network Criterion groups | Local Wiremock, bounded readiness, owned runtime | deterministic protocol; bench; satisfactory |
| `lib/benches/support/remote_report_fixture.rs` | Remote report benches | Captured data; no live API | deterministic; `remote`; satisfactory |
| `lib/benches/support/bench_ids.rs` | Manifest synchronization | Static catalog checked by three tests | deterministic; test/bench; satisfactory |
| `lib/benches/support/util.rs` | Benchmark setup | Fixture state outside timed sections | deterministic; bench; satisfactory |
| `render_cicd_fixture` | L2 CI/CD styling | Build-only with fixtures; spawned in tmux | external-resource; L2; Phase 7 readiness remediation |
| `render_git_status_fixture` | L2 Git styling | Build-only with fixtures; spawned in tmux | external-resource; L2; Phase 7 readiness remediation |
| product `sniff` bin | Product binary | Zero tests by design | build-only; satisfactory |
| `sniff::fixtures` target | Module-only integration support | Zero tests; imported by siblings | deterministic support; satisfactory |
| Windows orphan/priority targets | Windows-only integration bins | Zero on macOS; tests declared above | cfg L1; satisfactory |

## Doctests, benches, and fuzz

The canonical doctest route selects library `remote` and CLI `test-fixtures`. The focused listing found 113 library doctests and zero CLI doctests, all L1.

`lib/benches/perf.rs` is the sole Criterion harness. It registers ten case modules (`filesystem`, `git`, `git_ops`, `hardware`, `inventory`, `network`, `programs`, `repo`, `system`, `workload_matrix`) plus shared support and 78 benchmark registration sites. `ci-bench-ids.txt` selects 15 stable CI IDs. The route is `just bench` with `network,bench-internals`. No fuzz directory, targets, or cargo-fuzz manifest exists; `just fuzz` says not applicable.

## Runner override census

| Scope | Effective override | Judgment |
|---|---|---|
| Default, all Sniff | retries 0; slow 5 s, terminate 30 s; leak 30 s fail | justified: exposes slow/hung/leaked work; six distinct L1 tests were slow, none timed out/leaked |
| CI, all Sniff | retries 0; slow 30 s, terminate 90 s; leak 30 s fail | justified CI contention allowance; no Sniff-specific extension |
| CI Windows, both packages | `sniff-windows-l1`, max threads 1 | follow-up: comment cites overlapping host-network API fail-fast; retain pending Windows evidence |
| CI detector elapsed test | threads required `num-cpus` | justified: elapsed-time assertion needs idle runner |
| L2 | retries 0 | justified: terminal failure remains visible |

No other override selects Sniff. Phase 1 ran 2,599 L1 tests in 28.552 s (439.14 s summed), sanity 1,820 in 8.345 s (112.19 s summed), and L2 two in 1.410 s, with no failures, retries, timeouts, or leaks.

## Recipe and reachability findings

- `test-real` uses `network`, not `remote`, and reaches all eight embedded `package::network::tests::real_*`; none requires `remote`.
- Local `test` has 790 CLI identities. CI/sanity/L2 use fixtures and have 792, adding only the two terminal tests/helper bins.
- `install_interactive_pty` is Unix-compiled but no recipe sets `SNIFF_INTERACTIVE_PTY=1`; its no-op is unreachable, assigned remediation.
- `lib/tests/foo.rs::bar` is a dead placeholder assigned remediation; deletion cannot count as optimization coverage.
- The eight real network tests are embedded in `lib/src/package/network.rs`, explicitly separated as `lib-real-network`.
- `serial_test` is process-local. Environment-mutating provider cases and the shared tmux session have real within-binary resources; all other uses require Phase 7 review because it does not coordinate nextest processes.

## Phase 2 disposition summary

No family was excluded or moved because it was slow. CLI spawn families, the bespoke PTY gate, L2 readiness sleeps, and placeholder binary are marked for later remediation. Native and external-resource coverage remains real. Other families are satisfactory for this phase; budgets remain Phase 3.

## Phase 3 attribution and budgets

The attribution tool joined the Phase 1 JUnit report to all 81 enumerated
families. The local L1 run spent 28.552 seconds of runner elapsed time and
439.14 seconds of summed test duration; build/setup was 10.88 seconds. Those
three costs are intentionally not combined. The table below retains every
family responsible for at least two percent of summed duration, plus every
native-detector family and the other remediation targets needed for the Phase
4–7 decisions. Families below one percent remain individually enumerated in
`family-members.json` and inherit the budget class assigned below; their total
is not treated as free or excluded work.

| Family | Tests | Local summed | Local max | Attribution and budget class |
|---|---:|---:|---:|---|
| `lib-unit-filesystem` | 587 | 93.18 s | 9.40 s | Owned filesystem fixtures; W1 |
| `lib-bench-fixtures` | 5 | 29.18 s | 13.05 s | Shared deterministic repository builders; W1 |
| `lib-integration` | 117 | 27.98 s | 6.44 s | Intentional native observation; H1 |
| `lib-git-parity` | 85 | 24.84 s | 1.95 s | Owned repositories; W1 |
| `cli-software` | 21 | 21.35 s | 3.21 s | Deterministic software roots, but ambient CWD/config remains; C0 |
| `cli-unit-output` | 281 | 16.74 s | 0.19 s | In-process projection; D0 |
| `cli-repo-json-aggregate` | 16 | 14.47 s | 4.79 s | Aggregate behavior intentionally observes an owned repository; C1 |
| `lib-unit-programs` | 285 | 13.94 s | 1.06 s | Owned inputs and bounded tool stubs; D0/W1 by test |
| `cli-repo-leaf-e2e` | 12 | 12.31 s | 2.02 s | Focused request against an owned repository; C1 |
| `cli-terminal-subset` | 8 | 12.07 s | 4.67 s | Intentional terminal/native observation; H1 |
| `cli-snapshots` | 14 | 10.03 s | 1.35 s | Deterministic output with ambient process state; C0 |
| `lib-unit-hardware` | 50 | 9.87 s | 2.05 s | Parser/projection tests use owned inputs; D0 |
| `cli-filesystem-detail` | 14 | 9.85 s | 3.50 s | Owned repository/filesystem request; C1 |
| `lib-unit-os` | 165 | 7.58 s | 0.17 s | Parser/projection tests use owned inputs; D0 |
| `lib-unit-network` | 55 | 7.06 s | 2.09 s | Owned inputs/local protocols; D0/W1 by test |
| `lib-merge-conflict-prediction` | 13 | 6.84 s | 1.57 s | Captured tips and owned repositories; W1 |
| `lib-unit-process` | 13 | 6.64 s | 3.09 s | Bounded owned children; P1 |
| `cli-section-subcommands` | 11 | 4.80 s | 1.56 s | Intentional native observation; H1 |
| `cli-hardware-detail` | 8 | 0.88 s | 0.20 s | Intentional native observation; H1 |
| `cli-services` | 5 | 0.51 s | 0.12 s | Intentional native observation; H1 |
| `lib-host-capability-cache` | 6 | 0.23 s | 0.04 s | Intentional native observation; H1 |
| `cli-install-interactive-pty` | 1 | 0.01 s | 0.01 s | No-op because its bespoke gate is unreachable; no performance credit; E1 after routing |
| `lib-foo` | 1 | 0.02 s | 0.02 s | No product assertion; removal receives no performance credit |

The stored work-count workload supplies compatible, collector-propagated
readings for the repository-heavy suspects: a full staged filesystem request
starts one walk, visits 638 entries, performs one Git discovery and one status
walk, and performs no subprocess, remote, or WAN work. The focused summary
also performs one Git discovery and one status walk. Independently, the real
aggregate CLI regression asserts one discovery, one status walk, one ref walk,
and zero linked-worktree opens. These are request-shape budgets, not ratios
inferred from timings. Phase 1 did not collect a counter report from every CLI
test process, so no per-process zero is fabricated: Phase 4's contamination
probes must establish the C0/C1 values under the shared fixture before Phase 6
compares them.

### Ratified budget classes

Each family in the reconciled index has a budget through its purpose and the
following class. This keeps budgets adjacent to the baseline without turning
one noisy local sample into a CI target.

| Class | Families | Ratified work budget | Timing budget |
|---|---|---|---|
| D0 | Deterministic in-process parsing, planning, serialization, and projection families | Zero host, Git, filesystem-walk, subprocess, remote, or WAN observations unless the individual test explicitly supplies that owned input; no request widening | Preserve identity count and current slow/timeout policy; numeric CI family target pending three compatible green runs |
| W1 | Deterministic repository, filesystem, Wiremock, benchmark-fixture, and process-stub families | Only owned fixture work; one acquisition per operation; seeded execution and projection perform zero reacquisition; Wiremock request counts and process deadlines remain asserted | Same pending CI rule; semantic timeout floors remain unchanged |
| C0 | Deterministic CLI families that do not request repository/host discovery | After Phase 4 isolation: zero accidental checkout, user config/cache, Git plumbing, software-roster, or rendering discovery | Same pending CI rule |
| C1 | Deterministic CLI repository/filesystem families | Owned fixture only; request tier must not widen; aggregate retains aggregate execution; focused commands retain focused execution; ordinary projection reacquires nothing | Same pending CI rule |
| H1 | `lib-host-capability-cache`, `lib-integration`, `cli-terminal-subset`, `cli-section-subcommands`, `cli-hardware-detail`, and `cli-services` | Preserve real platform APIs and bounded subprocess behavior; never substitute captured/fake data to claim a reduction | Assessed separately by native environment below; numeric CI target pending three compatible green runs |
| P1 | Process-owning deterministic tests | Existing deadlines, drain-while-waiting, termination behavior, timeout floors, and zero leaked children | No raised timeout or retry; numeric CI target pending |
| E1 | Real network and L2 terminal families | Preserve the real resource, explicit gate, bounded protocol/readiness, and cleanup | L2 keeps the observed 1.410 s local cohort as directional evidence; external network timing remains variable and is not an L1 budget |

The only numeric area-level timing budget ratified from local evidence is the
existing `just sanity` ceiling: 15 seconds runner elapsed. Phase 1 measured
8.345 seconds. Per-family CI timing budgets remain explicitly **pending**:
run 34008778001 is one green sample per environment, while the audit tool
requires at least three and refuses local provenance. Phase 8 will collect the
required matched samples before judging candidate performance.

### Draft decisions

1. **Intentional versus accidental host observation.** H1 and E1 observations
   are intentional product coverage. W1/C1 repository and tool work is
   intentional only against fixture-owned inputs and within the requested
   tier. Any ambient checkout, user configuration/cache, Git plumbing,
   software roster, or rendering input in D0/C0—and any such input beyond the
   owned request in W1/C1—is accidental. Aggregate tests continue to execute
   aggregate discovery; they are not narrowed for speed.
2. **Existing seams and remaining gap.** Parser/planner/projection families use
   direct owned values or captured observations. Git parity, repository CLI,
   and benchmark families use `lib/tests/fixtures.rs` or
   `lib/benches/support/builder.rs`; remote families use captured snapshots and
   Wiremock; focused CLI families use existing focused request plans; process
   tests use bounded child helpers. The one cross-family gap is a shared CLI
   process fixture that applies the same CWD/environment/PATH policy to
   `assert_cmd::Command` and `std::process::Command`; Phase 4 supplies it and
   its structural guard.
3. **Feature and tier reachability.** `network` alone reaches the eight
   `real_*` tests but not the `remote` provider suites; `remote` transitively
   supplies both. Bare CLI L1 has 790 identities; `test-fixtures` adds the two
   L2 tests/helper binaries for CI, sanity, and `test-l2`. No L3/browser route
   applies. The sole unreachable assertion is
   `install_interactive_pty`, whose test body exits unless the bespoke
   `SNIFF_INTERACTIVE_PTY` variable is set.
4. **Bespoke gates.** Remove `install_interactive_pty`: its manufactured PTY
   did not exercise a real-terminal behavior distinct from the reachable L1
   install-interview test, and its `SNIFF_INTERACTIVE_PTY` gate made it absent
   from every canonical recipe.
   Delete `lib/tests/foo.rs`; it proves no behavior and its removal is not a
   speed improvement. No other bespoke gate was found.
5. **Native-platform cost and concurrency.** Native detectors stay real and
   separately measured. The current Windows serialization is retained: the
   one available Windows run is green but fully serial (runner elapsed and
   summed duration are effectively equal), so it supplies no safe
   counterfactual evidence that overlapping host-network calls can run in
   parallel. No concurrency change is proposed in this fix without a bounded
   Windows experiment.

### Native-detector cohort

The cohort below is exactly the six H1 families; it excludes deterministic
hardware/OS parser tests and all fake/captured projections.

| Environment | Native summed duration | Scheduler decision | Timing-budget status |
|---|---:|---|---|
| local native macOS | 46.46 s | Normal local scheduler | Attribution only |
| `macos-latest` | 38.50 s | Normal CI scheduler | Pending two more green baselines |
| `ubuntu-latest` | 13.34 s | Normal CI scheduler | Pending two more green baselines |
| `windows-latest` | 36.91 s | Retain `sniff-windows-l1` at one process | Pending two more green baselines and any concurrency experiment |
| `wsl2-ubuntu` | 36.77 s | Normal WSL scheduler; not Windows-native | Pending two more green baselines |

The Windows attribution gate also reported two passing Windows-only
integration binaries (`windows_app_paths_orphan` and
`windows_find_program_priority`) that are recorded in the platform-exclusion
table but are not matchable by the macOS-derived family rules. They contribute
0.03 seconds combined and do not change the native cohort. This is an evidence
classification limitation, not lost execution or a production defect.

### Counter accounting and defect disposition

Acquisition and execution are compared only when their collector boundaries
match. Acquisition budgets include fixture construction and observation
capture; execution budgets begin after seeding and require zero Git
rediscovery or observation reacquisition. Phase 6 proved `WorkerCollector`
propagation through spawned threads, Rayon jobs, and parallel walkers before
interpreting lower counts; comparisons still require identical request shapes,
counter versions, platforms, phases, and caching-boundary sides.

Attribution surfaced no production defect, so Phase 3 opens no separate
production issue/spec. The Windows family-classification limitation above is
kept in the audit evidence and does not warrant a product-code fix.

## Phase 6 library requested-work audit

The deterministic external library targets were scanned for aggregate
`detect`, `detect_with_plan`, `detect_with_config`, and `ProgramsInfo::detect`
calls. The only incidental host-wide call was in `program_installable.rs`:
both installability contracts now construct an empty public
`CategoryDetector` and retain the exact platform-specific program inputs.
`integration.rs` keeps one convenience `detect()` call because that test's
subject is the aggregate latency contract; its remaining plan calls already
select only the domains their assertions consume. The focused provider,
remote observation/provider/resolution, host-capability cache,
merge-conflict, install-plan, serialization, and Windows targets already use
owned values, local repositories, registry state owned by the test, or
Wiremock and need no request narrowing.

### Counter and worker-propagation proof

`benchmark_workloads::seeded_git_execution_and_projection_do_not_rediscover_the_repository`
uses the public library boundary and the shared dirty-repository builder. It
accounts for acquisition and execution with separate collectors: acquisition
records one `GIT_DISCOVERIES`; seeded full Git execution followed by
`detect_file_changes` records zero discoveries, zero known-path opens, and
two requested status walks. The public result remains dirty and the four
fixture paths remain present, so a missing counter cannot masquerade as
missing work. The formatting workload now also asserts zero Git discoveries
and status walks alongside its zero descendant-walk counters.

Lower counter readings are supported by these independently targeted proofs:

| Worker route | Test and observable oracle |
|---|---|
| scoped threads | `integration::test_performance_collector_thread_local_aggregation` and `filesystem::planner_counter_propagation::git_stage_counters_survive_the_scoped_thread`; exact main/worker totals and one dirty status walk |
| pool/Rayon worker | `performance::tests::pooled_worker_attributes_thread_work_to_the_request`; four workers contribute exactly 20 increments |
| shared parallel walker | `filesystem::system_view::tests::parallel_and_serial_walks_agree_on_accepted_work`; parallel accepted-work count equals an independent serial scan |
| manifest walker | `filesystem::repo::detection::observation_index::manifest_index_build_reports_its_reads`; manifest file opens and bytes are nonzero |
| network thread hop | `network::tests::endpoint_attempts_are_counted_across_the_thread_hop`; two local Wiremock attempts survive the hop |

### Fixture size, snapshots, and assertion dispositions

The benchmark-fixture targets retain their separate large-topology tests:
`large_monorepo_has_expected_shape` proves the 90-package/21-commit shape and
`large_monorepo_dirty_files_are_rust_and_js` proves its cross-ecosystem dirty
state. The repeatability test did not need to build that fixture twice; it now
uses two fresh 10-package mixed-ecosystem repositories and still compares
commit count, dirty count, and Rust package fan-out. The focused target fell
from the Phase 1 family's 29.18-second summed baseline (13.05-second maximum)
to 0.71 seconds elapsed in the Phase 6 diagnostic run; Phase 8 remains the
measurement authority.

There are no Insta/golden snapshot normalizers in `sniff/lib/tests`. Remote
"snapshots" are typed captured observations whose tests assert provider
identity, selection, ordering, request count, and typed errors; no volatile
field stripping was changed. The two literal self-comparisons found in
embedded unit tests were strengthened without changing test identities:
`StableUserId` equality is now observed through independent `HashMap` lookup
keys across same/different values, and `ExecutableSource` compares enum values
against independently deserialized wire values before checking cross-variant
inequality. The existing success checks in recent-commit tests all lead to
dependent commit, file, Markdown, or typed-error assertions and therefore are
not success-only tests.

Phase 3's only remediation-in-scope library population was the empty
`lib/tests/foo.rs` target, already removed and mapped in Phase 5. No linked
production follow-up was identified, and no parser, schema, shipped artifact,
configuration, persistence, or snapshot format changed in Phase 6.

## Phase 7 external-effect and cleanup audit

The three deterministic remote targets use Wiremock's unique ephemeral
loopback listeners. Focused-provider and observation tests use deny-by-default
host policies, assert denial before I/O, preserve exact-host consent and
credential scope, and distinguish malformed, missing, authentication,
rate-limit, capability, and transport outcomes. Provider snapshot and focused
pagination tests assert exact request counts and hard result/domain bounds.
Wiremock guards own server shutdown; no test starts an unjoined custom server
thread. The sole exception found was
`from_shorthand_tries_github_when_token_set`: it called public APIs with fake
credentials, accepted every transport error, and could not prove its stated
GitHub-selection claim. It was removed; the local `GitRemote` dispatch test and
provider-specific Wiremock suites retain every meaningful assertion.

### Process ownership and timing floors

| Cohort | Drain/deadline/reap proof | Retained timing rationale |
|---|---|---|
| `process::tests` | The production supervision boundary drains stdout and stderr on concurrent reader threads, applies an injected deadline, terminates recorded descendants/process groups or Windows jobs, waits on the direct child on every failure path, and joins the readers. Targeted tests assert complete 1 MiB output on both pipes, timeout typing, `ECHILD` reaping, detached-descendant termination, and bounded inherited-pipe cleanup. | Fixture children retain 30-second sleeps solely to remain alive beyond 100–3,000 ms injected deadlines. The 40 ms poll cadence matches the process supervisor; 3–5 second assertion margins cover signal delivery/reparenting under CI load without raising the runner's 30-second ceiling. |
| remote-refresh timeout | The real configured-command boundary runs the ignored 30-second child with a zero deadline, returns typed timeout, and finishes below five seconds. | The child sleep is intentionally longer than the deadline; the five-second upper assertion is termination margin, not a blind readiness wait. |
| CLI PTY | `SniffCliFixture` owns the isolated directory and command environment; `expectrl::Session` owns the child and drops it on every `?` error. The test now gives both heading and EOF observations an explicit ten-second expect deadline. | No semantic sleep remains. Ten seconds bounds slow native OS detection while nextest's 30-second process/leak ceiling remains the outer guard. |
| L2 tmux pair | The canonical broker owns and trap-cleans the shared tmux session. Each test clears stale pane content and polls every 40 ms for its complete final plain/raw predicate, returning the last frame after five seconds for assertion diagnostics. | Five seconds preserves the former harness prompt budget while the fast path observed 0.315/0.338 seconds. The removed 200 ms sleeps had no semantic floor. |
| CLI/Git setup commands | Commands consume complete output, so successful return implies pipes were drained and the child reaped. The per-test nextest 30-second termination/leak policy is the outer deadline and the root leak sweep detects detached survivors. | No deliberate child delay or timing assertion. |

Serialization remains only where process-global state is real under the Cargo
fallback runner: credential/environment mutation, cwd mutation, and Git config
fallback. The two L2-local `#[serial]` attributes were removed because they
cannot coordinate separate integration-test processes; canonical `just
test-l2` provides runner-visible `-j 1` serialization for its genuinely shared
broker pane. `sniff-windows-l1` remains at one process: the single available
Windows baseline is green but insufficient for an evidence-based concurrency
change, and WSL evidence is not native-Windows evidence.

## Phase 8 local candidate measurement

The preserved detached baseline and active candidate were warmed, then run in
strict baseline-to-candidate order for five full L1 and five sanity rounds.
Complete provenance, commands, per-run load, raw logs, identities, and three
separate cost columns are retained in
[`measurement/local-phase8/`](measurement/local-phase8/). Both source states
remain before the `production-caching-2026-07-22` boundary. The baseline's
only dirty entry is its untracked preserved build cache; candidate provenance
records the shared worktree's 1,361 dirty paths rather than implying unrelated
monorepo edits were absent.

| Cohort | Baseline alternating median | Candidate alternating median | Coverage and disposition |
|---|---:|---:|---|
| full local L1 runner / summed | 21.53 / 324.99 s | 44.80 / 684.81 s | 2,599 versus 2,609 identities; +13/−3 are the reconciled Phase 4–7 changes. All runs passed, but candidate timings are confounded by host load and shared-worktree state and are attribution only. |
| sanity runner / summed | 7.94 / 104.80 s | 17.03 / 254.82 s | 1,820 stable identities; all runs passed. Candidate runner spread 16.73–19.17 s misses the 15-second budget on all five alternating samples under this host load. |
| all `sniff-cli` integration binaries | 115.54 s summed | 94.63 s summed | Directional 18.1% improvement, inside the baseline/candidate drift brackets; no causal timing claim. |
| requested-work / representative fixtures | 23.45 s summed | 4.30 s summed | 81.7% lower and outside both drift brackets; one new seeded-work identity retains dependent result and counter assertions. |
| inherited-Git fixture repairs | 29.89 s summed | 51.17 s summed | 71.2% higher under the loaded candidate state; no product regression inferred. |
| remote-provider ownership audit | 5.07 s summed | 158.90 s summed | The 70 loopback-owned tests clustered near 2.4 s each despite the only target change being removal of the weak live-network test. Treat as a host/candidate-state confounder for Phase 9 CI comparison, not a Sniff production conclusion. |

Build/setup medians stayed separate from runner elapsed: full L1 was 1.2 s
baseline and 1.1 s candidate; sanity was 2.6 s baseline and 2.4 s candidate.
No cold-build claim is made, so no cold-build measurement applies. An
inadvertent release-profile `work_counts` build is retained as a diagnostic
and excluded from every comparison.

The changed timing contracts each completed ten times with no failure, retry,
timeout, or leak: the CLI PTY deadline was 0.304–0.446 s; the tmux CI/CD final
frame was 0.317–0.378 s; and the tmux Git final frame was 0.340–0.374 s. The
L2 runs used the canonical broker route with tmux required, so an unavailable
or unexecuted backend could not satisfy the evidence.

The Phase 1 `staged_filesystem_full_all_stages` counter shape was collected
again under the same native-macOS environment, dev profile, request shape,
counter version, total phase, propagated collector, and boundary side. All
eight signal aggregates are unchanged: filesystem walk 639, filesystem I/O
17,337, inventory 1,367, repo structure 185, Git 19, and zero process, remote,
and WAN work. This unchanged production case is the drift bracket. Eliminated
test-only rediscovery remains proved by the Phase 6 targeted sentinels:
acquisition records one Git discovery, seeded execution/projection records
zero additional discoveries and opens while retaining dirty state and all four
paths, and the worker-propagation tests prove counter completeness.

## Phase 9 CI evidence handoff

The local affected-scope validation is complete. Root `just ci-local sniff`
selected exactly `sniff --features remote` and `sniff-cli --features
test-fixtures`; its final run passed the CI-infrastructure preflight, both
package lints, 1,808 library tests, and 801 CLI tests. Area `just test` then
passed 2,609 tests with 24 declared policy skips. `just lint`, `just check`,
and `just doctest` passed; doctests ran 91 and ignored 22. Required-tmux
`just test-l2` passed both affected tests in 0.675 seconds and recorded two
tmux `run` decisions.

The first consolidated attempt exposed the exact Clippy input
`assert!(host.len() >= 1)` in the new fixture contract test. Replacing it with
the equivalent non-empty assertion fixed the lint; the named test, complete
L1 population, and final consolidated validation all passed afterward. This
is a test-source lint correction and changes no Sniff product behavior.

Candidate CI evidence is pending. The working candidate is uncommitted at
local HEAD `c2dee9217f3e6be14d7a6adfeb2c90cd2cd31966`, GitHub CLI has no
authenticated session, the public API reports no current
`fix/cli-slow-tests` branch, and the session is explicitly prohibited from
committing, staging, or pushing. Therefore there is no exact candidate SHA,
PR, `windows-latest` result, or three-run environment series to fetch. The
preserved Phase 1 run `34008778001` remains baseline-only and must not be
presented as candidate evidence.

After an authorized human commits and pushes this exact source state, run the
declared `_package-ci.yml` cells for native Ubuntu, macOS, and Windows plus the
`_wsl-ci.yml` WSL2 cells. Retain three consecutive green candidate runs per
leg, fetch them with the shared test-audit tool, compare matched tests against
the compatible per-environment baseline, and evaluate the Phase 3 budget
classes. Additions, removals, platform exclusions, and intervening failures
must remain separate. `windows-latest` is the only Windows compile/runtime
authority; WSL2 is not a substitute.

`test-real` was not run: Phase 9 changed no real-resource behavior, and the
audited route still selects `sniff --features network` plus bare `sniff-cli`.
That selection reaches the real network tests but intentionally does not reach
the `remote` Wiremock suites. No relevant available real resource was required
for the Phase 9 lint correction.
