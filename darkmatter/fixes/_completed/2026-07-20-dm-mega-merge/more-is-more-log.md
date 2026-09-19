## Overview

The `more-is-more` branch turned two related authoring ideas into a broad, cross-package capability set. `2026-07-13-more-is-more` expanded composition with indexed-file discovery, structured expression literals, Git/worktree state, hermetic conflict prediction, and provider-neutral remote repository queries. `2026-07-13-meta-schema` made SimplifiedSchema capable of describing its own property definitions and whole schema declarations, then carried that semantic knowledge into validation, the CLI, and DMLS. Both streams converged on shared authorities rather than parallel implementations: Sniff owns Git and provider discovery, Darkmatter owns expression/schema semantics and Markdown projection, and DMLS consumes passive parse products without performing repository mutations, shell execution, or network access. At branch head `0584d8297f57f5eb30b52d03b1241ba55184bb44`, More-Is-More Review 27 calls all 30 criteria production-ready, while Meta-Schema Review 14 calls its 13-criterion feature production-ready and closed.

### `2026-07-13-more-is-more`

This feature aimed to make composed documents substantially more aware of their local file series, repository state, and remote delivery systems. The local expression surface gained `find_first_index(file)` and `find_last_index(file)`, which scan one directory using the existing indexed-stem grammar and return the actual lowest/highest on-disk family member while preserving portable path behavior. The expression language itself gained immutable JSON-like array and object literals, including span tracking, computed values, deterministic duplicate-key rejection, and preservation of existing postfix indexing behavior. Demand-driven Git context added `ctx.branch`, `ctx.worktree`, and `ctx.merge_conflicts`, with one shared repository discovery and independent per-field degradation.

The Git prediction work established a read-only Sniff authority for merging committed branch tips in memory. `predict_conflicts(branch)` is anchored to the caller/launch repository, merges the named local branch into the current branch in the correct direction, returns sorted portable conflict paths, and refuses unsafe merge configurations. Its acceptance contract required parity with Git/git2 conflict oracles, independence from staged/unstaged/untracked or corrupt live-index state, and proof that HEAD, refs, the worktree, index, and object database remain unchanged.

The remote half added `branch_exists_on_remote`, `remote_vendor`, `pr`, `pr_list`, `cicd`, and `cicd_list`. Sniff supplies preferred-remote selection, provider/flavor discovery, exact and paginated PR/CI-job models, capability checks, normalized identities, bounded traversal, and provider-aware host-bound credentials. Darkmatter supplies authored query validation, deny-by-default exact-host policy, run-wide single-flight execution, typed error projection, safe compact Markdown links, and parity across body, frontmatter, and `$()` expression surfaces. Acceptance required no silently ignored filters, closed enum return metadata, deterministic newest-first lists with bounded counts, correct no-result versus error behavior, complete provider error classification, DMLS catalog parity without hard-coded function lists, Wiremock-only provider tests, and macOS/Linux/Windows compilation evidence. Review 27 records all 30 criteria as passed.

Packages and modules changed for this feature:

- `darkmatter` library: `markdown::compose::context::{capture,catalog,options}`; expression AST, lexer, parser, catalog, context resolution, and the `functions::{paths,git,provider,pull_requests,cicd,escape}` domain slices; shared remote runtime/fetch policy; frontmatter/body/shell/pipeline/subtree propagation; provider-network, Git-context, conflict-prediction, literal/index, and transclusion tests.
- `darkmatter-cli`: compose integration/error handling and focused test fixtures. No independent Git or provider implementation was added.
- `dmls`: expression overlay/provider completion and hover, passive no-side-effects coverage, query-vocabulary document links, and related diagnostics/graph integration.
- `sniff` and `sniff-cli`: Git open/status/worktree APIs; the hermetic merge-conflict module; configured-remote resolution and live observation; focused provider clients, URL/web-link normalization, credentials, canonical PR/CI types and filters, pagination/capability enforcement, tests, snapshots, docs, and feature wiring.
- `claudine` and `claudine-cli`: traversal/validation/rendering of the new container expressions and semantic schema types used by composed lifecycle inputs.
- Root CI workflows, review schemas, skills, and feature/review documentation. No `biscuit-terminal` file was changed on this branch.

### `2026-07-13-meta-schema`

This feature introduced two nominal, grammar-backed SimplifiedSchema types. `type-definition` describes one complete `PropertyDef`: a scalar type expression, nested mapping definition, or non-empty union. `schema` describes a complete `$schema` declaration: an inline shape, valid local file reference, or non-empty root union. The purpose was to replace vague `string`, `object`, or `any` annotations with the actual language-level meaning, while reusing the existing schema AST and parser rather than building a second meta-schema grammar. The Darkmatter base schema consequently changed `$schema` from `any` to `schema`.

Acceptance required both keywords to parse, serialize, appear in descriptors/`md schema about`, compile to the portable `string | object | array` carrier domain plus Darkmatter custom keywords, and validate through shared passive parsers without I/O or coercing the authored representation. Ordinary array postfix forms had to work, and the same recursion limit had to protect scalar and YAML-native definitions. Existing schemas needed byte-identical behavior except for the intended nominal hover/type correction and earlier rejection of malformed declarations.

The second half supplied source-aware parsing as a structural sidecar rather than a duplicate spanned AST. Within the frozen v1 presentation grammar, it preserves exact spans for keys, definitions, atoms, constraints, imports, references, and union arms across quoted/plain scalars, UTF-8, CRLF, mappings, sequences, aliases, and supported explicit mapping pairs. DMLS uses these passive products for content-based standalone activation, last-good recovery during malformed edits, nominal/union-aware hover, parser-state completion, and precise diagnostics; it performs no reference loading, composition, expression/shell execution, or network access. Later review cycles repaired structural frontmatter paths, scalar-reference completion, reference cycles/depth errors, pattern-key projection, quote-state handling, and explicit/compact mapping-pair source projection. Review 14 records all 13 criteria closed and freezes further YAML presentation expansion behind a future specification.

Packages and modules changed for this feature:

- `darkmatter` library: `markdown::schemas::simplified::{types,grammar,cursor,source,yaml_scalar,standalone,convert}`; schema `about`, `format`, `validate`, `coerce`, `resolve`, `reference`, errors, and trigger matching; base-schema artifacts and dedicated meta-schema/parser/source-projection/reference tests.
- `darkmatter-cli`: `schema about` descriptor presentation, compose/schema integration, and focused L1/L2 tests.
- `dmls`: frontmatter/schema overlays, structural document links, diagnostics, provider completion/hover arbitration, graph substrate integration, last-good state, no-side-effects checks, and in-memory LSP-session coverage.
- `claudine` and `claudine-cli`: classification, validation, lifecycle traversal, and structural rendering for semantic schema values.
- Authored Darkmatter schema/topic documentation, the required `darkmatter` skill update, feature plans/test maps/reviews, and root feature/suggestion review schemas.

## Timeline

The timeline includes every commit from the fork point `d672388dd0fed4196295e7f21514cac6fa59f0ae` through `0584d8297f57f5eb30b52d03b1241ba55184bb44`, in chronological order. The fork-point commit itself is included as requested.

- `d672388dd0` (2026-07-17) — docs(darkmatter): record performance-followup review 8 implementation cycle
  - This is the stated fork point shared by both branches. It records the last pre-split performance-followup review cycle.
- `c6e682e15d` (2026-07-18) — feat(darkmatter): add schema meta-types and passive DMLS support
  - The foundational Meta-Schema landing added both semantic types, passive/source-aware parse products, JSON Schema custom keywords, CLI descriptors, DMLS integration, and the initial phase test suites.
- `8622fe5b4f` (2026-07-18) — feat(darkmatter): add Git context and conflict prediction
  - The foundational local-Git landing added demand-driven Git context plus caller-anchored, side-effect-free committed-tip conflict prediction.
- `836bd78194` (2026-07-18) — docs(darkmatter): record meta-schema and more-is-more review artifacts
- `49827efdcf` (2026-07-18) — feat(darkmatter): add meta-schema Phase 7 semantic-region LSP intelligence
  - Completed the planned Meta-Schema editor surface with semantic-region diagnostics, descriptor-driven completion, union-aware hover, standalone support, and passive LSP tests.
- `9815050aa3` (2026-07-18) — docs(darkmatter): promote meta-schema plan to phase 7 and refine test map
- `d8c801e3a0` (2026-07-18) — feat(dmls): specialize Phase 7 authoring diagnostics and completion
- `f5bba5c813` (2026-07-18) — test(sniff): isolate detect_area negative test with tempfile::tempdir
- `b86928270f` (2026-07-18) — feat(sniff): add bare-repo gix layer and focused remote queries
  - Established Sniff as the shared authority for bare-repository handling, hermetic conflict prediction, preferred remotes, live branch/vendor observation, and bounded provider-neutral PR/CI queries.
- `946c7a1315` (2026-07-18) — chore: refresh GitNexus symbol/relationship counts in CLAUDE.md
- `f13ecc38c7` (2026-07-18) — test(darkmatter): move pixel-readback image test from L2 to macOS-gated L3
- `0a987d62e7` (2026-07-18) — test(sniff-cli): normalize kernel/version in os_json snapshot
- `3ee83c41ea` (2026-07-18) — feat(claudine): render semantic schema types as structural contexts
- `171ac66792` (2026-07-18) — docs(darkmatter): record meta-schema r2 and more-is-more r16/r17 cycles
- `abee8a9225` (2026-07-18) — feat(darkmatter): complete more-is-more AC17-30 expression and remote
  - Completed More-Is-More AC17–30: structured literals, indexed-file endpoints, enum-return catalogs, provider functions, and one deny-by-default run-wide remote runtime across all expression surfaces.
- `3856627114` (2026-07-18) — feat(darkmatter): unify schema authoring parser authority
  - Unified schema authoring around tolerant cursor/source projection and canonical references, removing DMLS text heuristics in favor of shared parser state.
- `a9ab71d8f2` (2026-07-19) — chore: refresh GitNexus symbol/relationship counts in CLAUDE.md
- `6fff5a279c` (2026-07-19) — docs(darkmatter): correct review-7 reference and delete review-8
- `5432fc2d58` (2026-07-19) — docs(darkmatter): record more-is-more review 17 implementation cycle
- `94f6812d93` (2026-07-19) — test(sniff): exercise advertised remote provider filters
- `83d5d7ee34` (2026-07-19) — fix(sniff): align preferred remote selection
- `7a971210e9` (2026-07-19) — fix(claudine): traverse container expressions
- `7ba8a7c022` (2026-07-19) — test(dmls): assert query-list vocabulary link on hover and completion
- `208f534785` (2026-07-19) — chore: refresh GitNexus index counts
- `6bc188b981` (2026-07-19) — docs(darkmatter): publish pr_list and cicd_list query vocabulary
- `b17af5707a` (2026-07-19) — feat(darkmatter): harden provider expression pipeline
  - Hardened the provider boundary with one CommonMark-safe escape path, pre-network validation, shared-executor execution, preserved focused errors, memoization checks, and hostile-text Wiremock fixtures.
- `a22641969d` (2026-07-19) — feat(sniff): exhaust bounded PR/job domains before exact filtering
  - Made PR/job filtering complete within explicit bounds, centralized canonical query validation, normalized provider-specific records, and changed bound exhaustion into a typed error rather than a partial answer.
- `c200bc7052` (2026-07-19) — ci: add sniff remote coverage and claudine cross-platform checks
- `96a1343c7b` (2026-07-19) — docs(darkmatter): record more-is-more review 18 cycle
- `58b2e0a167` (2026-07-19) — docs: codify hook-bypass prohibition in commit memory
- `273d1aeaff` (2026-07-19) — chore: refresh GitNexus symbol/relationship counts in CLAUDE.md
- `15f6abf2d8` (2026-07-19) — docs(darkmatter): record more-is-more review 19 cycle
- `7bbd89a491` (2026-07-19) — fix(darkmatter): classify provider failures and make them authoring-fatal
  - Introduced typed provider-failure classes and made focused remote failures authoring-fatal on frontmatter, body, and shell-expression surfaces while retaining lenient handling for unrelated expression errors.
- `e6e153a103` (2026-07-19) — feat: add review findings workflow prompts
- `979c9cd80d` (2026-07-19) — fix(dmls): rewrite authored vocabulary links to resolvable file URIs
- `96930e01a2` (2026-07-19) — feat(darkmatter): bound provider query authoring and serialize links
  - Closed query-authoring and output-injection gaps with deny-unknown-fields DTOs, contradictory-filter rejection, discovered provider flavors, and safely serialized same-origin Markdown links.
- `225c52bb33` (2026-07-19) — chore: refresh GitNexus repository metadata
- `7cca47e445` (2026-07-19) — docs(darkmatter): record more-is-more review 20 cycle
- `030af8f595` (2026-07-19) — feat(sniff): harden focused remote provider resolution
  - Hardened neutral-host/self-managed provider discovery, flavor-specific canonical URL parsing, capability/order enforcement, and same-origin output normalization.
- `039cb71485` (2026-07-19) — docs(darkmatter): record more-is-more review 21 cycle
- `9af486de8c` (2026-07-19) — docs(darkmatter): record meta-schema review 3 and ac13 exception
- `9447cee0b1` (2026-07-19) — feat(dmls): preserve structural frontmatter paths
  - Moved DMLS frontmatter lookup to RFC 6901 structural paths so quoted keys, CRLF documents, last-good models, and precise diagnostic ranges survive editing.
- `e78ab4887b` (2026-07-19) — feat(darkmatter): align standalone schema parsing
  - Aligned standalone and inline schema declarations on the shared parser and reference rules, including rejection of invalid reference arms and named imports from reference documents.
- `ce3c405a1d` (2026-07-20) — docs(darkmatter): record meta-schema review 4 implementation cycle
- `3bc5a53318` (2026-07-20) — fix(dmls): repair frontmatter container lookup and meta-schema matching
- `cff40627cf` (2026-07-20) — fix(darkmatter): bound $schema reference delegation and surface cycles
  - Added bounded, canonical-path schema-reference delegation so self/mutual/union cycles become structured errors and transitive dependency tracking stays deterministic.
- `93ec9f67d1` (2026-07-20) — fix(dmls): activate scalar $schema reference completion
- `4e7103ec7e` (2026-07-20) — docs(darkmatter): record meta-schema review 5 cycle
- `edb03f5fb0` (2026-07-20) — fix(darkmatter): separate $schema depth exhaustion from cycle errors
- `5c299e3b90` (2026-07-20) — test(darkmatter-cli): route md compose through cargo build shim
- `841d21345a` (2026-07-20) — docs(darkmatter): record meta-schema review 6 implementation cycle
- `e0b071f63c` (2026-07-20) — fix(dmls): project pattern keys and share hover/owners
- `b49b6176c4` (2026-07-20) — docs(darkmatter): record cross-platform verification decision
- `62b5cc558c` (2026-07-20) — docs(darkmatter): record meta-schema review 7 cycle
- `1195933e45` (2026-07-20) — docs(darkmatter): record meta-schema review 8 cycle
- `ee9d52d531` (2026-07-20) — fix(dmls): repair flow completion, hover arbitration, and escape handling
  - Repaired several editor-visible Meta-Schema edge cases together: flow completion, hover arbitration, and escaping behavior.
- `44bae9cda7` (2026-07-20) — docs(darkmatter): clarify semantic-arrays exemption in schema-definition
- `c15b980ee0` (2026-07-20) — docs(darkmatter): record meta-schema review 9 cycle
- `6c4e4a6faf` (2026-07-20) — test(darkmatter): classify repo-root schemas under the tagged-envelope contract
- `88e386b085` (2026-07-20) — fix(dmls): bind nested plain-scalar quote from leaking into top-level quote state
- `6e48c1ebd9` (2026-07-20) — test(darkmatter-cli): switch level2 code-block styling tests to focused tmux harness
- `c1bab69ce6` (2026-07-20) — docs(darkmatter): record meta-schema review 10 cycle
- `a5272f3052` (2026-07-20) — fix(dmls): bind flow scalar-boundary rule to opening of quoted scalars
- `b9b8684c2a` (2026-07-20) — fix(dmls): keep mid-token hyphen inert when followed by whitespace
- `ee3d8eaa5c` (2026-07-20) — docs(darkmatter): record meta-schema review 11 cycle
- `6ffb071355` (2026-07-20) — docs(darkmatter): record meta-schema review 12 and 13 cycles
- `f2a08103fd` (2026-07-20) — fix(darkmatter): recognize explicit mapping pairs in schema locator
  - Extended source projection and DMLS activation to standard explicit YAML mapping pairs while keeping semantic completion routed through shared regions.
- `300dc2275d` (2026-07-20) — test(dmls): cover compact explicit mapping pair in union sequence
- `ef7d537e7b` (2026-07-20) — chore(schemas): drop redundant kind: schema from review schemas
- `4027801460` (2026-07-20) — docs(darkmatter): record meta-schema review 14 cycle
- `9d2f556a88` (2026-07-20) — chore: refresh GitNexus symbol/relationship counts in CLAUDE.md
- `ff88aa0f12` (2026-07-20) — fix(darkmatter): close compact explicit pair gap and freeze v1 grammar
  - Closed the final compact explicit-pair gap and explicitly froze the v1 source-aware YAML presentation boundary.
- `df62ba9a04` (2026-07-20) — docs(darkmatter): document v1 source-aware presentation boundary
- `f9607c918b` (2026-07-21) — feat(darkmatter): classify version errors as unsupported capabilities
- `a1117cc58f` (2026-07-21) — docs(darkmatter): record Review 21 implementation cycle
- `4a04718c1c` (2026-07-21) — test(darkmatter): use rustc for cross-platform shell expansion fixtures
- `62c27747f7` (2026-07-21) — fix(darkmatter-cli): drop unused origin binding in approval error match
- `0158fd4da1` (2026-07-21) — docs(darkmatter): record Review 22 implementation cycle
- `4640e0216c` (2026-07-21) — docs(darkmatter): record Review 23 implementation cycle
- `c89429aac0` (2026-07-21) — test(darkmatter): widen provider-network job allowlist for six-flavor discovery
- `a110b70389` (2026-07-21) — docs(darkmatter): record Review 24 production findings
- `9632eae1ef` (2026-07-21) — fix(sniff): harden remote discovery and Git observations
  - A large late hardening pass added capability discovery and credential handling, preserved safe URL/SSH identities, and distinguished stale from corrupt worktrees.
- `06f6950080` (2026-07-21) — chore: refresh GitNexus symbol and relationship counts
- `59c13429b5` (2026-07-21) — chore: refresh GitNexus symbol and relationship counts
- `b954487267` (2026-07-21) — docs(sniff): document ambiguous-host credential scope and version-aware capability discovery
- `10a45d7dd8` (2026-07-21) — docs(darkmatter): record Review 24 implementation cycle
- `a7311539de` (2026-07-21) — fix(sniff): scope discovery credentials and align Azure identity
- `fdc5963fe9` (2026-07-21) — test(sniff): isolate credentials and pin Azure DevOps discovery
- `4b269ac484` (2026-07-21) — docs(darkmatter): record Review 25 cycle
- `95f44000a5` (2026-07-21) — chore: refresh GitNexus symbol and relationship counts
- `6958203675` (2026-07-21) — chore: refresh GitNexus symbol and relationship counts
- `8318daea12` (2026-07-21) — docs(darkmatter): record Review 25 implementation cycle
- `44d6055788` (2026-07-21) — fix(sniff): retry unsigned-challenge discovery with one host-bound credential
- `5614b237fc` (2026-07-21) — docs(darkmatter): record Review 26 cycle
- `936ab855e4` (2026-07-21) — chore: refresh GitNexus symbol and relationship counts
- `83e3981ef0` (2026-07-21) — fix(sniff): use provider-aware host-bound authentication for discovery
  - Centralized provider-aware authentication for both discovery and focused queries, including correct Gitea/Forgejo, GitLab, Azure, GitHub Enterprise, and Bitbucket schemes with credential-isolation tests.
- `6019f47578` (2026-07-21) — docs(darkmatter): record Review 26 implementation cycle
- `188a68a27f` (2026-07-21) — chore: refresh GitNexus symbol and relationship counts
- `9272bdf434` (2026-07-21) — chore: refresh GitNexus symbol and relationship counts
- `c71181066f` (2026-07-21) — docs(commits): document --only + -F - argument order for path-limited commits
- `0584d8297f` (2026-07-21) — docs(darkmatter): record Review 27 cycle
  - The branch head records Review 27, whose final verdict is production-ready with no remaining findings.

## File Blast Radius

This is the union of every path mutated by the 101 commits in `d672388dd0fed4196295e7f21514cac6fa59f0ae^..0584d8297f57f5eb30b52d03b1241ba55184bb44`, collected from per-commit history with rename detection disabled so files later reverted or removed are not hidden by the final net diff. It contains 203 paths.

### Repository metadata, workflows, prompts, and shared schemas

- `.claude/skills/darkmatter/SKILL.md`
- `.claude/skills/rust-devops/SKILL.md`
- `.claude/skills/rust-devops/gitoxide.md`
- `.claude/skills/sniff/SKILL.md`
- `.claudine/memory/commits.md`
- `.github/workflows/claudine-tests.yml`
- `.github/workflows/test.yml`
- `CLAUDE.md`
- `docs/testing-strategy.md`
- `prompts/_implement/implement-review-findings-plan.md`
- `prompts/_implement/review-findings-plan.md`
- `schemas/feature-review.yaml`
- `schemas/suggestion-review.yaml`

### `claudine`

- `claudine/cli/src/commands/context/format.rs`
- `claudine/lib/src/composition/lifecycle/executor.rs`
- `claudine/lib/src/composition/lifecycle/executor/tests.rs`
- `claudine/lib/src/composition/lifecycle/tests.rs`
- `claudine/lib/src/composition/lifecycle/validate.rs`
- `claudine/lib/src/composition/looping/config.rs`
- `claudine/lib/src/composition/preflight.rs`
- `claudine/lib/src/composition/schema/classify.rs`
- `claudine/lib/src/dispatch/matcher.rs`

### `darkmatter` package area

- `darkmatter/cli/src/commands/compose.rs`
- `darkmatter/cli/src/commands/schema/about.rs`
- `darkmatter/cli/tests/level2_code_block_styling.rs`
- `darkmatter/cli/tests/level2_errors.rs`
- `darkmatter/cli/tests/schema_about.rs`
- `darkmatter/dmls/docs/diagnostics.md`
- `darkmatter/dmls/src/diagnostics/codes.rs`
- `darkmatter/dmls/src/diagnostics/frontmatter.rs`
- `darkmatter/dmls/src/graph/substrate.rs`
- `darkmatter/dmls/src/overlay/doc_links.rs`
- `darkmatter/dmls/src/overlay/expressions.rs`
- `darkmatter/dmls/src/overlay/frontmatter.rs`
- `darkmatter/dmls/src/overlay/mod.rs`
- `darkmatter/dmls/src/overlay/schema.rs`
- `darkmatter/dmls/src/providers/dsl.rs`
- `darkmatter/dmls/src/providers/frontmatter.rs`
- `darkmatter/dmls/src/providers/mod.rs`
- `darkmatter/dmls/tests/lsp_session.rs`
- `darkmatter/dmls/tests/no_side_effects.rs`
- `darkmatter/docs/schemas/darkmatter.yaml`
- `darkmatter/docs/schemas/expression-functions.yaml`
- `darkmatter/docs/topics/context-variables.md`
- `darkmatter/docs/topics/darkmatter-expressions.md`
- `darkmatter/docs/topics/schema-definition.md`
- `darkmatter/features/2026-07-13-meta-schema/log.md`
- `darkmatter/features/2026-07-13-meta-schema/phase1-baseline-compiled-json-schema.txt`
- `darkmatter/features/2026-07-13-meta-schema/phase1-baseline-dmls-hover.txt`
- `darkmatter/features/2026-07-13-meta-schema/phase1-baseline-schema-about.txt`
- `darkmatter/features/2026-07-13-meta-schema/phase1-baseline-validation.txt`
- `darkmatter/features/2026-07-13-meta-schema/phase1-impact.md`
- `darkmatter/features/2026-07-13-meta-schema/phase1-test-matrix.md`
- `darkmatter/features/2026-07-13-meta-schema/phase2-test-map.md`
- `darkmatter/features/2026-07-13-meta-schema/phase3-test-map.md`
- `darkmatter/features/2026-07-13-meta-schema/phase4-test-map.md`
- `darkmatter/features/2026-07-13-meta-schema/phase5-baseline-replay.md`
- `darkmatter/features/2026-07-13-meta-schema/phase5-test-map.md`
- `darkmatter/features/2026-07-13-meta-schema/phase6-test-map.md`
- `darkmatter/features/2026-07-13-meta-schema/phase7-test-map.md`
- `darkmatter/features/2026-07-13-meta-schema/plan.md`
- `darkmatter/features/2026-07-13-meta-schema/review-10.md`
- `darkmatter/features/2026-07-13-meta-schema/review-11.md`
- `darkmatter/features/2026-07-13-meta-schema/review-12.md`
- `darkmatter/features/2026-07-13-meta-schema/review-13.md`
- `darkmatter/features/2026-07-13-meta-schema/review-14.md`
- `darkmatter/features/2026-07-13-meta-schema/review-2.md`
- `darkmatter/features/2026-07-13-meta-schema/review-3.md`
- `darkmatter/features/2026-07-13-meta-schema/review-4.md`
- `darkmatter/features/2026-07-13-meta-schema/review-5.md`
- `darkmatter/features/2026-07-13-meta-schema/review-6.md`
- `darkmatter/features/2026-07-13-meta-schema/review-7.md`
- `darkmatter/features/2026-07-13-meta-schema/review-8.md`
- `darkmatter/features/2026-07-13-meta-schema/review-9.md`
- `darkmatter/features/2026-07-13-meta-schema/spec.md`
- `darkmatter/features/2026-07-13-more-is-more/log.md`
- `darkmatter/features/2026-07-13-more-is-more/plan.md`
- `darkmatter/features/2026-07-13-more-is-more/review-15.md`
- `darkmatter/features/2026-07-13-more-is-more/review-16.md`
- `darkmatter/features/2026-07-13-more-is-more/review-17.md`
- `darkmatter/features/2026-07-13-more-is-more/review-18.md`
- `darkmatter/features/2026-07-13-more-is-more/review-19.md`
- `darkmatter/features/2026-07-13-more-is-more/review-20.md`
- `darkmatter/features/2026-07-13-more-is-more/review-21.md`
- `darkmatter/features/2026-07-13-more-is-more/review-22.md`
- `darkmatter/features/2026-07-13-more-is-more/review-23.md`
- `darkmatter/features/2026-07-13-more-is-more/review-24.md`
- `darkmatter/features/2026-07-13-more-is-more/review-25.md`
- `darkmatter/features/2026-07-13-more-is-more/review-26.md`
- `darkmatter/features/2026-07-13-more-is-more/review-27.md`
- `darkmatter/features/2026-07-13-more-is-more/review-plan-19.md`
- `darkmatter/features/2026-07-13-more-is-more/spec.md`
- `darkmatter/features/2026-07-15-performance-followup/log.md`
- `darkmatter/features/2026-07-15-performance-followup/performance-compliance.md`
- `darkmatter/features/2026-07-15-performance-followup/review-7.md`
- `darkmatter/features/2026-07-15-performance-followup/review-8.md`
- `darkmatter/lib/Cargo.toml`
- `darkmatter/lib/src/markdown/compose/context/capture/git.rs`
- `darkmatter/lib/src/markdown/compose/context/capture/groups.rs`
- `darkmatter/lib/src/markdown/compose/context/capture/mod.rs`
- `darkmatter/lib/src/markdown/compose/context/capture/snapshot.rs`
- `darkmatter/lib/src/markdown/compose/context/catalog.rs`
- `darkmatter/lib/src/markdown/compose/context/options.rs`
- `darkmatter/lib/src/markdown/compose/expression/ast.rs`
- `darkmatter/lib/src/markdown/compose/expression/catalog/ast.rs`
- `darkmatter/lib/src/markdown/compose/expression/catalog/mod.rs`
- `darkmatter/lib/src/markdown/compose/expression/catalog/parser.rs`
- `darkmatter/lib/src/markdown/compose/expression/error.rs`
- `darkmatter/lib/src/markdown/compose/expression/functions/cicd.rs`
- `darkmatter/lib/src/markdown/compose/expression/functions/escape.rs`
- `darkmatter/lib/src/markdown/compose/expression/functions/git.rs`
- `darkmatter/lib/src/markdown/compose/expression/functions/mod.rs`
- `darkmatter/lib/src/markdown/compose/expression/functions/paths.rs`
- `darkmatter/lib/src/markdown/compose/expression/functions/provider.rs`
- `darkmatter/lib/src/markdown/compose/expression/functions/pull_requests.rs`
- `darkmatter/lib/src/markdown/compose/expression/lexer.rs`
- `darkmatter/lib/src/markdown/compose/expression/mod.rs`
- `darkmatter/lib/src/markdown/compose/expression/parser.rs`
- `darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs`
- `darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs`
- `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs`
- `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/tests/tests.rs`
- `darkmatter/lib/src/markdown/compose/interpolation/fatality_characterization.rs`
- `darkmatter/lib/src/markdown/compose/pipeline/mod.rs`
- `darkmatter/lib/src/markdown/compose/remote.rs`
- `darkmatter/lib/src/markdown/compose/remote_fetch.rs`
- `darkmatter/lib/src/markdown/compose/subtree.rs`
- `darkmatter/lib/src/markdown/compose/tests/mod.rs`
- `darkmatter/lib/src/markdown/compose/tests/provider_network.rs`
- `darkmatter/lib/src/markdown/compose/tests/transclusion.rs`
- `darkmatter/lib/src/markdown/errors/blocks.rs`
- `darkmatter/lib/src/markdown/schemas/about.rs`
- `darkmatter/lib/src/markdown/schemas/coerce.rs`
- `darkmatter/lib/src/markdown/schemas/errors.rs`
- `darkmatter/lib/src/markdown/schemas/format.rs`
- `darkmatter/lib/src/markdown/schemas/mod.rs`
- `darkmatter/lib/src/markdown/schemas/reference.rs`
- `darkmatter/lib/src/markdown/schemas/resolve.rs`
- `darkmatter/lib/src/markdown/schemas/simplified/convert.rs`
- `darkmatter/lib/src/markdown/schemas/simplified/cursor.rs`
- `darkmatter/lib/src/markdown/schemas/simplified/grammar.rs`
- `darkmatter/lib/src/markdown/schemas/simplified/mod.rs`
- `darkmatter/lib/src/markdown/schemas/simplified/source.rs`
- `darkmatter/lib/src/markdown/schemas/simplified/standalone.rs`
- `darkmatter/lib/src/markdown/schemas/simplified/types.rs`
- `darkmatter/lib/src/markdown/schemas/simplified/yaml_scalar.rs`
- `darkmatter/lib/src/markdown/schemas/triggers/matcher.rs`
- `darkmatter/lib/src/markdown/schemas/validate.rs`
- `darkmatter/lib/tests/git_context_integration.rs`
- `darkmatter/lib/tests/level2_render_tree_terminal/images.rs`
- `darkmatter/lib/tests/level2_render_tree_terminal/support/mod.rs`
- `darkmatter/lib/tests/level3_image_painting.rs`
- `darkmatter/lib/tests/meta_schema_phase1.rs`
- `darkmatter/lib/tests/meta_schema_phase3.rs`
- `darkmatter/lib/tests/meta_schema_phase4.rs`
- `darkmatter/lib/tests/meta_schema_phase5.rs`
- `darkmatter/lib/tests/meta_schema_phase6.rs`
- `darkmatter/lib/tests/meta_schema_reference_graph.rs`
- `darkmatter/lib/tests/meta_schema_repo_schemas.rs`
- `darkmatter/lib/tests/more_is_more_literals_and_indexes.rs`
- `darkmatter/lib/tests/predict_conflicts.rs`
- `darkmatter/lib/tests/schemas_grammar_proptest.rs`
- `darkmatter/lib/tests/schemas_source_projection.rs`
- `darkmatter/lib/tests/suggest_constraint_phase4.rs`

### `sniff` package area

- `sniff/cli/tests/snapshots.rs`
- `sniff/cli/tests/snapshots/snapshots__os_json_summary.snap`
- `sniff/docs/sniff-library-architecture.md`
- `sniff/justfile`
- `sniff/lib/Cargo.toml`
- `sniff/lib/README.md`
- `sniff/lib/src/credentials.rs`
- `sniff/lib/src/error.rs`
- `sniff/lib/src/filesystem/blast_radius.rs`
- `sniff/lib/src/filesystem/formatting.rs`
- `sniff/lib/src/filesystem/git/api.rs`
- `sniff/lib/src/filesystem/git/merge_conflicts.rs`
- `sniff/lib/src/filesystem/git/mod.rs`
- `sniff/lib/src/filesystem/git/open.rs`
- `sniff/lib/src/filesystem/git/recent_commits.rs`
- `sniff/lib/src/filesystem/git/remote_observation.rs`
- `sniff/lib/src/filesystem/git/remote_refresh.rs`
- `sniff/lib/src/filesystem/git/remote_resolver.rs`
- `sniff/lib/src/filesystem/git/status.rs`
- `sniff/lib/src/filesystem/git/types.rs`
- `sniff/lib/src/filesystem/git/worktree.rs`
- `sniff/lib/src/filesystem/mod.rs`
- `sniff/lib/src/filesystem/repo/area.rs`
- `sniff/lib/src/filesystem/repo/identity.rs`
- `sniff/lib/src/lib.rs`
- `sniff/lib/src/network/mod.rs`
- `sniff/lib/src/remote/focused.rs`
- `sniff/lib/src/remote/mod.rs`
- `sniff/lib/src/remote/provider.rs`
- `sniff/lib/src/remote/provider_url.rs`
- `sniff/lib/src/remote/types.rs`
- `sniff/lib/src/remote/url_parser.rs`
- `sniff/lib/src/remote/web_link.rs`
- `sniff/lib/tests/focused_provider.rs`
- `sniff/lib/tests/git_parity.rs`
- `sniff/lib/tests/integration.rs`
- `sniff/lib/tests/merge_conflict_prediction.rs`
- `sniff/lib/tests/remote_observation.rs`
- `sniff/lib/tests/remote_resolution.rs`

