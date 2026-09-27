---
total_phases: 6
created: 2026-09-17
phase: 1
agent: codex/default
yolo: true
---

# Execution Plan: Unify Markdown Hyperlink File Resolution

## Work Summary and Success Criteria

This fix introduces one reusable Darkmatter hyperlink-target resolver, backed by
`biscuit_file::FileReference`, and makes the reference graph, reference
validation, CLI graph surfaces, and DMLS consume its structured result. Request
owners will capture file-resolution state once, turn source files into absolute
identities, and derive per-document contexts with `for_source`; no downstream
consumer will rediscover the process CWD, repository, home, or environment.

Completion means the Messenger `^docs/topics/...` reproduction succeeds for
relative and absolute input spellings; every local `FileReference` form retains
its candidate ordering and typed failure; fragments, external targets, and wiki
links keep their distinct behavior; and DMLS definitions, document links,
diagnostics, reference edges, and invalidation all agree on the same resolved
identity. The package's L1 library, CLI-process, and LSP-protocol tests must pass
portably on macOS, Linux, native Windows, and WSL2 without shell execution,
remote fetches, document mutation, path string replacement, or real editor
interaction.

## Phase 1: Lock Resolution and Context Contracts

### Necessary Rules

- The reusable resolver belongs to the public
  `darkmatter::markdown::reference` surface because `dmls` is a separate crate;
  neither the CLI nor DMLS may parse sigils, reconstruct candidate ordering, or
  split an already-resolved result a second time.
- The result is a structured value that preserves the authored target and
  optional fragment and distinguishes same-document fragments, external
  targets, matched local paths, well-formed local no-match, and typed local
  failures. Local results expose the parsed `FileReference` class/effective
  kind needed for diagnostics without turning error strings into policy.
- External/fragment classification happens before local parsing. HTTP(S),
  protocol-relative, `mailto:`, `tel:`, `data:`, and other supported schemes
  never reach local probing; DMLS never fetches them.
- Request contexts are captured at the CLI or LSP initialization boundary.
  Every file-backed author uses an absolute source identity and `for_source`;
  ambient compatibility APIs may remain only for callers that do not supply an
  explicit request context.
- A DMLS multi-root request owns one captured context per workspace folder.
  The most specific containing folder supplies a document's context; unrelated
  roots never become magic or fallback candidates. An absolute document outside
  all roots receives only an explicit context derivable from that URI, so
  repository-dependent forms fail with typed missing-context state.
- DMLS stores the resolved local path independently of whether it is an indexed
  Markdown document. Only indexed Markdown targets gain graph-node/heading
  edges, while any existing local file may back `documentLink` and dependency
  tracking.
- Wiki-link matching, transclusion grammar, rendered browser URLs, graph
  document IDs, heading slug rules, and `biscuit-file` grammar/candidate order
  are unchanged.

### Risk-Lowering Spikes

#### Wave 1 — Baseline and characterization

- [ ] **Task 1.1: Baseline Evidence.** Record `git status --short`, the current
  Messenger reproduction result, and focused Darkmatter L1 results before any
  implementation. Run GitNexus upstream impact analysis for every existing
  symbol selected for editing, including `validate_local_path`,
  `classify_link_target`, `resolve_link`, `WorkspaceGraph::assemble`, and
  `WorkspaceIndex`; explicitly report the specification's CRITICAL DMLS graph
  risk before proceeding and confirm any `UNKNOWN` result with `rg`.

- [ ] **Task 1.2: Behavior Matrix.** Add or record focused characterization
  cases for inline and reference-style hyperlinks, fragments, external schemes,
  implicit repository fallback, and the current DMLS graph/provider behavior.
  Pin document IDs, heading edges, wiki edges, reverse indexes, and incremental
  generation behavior so later resolver work cannot silently change unrelated
  graph semantics. This task can run concurrently with Task 1.1 because it is
  read-only/test-only discovery.

#### Wave 2 — Interface and ownership decisions

- [ ] **Task 1.3: Resolver Contract.** Specify the concrete public result and
  error/no-match representation, fragment-splitting rules, remote/local policy,
  and mapping from `DetailedResolution`/`ResolutionFailure`. Require one call to
  return all information needed by validation and DMLS, including an existing
  non-Markdown local path, without exposing consumer-specific diagnostics.

- [ ] **Task 1.4: Context Ownership.** Map the CLI launch context,
  `ReferenceGraphOptions`, DMLS initialization roots, `WorkspaceIndex`, graph
  assembly, and per-document `for_source` derivation. Decide the stable
  longest-containing-root selection rule and identify the existing Darkmatter
  repository-scope capture helper that each boundary will reuse rather than
  introducing new discovery code.

- [ ] **Task 1.5: Invalidation Contract.** Define a path-keyed hyperlink
  dependency index separate from Markdown node membership. Specify how create,
  change, and delete notifications and the server-rescan fallback re-resolve
  affected sources, including resolved non-Markdown targets, without broadening
  unrelated workspace roots into candidates.

- [ ] **Validation checkpoint 1:** Review the result/context/invalidation
  contracts against all ten acceptance criteria and every row of the
  `FileReference` table. Confirm the plan requires no `biscuit-file` grammar
  change, no remote read, no wiki-link change, and no L2/editor-focus test.

## Phase 2: Build the Shared Hyperlink Resolver

#### Wave 3 — Resolver model and implementation

- [ ] **Task 2.1: Target Model.** Add the focused public hyperlink resolution
  types under `darkmatter::markdown::reference`. Preserve authored text,
  optional fragment, external or same-document classification, parsed local
  class/effective kind, resolved path, and typed failure/no-match. Keep source
  spans on existing reference records instead of duplicating them in the
  resolver model.

- [ ] **Task 2.2: Fragment Parsing.** Implement one target parser that separates
  local path and fragment without corrupting URL schemes, protocol-relative
  URLs, Windows drive-qualified paths, UNC paths, or separator forms. Route the
  local portion through `FileReference::new` and `resolve_detailed` using the
  explicit per-source context; never normalize with string replacement.

- [ ] **Task 2.3: Passive Policy.** Make local probing the only effect of the
  resolver. Prove that external targets are classified but not fetched and that
  resolution performs no composition, interpolation execution, shell command,
  or mutation. Tasks 2.1-2.3 form one coordinated wave because they define one
  public contract, but their focused tests may be authored concurrently once
  the type shape is fixed.

#### Wave 4 — Library consumers and exhaustive proof

- [ ] **Task 2.4: Graph Adoption.** Replace hyperlink-specific calls through
  `resolve_transclusion_target` with the shared hyperlink result in reference
  graph construction. Retain existing transclusion resolution, graph
  provenance, cache identity, composed ordering, document IDs, and heading
  extraction while ensuring local hyperlink path and fragment resolution are
  performed once.

- [ ] **Task 2.5: Validation Adoption.** Make local-path and cross-document
  fragment validation consume the same stored/shared resolution result. Map
  no-match to the existing missing-target issue and retain typed syntax,
  missing-context, repository-escape, I/O, and unsupported-remote failures
  without flattening them into a missing file.

- [ ] **Task 2.6: Resolver Matrix.** Add temporary-repository unit tests with
  explicit `FileResolutionContext` fixtures for `./`, `../`, bare paths,
  absolute paths, `~`, `@`, `&`, `^`, vault, `%`, and `{{VAR}}`, both with and
  without fragments. Prove document-first fallback, package/package-area order,
  exact repository-root behavior, magic/vault roots, recursive matching,
  malformed syntax, no-match, missing context, repository escape, I/O failure,
  and stability after CWD/environment mutation. Include portable Windows drive,
  UNC, and separator cases behind the appropriate target coverage.

- [ ] **Validation checkpoint 2:** Run the focused Darkmatter library tests via
  nextest/`just test` filters. Confirm all local candidates come from
  `FileReference`, all external cases avoid local probing, relative and absolute
  source identities agree, and existing reference graph provenance and
  mismatch tests remain green.

## Phase 3: Capture CLI Context Once

#### Wave 5 — Shared CLI boundary

- [ ] **Task 3.1: Input Identity.** Resolve `md validate refs` input once at the
  command boundary using the existing CLI file-path authority, load the
  document by that absolute path, capture repository/package-area/home/env
  state from the launch boundary, and derive the document context with
  `for_source`. Preserve the authored input spelling only for presentation.

- [ ] **Task 3.2: Option Plumbing.** Build one context-bearing
  `ComposeOptions`/`ReferenceGraphOptions` value and reuse it for ordinary
  validation and `md validate refs --graph`. Do not allow either branch to
  construct `ReferenceGraphOptions::default()` after the input has been loaded.

- [ ] **Task 3.3: Graph Boundary.** Route `md graph --validate` and its
  `FileTree` builder through the same absolute-input/context helper. Preserve
  text, JSON, Mermaid, DOT, exit-code, and terminal-rendering behavior; only
  resolution ownership changes. Tasks 3.1-3.3 should be implemented together
  because they establish one CLI request boundary.

#### Wave 6 — CLI regression matrix

- [ ] **Task 3.4: Fixture Repository.** Extend `CliProcessFixture` coverage with
  a nested package-area repository and a real `^...#heading` target. Exercise
  `md validate refs` from the package area, from the repository root, and with
  an absolute input, plus `--fragments`, `--graph mermaid` or `dot`, and
  `md graph --validate`.

- [ ] **Task 3.5: Isolation Proof.** Declare only the fixture CWD/home/env/PATH
  policy required by each test and prove a post-capture ambient CWD or
  environment change cannot affect the result. Add negative cases for typed
  repository escape and missing context without inheriting the developer's
  repository or home.

- [ ] **Validation checkpoint 3:** Run the CLI integration target through the
  Darkmatter `just test` recipe. Re-run the exact Messenger reproduction and
  confirm every input spelling succeeds when targets exist, while JSON and
  graph output retain their established contracts.

## Phase 4: Give DMLS Stable Workspace Contexts

#### Wave 7 — Initialization and graph state

- [ ] **Task 4.1: Workspace Contexts.** Capture one
  `FileResolutionContext` for each absolute LSP workspace folder during
  initialization, including its repository scope catalog and captured
  home/environment. Store the immutable context set in server/index state and
  select the longest containing root for every indexed or open document.

- [ ] **Task 4.2: External Documents.** Derive document contexts from absolute
  file URIs. For documents outside initialized roots, construct only the
  explicitly available URI-based context and leave repository-dependent forms
  in typed missing-context state unless the server already owns a trusted
  context for that document.

- [ ] **Task 4.3: Link Substrate.** Replace DMLS's `is_external`/
  `classify_link_target` grammar and lexical `normalize_join` hyperlink path
  with the shared Darkmatter resolver result. Keep wiki links, transclusions,
  schema/file values, and their existing authorities separate. Store the result
  on each link fact/node so assembly, providers, diagnostics, and invalidation
  do not reparse or reprobe it.

#### Wave 8 — Edge and dependency assembly

- [ ] **Task 4.4: Reference Edges.** Build `references` edges from a matched
  local result to an indexed Markdown document root or fragment heading. Keep
  same-document anchors and external links stable; preserve an unresolved edge
  plus typed reason when syntax/context/path/fragment resolution fails.

- [ ] **Task 4.5: Path Dependencies.** Add deterministic forward and reverse
  indexes from source documents to resolved local paths independently of graph
  node membership. Use them to identify affected source documents before a
  target create/change/delete snapshot swap and preserve stable ordering and
  generation semantics.

- [ ] **Task 4.6: Change Tracking.** Feed hyperlink dependencies into both
  client-watched changes and server-rescan reconciliation. A target appearing,
  changing headings, or disappearing must refresh edges, navigation, and
  diagnostics without restart; open buffers remain authoritative and unrelated
  roots remain isolated.

- [ ] **Validation checkpoint 4:** Run focused graph/index tests proving that
  existing document IDs, wiki resolution, heading edges, compositional reverse
  indexes, generation behavior, and unchanged-content fast paths are preserved.
  Confirm a non-Markdown local target is retained as a resolved path without
  being invented as a Markdown graph node.

## Phase 5: Align DMLS Navigation, Diagnostics, and Protocol Behavior

#### Wave 9 — Provider consumers

- [ ] **Task 5.1: Definition Navigation.** Make
  `textDocument/definition` consume the resolved link result: navigate to an
  indexed target document root or matching heading, preserve same-document
  anchor behavior, and return no target for malformed, missing, escaped, or
  external links.

- [ ] **Task 5.2: Document Links.** Build `file://` URIs directly from matched
  local paths and reattach fragments. Return links for existing non-Markdown
  files even when no graph node exists, preserve external URIs, and emit no
  navigable target for typed failures/no-match.

- [ ] **Task 5.3: Typed Diagnostics.** Replace `diagnose_unresolved` lexical
  joins with the stored resolution state. Preserve existing diagnostic codes
  where applicable, distinguish missing fragments from path failures, include
  typed syntax/context/escape/I/O reasons in messages, and ensure `^`, `&`, or
  `@` never produces a false broken-path warning merely because its literal
  spelling is absent beside the source document. Tasks 5.1-5.3 can proceed in
  parallel after Wave 8 because they are independent consumers of the same
  immutable node state.

#### Wave 10 — Protocol and incremental proof

- [ ] **Task 5.4: Navigation Matrix.** Extend the normal `lsp_session` harness
  with real files and requests for `./`, bare repository fallback, `^`, `&`,
  and `@`. Assert definition and document-link URIs/ranges, a cross-document
  fragment heading, a resolvable non-Markdown file link, a missing target,
  malformed reference, repository escape, and external/fragment-only targets.

- [ ] **Task 5.5: Multi-Root Proof.** Initialize two unrelated workspace
  folders containing colliding candidate names and prove the source document
  selects only its containing context. Also cover an absolute file URI outside
  every root and verify repository-only forms report missing context.

- [ ] **Task 5.6: Incremental Proof.** Through LSP notifications and the
  server-rescan fallback, create, edit headings in, and delete a linked target
  after initial indexing. Assert reference edges, definition/document-link
  results, diagnostics, dependency fan-out, and snapshot generations update
  without restarting DMLS.

- [ ] **Task 5.7: Passive Proof.** Extend the no-side-effects protocol corpus
  with hyperlink forms containing environment interpolation and remote URLs.
  Prove DMLS reads only allowed local metadata/content and never executes
  expressions or shell commands, fetches remotes, mutates documents, focuses an
  editor, or depends on process CWD after initialization.

- [ ] **Validation checkpoint 5:** Run all DMLS L1 graph and protocol tests via
  `just test`. Confirm URI fragments and ranges are platform-correct and no
  test launches or focuses a terminal/editor window. No L2 run is required.

## Phase 6: Synchronize Documentation and Close Verification

#### Wave 11 — Documentation and portability

- [ ] **Task 6.1: Darkmatter Docs.** Update
  `darkmatter/docs/topics/file-referencing.md`, link-validation/reference-graph
  documentation, and affected public API docs to state that Markdown hyperlink
  path portions use `FileReference`, fragments are navigation metadata, and
  request-scoped callers must supply captured contexts. Correct any comments
  whose lexical-relative or `@`-means-repository wording becomes stale.

- [ ] **Task 6.2: DMLS Docs.** Update DMLS feature and diagnostics documentation
  to cover all local forms, typed failure behavior, non-Markdown document links,
  multi-root isolation, and the distinction between ordinary Markdown links,
  external URLs, and wiki links. Update the Darkmatter skill references if the
  public architecture or verification workflow changes. This task can run
  concurrently with Task 6.1, then both must be reconciled against tested
  behavior.

- [ ] **Task 6.3: Platform Audit.** Inspect every touched path operation for
  `Path`/`PathBuf`, portable URI conversion, Windows drive/UNC behavior, and
  case/separator assumptions. Ensure no Unix-only leading-slash assertion,
  manual `\\` replacement, ambient repository rediscovery, or cross-root
  fallback remains.

#### Wave 12 — Final gates and acceptance

- [ ] **Task 6.4: Quality Gates.** From `darkmatter/`, run `just build`,
  `just test`, and `just lint` without running `cargo fmt`. Run the focused CLI
  and protocol negative cases again, and obtain native Windows/WSL2/Linux CI or
  available host evidence for the portable L1 fixtures before making a
  cross-platform completion claim.

- [ ] **Task 6.5: Regression Command.** Run the exact Messenger reproduction
  from its package area, then repeat from the repository root and with an
  absolute input. Confirm fragment validation and both graph-validation
  surfaces use the same resolved target and typed error behavior.

- [ ] **Task 6.6: Change Audit.** Run GitNexus `detect_changes --scope all` and
  the compare-to-`main` regression view; re-run if either result is partial or
  truncated. Inspect `git diff`, preserve unrelated worktree changes, review
  touched comments/docs for drift, and confirm no `biscuit-file` grammar,
  transclusion, wiki, browser-rendering, or remote-fetch behavior changed. Do
  not commit or move the fix to `_completed`; leave it implementation-complete
  and ready for author review.

- [ ] **Validation checkpoint 6:** Map AC1-AC10 to named passing unit,
  CLI-process, graph/index, protocol, documentation, and platform checks.
  Closure requires the observed `^` regression to be green, one shared resolver
  on every hyperlink consumer path, stable request contexts, complete local-form
  and fragment coverage, DMLS navigation/diagnostic/invalidation parity,
  passive safety, multi-root isolation, and cross-platform L1 evidence.
