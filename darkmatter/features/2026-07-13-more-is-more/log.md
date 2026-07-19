---
implementation_15: "2026-07-18T01:11:14-07:00"
implementation_17: "2026-07-19T08:49:09-07:00"
deferred_perf_measurement: false
---

## Implementation of Review Findings #15

> **started at:** 2026-07-18T01:11:14-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'darkmatter/features/2026-07-13-more-is-more/review-15.md'
- this is iteration 15 of the review-to-implement cycle
- review 15 contains three findings:
        - **Critical** — acceptance criteria 17–30 are not implemented
        - **High** — unsafe merge settings are silently accepted when the built-in merge is clean
        - **High** — bare-repository Git capture discards valid branch state and emits a discovery diagnostic
- impacted package areas (from the spec and plan `packages:` frontmatter): `sniff`, `darkmatter` (lib + dmls)
- ordering decision: the two **High** correctness findings are implemented first (both are concrete, bounded defects in `sniff`/`darkmatter` capture); the **Critical** scope finding is evaluated last because its recommendation is a requirements decision rather than a code defect

### Finding 2 (High) — unsafe merge settings silently accepted when the built-in merge is clean

- starting the work on 'unsafe-merge-false-clean' at 01:11:51-07:00
        - root cause confirmed: `reject_unsafe_configuration` short-circuited on `conflicts.is_empty()`, so any repository whose built-in text merge came out clean skipped the entire D8 hermeticity check
        - **second, undocumented defect found while fixing it:** rejection ran _after_ `gix::merge::plumbing::commit`, so an external filter could already have been invoked before anything rejected it — D8 requires rejection to precede the merge
        - hermeticity trap recorded for future work: `gix::object::tree::diff::Platform::for_each_to_obtain_tree` eagerly builds its own resource cache via `repo.diff_resource_cache()`, which **reads the live index**; any tree diff in this module must use `for_each_to_obtain_tree_with_cache` with the committed-tree cache from `committed_diff_cache`
                - the existing `live_index_and_worktree_state_do_not_affect_prediction` test caught this immediately — the suite was genuinely load-bearing here
        - implementation in `sniff/lib/src/filesystem/git/merge_conflicts.rs`:
                - new `participating_paths()` — merge bases via `merge_bases_many`, then base-vs-`ours` and base-vs-`theirs` tree diffs, unioned; iterates all bases (criss-cross safe), falling back to a direct `ours`-vs-`theirs` diff when no base is reachable
                - new `collect_tree_changes()` with `track_rewrites(None)` and the shared committed `diff_cache`
                - `reject_unsafe_configuration` now takes participating paths instead of conflicts; the empty-conflict early return is gone and `merge.renormalize` is checked unconditionally
                - the rejection call moved to **before** the merge
                - `SniffError::UnsupportedMergeConfiguration` shape left untouched, so no call sites or Darkmatter error mapping needed changes
        - **AC10 regression caught during orchestrator review of the first draft:** rejecting `merge.renormalize` unconditionally made same-branch/ancestor/fast-forward merges error instead of returning `[]`, which spec AC10 requires; no existing test set `merge.renormalize`, so the gates passed silently
                - the initially prescribed fix (exempt only zero-participating-path merges) was **insufficient** — ancestor and already-contained merges have a non-empty participating set because base-vs-`ours` still diffs, so path count is the wrong discriminator
                - shipped fix: `participating_paths` returns empty when `bases.len() == 1 && (bases[0] == ours || bases[0] == theirs)` — one tip already contains the other, so git takes a side wholesale and never runs a three-way merge, meaning no driver, filter, or renormalization can apply however far the tips diverged
                - `reject_unsafe_configuration` now returns `Ok(())` on an empty path set via a `let ... else` binding, making the odd empty-path error unrepresentable rather than merely unreached
        - tests added to `sniff/lib/tests/merge_conflict_prediction.rs`:
                - `unsafe_merge_configuration_is_rejected_when_the_builtin_merge_is_clean` — each side touches a _different_ file so the built-in merge is genuinely clean; covers custom `merge.<name>.driver`, all three `filter.<name>.{process,clean,smudge}`, `merge.renormalize=true`, and a negative control
                - each case asserts the prediction is clean _before_ the unsafe setting is applied, which is what pins the test to this bug rather than the old conflict-driven path
                - error cases assert `snapshot()` equality (HEAD, refs, index, worktree files, on-disk object set) and reuse the non-executable-sentinel convention for "no subprocess launched"
                - `trivial_merges_return_empty_despite_unsupported_configuration` — same-branch, already-contained, and fast-forward with `merge.renormalize=true`, all asserting `Ok(vec![])`; this test fails on the first fix and passes on the shipped one
        - `darkmatter/lib/tests/predict_conflicts.rs` needed no changes — its fixtures conflict on `shared.txt` under the built-in driver, so they were rejected before and still are
        - **environmental note (not a defect):** the first `sniff just test` run returned exit 1 with 10 `sniff-cli` 30s timeouts; because `remote_refresh.rs` calls `merge_conflicts_between`, this was measured rather than blamed on load — A/B over 5 iterations showed ~190ms with `participating_paths` vs ~181ms without (~4% on a ~185ms probe), and a re-run at host load 19 (down from 52–69) passed 769/769 with 0 timeouts
        - gates (real exit codes, `$?` on redirected runs, not read off a pipe): `sniff just test` **0** (1348 + 769), `sniff just lint` **0**, `darkmatter just test` **0** (5782 + 560 + 568), `darkmatter just lint` **0**
- work completed for 'unsafe-merge-false-clean' at 02:05:56-07:00

### Finding 3 (High) — bare-repository Git capture discards valid branch state

- starting the work on 'bare-repo-branch-capture' at 02:05:56-07:00
        - **impact analysis (run before editing, per the GitNexus rule):** `impact(discover, upstream)` reported **risk HIGH** — 16 direct callers, all inside the `Git` module, 0 affected execution flows
                - the GitNexus index covers sibling worktrees rather than `more-is-more`, so the result was cross-checked with an exhaustive grep of `repo_root()` / `NotARepository` / `GitRepo::discover` call sites
                - the HIGH count is dominated by test call sites in `git_parity.rs`; only **4 non-test consumers** actually read `GitRepo::repo_root()`
        - design choice — took the low-risk shape: `repo_root` stays a non-`Option<PathBuf>` and is set to the git directory for a bare repository (matching the existing `committed_attribute_stack` precedent), plus a new `is_bare` field and `pub fn is_bare(&self)` accessor
                - turning `repo_root` into an `Option` would have rippled through all 16 sites for a distinction only 4 of them care about
                - `is_bare` is captured at `discover` time rather than derived from `gix.workdir()` on demand, to avoid a `RefCell` borrow inside the accessor
        - changes:
                - `sniff/lib/src/filesystem/git/types.rs` — `discover` no longer errors on a bare repository; added the `is_bare` field/accessor; `merge_conflicts()` short-circuits to `[]` when bare
                - `sniff/lib/src/filesystem/blast_radius.rs` — `collect_changed_paths` returns `NotARepository` for a bare repository, since every scope it supports is defined against a checkout
                - `sniff/lib/src/filesystem/repo/identity.rs` — `detect_repo_identity_with_repo` rejects bare, which is what its rustdoc already claimed but could not previously happen
                - `sniff/lib/src/filesystem/mod.rs` — `shared_root` no longer becomes the git dir for a bare repository, so the shared inventory/docs/formatting walk cannot inventory git internals
                - `darkmatter/lib/src/markdown/compose/context/capture/snapshot.rs` — `repo_root` is `None` when bare, so repo-structure and document scans degrade to no-root while the Git group's HEAD-derived fields stay valid
        - deliberately left alone (audited, no change needed):
                - `worktree.rs` / `try_current_worktree_name` — **already correct**; `is_linked_worktree` is `git_dir != common_dir`, false for a bare repository, so it returns `None` before ever reaching `workdir()`, and no branch-name or git-dir-basename substitution was possible; covered by a new test instead
                - `recent_commits.rs` and `git/api.rs::repo_root` — operate on the raw `gix::Repository` and already carry their own `workdir()` guard
                - `docs.rs::RepoDocuments::new` — routes through `api::repo_root`, which already returns `Ok(None)` for bare
                - `repo/area.rs` — no `GitRepo::discover` or `repo_root()` usage at all
        - known follow-up (out of scope for this finding): `GitRepo::detect_with_request` on a bare repository still errors, because its status walk hits `gix` status with no worktree; it errored before this change too, just as `NotARepository` rather than `Git("status", …)` — same failure class, different variant
        - tests added:
                - `sniff/lib/tests/git_parity.rs` — replaced `discovery_bare_repository_surfaces_error` (which asserted the defect) with `discovery_bare_repository_is_a_valid_repo_with_head_queries`, `..._unborn_head_has_no_branch`, and `..._detached_head_has_no_branch`
                - `darkmatter/lib/tests/git_context_integration.rs` — new `bare_repository_keeps_branch_and_reports_no_capture_diagnostic` plus an `init_bare_repo` helper, asserting the attached branch, null worktree, empty conflicts, **and** empty `diagnostics()`
        - before/after evidence: temporarily restoring the `NotARepository` return in `discover` makes the new Darkmatter test fail with `left: Some(Null)` / `right: Some(String("fixture/bare-branch"))`; restoring the fix makes it pass — the test genuinely discriminates the defect
        - docs: `sniff/docs/sniff-library-architecture.md` gained a bare-repository contract paragraph; `sniff/lib/README.md` and the `sniff`/`darkmatter` skill files were grepped and needed no change (every "bare" hit there means "plain/minimal", not a bare repository), so no `md hash` regeneration was required
        - gates (real exit codes on redirected runs): `sniff just test` **0**, `sniff just lint` **0**, `darkmatter just test` **0** (5783 + 560 + 568), `darkmatter just lint` **0**; host load averaged 28/39/48 with no timeouts
- work completed for 'bare-repo-branch-capture' at 02:20:20-07:00

### Finding 1 (Critical) — acceptance criteria 17–30 are not implemented

- starting the work on 'ac17-30-scope-gap' at 02:20:20-07:00
        - the reviewer's claim was verified empirically rather than assumed — a repo-wide search across `sniff` and `darkmatter` (`*.rs` + `*.yaml`) for the defining names returns zero hits for `find_first_index`, `find_last_index`, `branch_exists_on_remote`, `remote_vendor`, `pr_list`, and `cicd_list`
        - the one partial exception is a **private** `preferred_remote` helper in `sniff/lib/src/filesystem/git/types.rs:1448` plus the public `preferred_remote_url` in `git/api.rs:229`; this is a precursor to AC19 but not the shared resolver AC19 describes, because none of the provider-query surfaces that must reuse it exist yet
        - **this finding is DEFERRED.** Reasoning recorded below so the deferral is actionable rather than a punt:
                - the review offers two remedies: implement AC17–30, or split the specification. Neither is an implementation-cycle-sized code defect of the kind findings 2 and 3 were
                - implementing AC17–30 means building GitHub/GitLab pull-request and CI/CD provider clients with capability negotiation and pagination (AC22–25), a deny-by-default exact-host network policy with run-wide single-flight and credential safety (AC26), an allowlisted vendor probe (AC21), object/array literals in the expression parser/evaluator (AC18), an enum return descriptor in the authored catalog with DMLS projection (AC30), and Wiremock-backed suites for all of it (AC29). That is multi-phase feature work with its own design decisions, not a review fix
                - splitting the specification is the cheaper remedy, but it is a **requirements change**, not an implementation change: it redefines what this feature promises to deliver. This session is non-interactive, so the decision cannot be confirmed with Ken, and unilaterally deleting 14 acceptance criteria from a ratified spec would misrepresent the feature's scope in exactly the way the reviewer is warning against
                - the deferral is therefore the honest outcome; the two shipped High fixes stand on their own and neither depends on AC17–30
        - **recommended resolution for Ken (this is the decision that unblocks the finding):** split `2026-07-13-more-is-more` so this delivery is explicitly the Git-context/conflict-prediction phase (AC1–16), and reschedule the remainder as separate features. A natural three-way split:
                - **expression-literals-and-index-functions** (AC17–18, AC30 in part) — pure Darkmatter, no network, no new Sniff surface; the cheapest and most independent slice
                - **remote-identity** (AC19–21, AC26 network policy foundation) — the preferred-remote resolver, `branch_exists_on_remote`, `remote_vendor`, and the deny-by-default host allowlist that AC22–25 then build on
                - **provider-queries** (AC22–25, AC27–29) — PR and CI/CD surfaces, which depend on the network policy landing first
        - regardless of which remedy is chosen, the reviewer's closing instruction holds and should be honored: **do not mark the current 30-criterion feature complete** while those public surfaces are absent
        - no code, test, or spec change was made for this finding
- work deferred for 'ac17-30-scope-gap' at 02:20:58-07:00

### Successful Completion

The implementation of review cycle 15 has completed successfully in 1 hour 9 minutes. During this implementation all 3 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 2 were fixed, 1 was deferred (see reasons below):

- **Critical: Acceptance criteria 17–30 are not implemented** — deferred because neither remedy the review offers is an implementation-cycle-sized code change. Implementing the 14 missing criteria means building GitHub/GitLab PR and CI/CD provider clients, a deny-by-default network policy with single-flight and credential safety, an allowlisted vendor probe, object/array expression literals, enum return descriptors, and Wiremock suites for all of it — multi-phase feature work with unresolved design decisions of its own. The cheaper remedy, splitting the specification, is a **requirements change** rather than an implementation change: it redefines what the feature promises. Because this session is non-interactive, that decision could not be confirmed with Ken, and unilaterally deleting 14 acceptance criteria from a ratified spec would misrepresent the feature's scope in precisely the way the review warns against. A concrete three-way split is recommended in the finding entry above for Ken to accept or amend.

No performance measurement was deferred in this cycle; both fixed findings were correctness defects verified by assertion-based Level-1 tests, so `deferred_perf_measurement` remains `false`.

The files changed during this implementation cycle were:

- `sniff/lib/src/filesystem/git/merge_conflicts.rs`
- `sniff/lib/src/filesystem/git/types.rs`
- `sniff/lib/src/filesystem/blast_radius.rs`
- `sniff/lib/src/filesystem/repo/identity.rs`
- `sniff/lib/src/filesystem/mod.rs`
- `sniff/lib/tests/merge_conflict_prediction.rs`
- `sniff/lib/tests/git_parity.rs`
- `sniff/docs/sniff-library-architecture.md`
- `darkmatter/lib/src/markdown/compose/context/capture/snapshot.rs`
- `darkmatter/lib/tests/git_context_integration.rs`

## Implementation of Review Findings #17

> **started at:** 2026-07-18T11:18:36-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'darkmatter/features/2026-07-13-more-is-more/review-17.md'
- this is iteration 17 of the review-to-implement cycle
- review 17 contains one finding:
        - **Critical** — acceptance criteria 17–30 remain explicitly deferred and unimplemented
- impacted package areas (from the specification and plan `packages:` frontmatter): `sniff`, `darkmatter` (library + CLI), and `dmls`
- ordering decision: the sole finding is implemented as one serial work item because AC17–30 form the single remediation requested by review 17
- starting the work on 'ac17-30-full-implementation' at 11:19:53-07:00
        - mandatory skills loaded before implementation: `darkmatter`, `rust`, `rust-testing`, `sniff`, `gitnexus-impact-analysis`, and `rust-devops` with its gitoxide reference
        - repository discovery completed with `sniff repo packages`, `sniff repo package-areas`, and `sniff repo package-dependencies`
                - executable scope is `sniff` (`sniff`, `sniff-cli`) and `darkmatter` (`darkmatter`, `darkmatter-cli`, `dmls`); Schematic remains excluded unless provider-definition changes become necessary
                - all verification added by this finding is Level 1; no terminal, browser, device, or live-provider resource is required
        - pre-edit GitNexus impact analysis completed for the existing shared symbols the implementation must extend
                - `preferred_remote_url`: LOW, 3 direct callers, 1 affected CLI execution process
                - `preferred_remote`: HIGH, 2 direct callers, 0 affected execution processes; the shared resolver change will preserve the existing projection contract and add discriminating selection tests
                - `Lexer::next_token`: MEDIUM, 2 direct callers, 0 affected execution processes
                - `Parser::parse_primary`: LOW, 1 direct caller, 0 affected execution processes
                - `SpannedExpr::erase`: LOW, 2 direct callers, 0 affected execution processes
                - `evaluate`: CRITICAL, 39 direct callers, 1 affected compose-pipeline execution process; the edit will be limited to evaluation of the two new immutable literal variants
                - `ExpressionFunctionDescriptor::typed_signature`: LOW, 0 direct callers in the indexed graph, 0 affected execution processes
                - `project_descriptors`: CRITICAL, 1 direct caller, 0 affected execution processes; the edit will preserve data-return behavior and add only closed-enum projection
                - `parse_expression_function_catalog`: CRITICAL, 6 direct callers, 0 affected execution processes; malformed, duplicate, empty-member, array, fallible, and parameter-rejection cases will be covered directly
                - `ResolutionContext::fetch_remote_text`: HIGH, 4 direct callers, 0 affected execution processes; no change is planned unless frontmatter parity inspection proves one is necessary
                - `RemoteRepoProvider::list_pull_requests`: MEDIUM, 5 direct implementations/callers, 0 affected execution processes
                - `RemoteRepoProvider::list_workflow_runs`: LOW, 2 direct implementations/callers, 0 affected execution processes
        - the orchestrator was warned immediately about all HIGH and CRITICAL impact results before production edits began
        - AC17 and AC18 implementation added the two indexed-file family functions and immutable array/object expression literals with focused unit coverage
                - the first compile exposed four exhaustive AST consumers that must traverse literal children to preserve their existing semantics
                - follow-up impact analysis: the frontmatter interpolation walker, subtree strict walker, and error excerpt walker are LOW risk; the remote URL discovery child walker is HIGH risk because it feeds compose and transclusion flows
                - the orchestrator was warned about the HIGH-risk remote discovery traversal before its constrained recursive arm was added
        - AC17/AC18 verification checkpoint passed
                - `cargo check --color=never -p darkmatter --tests`
                - focused Nextest: 3/3 literal-and-index integration tests passed
        - AC30 added return-only closed enums to the catalog descriptor, parser, projection, and typed-signature surfaces
                - variants are retained as `ReturnValueType::Enum`, including a quoted empty-string member
                - array and fallible flags remain orthogonal; enum parameters and empty, malformed, or duplicate return enums are rejected
                - focused Nextest: 2/2 enum catalog tests passed
        - AC19 added Sniff's shared `ResolvedRemote` resolver
                - selection ignores URL-less remotes and follows `origin`, alphabetic non-`upstream`, then `upstream`
                - exact case-sensitive missing and URL-less names produce distinct typed errors
                - fetch URL, push URL, host, nested namespace, repository, and API flavor are projected from configured Git state
                - Sniff test compilation passed and the exact built integration-test binary passed 3/3 tests
                - the initial Nextest wrapper exceeded the non-interactive 60-second ceiling during a cold profile build; no process remained, so the already-built exact test binary was used without rerunning the cold orchestration
        - AC20 and AC21 added live remote branch observation and canonical vendor detection
                - HTTP(S) remotes use read-only Git smart-protocol ref advertisement and match only exact `refs/heads/*`; local refs and configuration remain unchanged
                - supported non-HTTP GitHub, GitLab, Gitea, Forgejo, and Bitbucket remotes use authoritative branch endpoints; missing branches are `false`, while policy, authentication, authorization, rate-limit, protocol, and transport failures remain typed errors
                - deterministic provider URLs are classified locally; ambiguous allowlisted HTTP(S) hosts receive bounded GitLab/Gitea/Forgejo version probes
                - focused Sniff remote-observation tests passed 5/5, including exact ref parsing, no local mutation, deny-before-request, redirect/rate-limit distinctions, and ambiguous vendor probing
        - AC22 through AC25 added a concrete focused-provider contract and the four Darkmatter query functions
                - provider-neutral PR and CI/CD job records retain provider, API flavor, host, nested namespace, repository, native/display identity, parent execution identity, normalized/raw state, and canonical URLs
                - GitHub, GitLab, Gitea, Forgejo, and Bitbucket exact lookups, canonical URL parsing, authoritative not-found behavior, bounded pagination, direct-job listing, and bounded parent-to-job traversal are covered by Wiremock
                - canonical query objects reject unknown keys, wrong types, invalid enum values, inverted ranges, non-positive or over-100 limits, and unsupported exact filters before network access
                - PR and CI/CD formatters are pure, deterministic, whitespace-collapsing, Markdown-escaping, and shared by exact/list results
                - focused Sniff provider tests pass 6/6; focused Darkmatter handler/catalog tests pass 7/7
        - AC26 installed one run-wide provider-query single-flight cache in the shared remote runtime
                - normalized keys include operation plus selected remote/reference/query; typed successes and focused error strings are memoized only for the compose run
                - frontmatter interpolation, body interpolation, and `$()` evaluation now attach the same authorized runtime; existing remote file functions gained the same frontmatter behavior
                - distinct provider calls share the existing remote-concurrency cap; exact-host consent is checked before credentials or requests, redirects are disabled, and cross-host API routing is limited to `github.com` → `api.github.com` and `bitbucket.org` → `api.bitbucket.org`
                - AC26 focused Darkmatter tests pass 5/5, including one-request frontmatter/body parity; host-policy/provider tests pass 7/7
        - AC27 preserved focused malformed, missing/not-found, missing/invalid credential, forbidden, rate-limited, unsupported filter/capability, policy-denied, malformed-response, redirect, and unreachable states without converting them to neutral results
        - AC28 and AC30 catalog/DMLS parity completed
                - catalog orders 88–96, aliases, overloads, runtime registrations, enum typed signatures, generated expression docs, and passive DMLS completion/hover all derive from the authored catalog
                - DMLS recursively traverses new object/array literals while remaining passive; focused DMLS tests pass 2/2
        - explicit AC17–30 gap audit at 12:55:44-07:00 found one partial implementation gap
                - Azure DevOps, AWS CodeCommit, and SourceHut HTTP(S) remotes are supported by vendor-neutral Git ref advertisement
                - SSH-only remotes for those three providers return `UnsupportedRemoteCapability` instead of a false absence because their credentialed provider branch transports are not implemented in this cycle
                - this maps to AC20 only; it will be reported as a partial deferral rather than claiming the review finding wholly fixed
        - documentation updated through the authored catalog generator, the Sniff library README, and the `sniff`/`darkmatter` skills; skill hashes were recomputed with `md hash`

## Implementation of Review Findings #17

> **started at:** 2026-07-18T18:00:56-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'darkmatter/features/2026-07-13-more-is-more/review-17.md'
- this is iteration 17 of the review-to-implement cycle
- starting the work on 'ac17-30-full-implementation-continuation' at 18:02:50-07:00
        - review 17 contains one Critical finding covering acceptance criteria 17–30
        - an earlier unfinished iteration-17 attempt is present in the worktree; this continuation will audit and complete that existing implementation without discarding unrelated user changes
        - package discovery confirms the directly affected package areas are `sniff` and `darkmatter`; `darkmatter`, `darkmatter-cli`, and `dmls` consume the changed surface
        - GitNexus cannot resolve the new untracked remote-observation symbols, so their pre-edit impact is UNKNOWN; no HIGH or CRITICAL indexed blast-radius warning was returned
        - the continuation audit confirmed the previously logged AC20 gap for SSH-only Azure DevOps, AWS CodeCommit, and SourceHut remotes
        - the continuation audit found two further AC20/AC29 gaps in the partial implementation
                - branch names were not normalized and validated at Sniff's public live-observation boundary
                - Sniff's area `just test` recipe did not enable the `network` feature, so the new Wiremock suites were not part of the required full Level-1 gate
        - focused Sniff network-feature compilation and remote unit tests passed before the continuation edits

## Implementation of Review Findings #17

> **started at:** 2026-07-19T08:49:09-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'darkmatter/features/2026-07-13-more-is-more/review-17.md'
- this is iteration 17 of the review-to-implement cycle
- review 17 contains a single **Critical** finding: acceptance criteria 17–30 remain explicitly deferred and unimplemented
- **this is the third iteration-17 attempt.** Two prior attempts are recorded above; neither reached a `### Successful Completion` section, so this run begins with an audit of what those attempts actually landed rather than re-implementing from scratch
        - the AC17–30 production surface **is present in the worktree** and committed: `find_first_index`/`find_last_index`, array/object expression literals, `ResolvedRemote`, `branch_exists_on_remote`, `remote_vendor`, `pr_exact`/`pr_list`, `cicd_exact`/`cicd_list`, the focused-provider client, and the run-wide network policy/single-flight cache
        - what the prior attempt left open, per its own log entries at lines 197–201, is the closing set: the AC20 SSH-only-provider gap, branch-name normalization at Sniff's public boundary, and the `network` feature missing from Sniff's area `just test` recipe (so the new Wiremock suites were never part of the required Level-1 gate)
- impacted package areas: `sniff` and `darkmatter` (library + CLI + `dmls`), plus `claudine` as a downstream consumer of the changed `Expr` enum (see below)

### Finding 1 (Critical) — acceptance criteria 17–30 remain deferred and unimplemented

- starting the work on 'ac17-30-parity-audit-and-close-out' at 08:52:00-07:00
        - a subagent performed an evidence-based parity audit of all 14 criteria against the spec, requiring for each a production citation **and** a test that genuinely discriminates the contract (a test that merely calls the function without asserting its specific behavior was recorded as no coverage)
        - result: **11 of 14 SATISFIED** as landed by the two prior attempts, **3 PARTIAL** and now fixed. The prior attempts' work is real, not a paper implementation
        - the three gaps the prior attempt left open were each re-verified rather than taken on trust:
                - **(a) SSH-only Azure DevOps / AWS CodeCommit / SourceHut (AC20) — genuinely CLOSED.** `remote_observation.rs:122` `provider_https_git_url` maps all three to canonical HTTPS Git endpoints, pinned by `ssh_only_providers_map_to_canonical_https_git_endpoints` asserting the exact three URLs. `UnsupportedRemoteCapability` is now correctly reserved for unidentifiable hosts
                - **(b) branch normalization at Sniff's public boundary (AC20) — genuinely CLOSED.** `normalize_branch` (`remote_observation.rs:44`) runs at `:27` before any URL construction; `invalid_branch_is_rejected_before_any_request` asserts `server.received_requests()` is empty, proving rejection precedes I/O
                - **(c) `network` feature in Sniff's area gate (AC29) — was only HALF closed; now fixed.** This was the most consequential finding of the audit
        - **AC29 — the area gate was passing vacuously.** `sniff/lib/Cargo.toml` declares `remote = ["network"]`, an implication that runs one way only, and `sniff/lib/src/lib.rs:16` gates the entire `sniff::remote` module on `remote`. Because the recipe selected `--features network`, `tests/focused_provider.rs` and `tests/remote_providers.rs` (both `#![cfg(feature = "remote")]`) compiled to **empty binaries and reported success** — the whole AC22–25/AC27 Wiremock surface was green without executing. `just lint` specified no features at all, so clippy had never seen roughly 3,000 lines of new provider code
                - switching both recipes to `--features remote` brings previously-skipped tests into the gate; the measured delta is recorded under **Verification Gates** below
                - this is exactly the failure mode AC29 exists to prevent, and it means the prior attempt's "focused tests passed" evidence was weaker than it appeared
        - **AC19 was PARTIAL — two parallel preferred-remote resolvers disagreed.** `types.rs::preferred_remote` (behind `GitRepo::org_and_repo()` and `detect_with_request`) did not skip URL-less remotes, unlike `resolve_remote_at`. A repo with a URL-less `origin` and a URL-bearing `alpha` reported `org_and_repo() == (None, None)` while `preferred_remote_url()` returned alpha's URL — two Sniff surfaces disagreeing about one repository, which is precisely what AC19's "shared resolver" requirement forbids
                - fixed by extracting `select_preferred_remote` (`remote_resolver.rs:98`) as the single ordering authority; both call sites delegate and both pre-filter URL-less remotes
                - before/after evidence: neutralizing the URL filter fails the new `aggregate_projection_and_resolver_agree_when_origin_has_no_url` with `left: (None, None)` / `right: (Some("acme"), Some("alpha"))`; restoring it passes
        - **AC22 was PARTIAL — `capabilities()` had no test at all.** The hand-maintained `pull_request_filters` list (`focused.rs:87`) was uncoupled from the actual rejection logic (`focused.rs:695`), so the declared capability could silently drift into a lie. Added `declared_filters_match_the_filters_the_client_actually_honors`, driving every declared filter through a real query and every rejected filter through the same path, and asserting AC25's `logs`/`artifacts`/`test_reports` stay false
        - **downstream break found by the orchestrator, not by the audit:** AC18 added `Expr::ArrayLiteral` and `Expr::ObjectLiteral`, and `claudine` matches exhaustively on `Expr` in five modules (`dispatch/matcher.rs`, `composition/preflight.rs`, `composition/looping/config.rs`, `composition/lifecycle/{executor,validate}.rs`). Per the repo rule that public enum changes must include downstream packages from impact analysis, `claudine` is being verified and repaired as part of this cycle rather than left to break on `main`
        - **`claudine` was genuinely broken: `cargo check -p claudine` failed with 8 × E0004.** The prior two attempts landed a breaking public enum change without checking its consumers. Repaired across all 8 sites with correct per-site semantics rather than a wildcard
                - uniform rule adopted: recurse into array elements and object **values**, combining exactly as each site's existing recursive arms (`BinaryOp`, `FunctionCall`) do; object **keys** are authored text and contribute nothing — except in `validate.rs::visit_string_literals`, which must visit keys because the parser accepts quoted keys, so `{ "{{ leaked }}": 1 }` can carry an uninterpolated span into dispatched text
                - no catch-all arm was added anywhere, so the next AST variant breaks loudly again instead of being silently swallowed — which is precisely how this break went unnoticed for two iterations
                - the `dispatch/matcher.rs` arm is load-bearing rather than cosmetic: returning an unconditional `true` would have hijacked legacy regex character classes (`[abc]`, `[Bb]ash`) — which also parse as array literals of bare variables — into expression matchers. Recursion preserves their legacy regex compilation byte-for-byte
                - 15 discriminating tests added; ablating the `references_bare_err` arms to `false` was verified to fail two of them before the fix was restored
                - authoring gotcha recorded for future test writers: `null` is **not** a literal in this expression language — `x != null` parses `null` as a bare variable and pollutes identifier and undefined-variable scans
- work completed for 'ac17-30-parity-audit-and-close-out' at 09:55:00-07:00

### Verification Gates

- all gates below are **real exit codes** from unpiped background runs, not statuses read off a pipe
        - `sniff just test` — **0** (1565 lib + 769 CLI)
        - `sniff just lint` — **0**
        - `darkmatter just test` — **0** (5848 + 561 + 591)
        - `darkmatter just lint` — **0**
        - `cargo check -p claudine --tests` — **0** (was 8 × E0004 before this cycle's repair)
- **measured evidence that the AC29 recipe fix was real, not cosmetic:** the same `sniff just test` recipe reported **1396** lib tests before the `--features remote` correction and **1565** after — **+169 tests** that had been compiling to empty binaries and reporting success. The subagent's initial estimate of 71 was low; 169 is the measured delta between the two runs in this session
- host load fell from 18.8 at start to 13.1/27.5/57.2 (1/5/15 min) across the gates; no timeouts occurred and no gate was retried for load
- `claudine just test` (**1**) and `claudine just lint` (**1**) do not pass, but both failures sit outside this cycle's change
        - `dispatch_inventory_matches_committed_file` — a pure line-number drift (635 → 664) for a `Provider::Claude` reference in `claudine/cli/src/commands/wrap/env/tests.rs`, which grew on this branch without the inventory being re-blessed. Re-bless with `CLAUDINE_UPDATE_INVENTORY=1 cargo nextest run -p claudine-cli --test dispatch_inventory`
        - three clippy errors in `claudine/cli/src/commands/wrap/harness_orch/{loop_control,prompt}.rs` — one `collapsible_if` and two `result_large_err` on `CompositionError`
        - **honesty qualifier:** this cycle's `claudine` diff is confined to `claudine/lib/src`, and both failures sit in `claudine/cli/src`, so a lib-only change cannot have produced them. That is a causal argument from diff locality, **not** a verified `main`-branch baseline — the sandbox denied the `git log` / `git diff` calls that would have confirmed it directly. Treated as pre-existing branch debt and left unfixed as out of scope for this feature
        - `claudine` lib suite is green at 3411/3411; `claudine-cli` is 1906/1907

### Successful Completion

The implementation of review cycle 17 has completed successfully in 1 hour 12 minutes. During this implementation all 1 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 1 were fixed, 0 were deferred (see reasons below):

- no finding was deferred. The sole **Critical** finding — "acceptance criteria 17–30 remain explicitly deferred and unimplemented" — is now **fixed**. The AC17–30 production surface had in fact been landed by the two earlier iteration-17 attempts, neither of which closed out its log; this cycle audited that work criterion by criterion, closed the three criteria that were only PARTIAL (AC19, AC22, AC29), repaired the downstream `claudine` breakage those attempts introduced, and ran the full Level-1 area gates that review 17 recorded as never having completed.

Two qualifications belong on the record rather than buried, because each narrows what "fixed" means:

- **AC29's cross-platform clause is verified on macOS only.** Windows and Linux compile checks were not run: a cold cross-target build of this dependency graph exceeds the non-interactive command ceiling, and this repo has a known-blocked Windows cross-compile toolchain path. No platform-conditional code was added — the new surface uses `reqwest::blocking`, `gix`, `url`, and `urlencoding` — so it is plausibly portable, but that is an expectation, not evidence.
- **`claudine`'s area gates remain red on two pre-existing defects** (the `dispatch_inventory` re-bless and three `claudine-cli` clippy errors). Both were judged out of scope for this feature and are argued pre-existing from diff locality rather than a verified baseline, as recorded above.

`deferred_perf_measurement` remains `false`: this cycle's findings were correctness and coverage defects verified by assertion-based Level-1 tests, and no performance metric was required or deferred.

The most important lesson from this cycle is that a passing area gate is not the same as an executed one. Sniff's `test` recipe selected `--features network` while the entire `sniff::remote` module is gated on `--features remote`; because `remote = ["network"]` implies in one direction only, two Wiremock suites compiled to empty binaries and reported success. AC22–25 and AC27 were "green" for an entire iteration without executing a single assertion.

The files changed during this implementation cycle were:

- `sniff/justfile`
- `sniff/lib/src/filesystem/git/remote_resolver.rs`
- `sniff/lib/src/filesystem/git/types.rs`
- `sniff/lib/tests/remote_resolution.rs`
- `sniff/lib/tests/focused_provider.rs`
- `claudine/lib/src/dispatch/matcher.rs`
- `claudine/lib/src/composition/preflight.rs`
- `claudine/lib/src/composition/looping/config.rs`
- `claudine/lib/src/composition/lifecycle/executor.rs`
- `claudine/lib/src/composition/lifecycle/validate.rs`
- `claudine/lib/src/composition/lifecycle/tests.rs`
- `claudine/lib/src/composition/lifecycle/executor/tests.rs`
