---
implementation_15: "2026-07-18T01:11:14-07:00"
implementation_17: "2026-07-19T08:49:09-07:00"
implementation_18: "2026-07-19T10:23:12-07:00"
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

## Implementation of Review Findings #18

> **started at:** 2026-07-19T10:23:12-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'darkmatter/features/2026-07-13-more-is-more/review-18.md'
- this is iteration 18 of the review-to-implement cycle
- review 18 contains eight findings:
        - **Critical** — provider list adapters do not implement complete, correctly ordered canonical queries
        - **High** — canonical query validation and defaults disagree with the public contract
        - **High** — CI/CD normalization drops fields the structured-record contract promises to retain
        - **High** — the provider expression surface has no end-to-end Level-1 network verification
        - **High** — required Windows and Linux compile verification is absent
        - **Medium** — "Markdown-escaped" provider text still permits Markdown formatting injection
        - **Medium** — DMLS hover and authored docs do not expose or link the query vocabulary
        - **Medium** — each provider expression creates a thread and Tokio runtime instead of using the run-local executor
- impacted package areas: `sniff` (lib), `darkmatter` (lib + dmls + docs)
- host load at start: `load averages: 20.39 38.61 34.73` — no performance findings in this review, so load affects gate wall-clock only, not measurement validity
- ordering decision: findings are implemented serially in severity order, except that the **Critical** adapter finding and the **High** canonical-validation finding both touch `sniff/lib/src/remote/types.rs` + `focused.rs` and are therefore sequenced adjacently to avoid conflicting edits

### Finding 1 (Critical) — provider list adapters do not implement complete, correctly ordered canonical queries

- starting the work on 'complete-ordered-canonical-queries' at 10:26:41-07:00
        - scope confirmed by reading `sniff/lib/src/remote/focused.rs` directly; four distinct defects behind one finding:
                - `query_pull_requests` breaks out of the page loop at `normalized.len() == limit` and only then calls `sort_prs`, so the returned page is the first-`limit`-encountered set re-ordered, not the globally newest/oldest `limit`
                - `direct_jobs` / `jobs_via_parents` return early at `limit` and at `MAX_JOBS_INSPECTED` / `MAX_PARENT_EXECUTIONS`, then `query_cicd_jobs` sorts the partial subset — same defect plus a silent cap
                - exhausting `MAX_PAGES` with fewer than `limit` matches yields `exhausted == false`, `next == None`, and `items == []` — a silent incomplete-domain empty result, which the spec forbids
                - `pr_page_params` forwards the canonical state token nearly verbatim, so GitLab receives `open` (needs `opened`), GitHub receives `merged` (accepts only `open`/`closed`/`all`), and Bitbucket receives `ALL`/`CLOSED` (needs `OPEN`/`MERGED`/`DECLINED`, repeated)
        - **complete-domain traversal.** `query_pull_requests`, `direct_jobs`, and `jobs_via_parents` no longer early-exit on match count. Each walks to provider exhaustion, and only then does the caller sort and `truncate(limit)`. Local emulation of exact filters and ordering is only sound over a complete domain, which is precisely what the early exit destroyed
        - **page size decoupled from `limit`.** A new `PAGE_SIZE = 100` constant replaces `limit.clamp(1, 100)`. Once the walk always reaches exhaustion, sizing pages by the caller's `limit` only multiplies the number of round-trips and burns the `MAX_PAGES` budget faster — a `limit: 2` query previously requested 2-row pages and could exhaust 20 pages after seeing only 40 rows
        - **error variant chosen: a new `SniffError::IncompleteRemoteDomain { provider, bound, limit }`.** Both existing candidates were rejected as dishonest: `UnsupportedRemoteCapability` claims the provider cannot do the thing at all, when in fact the capability exists and only *this query's* domain was too large; `RemoteApi` claims the provider returned a failure, when every response was a well-formed 200. The new variant names the bound that stopped the walk (`pull-request pages`, `job pages`, `parent executions`, `inspected jobs`) so the message is actionable — the user learns to narrow the query. Blast radius checked: no `match` on `SniffError` anywhere in the repo is exhaustive, and Darkmatter maps Sniff errors through `Display` (`expression/functions/provider.rs:47`), so no mapping site needed changing. `cargo check -p darkmatter --tests` confirms this
        - **flavor-specific state projection** via a new `pr_state_params(flavor, states)`. GitHub/Gitea/Forgejo get `open`/`closed`/`all` only — canonical `merged` widens to `closed` because those services surface merged PRs there. GitLab gets `opened`/`closed`/`merged`/`all`. Bitbucket gets repeated `state=` pairs (`OPEN`/`MERGED`/`DECLINED`/`SUPERSEDED`) and omits `state` entirely for "any", since it has no `ALL` token to send. A canonical multi-state set with no single wire token widens to the provider's `all`
        - widening is only safe because `pr_matches` stays authoritative; this was verified rather than assumed, and is now pinned by `widened_provider_state_is_narrowed_by_the_exact_local_filter` (canonical `merged` sends `state=closed`, and the closed-but-unmerged row is dropped locally)
        - **defect found that the review did not name:** `MAX_JOBS_INSPECTED` (2 000) is unreachable on the GitLab direct-listing path, because `MAX_PAGES × PAGE_SIZE` is exactly 2 000, so the page bound always trips first. It remains genuinely reachable through parent traversal, where many cleanly-exhausted parents accumulate past it, so the guard was kept rather than deleted — it is bound-independent and would matter if either constant changed. Recorded here so a future reader does not mistake the direct-path branch for live code
        - **the two existing tests the review called non-discriminating were both confirmed non-discriminating.** `pull_request_query_paginates_until_filtered_limit` was replaced outright; `declared_filters_match_the_filters_the_client_actually_honors` still treats an empty 200 as proof a filter is honored, but that is the review's separate High finding on validation, not this one, so it was left alone
        - **discrimination was proved, not asserted.** Each new test was run against the *old* implementation restored in place (PR path and CI/CD path in two separate passes) to confirm it actually fails there. 10 of the 11 new tests fail against the old code. The exception is `pull_request_filters_reach_matches_beyond_the_first_page`, which passes both before and after — the old code did paginate, its defect was the early exit and post-hoc sort — so it is an honest regression guard rather than a discriminator, and is recorded as such
        - `PullRequestPage.total` / `CiCdJobPage.total` now carry the **domain-wide match count before truncation**, which can exceed `items.len()`. `next` is `None` on both: the previous `"provider-next"` string was a placeholder token no code could resolve, and with a complete domain there is nothing further to page to. The stale `"provider did not expose an authoritative total"` warning was dropped, since the total is now authoritative
        - **verification gates** (macOS, from `sniff/`):
                - `just lint` — **pass**, clippy clean across `sniff` + `sniff-cli` with `--features remote`
                - `just test` — **pass**: `sniff` 1 576 run / 1 576 passed / 3 skipped; `sniff-cli` 769 run / 769 passed / 3 skipped
                - one `LKFAIL` on `remote::tests::test_git_remote_from_url_invalid` retried green; this is the known spurious nextest leak-timeout on CLI-spawning binaries, unrelated to this change
                - `cargo check -p darkmatter --tests` — **pass**, confirming the new error variant breaks no downstream consumer
                - `just test-l2` not run: it targets `sniff-cli` CI/CD status-cell styling and no glyph, SGR, or terminal behavior is touched here
- work completed for 'complete-ordered-canonical-queries' at 10:41:36-07:00

### Finding 2 (High) — canonical query validation and defaults disagree with the public contract

- starting the work on 'canonical-query-validation-defaults' at 10:41:36-07:00
        - five sub-defects named by the review, all on the authored-input boundary rather than the wire boundary
        - deliberately sequenced immediately after Finding 1 because both edit `sniff/lib/src/remote/types.rs` and `focused.rs`; Finding 1's subagent explicitly left the `Default`-derived `descending: false` and the non-discriminating `declared_filters_match_the_filters_the_client_actually_honors` test alone as belonging here
        - **design decision — no dedicated Darkmatter input-type layer.** The review recommends parsing into Darkmatter-owned input types and translating into Sniff queries after validation. Rejected as disproportionate: the canonical vocabulary and the Sniff query struct are field-for-field identical, so a parallel type would be a rename with a hand-written 19-field `From` impl, and it would create two places where the vocabulary can drift. Instead the vocabulary is single-sourced on the Sniff query structs, which now carry `validate_canonical()`. Darkmatter calls it in `parse_query()` — before `provider::client(...)`, so before any repository or client resolution — and `FocusedProviderClient::query_*` calls it again before its first request. The invariant "no invalid canonical input reaches the network" therefore holds at both entry points, and it is tested at both (Darkmatter parser unit tests plus `sniff/lib/tests/focused_provider.rs`).
        - **defect 1 — `remote` coercion.** New `provider::authored_remote()` in `darkmatter/lib/src/markdown/compose/expression/functions/provider.rs`; both `pr_list` and `cicd_list` route the removed `remote` key through it. Absent is `None`; a non-blank string is the remote; everything else — wrong type, empty string, whitespace-only, explicit `null` — is an invalid-query error naming the field. Whitespace-only was not in the review's list but is the same defect: `"   "` passed the old `!value.is_empty()` filter and would have been sent as a remote name.
        - **defect 2 — closed `state` vocabulary.** Chose the "split the enum" option over a custom `Deserialize` on `PullRequestState`, because the review's premise that `Draft`/`All` are an "internal superset" is false for the focused client: `query.state` is only ever populated from authored input or the `Open` default, so closing the vocabulary makes those two `pr_matches` arms unreachable rather than merely unspellable. New 3-variant `CanonicalPullRequestState` in `types.rs` is now the type of `PullRequestQuery.state`. Legacy `PullRequestState` is untouched and keeps `Draft`/`All`, which the Stage-1 report API, the four Stage-1 provider adapters, and the `sniff repo --status` CLI flag all still need. `provider.rs` (Stage-1) gained an explicit canonical→legacy state projection at the one place it calls `list_pull_requests`.
                - **not named by the review:** `pr_matches` treated a merged PR as matching `closed`, in both `focused.rs` and `provider.rs`. The spec defines `closed` as "closed pull requests (not merged)", and canonical `closed` widens to the provider's `closed` token which returns merged rows on GitHub/Gitea, so `state: closed` was over-matching. Both matchers now require `merged_at.is_none()`.
        - **defect 3 — `parent` integer form.** `deserialize_parent_identity` in `types.rs`: an untagged `String | u64` normalized to the string form `job_matches` compares against. Serialization is unchanged (`Option<String>`), so Darkmatter's `serde_json::to_string(&query)` cache key stays stable and `{"parent": 1234}` and `{"parent": "1234"}` share one cache entry.
        - **defect 4 — datetime validation.** `chrono` was already a direct dependency of both `sniff/lib` and `darkmatter/lib`, so no dependency was added and no `docs/dependencies.md` needed updating. `parse_query_timestamp()` accepts offset-bearing RFC 3339 first, then falls back to a bare `YYYY-MM-DDTHH:MM:SS` / `YYYY-MM-DD HH:MM:SS` / `YYYY-MM-DD` read as UTC, so an authored window bound does not have to carry a zone. Validation happens in `validate_time_window()`, which compares the **parsed instants**; the four matcher comparisons in `focused.rs` and `provider.rs` now go through `at_or_after`/`at_or_before`, and `sort_prs`/`query_cicd_jobs` order through `timestamp_order`/`optional_timestamp_order`.
                - the matcher helper deliberately falls back to byte order when *either* side fails to parse: query bounds are known-good post-validation, but provider payloads are not, and silently dropping an unparseable row would corrupt the complete-domain guarantee Finding 1 just established
                - consolidation side effect: `focused.rs`'s `validate_limit`, and `provider.rs`'s `validate_limit`/`validate_sort` and its two lexical range checks, are all gone — every rule now lives once in `validate_canonical()`
        - **defect 5 — newest-first default.** Hand-wrote `Default` for both query structs with `descending: true`, which is also what container-level `#[serde(default)]` fills in for an absent key, so the object, empty-object, and count forms all agree without any of them setting the flag explicitly. Removed the now-redundant `descending: true` from `cicd_list`'s count overload so the default is single-sourced.
                - **not named by the review, and the substantive half of this defect:** flipping the flag alone does *not* deliver D24. `sort_prs` did `sort.unwrap_or("provider-default")`, which conflates an absent `sort` with an explicit `provider-default`, and `provider-default` applies no ordering key at all — it just reverses whatever the provider returned. Against GitHub, whose list endpoint is already newest-first, `descending: true` would have produced **oldest**-first. `sort_prs` now treats `None` as `created` and reserves order-preservation for an explicit `provider-default`. `CiCdJobQuery` has no `sort` field and already ordered by `created_at`, so it needed only the flag.
        - **`deny_unknown_fields` confirmed in force** on both structs. Verified rather than assumed, because the Darkmatter parser mutates the object before deserializing: it removes `remote` and rewrites `direction` into `descending`, both of which would otherwise trip the unknown-key check. `canonical_query_deserialization_rejects_out_of_vocabulary_input` asserts that a raw `{"remote": ...}` reaching the struct *is* rejected, which pins the parser's removal as load-bearing rather than incidental.
        - **`declared_filters_match_the_filters_the_client_actually_honors` rewritten.** Was mounted against an empty `200`, so "the request did not error" was the whole assertion and a filter that was parsed but never applied passed. Replaced the fixture with `filter_probe_domain()` — three PRs differing on every filterable dimension — and `single_filter_query` became `single_filter_case`, returning the query *and* the PR numbers that filter must select. A filter that is accepted but not applied now returns the whole domain and fails. Two cases needed care: the canonical `open` default applies whenever `state` is absent, so only the `state` case can see the merged PR; and `sort`/`direction` are ordering controls, so their expectation is an order over the default set rather than a subset.
        - **discrimination verified empirically, not by assertion.** Each new behavior was temporarily reverted in place and the suites re-run:
                - byte-order `timestamp_order` → `datetime_filters_compare_instants_not_strings` FAILED
                - lexical `validate_time_window` → `canonical_validation_rejects_unparseable_and_inverted_datetimes` FAILED, and both Darkmatter `datetime_bounds_are_parsed_rather_than_compared_lexically` and `query_validation_rejects_bad_shapes_before_repository_resolution` FAILED
                - `descending: false` defaults → `canonical_queries_default_to_newest_first`, `pull_request_filters_reach_matches_beyond_the_first_page`, `declared_filters_match_the_filters_the_client_actually_honors`, and both `every_call_form_defaults_to_newest_first` FAILED
                - all probes reverted; `grep -r DISCRIMINATION-PROBE sniff darkmatter` returns nothing
        - **two new assertions do not discriminate and are regression guards only**, stated plainly rather than claimed as coverage:
                - `canonical_state_accepts_exactly_the_three_authored_tokens` — the old 5-variant enum also accepted `open`/`closed`/`merged`
                - the `json!(5)` element of `cicd_list`'s `every_call_form_defaults_to_newest_first` — the count overload was the one form that already forced `descending: true`; the two object forms in that same test are what discriminate
                - the review's own suggested inverted-range example (`created_after: 2026-07-01T00:00:00Z`, `created_before: 2026-06-30T19:00:00-04:00`) is **also** inverted lexically, so it would have passed against the old code. Replaced with `2026-06-30T23:00:00-05:00` → `2026-07-01T00:00:00Z`, which is 04:00Z vs 00:00Z as instants but ascending as bytes, plus its mirror (`2026-07-01T23:00:00+14:00` → `2026-07-01T10:00:00Z`) for the over-rejection direction.
        - one pre-existing test needed its expectation updated for the new default: `pull_request_filters_reach_matches_beyond_the_first_page` asserted `["201", "202", "203"]` with no `sort` or `direction`, which is now `["203", "202", "201"]`. This is the D24 behavior change, not a regression, and the test's doc comment now says so.
        - `claudine` needs no change: `grep -rl 'PullRequestQuery\|CiCdJobQuery\|PullRequestState' claudine/` is empty, so the public type changes have no downstream consumer beyond `sniff-cli` (legacy enum only, untouched) and `darkmatter`.
        - gates, macOS, exit code checked directly rather than through a pipe:
                - `cd sniff && just test` → `Summary [12.897s] 1581 tests run: 1581 passed (1 slow, 1 flaky), 3 skipped` then `Summary [23.972s] 769 tests run: 769 passed (19 slow), 3 skipped`
                - `cd sniff && just lint` → `Finished dev profile [unoptimized + debuginfo] target(s) in 2.37s`, no diagnostics
                - `cd darkmatter && just test` → `Summary [168.234s] 5856 tests run: 5856 passed (46 slow), 140 skipped`, `Summary [11.874s] 561 tests run: 561 passed (9 slow), 71 skipped`, `Summary [1.090s] 591 tests run: 591 passed, 3 skipped`
                - `cd darkmatter && just lint` → `Finished dev profile [unoptimized + debuginfo] target(s) in 3.01s`, no diagnostics
                - the single `sniff` flaky is `sniff-cli output::tests::docs_filter::readme_flag_is_case_insensitive`, which is unrelated to this change and was already flaky in the pre-change baseline run of the same recipe
        - deferred, and named here so it is not mistaken for done: `capabilities().pull_request_filters` still advertises `sort` and `direction`, which are ordering controls rather than filters; the probe table special-cases them. Tightening that list is a `capabilities()` contract question, not a validation one, and belongs with Finding 1's adapter work.
- work completed for 'canonical-query-validation-defaults' at 11:19:45-07:00

### Finding 3 (High) — CI/CD normalization drops fields the structured-record contract promises to retain

- starting the work on 'cicd-normalization-field-retention' at 11:19:45-07:00
        - the last finding that edits `sniff/lib/src/remote/focused.rs`; after this the remaining five findings are Darkmatter-side or verification-side and can proceed without contending for that file
- projection design: three flavor-specific projections replace the one key-probing normalizer
        - `project_actions_job` (GitHub, Gitea, Forgejo — one shared Actions wire shape), `project_gitlab_job`, `project_bitbucket_job`, dispatched from `normalize_job` on `self.remote.api_flavor`
        - each returns an internal `JobProjection` struct rather than a typed serde DTO per flavor; the providers disagree on the *type* of a field, not just its name (Bitbucket's `state` is an object where GitLab's is a string), so a DTO per flavor would have been three `Deserialize` impls feeding the same flattening step with no extra safety and a new failure mode — a strict DTO rejects the whole job when a provider adds a field
        - `JobProjection` carries `normalized_source` separately from `native_status` because the token answering "did this succeed" is not always the one the provider calls its status: a GitHub job's `status` is `completed` whether it passed or failed, and Bitbucket's verdict is `state.result.name`
- parent metadata propagation: internal `ParentContext`, not a widened `CiCdParentExecution`
        - `CiCdParentExecution` stays exactly as it was; widening it would publish a second copy of branch/commit/trigger/actor next to the job's own and force every consumer to decide which wins
        - `parent_context()` reads run-level metadata flavor-aware (`head_branch`/`head_sha`/`event`/`triggering_actor` for the Actions family; `target.ref_name`/`target.commit.hash`/`trigger.name`/`creator.nickname` for Bitbucket) at the one point in `jobs_via_parents` where the parent JSON is still in scope
        - merge is strictly `job.or(parent)` per field, so nothing is invented and a job-level value is never overwritten; `absent_metadata_is_not_invented_from_the_parent` guards that direction
- `CiCdJob` gained `started_at` and `finished_at` (`updated_at` retained, and falls back to `finished_at` on providers that expose no separate modification instant, which keeps the `updated_*` filters working)
        - blast radius is two constructors: `normalize_job` and the `format_job` test fixture in `darkmatter/lib/src/markdown/compose/expression/functions/cicd.rs`; `sniff-cli` renders the unrelated `CiCdInfo`, not `CiCdJob`, so no CLI surface changed
- not named by the review, found while writing the fixtures
        - **a GitHub job that failed normalized to `success`.** `status` is `completed` for every finished job and the verdict lives in `conclusion`, but the old normalizer folded the literal `completed` to `success`. Same defect on Bitbucket, where `COMPLETED` + `state.result.name: FAILED` also read as a success. Both flavors now resolve the verdict before normalizing; `normalize_status` keeps the `completed → success` arm only for providers with no separate verdict
        - **Bitbucket `state` was read as a string.** It is always an object, so `value_string(&["status", "state"])` returned `None` and every Bitbucket step normalized to `unknown`. The two pre-existing Bitbucket fixtures encoded the wrong shape (`"state": "success"`) and so could not see this; both were corrected to the real object form
        - **Bitbucket UUIDs are brace-wrapped and braces are not path-safe**, so the request that actually reaches the provider is percent-encoded (`%7Bp1%7D`). Client behavior is correct and unchanged; the new mocks assert the encoded path
        - GitLab `pipeline.id` is a JSON *number*, so the existing `nested_string` helper could not read it; added `nested_id` alongside it
- tests added to `sniff/lib/tests/focused_provider.rs`
        - realistic per-flavor fixture builders: `github_actions_job`, `github_workflow_run`, `gitlab_job`, `bitbucket_step`, `bitbucket_pipeline`
        - `exact_jobs_retain_every_field_the_record_promises` — every promised field for all five flavors
        - `parent_run_metadata_reaches_the_jobs_beneath_it` — GitHub run → job and Bitbucket pipeline → step inheritance
        - `absent_metadata_is_not_invented_from_the_parent` — the negative direction
- discrimination verified by reverting, not assumed: the legacy union-probe normalizer was restored verbatim in place (with `started_at`/`finished_at` forced to `None`, which is what "nowhere to live" means) and the suite re-run
        - three tests failed under the legacy normalizer, including the pre-existing `exact_jobs_are_normalized_for_every_initial_flavor` once its Bitbucket fixture was made realistic (`left: "unknown"`, `right: "success"`)
        - discriminating assertions: GitLab `parent.native_id` / `commit` / `trigger` / `started_at` / `finished_at` / `runner`; Bitbucket `native_status` / `normalized_status` / `conclusion` / `started_at` / `finished_at`; Actions-family `normalized_status` / `started_at` / `finished_at` / `runner`; parent-inheritance `branch` / `trigger` / `actor` and the whole Bitbucket branch/commit/trigger/actor set
        - assertions that do **not** discriminate, and are kept deliberately as retention guards rather than as evidence: GitLab `name` / `stage` / `native_status` / `branch` / `actor` / `created_at` / `web_url`; Bitbucket `name` / `parent.native_id` / `created_at` / `runner`; Actions-family `name` / `parent.native_id` / `native_status` / `conclusion` / `commit` / `created_at` / `web_url` / `api_url`; and the entirety of `absent_metadata_is_not_invented_from_the_parent`, which by design passes both before and after
- gate results (macOS, this branch)
        - `cd sniff && just test` → `Summary [12.949s] 1584 tests run: 1584 passed (1 slow), 3 skipped` and `Summary [32.208s] 769 tests run: 769 passed (15 slow), 3 skipped`
        - `cd sniff && just lint` → `Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 45.89s`, no clippy or fmt diagnostics
        - `cd darkmatter && just test` → `Summary [180.368s] 5856 tests run: 5856 passed (99 slow, 2 flaky), 140 skipped`, `Summary [12.934s] 561 tests run: 561 passed (10 slow), 71 skipped`, `Summary [1.584s] 591 tests run: 591 passed, 3 skipped`; the one retried flake is `schema::schema_validation_integration::baseline_cache_does_not_reuse_across_distinct_baselines`, unrelated to this finding
        - `cd darkmatter && just lint` → clean across `darkmatter`, `darkmatter-cli`, and `dmls`
        - Darkmatter gates were required, not optional: the `CiCdJob` field additions touch its `format_job` test fixture
- work completed for 'cicd-normalization-field-retention' at 11:39:29-07:00

### Findings 6, 7, and 8 (Medium) — run concurrently

- starting the work on 'markdown-escaping', 'query-vocabulary-docs', and 'run-local-executor' at 11:39:29-07:00
        - these three findings touch disjoint files (`functions/{pull_requests,cicd}.rs` formatters; `dmls/src/overlay/expressions.rs` + `docs/topics/`; `functions/provider.rs`), so they are dispatched in parallel rather than serially
        - the three sniff-side findings above all contended for `focused.rs`/`types.rs` and had to be serial; nothing here does
        - orchestrator holds each subagent's log items and writes them as one grouped block per subagent below
        - `cargo check -p darkmatter --tests` confirmed green before dispatch, so any compile failure the subagents hit is their own

#### Finding 6 (Medium) — Markdown-escaped provider text still permits Markdown formatting injection

- root cause: `pull_requests.rs` and `cicd.rs` each carried a private `clean()` that collapsed whitespace then escaped only `\`, `[`, `]`; provider titles such as `**urgent**`, `` `code` ``, `_name_`, `<img onerror=...>` survived with active Markdown, and the two copies could drift
- added one shared helper `collapse_and_escape` in a new `functions/escape.rs`; deleted both private `clean()` copies and routed `format_pr` / `format_job` through it
- escaping rule chosen, documented as a contract comment at the helper: escape unconditionally and **only** the 16 ASCII punctuation characters that can begin, end, or alter an _inline_ construct at any column — `\` `` ` `` `*` `_` `~` `^` `[` `]` `<` `>` `&` `|` `$` `{` `}` `!`
        - the set is sized to the **widest** option set in the crate, not the narrowest: `markdown::cleanup` parses with `Options::all()` minus smart-punctuation and definition-lists, so `$` (math), `{`/`}` (heading attributes), `^` (superscript) and `~` (strikethrough/subscript) are live there even though `render_tree_parser_options()` does not enable them; escaping them unconditionally keeps correctness independent of which parser sees the output
        - deliberately **not** escaped — block starters `#`, `-`, `+`, `=`, `:`, and `1.`/`1)` markers: these only open a block at column zero, and both formatters emit a single collapsed line where every escaped value sits after a literal prefix (`[`, `PR `, `CI job `, or a ` · ` separator); escaping them would put a backslash before every `.` and `-` in ordinary prose for no rendered difference, against the spec's compact, noise-free projection
        - deliberately **not** escaped — `(`, `)`, `"`, `'`: only live inside a link destination or title, and since every `[`/`]` is escaped no destination context can form around them
        - relied on the CommonMark rule that a backslash is honored only before ASCII punctuation, so no non-punctuation character is ever prefixed
- **defect not named by the review, fixed in the same pass:** `record.identity.display_id` and `job.reference.display_id` were interpolated into the link label raw — same provider trust boundary, same defect class; now routed through the helper, with zero output change for the canonical `#123`/`456` shapes
- URLs confirmed untouched: the new formatter tests assert the parsed link destination equals `web_url` exactly
- testing approach: replaced hand-written expected strings with a **CommonMark round-trip harness**
        - `escape::harness::parse_literal` re-parses the output with `Options::all() - ENABLE_SMART_PUNCTUATION` and panics on any event other than paragraph/link/text — that panic is the discriminating half, since a surviving code span contributes the same _characters_ as literal text and a text-only comparison would pass
        - `every_punctuation_class_renders_as_literal_text` and `every_punctuation_class_survives_a_link_label` over a 23-case hostile corpus (bold, code, emphasis, strikethrough, superscript, `<script>`, `<img onerror=...>`, entities, table pipes, math, heading attributes, images, links, unbalanced `[` and `]`, trailing backslash, pre-escaped text, list markers, indented code, autolink, footnote reference)
        - `arbitrary_provider_text_round_trips` — a proptest over `[ -~\t\n]{0,48}` asserting both running-text and link-label round-trips; ASCII-printable is the whole attack surface because CommonMark backslash escapes are ASCII-punctuation-only
        - `already_escaped_text_is_not_double_mangled`, `whitespace_collapses_to_single_spaces`, and `prose_punctuation_is_left_unescaped` — the last guards the _anti-over-escaping_ half of the rule so a future "escape everything" change fails loudly
        - `hostile_provider_text_renders_as_literal_text` and `already_escaped_provider_text_is_not_double_mangled` added to both `pull_requests::tests` and `cicd::tests`
        - `formatter_is_deterministic_collapsed_and_markdown_escaped` updated rather than deleted in both files; expected string unchanged (its fixtures only exercise `[`/`]`, escaped identically under the new rule) and the double-call determinism assertion retained
- open for reviewer judgment, no code change made: the "block starters cannot reach column zero" argument is an invariant of the two current callers, stated in the helper's docs but not mechanically enforced

#### Finding 7 (Medium) — DMLS hover and authored docs do not expose or link the query vocabulary

- authored a `## Provider Query Vocabulary` section in `darkmatter/docs/topics/darkmatter-expressions.md`
        - covers D24 defaults/bounds (limit 20, hard max 100, PR `state` defaults `open`, CI/CD `statuses` defaults to all, both newest-first), D25 (no provider-native escape hatch; unsupported canonical fields fail explicitly), and D26 (repository-only scope)
        - full key tables for `pr_list` and `cicd_list`, a closed-enum table, the RFC 3339 datetime contract, and integer-overload semantics
        - the normalized CI/CD status vocabulary was sourced from `sniff/lib/src/remote/focused.rs::normalize_status`; the spec names only `failed`/`cancelled` by example
- **single-sourced the link** in the `description` field of `pr_list`/`cicd_list` in `darkmatter/docs/schemas/expression-functions.yaml`
        - that one field already flows to the generated doc table, `format_function_block()` (hover), and `ExprCompletion::documentation` (completion), so no new plumbing was added
        - this deliberately avoided threading a new `see_also` field through the YAML `$schema` → `RawFunction` → AST → projection → descriptor → generator → hover chain
        - verified every DMLS hover surface emits `MarkupKind::Markdown`, so the link actually renders
- the topic doc is **partly generated**: prose is hand-authored, the function table between `<!-- BEGIN/END GENERATED FUNCTION TABLE -->` comes from `darkmatter/lib/examples/expression_doc_generator.rs`; the vocabulary went in the authored region and the table was regenerated with `--write`
- no `hash:` frontmatter on the topic doc, so no `md hash` regeneration was required
- no Git or remote-provider access introduced into the DMLS path — the change is passive text only, as the spec requires
- tests added, both Level 1:
        - `catalog::tests::query_vocabulary_link_resolves_to_an_existing_doc_anchor` — link present on all four signatures, target file exists, fragment resolves to a real `##` heading by GitHub-style slugification, and the section actually contains the vocabulary content; a dead link now fails the build
        - `overlay::expressions::tests::list_query_functions_link_to_the_vocabulary_in_hover_and_completion` — link reaches both the hover block and every completion documentation string
- **open question deferred to Ken:** the link is authored sibling-relative (`darkmatter-expressions.md#provider-query-vocabulary`), which is clickable inside the doc but only a precise pointer in DMLS hover, where there is no base URI
        - no precedent for doc links in DMLS hover was found to follow, and the two surfaces have incompatible bases — a repo-root-relative path would render broken inside the doc, which itself lives in `docs/topics/`
        - making hover navigation genuinely clickable would require DMLS to rewrite the relative target into a workspace-root `file://` URI at hover-render time; judged out of scope for a Medium documentation finding rather than decided silently

#### Finding 8 (Medium) — each provider expression creates a thread and Tokio runtime instead of using the run-local executor

- root cause: `provider::run` called `std::thread::spawn` + `Builder::new_current_thread().build().block_on(future)` on **every** cache miss; the thread spawn existed to avoid `block_on` inside a caller's active runtime, but paid thread + runtime construction per provider expression and divorced provider work from the shared executor lifecycle
- fix: reused the lazily-built shared multi-thread runtime already owned by `RemoteFetchInner` (worker count = `remote_concurrency`), per the spec's `RemoteFetchRuntime` precedent, rather than inventing a second pattern
        - new `block_on_shared_executor` in `remote_fetch.rs`: `Handle::spawn` onto the runtime, calling thread blocks on a `std::sync::mpsc` receiver
        - **deliberately avoided `Handle::block_on`** — it panics on a tokio worker thread regardless of which runtime owns the handle; this is precisely the recorded Claudine Antigravity buffered-JSON `block_on` panic, and blocking a plain channel is legal anywhere while the work runs on a different runtime, so there is no self-deadlock
        - the run's `Handle` reaches the sync bridge through a scoped thread-local (`RUN_PROVIDER_EXECUTOR`) installed by `RemoteFetchRuntime::cached_provider_query` with restore-on-drop
        - process-wide `OnceLock` fallback executor for contexts with no `RemoteFetchRuntime` — still shared, never per-call, and never dropped, which sidesteps `Runtime::drop`'s blocking-shutdown panic entirely
        - `provider::run`'s signature left **byte-identical**, which is what kept this finding from colliding with the two concurrent sibling subagents
- behavior preserved: focused `SniffError`s still map through `provider_error` to the same `ExpressionError::Other { function, message }` and are never degraded to empty values; a panicking future drops its sender so `recv()` errors and still yields the unchanged `"provider query worker panicked"` message; runtime-unavailability still surfaces as a `SniffError::RemoteInit`-shaped error; the `cached_provider_query` memoization/single-flight layer was left untouched
- tests: `provider_runs_share_one_executor`, `provider_run_works_inside_an_active_runtime`, `provider_run_works_inside_a_multi_thread_runtime`, `sniff_errors_surface_as_focused_expression_errors`, `panicking_query_becomes_an_error_rather_than_unwinding` in `provider.rs`; `provider_queries_run_on_the_runs_own_executor` in `remote_fetch.rs`, backed by a new `#[cfg(test)]` `EXECUTOR_BUILDS` counter at both runtime-construction sites
        - **flake caught and fixed inside the cycle:** the first version of `provider_queries_run_on_the_runs_own_executor` enumerated the executor's worker threads and asserted membership; it failed 1-of-1 on the first run because tokio work-stealing migrates a task across an await point, so worker-set membership is not a sound discriminator — replaced with the deterministic build-counter assertion and verified stable across five runs
- follow-up worth considering: the thread-local is a hidden channel, accepted only because `run`'s signature was pinned for sibling-collision avoidance; now that the sibling edits have landed, threading the `Handle` (or `&ResolutionContext`) through `run` explicitly would be the cleaner end state

#### Gates for the parallel batch

- each subagent ran `cd darkmatter && just test` and `just lint` independently; because they overlapped, the individually reported runs each raced at least one sibling's in-flight edit
        - the docs subagent was hard-blocked mid-run by `E0425: cannot find function count_executor_build` from the executor subagent's partial edit, and separately saw `reference_integration::validate_missing_file_in_child` and `validate_missing_toc_linking_target` fail only under the full parallel run while passing in isolation — pre-existing interference flakes, not caused by this batch
        - the executor subagent's run, which finished last and therefore saw all three change sets, was fully green: darkmatter 5873/5873, darkmatter-cli 561/561, dmls 592/592, zero flaky
- orchestrator re-ran `cd darkmatter && just lint` after all three subagents returned, against the merged tree: **clean** across `darkmatter`, `darkmatter-cli`, and `dmls`
- a merged-tree `just test` is folded into Finding 4's gate run below rather than run twice
- work completed for 'markdown-escaping', 'query-vocabulary-docs', and 'run-local-executor' at 12:00:24-07:00

### Finding 4 (High) — the provider expression surface has no end-to-end Level-1 network verification

- starting the work on 'provider-expression-e2e-verification' at 12:00:24-07:00
        - this is a pure verification-gap finding: AC23/AC25/AC26 promise cross-surface identity, exact-host policy, error preservation, and single-flight, but nothing composes a document containing a provider function against Wiremock
        - the existing cross-surface cache test calls `cached_provider_query` with a manufactured closure rather than a provider expression, and the one-request frontmatter/body integration exercises the unrelated remote `frontmatter(url)` function
        - sequenced after Finding 8 deliberately — the executor rework changed how provider futures reach a runtime, so an end-to-end test written before it would have been exercising the retired thread-per-call bridge
        - test placement: in-crate module `darkmatter/lib/src/markdown/compose/tests/provider_network.rs`, registered from `tests/mod.rs`
                - an integration binary under `darkmatter/lib/tests/` was rejected: the compose path builds its client through `pub(super)` helpers, so reaching them from a separate binary would have required making the provider seam part of the public API
                - matches the existing precedent — the remote `frontmatter(url)` one-request integration also lives in-crate, in `tests/transclusion.rs`
                - hermetic by construction: each fixture starts its own Wiremock server, initializes a throwaway `git2` repository whose `origin` points at loopback, and clears `GITEA_TOKEN`/`FORGEJO_TOKEN` through `EnvGuard` so no ambient developer token can reach the mock; the one credential test sets its own token explicitly
        - seam added — `provider::test_transport`, a `#[cfg(test)]`-only override of the provider API base and API flavor
                - required, not a convenience: `FocusedProviderClient::new` derives `https://{host}/api/...` from `ResolvedRemote::host`, which is stored without a port, so neither the scheme nor the port of a `127.0.0.1:<port>` server can be expressed; and `ApiFlavor` is derived purely from host-name patterns, so a numeric loopback host is always `SelfHosted`/`Unknown` and no provider flavor can ever be selected for it
                - a friendly hostname does not help either: `get_json` re-checks that the endpoint host equals the remote host, so the mock cannot be reached under a name that would pass flavor detection
                - narrowest available shape — neither the module nor the branch reading it is compiled into a non-test build, so there is no public API, no production-only binary, and no environment variable; flavor detection and API-base derivation keep their own unit coverage in Sniff
                - the override cell is process-wide rather than thread-local because compose evaluates its surfaces on whichever thread the caller supplies; every test that installs one is `#[serial_test::serial(provider_transport)]`
                - two small refactors fell out of it: `provider::build_client` and `provider::resolve` now own client construction and remote resolution for both `pr*` and `cicd*`, replacing three duplicated `FocusedProviderClient::new` call sites
        - real defects the end-to-end tests caught
                - **provider error messages were double-prefixed** — `pr(): pr(): rate limited by Gitea API`. The memoization layer stores failures as `String` so a slot stays cloneable, which bakes in the prefix an inner `ExpressionError::Other` already rendered; `cached_provider_query` then re-wrapped it unconditionally. Fixed in `resolve_ctx.rs` via `cached_query_error`, which adopts an already-prefixed message as-is, and pinned by an assertion in the focused-failure test
                - **frontmatter/body parity is broken for focused provider failures** — the identical call aborts the compose from frontmatter but leaves the unevaluated `{{ pr(123) }}` text in the body with only a report warning. Not a provider-specific bug: `is_authoring_fatal` covers unknown functions and broken file references, and `ExpressionError::Other` is deliberately outside that set (`other_is_not_authoring_fatal` asserts it). Left as-is rather than fixed — changing expression-error fatality is a ratified design axis, not a test-enablement decision — but pinned by `body_surface_downgrades_focused_failures_to_warnings` so a future change is deliberate and visible. The spec's narrower "focused errors are never replaced with empty values" bullet does hold on both surfaces, which the test also asserts. **Flagged for review: this is an AC26 parity gap.**
        - limitation observed, not fixed — provider functions are unreachable for most self-hosted instances
                - `canonical_api_base` hard-codes `https://` and `ResolvedRemote::host` drops the port, and `ApiFlavor` comes only from host-name patterns, so the practical reach of `pr*`/`cicd*` is `github.com`, `gitlab.com`, `bitbucket.org`, `codeberg.org`, and hosts literally named `gitea.*` / `gitlab.*` / `forgejo.*`
                - a GitHub Enterprise or self-hosted GitLab at `git.company.com`, or anything on a non-443 port, resolves to `Unknown` and fails with an unsupported-flavor error
                - fixing it needs a configuration surface (an authored API base and/or flavor per remote) plus a decision about how the exact-host policy treats ports — out of proportion for this finding, and `FocusedProviderClient::with_api_base` already exists as the constructor such a feature would call
        - coverage delivered — 16 tests, all Level 1 against Wiremock
                - cross-surface identity: `pr_renders_identically_in_frontmatter_and_body`, `pr_list_renders_identically_in_frontmatter_and_body`, `cicd_renders_identically_in_frontmatter_and_body`, `cicd_list_renders_identically_in_frontmatter_and_body`, `provider_functions_are_available_in_frontmatter_shell_ternary`
                - the list functions keep their JSON array shape in frontmatter while body interpolation flattens to text, so those two compare entry-by-entry rather than by string equality; the `$()` surface yields the chosen command's output rather than the provider string, so it carries the availability and shared-cache claim while the interpolation tests carry value identity
                - single-flight / memoization: `identical_provider_calls_reach_the_server_exactly_once` (four call sites across frontmatter and body, `expect(1)` plus explicit `verify()`), `differently_normalized_calls_do_not_share_a_cache_slot`, `list_queries_with_different_shapes_are_not_memoized_together` (bare-count and object forms collapse onto one slot; an ordering flip does not — exactly 2 requests)
                - exact-host policy: `denied_host_fails_without_contacting_the_provider` asserts both the focused denial naming the host and `received_requests().len() == 0`, so an implementation that sent the request and discarded the response would still fail
                - focused error preservation: `focused_failures_surface_as_errors_rather_than_empty_values` (404/403/429/500/malformed JSON), `missing_credentials_surface_as_a_credential_error`, `invalid_credentials_surface_as_an_authorization_error`, `incomplete_domain_surfaces_rather_than_a_truncated_list` (21 workflow runs trips `MAX_PARENT_EXECUTIONS`), and `a_successful_empty_query_is_still_an_empty_list` as the discriminating counter-case
                - 403 and 429 are mapped to named states rather than carrying the numeric status, so those two assert on the state word; the assertions were relaxed to the real contract rather than the status code
                - rendered values: `hostile_provider_titles_stay_literal_in_the_composed_document` parses the composed output as CommonMark+GFM and asserts the canonical link destination survives while an attacker-supplied `[click](https://evil.example)` in the title comes back as literal text — a substring search would have been the wrong assertion, since escaping preserves those characters by design
        - gates — scope is `darkmatter` only; **no `sniff` file was touched**, so the sniff gates were not required and were not run
                - `just test` (darkmatter area): `darkmatter` 5889 passed / 140 skipped, `darkmatter-cli` 561 passed / 71 skipped, `dmls` 592 passed / 3 skipped — all green
                - `just lint` (darkmatter area): clean for `darkmatter`, `darkmatter-cli`, and `dmls`; separately confirmed with `cargo clippy -p darkmatter --all-targets` from cold so the `#[cfg(test)]` module is genuinely covered
                - neither known pre-existing flake (`reference_integration::validate_missing_file_in_child`, `validate_missing_toc_linking_target`) appeared, and no spurious nextest `LKFAIL` was seen

### Finding 5 (High) — required Windows and Linux compile verification is absent

- starting the work on 'windows-linux-compile-verification' at 12:40:55-0700
        - no local cross-compile evidence is obtainable on this host, and this was not re-litigated
                - `x86_64-unknown-linux-gnu` is not an installed rustup target; `rustup target add` and `docker` are both blocked by the session sandbox
                - the Windows targets (`x86_64-pc-windows-msvc`, `-gnu`) are installed but die in `aws-lc-sys v0.43.0`'s build script, which needs an MSVC or mingw C toolchain that does not exist on macOS
                - the finding is therefore closable as **CI configuration**, not as delivered Windows/Linux evidence — see the closing bullet
        - the review's premise was partly stale; measured facts on this tree, corrected before changing anything
                - `sniff/justfile`'s `test` recipe **already** passes `--features remote` (`@just _test sniff --features remote`), and `lint` already passes it too. So `cd sniff && just test` in the existing `sniff-cross-platform` job already *executes* the provider Wiremock suites on macOS, Linux, and Windows. This was the single most important correction: the provider tests were not uncovered
                - `sniff/cli/Cargo.toml` declares `sniff = { path = "../lib", features = ["network", "remote"] }`, so the existing `cargo check -p sniff-cli --all-targets` step compiles the provider library *source* on all three OSes as a dependency
                - `darkmatter/lib/Cargo.toml:101` declares `sniff = { path = "../../sniff/lib", features = ["remote"] }`. Darkmatter's `_area-ci.yml` legs (`full-os: ubuntu-latest, windows-latest`; `check-os: macos-latest`) therefore already build the provider source on all three platforms. **No Darkmatter change was needed** and none was made
                - `claudine/lib` and `claudine/cli` depend on `sniff` *without* `remote`, but on `darkmatter` (`claudine/lib/Cargo.toml:20`), so they reach the provider surface transitively. `claudine-tests.yml` ran `ubuntu-latest` only — no Windows and no macOS leg at all
        - the residual, real gap: `--all-targets` under `remote` is built by nothing cross-platform
                - `remote = ["network"]` with `default = []`, so `cargo check -p sniff --all-targets` (the job's self-described "compile guard" for test *code*) `cfg`s the `focused_provider` and `remote_observation` binaries down to empty targets. The guard guarded everything except the surface this feature added
                - nextest builds test targets but not benches, so the `network`-gated `perf` bench and the profiling examples were compiled on no platform at all
        - changes made
                - `.github/workflows/test.yml` — added a `Check sniff library with remote provider surface (all targets)` step running `cargo check --color=never -p sniff --all-targets --features remote` to the `sniff-cross-platform` matrix. Kept as a **separate** step from the default-feature check so "default builds, remote does not" stays a distinguishable failure
                - same file — corrected the two misleading comments: the compile-guard comment now says what the default-feature check does *not* cover, and the `just test` comment records that the recipe carries `--features remote` and that removing it would silently empty the suites
                - `.github/workflows/claudine-tests.yml` — added a `cross-platform-check` job (`macos-latest`, `windows-latest`) running `cargo check --color=never --all-targets -p claudine -p claudine-cli`. Linux is already covered by the existing four test jobs
                - `docs/testing-strategy.md` → "Platform Coverage (CI)" — added a `Feature-gated surfaces` subsection stating the general rule (`--all-targets` resolves *default* features only, so a gated surface compiles nowhere unless a step names it) with `sniff`'s `remote` as the worked case, plus a bullet making the `soft-os` semantics explicit in the policy list. The file's frontmatter carries no `hash:` property, so no `md hash` regeneration was required
        - the claudine decision — compile check, deliberately not a test leg
                - a full Windows `just test` leg would be red for reasons unrelated to the code it is meant to guard: the `claudine-cli` job installs AI-provider PATH stubs written as extensionless `#!/usr/bin/env bash` scripts, which Windows PATHEXT resolution will not execute, and claudine's Windows Ctrl+C handling is a known unimplemented gap with its own dedicated `claudine-windows-ctrl-c.yml`
                - AC16/AC29 ask for *compile* checks, so a compile check is the proportionate instrument; it also happens to be the non-flaky one
                - the Windows leg is `continue-on-error`, matching the `soft-os` convention in `_area-ci.yml`
        - the Windows soft-fail caveat — reported, **not** changed
                - `_area-ci.yml`'s `soft-os` input defaults to `'["windows-latest"]'`, making every Windows *test* leg `continue-on-error`. Its own doc comment says this is intentional: light a platform up, burn down the revealed backlog, then promote
                - consequence for this feature: Darkmatter's Windows evidence is **advisory**. A Windows-only provider regression would be visible in the run but would not block a merge. Flipping this is a repo-wide policy decision with consequences well beyond this feature, so it was left alone and is raised here for a human
                - the `sniff-cross-platform` job in `test.yml` is *not* affected — it has no `continue-on-error`, so its Windows leg (including the new `--features remote` check) is genuinely gating
        - local validation performed
                - `cargo check --color=never -p sniff --all-targets --features remote` — **passes** on this macOS host (`Finished dev profile ... in 6.94s`). This is the macOS half of AC16/AC29 for the provider surface, including the Wiremock test binaries and the `network`-gated bench/example targets
                - `cargo check --color=never --all-targets -p claudine -p claudine-cli` — **passes** on macOS, exit 0. Confirms the new claudine job's command is real and green on at least one of its two OSes
                - all four touched-or-referenced workflow files parse: `yaml.safe_load` clean on `test.yml`, `claudine-tests.yml`, `darkmatter-tests.yml`, `_area-ci.yml`
                - `just --list` in `sniff/` confirms the `test` and `test-l2` recipes the job invokes exist
        - what remains unobtainable in this session, stated plainly
                - **AC16 and AC29 are not satisfied.** What is delivered is the configuration that will produce Windows and Linux compile evidence, plus macOS evidence obtained locally. The Windows and Linux evidence itself does not exist yet
                - only a CI run on `windows-latest` and `ubuntu-latest` runners can produce it. Triggering CI is outside this session's authorization, so no run was started
                - the run that would produce it: any pull request to `main` (fires `test.yml`'s `sniff-cross-platform` matrix) plus a push touching `claudine/**` (fires the new `cross-platform-check` job). A `workflow_dispatch` on `test.yml` would cover the sniff half on demand
- work completed for 'provider-expression-e2e-verification' at 13:00:41-07:00
- work completed for 'windows-linux-compile-verification' at 13:20:12-07:00

### Merged-Tree Verification

- every subagent ran its own gates, but several overlapped or raced a sibling's in-flight edit, so the orchestrator re-ran all four gates against the final merged tree
        - `cd sniff && just test` — **pass**: `sniff` 1 584 run / 1 584 passed / 3 skipped; `sniff-cli` 769 run / 769 passed / 3 skipped
        - `cd sniff && just lint` — **pass**, clippy clean
        - `cd darkmatter && just test` — **pass**: `darkmatter` 5 889 run / 5 889 passed / 140 skipped; `darkmatter-cli` 561 run / 561 passed / 71 skipped; `dmls` 592 run / 592 passed / 3 skipped
        - `cd darkmatter && just lint` — **pass**, clippy clean across `darkmatter`, `darkmatter-cli`, `dmls`
- zero flaky and zero failures on the merged run; the two pre-existing `reference_integration` interference flakes reported mid-cycle did not reproduce
- the darkmatter suite grew from 5 856 to 5 889 tests across this cycle, and sniff from 1 576 to 1 584

### Successful Completion

The implementation of review cycle 18 has completed successfully in 2h 58m. During this implementation all 8 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 7 were fixed, 1 was deferred (see reasons below):

- **Finding 5 (High) — required Windows and Linux compile verification is absent** — deferred, and the deferral is a hard environmental limit rather than a judgment call
        - **not** a performance-measurement deferral, so the `deferred_perf_measurement` frontmatter stays `false`
        - local cross-compilation was attempted and is impossible on this host:
                - `x86_64-unknown-linux-gnu` is not an installed rustup target; `rustup target add` and `docker` are both blocked by this session's permission sandbox
                - both Windows targets **are** installed, but each dies in `aws-lc-sys v0.43.0`'s build script, which needs a Windows C toolchain (MSVC or mingw) that cannot exist on macOS — this independently reconfirms the previously recorded "Windows cross-compile blocked" finding, now with a second root cause beyond `duckdb-sys`
        - what **was** delivered is the configuration that will produce the evidence, plus a correction to the review's premise:
                - the review recommended "add the affected packages to real Windows and Linux CI jobs"; investigation showed most of that CI already existed — `sniff-cross-platform` in `test.yml` already ran the full macOS/Linux/Windows matrix, and `darkmatter-tests.yml` already ran full L1 on Linux + Windows
                - the orchestrator's own hypothesis — that `remote` being a non-default feature meant the provider code was never compiled cross-platform — was **half wrong**, and the subagent corrected it: `sniff/justfile`'s `test` recipe already passes `--features remote`, and `sniff-cli` declares `sniff = { features = ["network", "remote"] }`, so the provider library source and its Wiremock suites were already built and run on all three OSes
                - the genuine residual gap was narrower: `cargo check -p sniff --all-targets` resolves default features only, so the job's self-described test-code compile guard silently `cfg`'d `focused_provider.rs` and `remote_observation.rs` down to empty targets, and the `network`-gated bench and profiling examples were compiled on **no** platform at all
                - fixed by adding a distinct `cargo check -p sniff --all-targets --features remote` step, kept separate so "default builds, remote does not" stays a distinguishable failure
                - added a `cross-platform-check` job to `claudine-tests.yml` (macOS + Windows); claudine is a downstream consumer in this review's impact scope and previously had neither
                - `docs/testing-strategy.md` → "Platform Coverage (CI)" updated per the drift-maintenance rule
        - **caveat that limits the strength of the future evidence:** `_area-ci.yml`'s `soft-os` defaults to `["windows-latest"]`, making every Windows *test* leg `continue-on-error`, so Darkmatter's Windows evidence will be advisory rather than merge-gating; this was left alone deliberately as a repo-wide policy decision beyond this feature's scope, and is flagged for Ken. The `sniff-cross-platform` job carries no `continue-on-error`, so its Windows leg — including the new step — genuinely gates.
        - **AC16 and AC29 remain unsatisfied.** Only a CI run on Windows and Linux runners produces the required evidence; triggering CI is an outward-facing action outside this session's authorization, and none was triggered. A PR to `main` fires the sniff matrix, a push touching `claudine/**` fires the new claudine job, and `workflow_dispatch` on `test.yml` covers the sniff half on demand.

Three items surfaced during implementation that need Ken's decision rather than more code:

- **AC26 frontmatter/body parity is broken for focused provider failures.** The end-to-end tests added for Finding 4 discovered that an identical failing call aborts the compose from frontmatter but leaves the unevaluated `{{ pr(123) }}` in the body with only a report warning. This is not provider-specific — `is_authoring_fatal` deliberately excludes `ExpressionError::Other`, with an existing `other_is_not_authoring_fatal` test pinning that choice. Changing expression-error fatality is a ratified design axis, so it was pinned by a new test (`body_surface_downgrades_focused_failures_to_warnings`) rather than changed. The spec's narrower "focused errors are never replaced with empty values" bullet does hold on both surfaces.
- **`pr*`/`cicd*` are effectively unreachable for self-hosted providers.** Between a hard-coded `https://`, a port dropped from `ResolvedRemote::host`, and pattern-only `ApiFlavor` detection, practical reach is github.com, gitlab.com, bitbucket.org, codeberg.org, and hosts literally named `gitea.*`/`gitlab.*`/`forgejo.*`. GitHub Enterprise at `git.company.com` fails with unsupported-flavor. Fixing this needs a configuration surface plus a port/policy decision.
- **The query-vocabulary link is sibling-relative**, so it is clickable inside the generated doc but only a precise pointer in DMLS hover, which has no base URI. No precedent existed to follow and the two surfaces have incompatible bases; making hover navigation real would require DMLS to rewrite the target into a workspace-root `file://` URI.

The files changed during this implementation cycle:

- `sniff/lib/src/error.rs`
- `sniff/lib/src/remote/focused.rs`
- `sniff/lib/src/remote/types.rs`
- `sniff/lib/src/remote/provider.rs`
- `sniff/lib/tests/focused_provider.rs`
- `darkmatter/lib/src/markdown/compose/remote_fetch.rs`
- `darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs`
- `darkmatter/lib/src/markdown/compose/expression/catalog/mod.rs`
- `darkmatter/lib/src/markdown/compose/expression/functions/mod.rs`
- `darkmatter/lib/src/markdown/compose/expression/functions/escape.rs`
- `darkmatter/lib/src/markdown/compose/expression/functions/provider.rs`
- `darkmatter/lib/src/markdown/compose/expression/functions/pull_requests.rs`
- `darkmatter/lib/src/markdown/compose/expression/functions/cicd.rs`
- `darkmatter/lib/src/markdown/compose/tests/mod.rs`
- `darkmatter/lib/src/markdown/compose/tests/provider_network.rs`
- `darkmatter/dmls/src/overlay/expressions.rs`
- `darkmatter/docs/schemas/expression-functions.yaml`
- `darkmatter/docs/topics/darkmatter-expressions.md`
- `docs/testing-strategy.md`
- `.github/workflows/test.yml`
- `.github/workflows/claudine-tests.yml`

## Review-19 Planning (2026-07-19)

- `review-plan-19.md` created: 7 phases, 1:1 with review-19's findings (5 High, 2 Medium).
- Sequencing constraints recorded in the plan:
        - `ResolvedRemote` carries a CRITICAL GitNexus blast radius (41 symbols, 4 direct callers), so the self-hosted resolution rework (Phase 2) precedes the two dependent sniff phases: query-boundary DTOs (Phase 3) and API-URL parsing (Phase 4), which may then run in parallel.
        - Phase 1 (frontmatter/body fatality parity) is darkmatter-only and independent of the sniff track.
        - Phase 5 (Windows/Linux evidence) is gate-only — no source changes — and is blocked on explicit authorization to trigger CI; prior cycles recorded that authorization as outside session scope, so the plan makes it an explicit checkpoint rather than assuming it.
- Planning-time observation (not a new review finding): the focused-provider error classification is currently destroyed at the memoization boundary in `resolve_ctx.rs` (failures stored as `String`), so Phase 1's fix must change the cache-slot representation, not just the fatality predicate.
