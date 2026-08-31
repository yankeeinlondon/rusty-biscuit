## Overview

The split period begins at `6cdb8bf` on 2026-07-16, the merge base of `claudine` and `proxy-with`; `error-prop-and-file-resolution` diverged later at `8fc8711`. The `claudine` side of that interval is a linear 15-commit history with no merge commits. It served primarily as the planning and documentation trunk: it added the execution plans for file resolution and error propagation, then continued with lifecycle-schema, CLI-switch, local-runner, and documentation work. No implementation of the four merge features was authored exclusively on `claudine` during the split. The implementation footprints described below are therefore the work that the fork branches must contribute to the merged branch, while the timeline and file blast radius record only direct `claudine` history.

### `2026-07-13-error-propogation`

This feature replaces lossy string-wrapped failures with end-to-end typed diagnostics. Its central contract is that in-process boundaries preserve concrete causes; process, wire, and persistence boundaries use a versioned `DiagnosticSnapshot`; and terminal rendering, lifecycle `err.*`, and machine-readable output all select the same effective diagnostic. The motivating missing-proxy-target failure must retain source and candidate context, render once as a structured component rather than a generic `Error:` line, and expose catalog-shaped detail without changing exit codes or lifecycle decisions.

- **Packages and modules:** `claudine` library diagnostics (`discovery`, `registry`, `facets`, `snapshot`, and restored diagnostics), composition error rendering and lifecycle context/execution, harness errors and resolution, reporting, dispatch, messaging/MCP consumers, and structured-stream provider parsers; `claudine-cli` output error walking, composition and harness orchestration, schema/sequence command paths, transport guards, and L2 capture suites. Follow-up wiring also reaches `claudine-contract`, `claudine-gen` tests, and repository error-transport scripts.
- **Acceptance criteria:** no known typed error may be flattened at an in-process production boundary; one registry must discover every Claudine diagnostic; rendering, `err.*`, and serialization must agree on diagnostic identity and one registered cause; resolution detail and frontmatter excerpts must remain lossless; rendering must remain exactly once and honor no-color behavior; existing exit/lifecycle/recovery semantics must not change; L1 registry/chain/snapshot tests, L2 CLI snapshots, drift guards, `just test`, `just test-l2`, and `just lint` must pass.

### `2026-07-13-file-resolution`

This feature makes `biscuit_file::FileReference` the sole syntax and resolution authority for document-backed paths. Explicit `./` and `../` references remain source-relative with no fallback; implicit bare references use repository-root-first and source-directory-second precedence; magic, package, vault, home, URL, recursive, interpolation, and platform-native absolute forms retain their defined meanings. Resolution is driven by an explicit request-scoped context so nested documents re-anchor to their own source without rereading ambient CWD, HOME, environment, repository, or package-area state.

- **Packages and modules:** `biscuit-file` library file-reference classification, context, candidate planning/probing, effective kind, home discovery, completion, and tests; `darkmatter` Markdown composition context, expressions, links, transclusion, reference graphs, and schema detect/format/resolve/rewrite/validate paths; `claudine` library harness resolution, composition prepare/resolve/preflight, sequence source loading, and system-prompt resolution; `claudine-cli` compose/sequence orchestration and shared completion.
- **Acceptance criteria:** all Claudine production parsing must go through `FileReference`; candidate order must be deterministic and observable; the motivating implicit router target must resolve without an `@` or `./` rewrite; all proxy routes and all Claudine/Darkmatter document surfaces must share the same semantics; failures must preserve ordered candidate/probe provenance as typed diagnostics; completion and execution must agree; native Windows and Unix absolute/home behavior must be covered; every workspace caller affected by the precedence change must be audited; L1/L2 tests and package-area test/lint gates must pass in `biscuit-file`, `darkmatter`, and `claudine`.

### `2026-07-11-sequence-plus`

Sequence Plus turns the former body-repetition feature into a typed task-and-group execution system. It defines stable per-step state (`state`, `previous`, `next`, and `sequence_id`), static and dynamic sources, file offsets/operators, formal task and group references, just-in-time composition, runtime `set` state, ordered outputs, lifecycle setup/teardown, serial and bounded-parallel scheduling, deterministic state merges, and attributed concurrent terminal output. The design deliberately makes step/task execution one model rather than separate headed and headless modes.

- **Packages and modules:** `claudine` library composition sequence submodules (`model`, `normalize`, `reserved`, `source`, grammar/data/expression handling, preflight, and task/group/shell execution), runtime state, lifecycle, looping, prepare/resolve, errors, and `render::task_stream`; `claudine-cli` compose preparation, sequence JIT/task/group orchestration, wrapper harness integration, streaming/performance output, and CLI/L2/L3 tests; `biscuit-file::list_format`; Darkmatter effects and Markdown composition context/interpolation/preflight; `biscuit-test-harness` cross-platform keyboard/terminal helpers; and a narrow Windows transport/test dependency adjustment in `rendezvous-daemon`.
- **Acceptance criteria:** retained sequence behavior must stay covered while invalid executable combinations, reserved writes, malformed sources, cycles, nested graphs, collisions, and invalid concurrency/timeouts become typed errors; every supported list and file format must be tested, including CRLF, Unicode, quoted delimiters, foreign scalar coercion, and empty dynamic sources; JIT state/output visibility and exact preflight shell bytes must be proven; serial/parallel ordering, teardown, merge, concurrency, interruption, and terminal framing must be deterministic and cross-platform; documentation must match the final grammar; `just test`, terminal-dependent `just test-l2`, and `just lint` must pass.

### `2026-07-13-proxy-with`

This feature makes a proxied prompt a canonical document handoff rather than an in-harness source-path substitution. A command-owned active-document coordinator consumes typed transitions and prepares every target through the same service used by direct execution, allowing the target to own its initialize stage, schema validation, file context, provider/launch plan, loop, shell approvals, closure, and output. The new optional mapping-valued `proxy.with` overlay is evaluated once in the source lifecycle context, applied transiently before target preparation, and layered between target-authored frontmatter and caller overrides.

- **Packages and modules:** `claudine` library composition coordinator, handoff/transition state, canonical preparation service, lifecycle parsing/evaluation/control, looping, schema translation, and typed error rendering; `claudine-cli` compose preparation, wrapper composition pipeline, harness orchestration/coordinator, target launch and session compatibility, launch-plan/environment rebuild, sequence integration, and L1/L2 seam and route tests; plus Darkmatter compose-context options and schema-validation support.
- **Acceptance criteria:** direct and proxied targets must use one preparation path and agree on document context, frontmatter, schema, lifecycle, provider/model/MCP, argv/environment, shell bytes, loops, closure, and typed failures; every proxy producer must return one consumed-or-rejected transition; initialize/terminal/loop routing must be atomic and must not synthesize source completion; retries and resumes must obey their specified refresh and compatibility rules; `with:` must preserve typed whole values, apply target < overlay < caller precedence, remove keys on null, survive immediate-target refreshes, avoid file/hash mutation and secret disclosure, and fail atomically; cross-platform L1 and real-CLI L2 equivalence matrices plus drift guards, `just test`, `just test-l2`, and `just lint` must pass.

## Timeline

Git does not record when a branch reference was created, so the two merge-base commits are used as the auditable fork markers. The list is chronological and includes every commit on `claudine` from the earlier fork point through `dc4cdeb`.

- **2026-07-16 — `6cdb8bf` — `docs(claudine): rename fleet-research topic to typed-knowledge pipeline`**
  - This is the merge base of `claudine` and `proxy-with`, and therefore the start of the three-branch split interval.
  - The commit itself is a 100% content-preserving documentation rename.
- **2026-07-16 — `f4180b3` — `docs(claudine): add execution plan for unified file-reference resolution`**
  - Added the phased implementation and cross-package migration plan for `2026-07-13-file-resolution` before that work moved onto the combined feature branch.
- **2026-07-16 — `8fc8711` — `docs(claudine): add execution plan for end-to-end typed error propagation`**
  - This is the later merge base of `claudine` and `error-prop-and-file-resolution`.
  - It established the planned error-propagation architecture around semantic adapters, typed snapshots, registry completeness, rendering parity, regression guards, and L2 acceptance.
- **2026-07-18 — `1877870` — `docs(claudine): rename lifecycle actions to lowercase and align recovery docs`**
- **2026-07-18 — `cdff9a8` — `feat(claudine): scaffold lifecycle ergonomics schema files`**
  - Introduced the schema-document scaffold for actions, lifecycle configuration, and the three composition commands; these files are likely merge-adjacent because the fork work also changes lifecycle and composition contracts.
- **2026-07-18 — `2852c4b` — `feat(claudine): add lifecycle ergonomics feature spec`**
- **2026-07-18 — `38a35f6` — `chore: refine implement-suggestions and feature-review prompts`**
- **2026-07-18 — `f35e367` — `docs(claudine): add CLI switch forwarding execution plan`**
- **2026-07-18 — `ffa25c4` — `chore: refresh gitnexus counts and complete lock-contention lesson`**
- **2026-07-19 — `8c2f54e` — `docs(claudine): add local-runners-plus improvement report`**
- **2026-07-20 — `d1108ed` — `chore: refresh gitnexus counts`**
- **2026-07-20 — `8e43a6c` — `docs(claudine): record Task 4 and 2026-07-19 verification pass in improvements`**
- **2026-07-20 — `99b4821` — `fix(claudine-cli): make gitignore entry name the directory we created`**
  - This is the interval's only direct production Rust change on `claudine`.
  - It aligns the transient system-prompt directory's `.gitignore` entry with the directory Claudine actually creates, updating the composition pipeline, system-prompt wrapper stages, tests, and topic documentation together.
- **2026-07-20 — `6a4c71c` — `docs(claudine): fix typo in completions topic`**
- **2026-07-20 — `dc4cdeb` — `docs(claudine): land identity_probes and version_probe schema with runner docs`**
  - Extended the local-runner fleet schema and all five runner records with identity/version probe documentation, which is unrelated to the four merge features but is direct `claudine` work that must survive the merge.

## File Blast Radius

These are all paths mutated directly by the 15 `claudine` commits in the interval. The history is linear, so no merged-parent files are included. Both sides of the documentation rename are listed.

- Repository metadata and shared prompts:
  - `.claudine/memory/commits.md`
  - `CLAUDE.md`
  - `prompts/_implement/implement-suggestions.md`
  - `prompts/_reviews/feature-review.md`
- `claudine-cli` wrapper modules:
  - `claudine/cli/src/commands/wrap/composition/pipeline.rs`
  - `claudine/cli/src/commands/wrap/system_prompt.rs`
  - `claudine/cli/src/commands/wrap/system_prompt/tests.rs`
  - `claudine/cli/src/commands/wrap/wrapper_stages.rs`
- Local-runner research:
  - `claudine/docs/research/local_runners/_fleet.md`
  - `claudine/docs/research/local_runners/_schema.yaml`
  - `claudine/docs/research/local_runners/llamacpp.md`
  - `claudine/docs/research/local_runners/lmstudio.md`
  - `claudine/docs/research/local_runners/ollama.md`
  - `claudine/docs/research/local_runners/omlx.md`
  - `claudine/docs/research/local_runners/vllm.md`
- Lifecycle/composition schemas:
  - `claudine/docs/schemas/action.yaml`
  - `claudine/docs/schemas/claudine.yaml`
  - `claudine/docs/schemas/compose.yaml`
  - `claudine/docs/schemas/inline-compose.yaml`
  - `claudine/docs/schemas/lifecycle.yaml`
  - `claudine/docs/schemas/sequence.yaml`
- Topic documentation:
  - `claudine/docs/topics/agentic-research-as-a-typed-knowledge-pipeline.md` (rename destination)
  - `claudine/docs/topics/completions/index.md`
  - `claudine/docs/topics/composition.md`
  - `claudine/docs/topics/execution-flow.md`
  - `claudine/docs/topics/fleet-research-in-claudine.md` (rename source)
  - `claudine/docs/topics/non-interactive-sessions.md`
  - `claudine/docs/topics/system-prompt.md`
- Feature/fix planning:
  - `claudine/features/2026-07-13-error-propogation/plan.md`
  - `claudine/features/2026-07-13-file-resolution/plan.md`
  - `claudine/features/2026-07-20-lifecycle-ergonomics/spec.md`
  - `claudine/features/2026-07-20-local-runners-plus/improvements.md`
  - `claudine/fixes/2026-07-13-cli-switches/plan.md`
