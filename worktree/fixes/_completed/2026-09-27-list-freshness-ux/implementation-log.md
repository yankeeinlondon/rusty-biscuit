---
spec: /Volumes/coding/wt/rusty-biscuit/fix-wt-ux/worktree/fixes/2026-09-27-list-freshness-ux/spec.md
plan: worktree/fixes/2026-09-27-list-freshness-ux/plan.md
implemented_by: claude/opus
started_phase: 1
source_files_during_phase_1: []
docs_updated_during_phase_1:
    - worktree/fixes/2026-09-27-list-freshness-ux/plan.md
docs_created_during_phase_1:
    - worktree/fixes/2026-09-27-list-freshness-ux/implementation-log.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - schematic/definitions/src/bitbucket/mod.rs
    - schematic/definitions/src/bitbucket/types/mod.rs
    - schematic/definitions/src/bitbucket/types/branches.rs
    - schematic/definitions/src/gitea/mod.rs
    - schematic/definitions/src/gitea/types.rs
    - schematic/definitions/src/github/mod.rs
    - schematic/definitions/src/github/types/releases.rs
    - schematic/definitions/src/gitlab/mod.rs
    - schematic/definitions/src/gitlab/types.rs
    - schematic/definitions/src/gitlab/types/branches.rs
    - schematic/definitions/src/gitlab/endpoints/mod.rs
    - schematic/definitions/src/gitlab/endpoints/branches.rs
    - schematic/definitions/src/lib.rs
    - schematic/definitions/src/prelude.rs
    - schematic/schema/Cargo.lock
    - schematic/schema/src/bitbucket/mod.rs
    - schematic/schema/src/bitbucket/requests.rs
    - schematic/schema/src/gitea/mod.rs
    - schematic/schema/src/gitea/requests.rs
    - schematic/schema/src/github/mod.rs
    - schematic/schema/src/github/requests.rs
    - schematic/schema/src/gitlab/mod.rs
    - schematic/schema/src/gitlab/requests.rs
    - schematic/openapi/bitbucket.json
    - schematic/openapi/gitea.json
    - schematic/openapi/github.json
    - schematic/openapi/gitlab.json
    - schematic/postman/bitbucket.postman_collection.json
    - schematic/postman/gitea.postman_collection.json
    - schematic/postman/github.postman_collection.json
    - schematic/postman/gitlab.postman_collection.json
    - sniff/lib/src/credentials.rs
    - sniff/lib/src/filesystem/git/commit_links.rs
    - sniff/lib/src/filesystem/git/mod.rs
    - sniff/lib/src/filesystem/mod.rs
    - sniff/lib/src/remote/blocking.rs
    - sniff/lib/src/remote/focused.rs
    - sniff/lib/tests/l1/main.rs
    - sniff/lib/tests/l1/branch_head.rs
    - sniff/lib/tests/l1/open_pull_requests.rs
    - sniff/lib/tests/l1/pr_for_branch.rs
    - biscuit-terminal/lib/src/components/spinner.rs
    - biscuit-terminal/lib/src/components/mod.rs
    - biscuit-terminal/lib/src/prelude.rs
    - worktree/lib/src/api_preference.rs
    - worktree/lib/src/error.rs
    - worktree/lib/src/lib.rs
    - worktree/lib/src/pull_requests.rs
    - worktree/lib/src/remote_head.rs
    - worktree/cli/src/commands/list.rs
    - worktree/cli/src/commands/list/tests.rs
    - worktree/cli/src/commands/refresh_worker.rs
    - worktree/cli/tests/list_remote_head.rs
    - worktree/cli/tests/perf_support/mod.rs
docs_updated_during_phase_2:
    - schematic/README.md
    - schematic/definitions/README.md
    - sniff/lib/README.md
    - sniff/lib/CHANGELOG.md
    - biscuit-terminal/README.md
    - biscuit-terminal/docs/components/index.md
    - worktree/fixes/2026-09-27-list-freshness-ux/plan.md
    - worktree/fixes/2026-09-27-list-freshness-ux/implementation-log.md
docs_created_during_phase_2:
    - biscuit-terminal/docs/components/spinner.md
skills_files_updated_during_phase_2:
    - .claude/skills/biscuit-terminal/components.md
    - .claude/skills/sniff/SKILL.md
    - .claude/skills/sniff/architecture.md
    - .claude/skills/sniff/remote-and-repository.md
    - .claude/skills/worktree/SKILL.md
source_files_during_phase_3:
    - worktree/lib/src/lib.rs
    - worktree/lib/src/live_remote.rs
    - worktree/lib/src/remote_update.rs
    - worktree/lib/src/remote_update/tests.rs
    - worktree/lib/src/remote_head.rs
    - worktree/lib/src/pull_requests.rs
    - worktree/lib/src/api_preference.rs
    - worktree/cli/src/args.rs
    - worktree/cli/src/main.rs
    - worktree/cli/src/commands/refresh_worker.rs
    - worktree/cli/tests/perf_support/mod.rs
    - worktree/cli/tests/list_prs.rs
    - worktree/cli/tests/list_remote_head.rs
    - worktree/cli/tests/level2_list_verbose.rs
docs_updated_during_phase_3:
    - worktree/fixes/2026-09-27-list-freshness-ux/plan.md
    - worktree/fixes/2026-09-27-list-freshness-ux/implementation-log.md
    - worktree/fixes/2026-09-27-list-freshness-ux/spec.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/worktree/SKILL.md
source_files_during_phase_4:
    - worktree/lib/src/fast_forward.rs
    - worktree/lib/src/lib.rs
    - worktree/lib/src/error.rs
    - worktree/lib/src/pull_requests.rs
    - worktree/lib/src/worktree.rs
    - worktree/cli/src/args.rs
    - worktree/cli/src/main.rs
    - worktree/cli/src/commands/mod.rs
    - worktree/cli/src/commands/list.rs
    - worktree/cli/src/commands/list/tests.rs
    - worktree/cli/src/commands/list/wait.rs
    - worktree/cli/src/commands/list/wait/tests.rs
    - worktree/cli/src/commands/list_table.rs
    - worktree/cli/src/commands/refresh_worker.rs
    - worktree/cli/tests/list_table.rs
    - worktree/cli/tests/list_flags.rs
    - worktree/cli/tests/list_remote_head.rs
    - worktree/cli/tests/list_prs.rs
    - worktree/cli/tests/perf_pr_request.rs
    - worktree/cli/tests/perf_support/mod.rs
    - worktree/cli/tests/remote_fixture/mod.rs
    - worktree/cli/tests/level2_list_verbose.rs
    - worktree/cli/tests/snapshots/list_flags__global_flag_completions.snap
    - worktree/cli/tests/snapshots/list_table__caption_every_row_in_every_comparison_state.snap
    - worktree/cli/tests/snapshots/list_table__caption_reasons.snap
    - worktree/cli/tests/snapshots/list_table__caption_age_boundaries_and_future_timestamps.snap
    - worktree/cli/tests/snapshots/list_table__caption_missing_refs_and_failed_comparison.snap
    - worktree/cli/tests/snapshots/list_table__caption_trunk_default_branch.snap
    - worktree/cli/tests/snapshots/list_table__credential_lines_every_condition_for_every_provider.snap
    - worktree/cli/tests/snapshots/list_table__closing_notes.snap
    - worktree/cli/tests/snapshots/list_table__output_order.snap
    - worktree/cli/tests/snapshots/list_table__the_spec_example_renders_as_ruled.snap
    - worktree/cli/tests/snapshots/list_table__the_table_at_99_columns_shows_no_counts.snap
    - worktree/cli/tests/snapshots/list_table__the_table_at_100_columns_shows_counts.snap
    - worktree/cli/tests/snapshots/list_table__caption_observation_rows.snap (deleted)
    - worktree/cli/tests/snapshots/list_table__caption_comparison_states.snap (deleted)
    - worktree/cli/tests/snapshots/list_table__caption_fresh_and_stale_answers.snap (deleted)
    - worktree/cli/tests/snapshots/list_table__caption_age_boundaries_and_future_answers.snap (deleted)
docs_updated_during_phase_4:
    - worktree/fixes/2026-09-27-list-freshness-ux/plan.md
    - worktree/fixes/2026-09-27-list-freshness-ux/implementation-log.md
    - worktree/fixes/2026-09-27-list-freshness-ux/spec.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/worktree/SKILL.md
    - .claude/skills/os/windows.md
source_files_during_phase_5:
    - worktree/cli/tests/perf_command_sla.rs
    - worktree/cli/tests/perf_pr_request.rs
    - worktree/cli/tests/perf_support/mod.rs
    - worktree/cli/tests/level2_list_verbose.rs
    - worktree/lib/src/remote_head.rs
    - worktree/lib/src/default_target.rs
    - .config/nextest.toml
docs_updated_during_phase_5:
    - worktree/docs/cli/list.md
    - worktree/README.md
    - worktree/docs/performance-testing.md
    - worktree/fixes/2026-09-27-list-freshness-ux/plan.md
    - worktree/fixes/2026-09-27-list-freshness-ux/implementation-log.md
    - worktree/fixes/2026-09-27-list-freshness-ux/spec.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/worktree/SKILL.md
source_code:
    - schematic/definitions/src/bitbucket/mod.rs
    - schematic/definitions/src/bitbucket/types/mod.rs
    - schematic/definitions/src/bitbucket/types/branches.rs
    - schematic/definitions/src/gitea/mod.rs
    - schematic/definitions/src/gitea/types.rs
    - schematic/definitions/src/github/mod.rs
    - schematic/definitions/src/github/types/releases.rs
    - schematic/definitions/src/gitlab/mod.rs
    - schematic/definitions/src/gitlab/types.rs
    - schematic/definitions/src/gitlab/types/branches.rs
    - schematic/definitions/src/gitlab/endpoints/mod.rs
    - schematic/definitions/src/gitlab/endpoints/branches.rs
    - schematic/definitions/src/lib.rs
    - schematic/definitions/src/prelude.rs
    - schematic/schema/Cargo.lock
    - schematic/schema/src/bitbucket/mod.rs
    - schematic/schema/src/bitbucket/requests.rs
    - schematic/schema/src/gitea/mod.rs
    - schematic/schema/src/gitea/requests.rs
    - schematic/schema/src/github/mod.rs
    - schematic/schema/src/github/requests.rs
    - schematic/schema/src/gitlab/mod.rs
    - schematic/schema/src/gitlab/requests.rs
    - schematic/openapi/bitbucket.json
    - schematic/openapi/gitea.json
    - schematic/openapi/github.json
    - schematic/openapi/gitlab.json
    - schematic/postman/bitbucket.postman_collection.json
    - schematic/postman/gitea.postman_collection.json
    - schematic/postman/github.postman_collection.json
    - schematic/postman/gitlab.postman_collection.json
    - sniff/lib/src/credentials.rs
    - sniff/lib/src/filesystem/git/commit_links.rs
    - sniff/lib/src/filesystem/git/mod.rs
    - sniff/lib/src/filesystem/mod.rs
    - sniff/lib/src/remote/blocking.rs
    - sniff/lib/src/remote/focused.rs
    - sniff/lib/tests/l1/main.rs
    - sniff/lib/tests/l1/branch_head.rs
    - sniff/lib/tests/l1/open_pull_requests.rs
    - sniff/lib/tests/l1/pr_for_branch.rs
    - biscuit-terminal/lib/src/components/spinner.rs
    - biscuit-terminal/lib/src/components/mod.rs
    - biscuit-terminal/lib/src/prelude.rs
    - worktree/lib/src/api_preference.rs
    - worktree/lib/src/error.rs
    - worktree/lib/src/lib.rs
    - worktree/lib/src/pull_requests.rs
    - worktree/lib/src/remote_head.rs
    - worktree/cli/src/commands/list.rs
    - worktree/cli/src/commands/list/tests.rs
    - worktree/cli/src/commands/refresh_worker.rs
    - worktree/cli/tests/list_remote_head.rs
    - worktree/cli/tests/perf_support/mod.rs
    - worktree/lib/src/live_remote.rs
    - worktree/lib/src/remote_update.rs
    - worktree/lib/src/remote_update/tests.rs
    - worktree/cli/src/args.rs
    - worktree/cli/src/main.rs
    - worktree/cli/tests/list_prs.rs
    - worktree/cli/tests/level2_list_verbose.rs
    - worktree/lib/src/fast_forward.rs
    - worktree/lib/src/worktree.rs
    - worktree/cli/src/commands/mod.rs
    - worktree/cli/src/commands/list/wait.rs
    - worktree/cli/src/commands/list/wait/tests.rs
    - worktree/cli/src/commands/list_table.rs
    - worktree/cli/tests/list_table.rs
    - worktree/cli/tests/list_flags.rs
    - worktree/cli/tests/perf_pr_request.rs
    - worktree/cli/tests/remote_fixture/mod.rs
    - worktree/cli/tests/snapshots/list_flags__global_flag_completions.snap
    - worktree/cli/tests/snapshots/list_table__caption_every_row_in_every_comparison_state.snap
    - worktree/cli/tests/snapshots/list_table__caption_reasons.snap
    - worktree/cli/tests/snapshots/list_table__caption_age_boundaries_and_future_timestamps.snap
    - worktree/cli/tests/snapshots/list_table__caption_missing_refs_and_failed_comparison.snap
    - worktree/cli/tests/snapshots/list_table__caption_trunk_default_branch.snap
    - worktree/cli/tests/snapshots/list_table__credential_lines_every_condition_for_every_provider.snap
    - worktree/cli/tests/snapshots/list_table__closing_notes.snap
    - worktree/cli/tests/snapshots/list_table__output_order.snap
    - worktree/cli/tests/snapshots/list_table__the_spec_example_renders_as_ruled.snap
    - worktree/cli/tests/snapshots/list_table__the_table_at_99_columns_shows_no_counts.snap
    - worktree/cli/tests/snapshots/list_table__the_table_at_100_columns_shows_counts.snap
    - worktree/cli/tests/snapshots/list_table__caption_observation_rows.snap
    - worktree/cli/tests/snapshots/list_table__caption_comparison_states.snap
    - worktree/cli/tests/snapshots/list_table__caption_fresh_and_stale_answers.snap
    - worktree/cli/tests/snapshots/list_table__caption_age_boundaries_and_future_answers.snap
    - worktree/cli/tests/perf_command_sla.rs
    - worktree/lib/src/default_target.rs
    - .config/nextest.toml
documentation:
    - worktree/fixes/2026-09-27-list-freshness-ux/plan.md
    - worktree/fixes/2026-09-27-list-freshness-ux/implementation-log.md
    - schematic/README.md
    - schematic/definitions/README.md
    - sniff/lib/README.md
    - sniff/lib/CHANGELOG.md
    - biscuit-terminal/README.md
    - biscuit-terminal/docs/components/index.md
    - biscuit-terminal/docs/components/spinner.md
    - worktree/fixes/2026-09-27-list-freshness-ux/spec.md
    - worktree/docs/cli/list.md
    - worktree/README.md
    - worktree/docs/performance-testing.md
completed_phase: 5
implemented: true
packages:
    - schematic-definitions
    - schematic-schema
    - sniff
    - biscuit-terminal
    - worktree
    - worktree-cli
implementation_1: "2026-09-27T12:36:00-07:00"
implementation_2: "2026-09-27T16:23:23-07:00"
---

# Implementation Log for 2026-09-27-list-freshness-ux (5 phases)

## Phase 1

Phase 1 is rulings, spikes, and a baseline. It changes no source code.

- Rulings 1–20 in `plan.md` are accepted as written unless amended below.

### S3 — Fetch side-effect and platform audit (macOS, git 2.55.0)

Method: a scripted local bare `origin`, a `pusher` clone, and a `local` clone, with `HOME` isolated and `GIT_CONFIG_NOSYSTEM=1`. Between the snapshots the pusher advanced `main`, pushed tag `v2`, advanced `feature`, and deleted `old`. `FETCH_HEAD` was pre-seeded with a sentinel. Each case diffed `for-each-ref` and `FETCH_HEAD` around the spec's command, `git -c maintenance.auto=false -c gc.auto=0 fetch --no-write-fetch-head --no-tags origin +refs/heads/main:refs/remotes/origin/main`.

| Configuration | Refs changed by the spec argv | Verdict |
|---|---|---|
| defaults | `refs/remotes/origin/main` only (`origin/HEAD` is its symref) | ok |
| `fetch.prune=true`, `fetch.pruneTags=true` | `origin/main` only; `origin/old` **not** pruned (prune is limited to the command-line refspec's destination) | ok |
| `fetch.recurseSubmodules=true` + `submodule.recurse=true`, with a submodule | superproject: `origin/main` only; **but the fetch printed "Fetching submodule sub" and updated the submodule repository's `refs/remotes/origin/main`** | **side effect** |
| extra `remote.origin.fetch = +refs/heads/*:refs/remotes/mirror/*` | `origin/main` **and a new `refs/remotes/mirror/main`** (git's "opportunistic remote-tracking update" from the configured refspecs) | **side effect** |
| `core.logAllRefUpdates=false` | `origin/main` only | ok |

In every case `FETCH_HEAD` kept its sentinel, no tag arrived (`v2` absent), and `origin/feature` did not move.

Additional flags `--no-recurse-submodules --refmap=` re-run against the same two repositories:
- The submodule's refs were unchanged, and no submodule fetch happened.
- `refs/remotes/mirror/*` was not created.
- A remote rewind (`push -f HEAD~1:main`) updated `origin/main` as `(forced update)` through the leading `+`.

**Rule 10 decision (amended).** The fetch argv is:

```text
git -c maintenance.auto=false -c gc.auto=0 fetch --no-write-fetch-head --no-tags --no-recurse-submodules --refmap= origin +refs/heads/<default>:refs/remotes/origin/<default>
```

`--no-recurse-submodules` is the deviation Rule 10 anticipated. `--refmap=` is a second deviation that Rule 10 did not anticipate: without it, a user's extra `remote.origin.fetch` entry makes the fetch update refs other than `origin/<default>`, which breaks the spec's "touches only `refs/remotes/origin/<default>`" guarantee. `--refmap=` is passed as one argv element with an empty value (no shell is involved on any OS). Both flags only *narrow* the spec's command, so the spec's intent is kept. The spec text should be updated to match in Phase 5's docs pass.

**Minimum Git.** `--no-write-fetch-head` needs Git 2.29 (2020-10). `--refmap` needs 2.1 and `--no-recurse-submodules` is older still. The WSL2 leg is Ubuntu 24.04 (`.github/workflows/_wsl-ci.yml`), which ships Git 2.43, and the hosted `ubuntu-latest`, `macos-latest`, and `windows-latest` images carry current Git. No supported environment is below 2.29. An older Git would fail with `error: unknown option` → `check-failed`/`fetch-failed{other}`, which is truthful.

**`git reflog -1 --format=%ct refs/remotes/origin/<default>`:**

| State | stdout | exit |
|---|---|---|
| reflog present | Unix seconds, e.g. `1790526093` | 0 |
| reflog file absent (deleted, or `core.logAllRefUpdates=false` after clone) | empty | 0 |
| ref itself absent | empty (stderr `fatal: ambiguous argument …`) | 128 |

So `tracking_ref_changed_at` must treat **empty stdout with exit 0** as "no reflog" (→ `LastKnown::Never`), not as a parse error, and exit 128 as "no tracking ref".

**`git check-ref-format --branch`:** `feat/x` → prints `feat/x`, exit 0. `ma..in` and `-x` → `fatal: '…' is not a valid branch name`, exit 128.

**`LC_ALL=C` stderr samples** (with `GIT_TERMINAL_PROMPT=0`, `GCM_INTERACTIVE=never`, `-c credential.interactive=never`, and an empty `credential.helper` unless noted):

| Case | stderr (first line; URLs are never rendered) | exit |
|---|---|---|
| ref lock held (`origin/main.lock` exists; concurrent fetch) | `error: cannot lock ref 'refs/remotes/origin/main': Unable to create '…/main.lock': File exists.` then ` ! a..b  main -> origin/main  (unable to update local ref)` | 1 |
| HTTP 401, no credentials, **with** `credential.interactive=never` (what `run_noninteractive` passes) | `fatal: unable to get password from user` | 128 |
| HTTP 401, no credentials, without that flag | `fatal: could not read Username for '<url>': terminal prompts disabled` | 128 |
| HTTP 401, wrong credentials (URL userinfo or helper) | `fatal: Authentication failed for '<url>'` | 128 |
| HTTP 403 | `fatal: unable to access '<url>': The requested URL returned error: 403` | 128 |
| HTTP 404 | `fatal: repository '<url>' not found` | 128 |
| connection refused | `fatal: unable to access '<url>': Failed to connect to … Couldn't connect to server` | 128 |
| DNS failure | `fatal: unable to access '<url>': Could not resolve host: …` | 128 |
| SSH key refused (`BatchMode`; stub `GIT_SSH_COMMAND` emitting OpenSSH's text) | `git@github.com: Permission denied (publickey).` then `fatal: Could not read from remote repository.` | 128 |
| SSH unknown host key (stub) | `Host key verification failed.` then `fatal: Could not read from remote repository.` | 128 |
| deadline | git is killed, so there is no stderr. `run_noninteractive` returns its own `origin did not answer within N s` string | — |
| `ls-remote` HTTP 401 with `credential.interactive=never` | `fatal: unable to get password from user` (same as fetch) | 128 |

The SSH rows used a stub `GIT_SSH_COMMAND` that prints OpenSSH's own messages, because this session may not run `ssh`. Git's wrapping line (`Could not read from remote repository.`) is real.

**Classifier pattern list (Rule 10, amended):**
- `credentials`:
    - `Authentication failed for`
    - `could not read Username`
    - `could not read Password`
    - `unable to get password from user` (**added**; this is the text under `credential.interactive=never`, which every call passes)
    - `Permission denied (publickey`
    - `The requested URL returned error: 401`
- `timeout`: never pattern-matched from stderr. The deadline branch of `run_noninteractive` should return a typed error (for example `Err(GitFailure::Deadline)`, or a distinct variant beside the message) rather than a string the classifier re-parses. Phase 2/3 should make that change in `live_remote.rs`. The current `Result<String, String>` loses the distinction.
- `other`: everything else, including 403 (ambiguous between rate limiting and missing permissions), 404 (ambiguous, never absence), `Host key verification failed`, lock contention (handled by Rule 11's re-read, not by the classifier), refused connections, and DNS failures.

### S1 — Provider branch-head endpoints and path encoding

I sent about 45 anonymous `GET`s to public repositories (GitHub `cli/cli`, GitLab `gitlab-org/gitlab-runner`, Codeberg `forgejo/forgejo`, Bitbucket `atlassian/aui`). Each probe used the plain default branch and a branch with `/` sent as `%2F`.

| Provider | Endpoint to add | Plain | `%2F` | Raw `/` | SHA field | Missing or private repo (anonymous) | Bad token | Rate limit |
|---|---|---|---|---|---|---|---|---|
| GitHub | `GetBranchReference` `GET /repos/{owner}/{repo}/git/ref/heads/{branch}` → `GitRef` | 200 | 200 | 200 | `object.sha` | 404 `Not Found` (a missing branch looks the same) | 401 `Bad credentials` | `x-ratelimit-{limit,remaining,used,reset}` (anonymous 60/h). Exceeded: **403 or 429** with `x-ratelimit-remaining: 0` |
| GitLab | `GetBranch` `GET /projects/{id}/repository/branches/{branch}` | 200 | 200 | **404** | `commit.id` | 404 `Project Not Found` (a missing branch says `Branch Not Found`) | 401 | `ratelimit-*`. Exceeded: 429 + `retry-after` |
| Gitea/Forgejo | `GetBranch` `GET /repos/{owner}/{repo}/branches/{branch}` | 200 | 200 | 200 | `commit.id` | 404 (a missing branch looks the same) | 401 | `ratelimit`/`ratelimit-policy` (IETF draft). Exceeded: 429; a proxy may return 403, which is ambiguous |
| Bitbucket | `GetBranch` `GET /2.0/repositories/{workspace}/{repo_slug}/refs/branches/{name}` | 200 | 200 | 200 | `target.hash` | 404 "may not have access … or no longer exists" | 401 | `x-ratelimit-*` (reset is seconds-until). Exceeded: 429 |

Verdict:
- **Every provider accepts `%2F`, and GitLab *requires* it.** Declare the branch as an ordinary encoded `{branch}` path parameter (not `{+branch}`). No URL is hand-rolled. `schematic/gen/src/codegen/request_structs/shared.rs:131-146` confirms that every non-`{+…}` path parameter goes through `urlencoding::encode`.
- **GitHub must use the singular `/git/ref/`.** The plural `/git/refs/heads/{b}` prefix-matches and returns a JSON *array* for a name like `af` when `af/*` branches exist. `GetTagReference` (`github/mod.rs:375-385`) is the template. The id lists in `github/mod.rs` tests (around lines 744 and 823) must gain the new id.
- **Gitea should use `/branches/{branch}`,** not its `git/refs/{ref}` array endpoint. It returns one object.
- **A 404 is never absence** on any provider. Anonymous private and missing repositories are identical, which confirms Rule 13's `NotFoundOrNotPermitted`.
- **Rate limit.** A 429 is unambiguous everywhere. GitHub's 403 is `RateLimited` only with `x-ratelimit-remaining: 0` or a rate-limit body. Otherwise a 403 is `CredentialsInsufficient` when a token was sent and the body says so, and `NotFoundOrNotPermitted` in any other case. This matches Rule 13.
- A bad token yields 401 on all four → `CredentialsRejected`.

Rule 13 is confirmed. No ruling is amended by S1. Full notes are kept outside the repository (`/tmp/lfux/s1.md`, not committed).

### S2 — Credential key metadata and identity parsing

Findings (file:line references from a read-only survey):

- **Schematic never reports which variable it used.** Generated clients resolve with `find_map(|n| std::env::var(n).ok())` and drop the name (`schematic/gen/src/codegen/client/helpers.rs:126-175`, `api_struct/request_method.rs:62-76`).
- **`remote::blocking` does not use schematic at all.** It goes through `FocusedProviderClient` (`sniff/lib/src/remote/focused.rs`), which reads tokens with sniff's own `credentials::provider_token` (`sniff/lib/src/credentials.rs:52-67`, `pub(crate)`, two call sites). That function also loses the matched name: it returns `names.first()` whatever variable was set (line 65). Its order and variable set **differ from schematic's `env_auth`**:

| Provider | Focused client (what `branch_head` will send) | Schematic `env_auth` |
|---|---|---|
| GitHub | `GH_TOKEN`, `GITHUB_TOKEN` | `GITHUB_TOKEN`, `GH_TOKEN` |
| GitLab | `GITLAB_TOKEN`, `GITLAB_PRIVATE_TOKEN` | same |
| Gitea/Forgejo | `GITEA_TOKEN`, `FORGEJO_TOKEN`, `CODEBERG_TOKEN` | `GITEA_TOKEN` (Codeberg: `CODEBERG_TOKEN`, `GITEA_TOKEN`) |
| Bitbucket | `BITBUCKET_TOKEN` (Bearer) | `BITBUCKET_USERNAME` + `BITBUCKET_APP_PASSWORD` (Basic) |

- **Today's status mapping on the blocking path** (`focused.rs:836-866` → `classify`, `blocking.rs:292-319`) is identical for every provider:
    - 401 with no token → `MissingCredentials` → `Auth`
    - 401 with a token → `InvalidCredentials` → `Auth`
    - **403 → `RemoteForbidden` → `Auth`, with no rate-limit inspection**
    - 404 on a list endpoint → `NotFoundOrNotPermitted`
    - 429 → `RateLimited`
    - There is **no anonymous retry** on this path. The schematic-backed `github.rs`/`gitea.rs`/`gitlab.rs`/`bitbucket.rs` do retry anonymously (only when no credential is set, in `list_pull_requests`), but none of that is on the blocking path.
- `PrUnavailable` (`blocking.rs:77-106`) is `#[non_exhaustive]`, with variants `Timeout`, `Network`, `Auth { message }`, `NotFoundOrNotPermitted`, `RateLimited`, `Unsupported`, and `Other`. `classify`, `client_for_url`, and `run_with_deadline` are private. `FocusedProviderClient` and its constructors are public.
- `Auth` sites: `blocking.rs:92` (definition), `:181` (a doc comment), `:300` (the **only constructor**, in `classify`), and `:431` (a unit test). L1 tests: `sniff/lib/tests/l1/pr_for_branch.rs:403` and `sniff/lib/tests/l1/open_pull_requests.rs:327`. Each covers (401, no token), (401, token), and (403, token) on every provider. **worktree never matches variants.** It only calls `.to_string()` (`worktree/lib/src/pull_requests.rs:69-70`, `remove/safety.rs:132-150`).
- **Identity parsing.** `parse_remote_identity` (`sniff/lib/src/filesystem/git/commit_links.rs:59-100`) is **`pub(crate)`** and cannot be reached from worktree. It returns `(Option<RemoteEndpoint { scheme, host, port }>, namespace, repository)`.
    - For HTTPS it drops userinfo, lowercases the host, and gives a port only when it is not the default.
    - For SCP-style and `ssh://` it keeps the host's case as typed. `ssh://…:22` keeps `Some(22)`, while the SCP form gives `None`.
    - The public `remote::parse_remote_url` strips ports and hides the host, so it does not fit.
- **Features.** `worktree/lib/Cargo.toml:24` already enables sniff `remote` (→ `network`). No Cargo change is needed.
- Drift noticed and left alone (sniff docs, out of scope):
    - `sniff/lib/src/remote/bitbucket.rs:162` says every 403 is `RateLimited`, but the code does that only for a rate-limit body.
    - `github.rs:26,42` describe schematic's token order.

**Rule 13 amended (key name).** sniff resolves the key name **in `credentials::provider_token`'s order** (the variables the focused client actually sends), not in schematic's `env_auth` order. Copying `env_auth` would get GitHub's order and Bitbucket's variable wrong. Implementation: change `provider_token` to return the name that matched (and `credential_env` lists the same candidates in the same order).

**Rule 13 addendum (403).** The blocking path maps every 403 to `Auth` today. Phase 2 must make the 403 split (`RateLimited` versus `CredentialsInsufficient` versus `NotFoundOrNotPermitted`) inside `focused.rs`'s error mapping, where the response headers and body are still available (GitHub: `x-ratelimit-remaining: 0` or a rate-limit body; see S1). `classify` receives only a `SniffError`.

**Rule 3 amended (identity).** Add to sniff a small public `remote_identity(url) -> Option<RemoteIdentity { scheme, host, port: Option<u16>, path }>` wrapping `parse_remote_identity`. It carries the raw facts: host ASCII-lowercased with no userinfo, `path` = `namespace/repository` with case kept, `.git` and slashes trimmed. Re-export it beside `repository_link`. Rule 3's *policy* (the effective port: 443 for HTTPS, 443 for SSH to a known provider host via `GitHostingProvider::from_url`, otherwise the explicit port or the scheme default) stays in `worktree::api_preference::RepoIdentity::from_origin`. So sniff gains no worktree-specific normalization, and `ssh://host:22/o/r` and `git@host:o/r` agree after worktree's step.

### S4 — Affected-test inventory (work list for Phase 4 Wave 3)

Counts:
- 40 test functions run `wt list` or the worker with an origin: 25 process-level and 15 unit.
- 3 sniff tests assert `PrUnavailable::Auth`.
- `worktree/cli/Cargo.toml` has no `autotests = false`, so each `tests/*.rs` is its own target. The six `level2_*` files are `[[test]]` entries with `required-features = ["terminal-tests"]`.

Ruling proposals arising from S4 (not changes to the numbered rules, but gaps they left open):
- **`--attempt` is optional on `wt internal-refresh`.** Without it the worker generates its own token (same generator) and runs the flow unchanged. Three direct callers pass none today (`perf_support::refresh_worker_via_gitea`, `list_prs::the_worker_command_prints_nothing…`, `list_remote_head::Fixture::refresh`), and nothing but `wt list` needs to follow a specific attempt. `--force` stays opt-in.
- **Test isolation for the new branch-head request.** It goes through the same `ProxyStub`/`FakeGitea` as PR requests, so every request-count assertion must count by path (`FakeGitea` records request lines), not in total.
- **The L2 fixture (`level2_list_verbose::DesignFixture`) does not isolate the user's or system's git config.** With the new `ls-remote` fallback, a user `insteadOf` rule could send it to the real github.com. Phase 4/5 must add `GIT_CONFIG_GLOBAL`/`GIT_CONFIG_NOSYSTEM` isolation and `protocol.https.allow=never` there before the flow lands.
#### Section A: tests that run `wt list` / bare `wt` / `internal-refresh` with an origin

Legend for **Asserts**: **NW** = asserts no worker launched or no lock taken; **RC** = request or connection count; **CAP** = specific caption or age-line text; **T** = timing bound; **STORE** = byte-equality or absence of a store.

##### `worktree/cli/tests/list_prs.rs` (12 of 13 tests affected; all `#[serial]`)

| Test | Origin kind | Invokes | New wait | Asserts | Notes |
|---|---|---|---|---|---|
| `a_fresh_pr_store_makes_no_request_and_shows_its_badges` | github + **hanging** ProxyStub | `wt list --perf` | **stalls 3 s** | **NW** (`refresh_workers` empty), **RC** (`connections()==0`), CAP (no "PRs as of") | Both NW and RC break. No cleanup, so the held worker outlives the test |
| `a_stale_store_shows_its_badges_at_once_and_a_detached_worker_makes_the_request` | github + **hanging** | `wt list --perf` ×2, worker | **stalls 3 s** per list | **RC** (`connections()==1`, then 2), lock `Contended`, CAP "PRs as of 12 min ago", STORE (PR) | Connection counts double because branch-head and PR requests share the proxy |
| `a_changed_origin_hides_the_stored_badges_and_starts_no_worker` | github (set-url to another repo) + refusing | `wt list --perf` | fast failure | **NW** (`!pr_lock_path.exists()`), CAP (no PR badges) | NW breaks: the worker's PR half takes the PR lock |
| `the_worker_command_prints_nothing_and_ignores_anything_but_a_main_checkout` | github + refusing | `internal-refresh <dir>` ×4 directly | fast failure | locks exist or not, **STORE** (PR and head bytes unchanged), empty stdout/stderr | Head-store byte equality breaks (attempt record). Needs a `--attempt` decision |
| `with_the_network_down_the_table_shows_without_badges_and_nothing_is_stored` | github + refusing | `wt list --perf` ×2 | fast failure | CAP (no badges, then "PRs as of 5 min ago"), **STORE** (no PR store) | Probably still passes, but the first run now also launches a worker. The "worker never ran" loop passes trivially |
| `a_detached_workers_answer_replaces_the_stale_one_on_the_next_list` | gitea + FakeGitea **held** | `wt list --perf` from a linked worktree, twice | **stalls 3 s** (first list) | **RC** (`requests()==1` twice), `wait_for_waiting(1)`, worker cwd, **NW** after the fresh answer, CAP | The second list launches a worker again, so NW and RC break. The released `Open` reply is fed to the branch-head request too (parse failure) |
| `concurrent_lists_and_workers_make_one_request_and_a_fresh_answer_stops_the_next` | gitea **held** | 4 concurrent `wt list`, 3 direct workers | **stalls 3 s** (all 4 lists) | **RC** (`requests()==1` ×2), worker count ==1, STORE | Becomes about 2 requests (PR + branch head) |
| `a_killed_worker_releases_its_lock_and_a_later_worker_refreshes` | gitea **held**, then released | direct worker (killed), direct worker, `wt list` | worker held; the final list is fast | **RC** (`requests()==2`), lock release, STORE, CAP | Becomes about 4 or more requests |
| `a_failed_or_unauthorized_refresh_keeps_the_stored_answer` | gitea, FakeGitea `Status(500)` / `Status(401)` | direct worker + `wt list` per status | fast failure | RC (`>=1`, tolerant), STORE (PR), CAP "PRs as of 12 min ago" | Probably survives; 401 on branch head goes to the `ls-remote` fallback, which is blocked |
| `an_origin_change_during_a_workers_request_discards_its_answer` | gitea **held** | direct worker, then `wt list` | worker held; list fast | **RC** (`requests()==1`), STORE, CAP (no badges) | `wait_for_waiting(1)` may fire on the branch-head request rather than the PR request, and RC breaks |
| `an_origin_change_during_a_foreground_request_shows_no_badges_from_the_old_origin` | gitea (then `gitea.example.invalid`, still proxied to FakeGitea), FakeGitea `Open`, not held, with `before_reply` | `wt list --perf` ×2 | fast (no hold) | **RC** (`requests()==2`), **NW** ("a PR miss alone starts no worker"), **STORE** (`!pr_store().exists()`), CAP | Breaks three ways: worker requests add to RC, NW is false, and the worker's PR half can **publish** the PR store on a miss |
| `a_missing_or_stale_live_head_never_holds_up_the_listing` | **HoldingOrigin** (×2 iterations: head missing, head 12 min stale) | `wt list --perf` via `wt_command_direct` | **stalls 3 s** per iteration | lock `Contended`, one worker, **CAP** ("Remote state has not been verified." / "differs from the remote head observed 12 min ago"), RC (exactly one `GET /r.git/info/refs?service=git-upload-pack`), **STORE** (head unchanged) | Its premise ("never holds up") is inverted and needs rewriting as the "still checking at 3 s" row. RC probably survives because a non-provider origin gets no API call |
| `the_worker_command_is_hidden_from_help_and_completion` | none | `wt --help`, completion | none | — | Unaffected (listed for completeness) |

##### `worktree/cli/tests/list_remote_head.rs` (4 of 5 affected; local bare origin; all `#[serial]`)

The fixture isolates the user and system git config. `Fixture::refresh()` runs `wt internal-refresh <main>` directly and waits for it. Drop runs `wait_for_refresh_workers(main, 0, 20 s)`.

| Test | Origin kind | Invokes | New wait | Asserts | Notes |
|---|---|---|---|---|---|
| `a_push_elsewhere_reads_as_a_difference_until_the_fetch_then_as_behind_and_matched` | file-path bare | `internal-refresh`, `wt list` ×2 | fast answer, **real fetch** | **CAP** (the old two-sentence caption, "differs … run git fetch origin"), **STORE** (head bytes unchanged by list), **NW** (`refresh_workers` empty) | The first list now fetches, giving the "updated from origin just now" wording. Every assertion needs rewriting |
| `a_fetch_newer_than_the_observation_is_a_difference_never_a_move` | file-path bare | `internal-refresh`, `wt list` | fast answer | **CAP** ("differs from the remote head observed"), **STORE** (`sha == observed`, the old observation kept) | The list re-checks and replaces the observation |
| `a_deleted_then_recreated_remote_branch_is_reported_absent_then_present` | file-path bare | `internal-refresh` ×2, `wt list` ×3 | fast answer | **CAP** ×3 (absent, then "No local tracking ref…absent", then "…present") | The caption table's wording changes, and each list re-checks. After re-creation, a list could fetch and recreate `origin/main` |
| `the_main_checkout_and_a_linked_worktree_share_one_live_head_store` | file-path bare | `wt list` from linked, then from main and linked | fast answer | **CAP** ("Remote state has not been verified.", "matched the remote when checked"), **NW** (empty at the end) | The first list now shows a checked result. NW at the end is racy or breaks |
| `without_an_origin_leftover_tracking_refs_show_no_caption_and_start_no_worker` | **none** (removed) | `wt list` | none | NW, no head lock, no store | Remains a valid guard; unaffected |

##### `worktree/cli/tests/perf_pr_request.rs` (4 of 4 affected; all `#[serial]`; **timing**)

`FULL_COMMAND_BOUND` is 1 s and `PR_DEADLINE` is 300 ms. The spec retires the 1 s full-command contract.

| Test | Origin kind | Invokes | New wait | Asserts | Notes |
|---|---|---|---|---|---|
| `perf_list_meets_sla_with_the_network_down` | github + refusing | 15 × `wt list` (5 cold, 5 warm `--perf`, 5 full) | fast failure | **T** (cold `list gather` < 300 ms, warm < 120 ms, full < 1 s) | Full time grows by the worker spawn plus a fast failure; probably still under 1 s. `list gather` holds if it is still measured separately after the wait |
| `perf_list_meets_sla_when_the_pr_request_hits_its_deadline` | github + **hanging** | 10 × `wt list` | **stalls 3 s each** (about 30 s total) | **T** (full < 1 s, warm < 120 ms, `pr gather` ≥ 300 ms), RC (`>= runs`) | Full < 1 s breaks. No worker cleanup |
| `perf_list_meets_sla_with_a_stale_answer_and_a_blocked_refresh` | github + **hanging** | about 17 × `wt list` | **stalls 3 s each** | **T** (full < 1 s, stale `pr gather` < 300 ms), **RC** (`connections()==0` after the fresh-answer runs), STORE (stays stale) | RC == 0 breaks, because fresh answers now launch too. Full < 1 s breaks |
| `perf_remote_select_stays_under_the_deadline_with_a_blocked_live_head_refresh` | **HoldingOrigin** | 5 × `wt list --perf` | **stalls 3 s each** | **T** (`remote select` < 300 ms, best full < 1 s), a worker made the request | Premise inverted: `remote select` (or a new wait stage) now contains the wait |

##### `worktree/cli/tests/level2_list_verbose.rs` (5 of 7 affected; L2, tmux, `#[serial(level2_terminal)]`)

`DesignFixture` sets origin to `https://github.com/owner/repo.git` and seeds a PR store plus a fresh live head at `origin/main`. It sets `refs/remotes/origin/main` one commit ahead of `main`. The pane env unsets only `GH_TOKEN`/`GITHUB_TOKEN`; git config is not isolated. `wait_for_pane` has a 15 s cap.

| Test | Origin kind | New wait | Asserts | Notes |
|---|---|---|---|---|
| `level2_list_styles_follow_the_design_in_tmux` (2 panes) | github, proxy `127.0.0.1:9` (refused) | fast failure (see the SSH `insteadOf` caveat above) | **CAP** ("main is 1 commit behind local tracking ref origin/main." + "origin/main matched the remote when checked less than 1 min ago."), styles, no "PRs as of" | Caption wording changes (§4 check-failed row). A spinner in the TTY pane may briefly appear |
| `level2_list_width_flag_leaves_the_counts_in_tmux` | same | fast failure | cells and counts only | Probably survives |
| `level2_list_stale_pr_answer_shows_a_dim_age_line_in_tmux` | github + **hanging** ProxyStub | **stalls 3 s** (the pane waits for "PRs as of", within 15 s) | **RC** (`connections()==1`), lock release, head lock free, STORE (PR), CAP "PRs as of 12 min ago" | RC becomes 2. The §6 refresh-hint line would follow the age line; the test asserts the age line directly follows the legend, so check its adjacency |
| `level2_list_hides_the_counts_in_a_99_column_pane` | github, refused | fast failure | cells, no-wrap row adjacency (`child + 1` is the bottom border) | Probably survives. The caption above the table is not asserted |
| `level2_list_shows_target_and_parent_counts_in_a_100_column_pane` | github, refused | fast failure | cells and wrap | Probably survives |
| `level2_list_verbose_renders_table_and_verbose_in_tmux`, `level2_list_verbose_renders_with_graph_path_active` | **none** | none | — | Unaffected |

##### `worktree/cli/src/commands/list/tests.rs` (unit; seam-driven, no real worker)

The `gather_remote` module (`ORIGIN = https://prs.example.invalid/owner/repo.git`) uses `ListSeams { connect, launch: counting_launch }`. No network is involved; these tests pin the **launch decision**, which the change replaces. All are `#[serial_test::serial]`.

| Test | Origin | Current launch assertion | After "always launch + wait" |
|---|---|---|---|
| `a_stale_answer_is_shown_at_once_and_refreshed_in_the_background` | example.invalid | launches == [main], 0 requests, git calls == origin lookup only | Survives if the launch seam stays; the new wait seam must be stubbed |
| `a_stale_empty_answer_is_still_an_answer` | example.invalid | launches == 1 | survives |
| `two_fresh_answers_neither_request_nor_refresh` | example.invalid | **events empty (NW)** | **breaks** |
| `a_stale_pr_answer_and_a_missing_head_launch_exactly_once` | example.invalid | events == [Launch] | survives |
| `a_missing_or_stale_head_launches_without_any_foreground_request` | example.invalid | events == [Launch]; git calls == origin lookup | Survives, unless the foreground adds a reflog or default-branch git call (§4 "tracking-ref age" runs reflog when there is no stored answer) |
| `a_pr_miss_with_a_fresh_head_requests_in_the_foreground_and_launches_nothing` | example.invalid | events == [Connect, Fetch] (**NW**) | **breaks** |
| `a_pr_miss_settles_before_the_worker_is_launched` | example.invalid | order [Connect, Fetch, Launch] | Ordering must be re-decided (launch-first vs. PR-miss-first) |
| `a_failed_miss_shows_no_badges_and_stores_nothing` | example.invalid | **launches empty (NW)** | **breaks** |
| `a_changed_origin_never_shows_the_old_answers` | example.invalid | launches == 1, requests == 1 | survives |
| `without_an_origin_stored_answers_are_ignored_and_nothing_is_requested_or_launched` | none (removed) | events empty | survives (valid guard) |

The 8 `run_pipeline` / overlap / perf-count tests in the same file (`list_worktrees_resolves_default_branch_once`, `run_skips_graph_git_calls_when_image_unavailable`, `run_pipeline_without_perf_produces_no_collector`, `run_pipeline_non_image_verbose_includes_verbose_gather_stage`, `run_pipeline_gathers_the_graph_while_list_gather_is_unfinished`, `run_pipeline_gathers_the_graph_on_a_narrow_image_terminal`, `run_pipeline_without_image_support_or_verbose_gathers_no_graph`, `perf_subprocess_counts_meet_sla`) use `NO_PRS` seams whose `launch` **panics**. Their repositories have **no origin**, so they are unaffected as long as the no-origin rule stays. If the PR stage and launch are restructured to gather local facts after the wait, the overlap tests (`tests::overlap::arrive`) need re-checking.

##### `worktree/cli/src/commands/refresh_worker.rs` `mod tests` (unit; injected halves)

`ORIGIN = https://refresh.example.invalid/owner/repo.git`. There are 7 tests: `a_worker_that_cannot_start_is_an_error_launch_discards` (spawns a missing exe), plus `the_head_half_publishes_while_the_pr_half_is_blocked`, `the_pr_half_publishes_while_the_head_half_is_blocked`, `a_failing_or_unsupported_pr_half_leaves_the_head_half_publishing`, `a_contended_pr_half_leaves_the_head_half_publishing`, `a_failing_head_half_leaves_the_pr_half_publishing`, and `only_the_top_level_of_a_main_checkout_is_accepted`. They make no network calls. They break only if `run_halves`, `refresh_remote_head`, or the `RemoteHeads` trait change shape, for example with attempt tokens, format 2, or a completion receipt.

##### Files checked and found unaffected (no origin, or no listing)

| File | Tests | Why unaffected |
|---|---|---|
| `tests/list_output.rs` | 3 | `wt list` without an origin (note: `list_output_is_the_redesigned_table` is an exact-output snapshot, so any new always-printed line would break it) |
| `tests/perf_command_sla.rs` | 1 (`perf_full_command_non_image_meets_sla`, bare `wt`, < 1 s) | `MixedFixture::new()` with no origin |
| `tests/cache_cold_path.rs` / `cache_warm_path.rs` | 1 + 1 | no origin; `list gather` timing only |
| `tests/perf_flag.rs` | 5 | no origin |
| `tests/list_table.rs` | pure renderer, no `wt` process | Not in scope for A, but its caption tests (`every_remote_observation_reads_as_ruled`, the `caption(…)` assertions around lines 250–630) encode the old wording and change with §4 |
| `tests/remove.rs` | 21 tests use `Fixture::with_origin()` (bare origin) | Only `wt remove` / `create` run; `remove` never renders a listing |
| `tests/wrapper_protocol.rs`, `shell_wrapper_exec.rs`, `powershell_wrapper_exec.rs`, `create_include.rs`, `styled_capture_parse.rs`, `level2_create.rs`, `level2_dirty_tree.rs`, `level2_graph_in_kitty.rs`, `level2_remove.rs`, `level2_powershell_remove.rs` | — | no origin, or no listing |

##### Section A counts

| File | Tests with origin that list or run the worker | Would stall about 3 s | Fast (answer or failure) | Assert NW | Assert RC | Assert CAP | Assert T |
|---|---|---|---|---|---|---|---|
| `list_prs.rs` | 12 | 5 listings stall (`a_fresh…`, `a_stale_store…`, `a_detached…`, `concurrent…`, `a_missing_or_stale_live_head…`); 2 more hold only a directly-run worker (`a_killed…`, `an_origin_change_during_a_workers_request…`) | 5 | 4 | 7 | 8 | 0 |
| `list_remote_head.rs` | 4 (+1 no-origin guard) | 0 | 4 (real local fetch) | 2 (+guard) | 0 | 4 | 0 |
| `perf_pr_request.rs` | 4 | 3 | 1 | 0 | 2 | 0 (1 checks "PRs as of") | 4 |
| `level2_list_verbose.rs` | 5 | 1 | 4 | 0 | 1 | 2 | 0 |
| `list/tests.rs` (seams) | 9 (+1 no-origin guard) | n/a (no real worker) | n/a | 3 break (+1 ordering) | — | — | — |
| `refresh_worker.rs` (injected) | 6 (+1 spawn) | n/a | n/a | — | — | — | — |
| **Total** | **40 test functions** (25 real-process integration + 15 unit) | **9 integration tests put a `wt list` into the 3 s stall** (5 in `list_prs.rs`, 3 in `perf_pr_request.rs`, 1 in L2), plus 2 with held direct workers | | | | | |

---

#### Section B: sniff tests asserting `PrUnavailable::Auth`

Variant definition: `sniff/lib/src/remote/blocking.rs:92` (`Auth { message: String }`, Display text "provider denied the query: {message}"). The producer is `classify` at `blocking.rs:292–300`: `MissingCredentials | InvalidCredentials | RemoteForbidden | RemoteApi{401|403}` all become `Auth`. The doc reference is at `blocking.rs:181`.

| File:line | Test | What it asserts | Cases |
|---|---|---|---|
| `sniff/lib/tests/l1/open_pull_requests.rs:327` (fn at :308) | `auth_failures_are_unavailable_not_empty_on_every_provider` | `matches!(result, Err(PrUnavailable::Auth { .. }))` | For every `FLAVORS` provider: `(401, no token)`, `(401, token "rejected")`, `(403, token "scoped")`; the token variable is `GITHUB_TOKEN`/`GITLAB_TOKEN`/`GITEA_TOKEN`/`BITBUCKET_TOKEN`. Under the split these become **CredentialsRequired / CredentialsRejected / CredentialsInsufficient** respectively. The bare 403 needs the "response establishes insufficiency" rule |
| `sniff/lib/tests/l1/pr_for_branch.rs:403` (fn at :384) | `auth_failures_are_unavailable_on_every_provider` | same `matches!` on `lookup(TARGET)` | the same three cases × every provider |
| `sniff/lib/src/remote/blocking.rs:431` (fn at :418, in-crate `mod tests`) | `a_denial_is_never_classified_as_an_answer` | `classify(RemoteForbidden{…}, None)` is `PrUnavailable::Auth { .. }` (plus 404 is `NotFoundOrNotPermitted`) | 1 case, which becomes `CredentialsInsufficient` |

**Count: 3 tests (2 L1 integration and 1 unit), 3 assertion sites.**

Related sites that do not assert `Auth` but are touched by the same split:
- `RateLimited { message }` becomes `RateLimited { authenticated: bool }`. No test currently asserts `PrUnavailable::RateLimited` (searched `sniff/lib/tests`, `sniff/lib/src`).
- `NotFoundOrNotPermitted` is kept, and is asserted at `pr_for_branch.rs:421, :441` and `open_pull_requests.rs:345`. The `Timeout` assertions at `pr_for_branch.rs:464` and `open_pull_requests.rs:369` are unaffected.
- Non-`sniff` consumers only stringify: `worktree/lib/src/pull_requests.rs:69` (`reason.to_string()`) and `worktree/lib/src/remove/safety.rs` (`PrLookup::Unavailable(reason.to_string())`). Two `worktree/lib/src/pull_requests.rs` unit tests hard-code the old Display text `"provider denied the query: 401"` (lines 554 and 679) as stub strings. They do not break, but the literal goes stale.
- `sniff/lib/src/remote/focused.rs:852` has a `"provider denied the query"` message literal (a `SniffError`, not `PrUnavailable`).
- `sniff/cli` has no reference to `PrUnavailable`.
#### How each origin kind behaves after the change

Assumes the new worker calls `branch_head` for a provider sniff supports (github.com, `gitea.*`, and similar). It falls back to `git ls-remote` on 404, credentials, or rate-limit errors, and possibly on other errors. For any other origin it runs `ls-remote` directly.

| Origin kind | Fixture | Provider API call? | Expected new wait |
|---|---|---|---|
| Local bare repository (file path) | `list_remote_head::Fixture`, `remove.rs::with_origin` | no (Unsupported) | **Fast answer.** Local `ls-remote`; a variance triggers a real local `fetch`, which **changes `origin/main`** |
| `https://github.com/owner/repo.git` + `ProxyStub::refusing()` | `MixedFixture::with_github_origin` + `wt_command_via` | yes, refused at once | **Fast failure.** The `ls-remote` fallback is blocked by `protocol.https.allow=never` |
| `https://github.com/...` + `ProxyStub::hanging()` | same | yes, **held** | **Stalls about 3 s.** The API check hangs until its 10 s deadline, and each run adds one more held proxy connection |
| `http://gitea.test/o/r.git` + `FakeGitea` (not held) | `with_gitea_origin` + `wt_command_via_gitea` | yes, answered at once | **Fast failure.** The server answers every path with its PR reply: `Open` is a JSON array, which is the wrong shape for a branch; `Status(500/401)` is an error. The fallback is blocked by `protocol.http.allow=never`. **The request is counted in `gitea.requests()`** |
| `gitea.test` + `FakeGitea::hold()` | same | yes, **held** | **Stalls about 3 s.** The branch-head request also shows up in `requests()`/`waiting()` |
| `HoldingOrigin` loopback (`http://127.0.0.1:P/r.git`) | `with_origin` + `wt_command_direct` | no (not a provider) | **Stalls about 3 s.** `ls-remote` is held |
| `https://github.com/...` + `HTTPS_PROXY=http://127.0.0.1:9` (in a tmux pane) | `level2_list_verbose::DesignFixture` | yes, refused | **Fast failure, with a caveat.** The user's and system git config are **not** isolated and `protocol.*.allow` is not set. An `insteadOf` rule that rewrites `https://github.com/` to SSH would send the `ls-remote` fallback to the **real github.com**. `SNIFF_*_TOKEN` is not cleared. Stderr is a TTY, so the spinner appears after 150 ms |
| `*.example.invalid` | `list/tests.rs` (seams), `refresh_worker.rs` unit tests | never reaches the network (seams or injected halves) | none |
| none | many | — | none; no worker is launched |

Cross-cutting breakages, for Phase planning:
- **Seeding a fresh live head no longer isolates a test.** Most PR tests rely on `seed_fresh_head` / `seed_remote_head_store(ZERO, …)` so that no worker starts (see `list_prs.rs` module docs and `perf_pr_request::seed_fresh_head`). After the change a worker starts on every listing with an origin.
- **Worker requests now show up in PR-request counters.** Assertions on `ProxyStub::connections()` and `FakeGitea::requests()`/`waiting()` count the branch-head request alongside PR requests.
- **"Nothing stored" byte-equality on the live-head store breaks.** Format 2 writes an `attempt` record even when the check fails.
- **The worker's PR half now runs on a PR miss.** On a miss with an answering source, the worker can publish the PR store. Today a miss launches nothing.
- **Direct `internal-refresh <main>` calls pass no `--attempt`.** Affected callers: `perf_support::refresh_worker_via_gitea`, `list_prs::the_worker_command_prints_nothing…`, and `list_remote_head::Fixture::refresh`. They need an optional attempt argument, or updating.
- **Shared `perf_support` helpers need updating.** `wait_until_unlocked` and `probe_head_refresh` call `refresh_remote_head(…, &NoRequest)`, and `seed_remote_head_store` writes `format_version: 1`. Both need to track the new store and lock API.


### Baseline (unmodified branch `fix/wt-ux` at `5ed279dd9`, macOS)

| Area | `just test` | `just lint` |
|---|---|---|
| `schematic/` | pass: 1700 run, 1700 passed, 5 skipped | pass |
| `sniff/` | pass: 2858 run, 2858 passed (13 slow), 31 skipped | pass |
| `biscuit-terminal/` | pass: 3315 run, 3315 passed, 55 skipped | pass |
| `worktree/` | pass: 527 run, 527 passed (3 slow), 21 skipped | pass |

There are no pre-existing failures. `just test-l2` was not part of the Phase 1 baseline; it is required only for `worktree/` at the end (Phase 5).

### Follow-ups recorded (out of scope)

- **Two user files.** `~/.worktree.json` (`WorktreeConfig`, `base_dir`, setup flow) and the new `~/.wt.json` (API preference, Rule 2) will coexist. Merging them into one file is a separate decision, not part of this fix.
- **sniff doc drift** noted in S2 (`bitbucket.rs:162`, `github.rs:26,42`).

### Checkpoint 1

- S1–S4 are recorded above.
- **Rule 10 is amended** (S3). The fetch argv adds `--no-recurse-submodules` and `--refmap=`. The classifier adds `unable to get password from user`. The deadline becomes a typed error rather than a matched string. `tracking_ref_changed_at` must handle empty-stdout/exit-0 (no reflog) and exit 128 (no ref).
- **Rule 13 is amended** (S2). Key names follow `credentials::provider_token`'s order. The 403 split is made in `focused.rs` where headers are available. S1 confirms the endpoints and `%2F`.
- **Rule 3 is amended** (S2). sniff exposes a raw `remote_identity`, and worktree keeps the port policy.
- An open gap is closed by a proposal (S4): `--attempt` is optional on `internal-refresh`.
- The baseline is green in all four areas.
- No source files were changed in Phase 1.

## Phase 2

Phase 2 builds the provider, component, and store foundations. Wave 1 ran as four parallel subagents over disjoint files: schematic, sniff, the Spinner, and remote-head format 2. The orchestrator wrote `api_preference` and did the Wave 2 worktree task. The sniff agent did its Wave 1 and Wave 2 tasks in sequence, because they share files.

The orchestrator added only `pub mod api_preference;` to `worktree/lib/src/lib.rs`. `remote_update` and `fast_forward` are left for Phases 3 and 4, which create them, so no empty module ships in between.

### Schematic endpoints

- GitHub `GetBranchReference` (`GET /repos/{owner}/{repo}/git/ref/heads/{branch}`) reuses `GitRef` (`object.sha`).
- Gitea `GetBranch` adds `Branch { name, commit: BranchCommit { id, .. } }`.
- GitLab `GetBranch` adds `Branch`, reusing `Commit` (`commit.id`).
- Bitbucket `GetBranch` adds `Branch { target: CommitInfo, .. }`. Its `target.hash` is an `Option<String>`.
- The branch is an ordinary encoded path parameter. The GitHub request was checked to pass it through `urlencoding::encode`.
- No auth variable changed.
- `schematic/schema` was regenerated, along with `openapi/*.json` and `postman/*`. Regeneration also rewrote `schematic/schema/Cargo.lock`, which dropped `cargo_metadata`, `camino`, and `cargo-platform`: that lock file was stale against `biscuit-file`'s current dependencies.
- **Tests (8 new):**
    - endpoint tests: `{github::tests::get_branch_reference_endpoint, gitea/gitlab/bitbucket::tests::get_branch_endpoint}`
    - payload parsing: `github::types::releases::tests::git_ref_branch_head`, `{gitea,gitlab}::types::tests::branch_deserialization`, `bitbucket::types::branches::tests::branch_deserialization`
    - the endpoint-count tests were renamed for the new counts
- `schematic/README.md` had wrong endpoint counts before this change. They now match the code.
- **Follow-up (out of scope):** `just check-drift` runs zero tests. Its filter passes `--ignored`, but the drift tests are neither ignored nor in the default target. `cargo test -p schematic-gen --test l1 artifact_drift::` is the working check, and all 5 of its tests pass. The Gitea section of `schematic/definitions/README.md` also says to include the `token ` prefix, which the code does not want.

### sniff: credentials split, `credential_env`, `remote_identity`, `branch_head`

- **`PrUnavailable`** (still `#[non_exhaustive]`):
    - It is now `Timeout { deadline }`, `Network`, `CredentialsRequired { key: Option<String> }` (always `None`), `CredentialsRejected { key }`, `CredentialsInsufficient { key }`, `NotFoundOrNotPermitted`, `RateLimited { authenticated, key: Option<String> }`, `Unsupported`, and `Other`.
    - `Auth` is gone.
    - `key` is a variable name, and no `Display` contains a value. Tests assert this with a sentinel secret.
- **Key flow.** `credentials::provider_token_variables(flavor)` is the one list of names. `provider_token` returns the `(name, value)` it matched, and `credential_env` reports the same list in the same order (the Rule 13 amendment). `FocusedProviderClient::credential()`/`credential_key()` also cover host-bound `SNIFF_*_TOKEN`.
- **403 split in `focused.rs`:**
    - A spent `x-ratelimit-remaining: 0`, or a body containing "rate limit", is `RateLimited` on every provider, not only GitHub.
    - With a token, a body naming a permission or scope is `CredentialsInsufficient`.
    - Any other 403 is `NotFoundOrNotPermitted`.
- **Deviation (hidden coupling, documented at the constant and in the skill).** An insufficient 403 travels as `SniffError::RemoteForbidden` carrying the exact message `INSUFFICIENT_CREDENTIALS_MESSAGE`, and `classify` keys on that text. A new `SniffError` variant or field would have broken sniff-cli and darkmatter code that builds these errors.
- **Behavior changes outside the blocking path** (recorded in sniff's CHANGELOG):
    - A rate-limit 403 is now `SniffError::RateLimited` for the focused client, so darkmatter shows RateLimit instead of Authentication.
    - `Unsupported` messages no longer echo the remote URL, since userinfo can carry a token.
- **Anonymous fallback.** `open_pull_requests` has no anonymous retry on the blocking path, so `branch_head` has none either. With no token the request goes out anonymously, and a test asserts that no auth header is sent. A rejected token is reported, not retried. Rule 9 already sends those cases to `ls-remote`.
- **`remote_identity`** is at `sniff::filesystem::git::remote_identity` (also re-exported as `sniff::filesystem::remote_identity`), not under `sniff::remote`. It returns `RemoteIdentity { scheme, host, port: Option<u16>, path }`.
- **`branch_head`** and `branch_head_with` send the branch as one percent-encoded segment. A SHA that is not 40 or 64 lowercase hex characters, or a body without the field, is `Other`. An empty branch is refused without a request.
- **Tests:**
    - `sniff/lib/tests/l1/branch_head.rs` (11 tests, declared in `tests/l1/main.rs` under `feature = "remote"`):
        - `each_provider_reports_its_own_sha_field`
        - `a_head_that_is_not_a_full_lowercase_object_id_is_rejected`
        - `a_body_without_the_sha_field_is_rejected`
        - `a_response_slower_than_the_deadline_times_out`
        - `a_branch_name_is_one_encoded_path_segment`, with the input `feature/a b/ü?x=1#y`
        - `without_a_token_the_request_is_anonymous_and_answers`
        - `every_credentials_condition_is_classified_on_every_provider`
        - `the_key_names_the_first_set_candidate_variable`
        - `an_empty_branch_is_refused_without_a_request`
        - `an_unsupported_remote_is_refused_without_echoing_its_url`
        - `credential_env_lists_each_providers_variables_in_lookup_order`
    - The two L1 `Auth` tests became `credentials_failures_are_unavailable[_not_empty]_on_every_provider`, sharing one case table: 401 and 404 with and without a token, three kinds of 403, and 429 with and without a token, plus GitHub's quota-header and rate-limit-body 403.
    - The unit test `a_denial_is_never_classified_as_an_answer` now asserts `CredentialsInsufficient`.
    - `commit_links.rs` gained 3 `remote_identity_*` unit tests.

### Spinner (`biscuit-terminal`)

- **API:** `biscuit_terminal::components::spinner::{Spinner, SpinnerHandle, frame, CLEAR_LINE, FRAMES, FRAME_INTERVAL}`; `Spinner` and `SpinnerHandle` are also in the prelude.
    - `Spinner::new(text).with_delay(d)[.with_width(w)].start_on_stderr()`, or `.start_on(writer, is_terminal)`.
    - `SpinnerHandle::set_text`, `finish()`, and `Drop`.
    - `with_width` is an addition for tests.
- **Contract:**
    - Nothing is written, and no thread is started, when the output is not a terminal.
    - The line is cleared exactly once, and only if a frame was drawn, so a spinner stopped before its delay writes nothing.
    - Frames are at most `width - 1` columns, because the Windows console wraps early.
- **Tests (9 unit tests):**
    - `first_frame_is_drawn_only_after_the_delay`
    - `finish_before_the_delay_writes_nothing`
    - `non_terminal_writes_nothing_and_spawns_no_thread`
    - `set_text_changes_the_next_frame`
    - `finish_writes_the_clear_sequence_exactly_once`
    - `drop_writes_the_clear_sequence_exactly_once`
    - `frames_are_truncated_below_the_width`
    - `frames_truncate_wide_characters_by_display_width`
    - `frames_cycle_through_the_glyphs`

### Remote-head store format 2 (`worktree::remote_head`)

- **Public API:**
    - Constants: `REMOTE_HEAD_FORMAT_VERSION = 2`, `RECEIPT_FORMAT_VERSION = 1`, and `ATTEMPT_MAX_AGE = 75 s`.
    - Types: `Answer`, `AnswerSource`, `Attempt`, `Phase`, `FallbackReason`, `Outcome`, `CheckFailure`, `FetchFailure`, `UnavailableReason` (`origin-changed` | `branch-changed`), `ApiCondition`, `ApiNote { condition, key, fallback_answered }`, `StoreState`, `PrFailure`, `HeadStatus`, `PrStatus`, and `Receipt`.
    - Store functions: `read_store`, `select_attempt`, `begin_attempt`, `set_phase`, `finish_attempt`, `publish_answer`, and `new_attempt_id` (delegates to `remove::handoff::new_token`).
    - Receipt functions: `refresh_receipt_path`, `write_receipt`, and `load_receipt`.
    - Tagged enums use `kind`; every other spelling is kebab-case.
- **Read rules:**
    - Only a non-JSON document or a missing or unknown `format_version` loses the whole file. Otherwise each half is validated on its own.
    - An attempt exactly 75 s old is still current.
    - Discarding happens on read and never writes.
- **Writers:**
    - They refuse another attempt's id, returning a `NotFound` I/O error and writing nothing.
    - They refuse invalid values.
    - `api: None` keeps the stored note.
    - `finish_attempt` keeps the phase reached.
- **Existing behavior kept for now.** `refresh_remote_head` and `select_cached_head` keep their signatures. The first publishes `source: git` and leaves any attempt alone; the second reads `answer`. The `AlreadyFresh` recheck is still there, for Phase 3 to remove (Rule 7).
- **Tests (15 new unit tests):**
    - `a_format_1_file_reads_as_a_git_answer_and_a_write_keeps_it`
    - `the_store_round_trips_with_its_documented_spellings` (read, write, read)
    - `every_phase_and_outcome_survives_a_round_trip`
    - `an_attempt_is_current_through_attempt_max_age_and_not_one_second_beyond`
    - `an_invalid_attempt_field_drops_the_attempt_and_keeps_the_answer` (24 cases)
    - `an_invalid_answer_drops_the_answer_and_keeps_the_attempt`
    - `writers_keep_the_answer_while_the_attempt_moves`
    - `publishing_an_answer_keeps_the_attempt`
    - `a_failed_or_stale_attempt_never_replaces_the_answer`
    - `writers_refuse_another_attempt_and_invalid_values_without_writing`
    - `a_refresh_keeps_the_stored_attempt`
    - `attempt_ids_are_32_random_hex_characters`
    - `a_receipt_round_trips_with_its_documented_spellings`
    - `a_receipt_for_another_attempt_or_older_than_the_attempt_is_ignored`
    - `a_malformed_or_missing_receipt_is_ignored`
- **Changed test helpers:**
    - In `cli/tests/list_remote_head.rs`, `stored_head()` now reads the `answer` half, `age_head` edits `answer.checked_at`, and a new `stored_document()` returns the whole file.
    - `perf_support::seed_remote_head_store` still writes format 1, which is a supported read path.
- **Duplication left in place:** `is_attempt_id` repeats `handoff::is_token`, which is private.

### API preference store (`worktree::api_preference`)

- **API:**
    - `preference_path()` = `dirs::home_dir()/.wt.json`.
    - `RepoIdentity { host, port, path }::from_origin(url)`, built on sniff's `remote_identity`, with the Rule 3 port policy:
        - `https` is 443 unless the URL gives a port.
        - `ssh` to a known provider host is 443. A host is known when `GitHostingProvider::from_url` returns GitHub, GitLab, Bitbucket, Gitea, or Forgejo.
        - Other `ssh` uses its port, or 22.
        - `http` is 80 and `git` is 9418.
        - A local path or `file://` URL has no identity.
    - `Preferences::is_ignored`, `load(path)`, and `add(path, identity)`.
    - `WorktreeError::PreferenceUnwritable { path, reason }` is new. The CLI's `exit_code` maps it to 1 through its default arm.
- **Rules:**
    - `load` treats a missing, unreadable, corrupt, or other-format file as empty.
    - `add` refuses such a file and leaves it byte-for-byte untouched. It is idempotent, and it takes `~/.wt.json.lock` for at most 2 s, polling `try_lock_sidecar` every 25 ms, before `atomic_write`.
- **Tests (12 unit tests):**
    - `https_and_ssh_remotes_of_one_provider_repository_share_an_identity` (6 URL spellings)
    - `user_information_never_reaches_the_identity_or_the_file`
    - `distinct_ports_and_paths_stay_distinct`
    - `ssh_to_an_unknown_host_keeps_its_own_port`
    - `a_local_path_has_no_identity`
    - `a_missing_file_ignores_nothing`
    - `add_round_trips_and_is_idempotent`
    - `a_corrupt_or_other_format_file_reads_as_empty_and_add_refuses_to_replace_it` (4 variants)
    - `an_unreadable_file_reads_as_empty_and_add_refuses_it`
    - `a_held_lock_times_out_as_a_write_error`
    - `concurrent_adds_keep_every_entry` (8 threads)
    - `the_file_sits_in_the_home_directory`
    - `the_file_resolves_under_userprofile_on_windows` (`cfg(windows)`)

### Typed PR failure (Rule 14)

- `OpenPrSource::fetch` now returns `Result<Vec<OpenPullRequest>, PrFailure>`.
- `PrFailure::from_unavailable(&PrUnavailable)` maps each credentials condition by kind, `NotFoundOrNotPermitted` to itself, and everything else to `Other`. It is implemented in `pull_requests.rs`; the type lives in `remote_head.rs`.
- `fetch_and_publish` now returns `Result<Option<PrListing>, PrFailure>`: `Err` is this run's failure (for §5), and `Ok(None)` means `origin` changed during the request. `list::gather_remote` flattens it to the old behavior until Phase 4 uses the failure.
- `RefreshOutcome::Failed` does not carry the failure yet. The receipt needs it, so Phase 3's worker wiring should add it. It was not added now because `RefreshOutcome` is shared with `refresh_remote_head` and is `Copy`.
- `remove::safety` still calls `to_string()`, and it compiles.
- **Tests:**
    - New: `every_sniff_reason_maps_to_its_credentials_condition_or_other` (10 cases) and `each_credentials_failure_reaches_the_foreground_caller_and_is_never_stored` (6 variants; checks the store stays absent and `select_cached` is `Miss`).
    - Updated: `a_failed_foreground_request_leaves_the_store_untouched` (typed errors) and `a_failed_refresh_leaves_the_stored_answer_untouched`. The CLI test stubs (`list/tests.rs`, `refresh_worker.rs`, `perf_support::NoRequest`) now return `PrFailure`.

### Requirement → test mapping (Phase 2 scope)

| Requirement | Tests |
|---|---|
| Endpoint definitions: path, method, and auth per provider | schematic `*_branch*_endpoint` (4) plus payload parsing (4) |
| sniff §5 split, key names, no secret in `Display` | `credentials_failures_are_unavailable*_on_every_provider`, `every_credentials_condition_is_classified_on_every_provider`, `the_key_names_the_first_set_candidate_variable` |
| `branch_head`: SHA parsing, invalid SHA, deadline, encoding, anonymous request, `credential_env` | `sniff/lib/tests/l1/branch_head.rs` (11) |
| Spinner: delay, not a terminal, `set_text`, clear once, width | `components::spinner::tests` (9) |
| `~/.wt.json`: identity, ports, SSH and HTTPS match, corrupt file, concurrency, Windows path | `api_preference::tests` (12) |
| Store format 2: format-1 read, discard rules, answer preserved, receipt checks | `remote_head::tests` (15 new) |
| Typed PR failure, one case per variant | `pull_requests::tests` (2 new, 2 updated) |

### Gates (macOS, final combined tree)

| Area | `just test` | `just lint` |
|---|---|---|
| `schematic/` | 1708 passed, 5 skipped (baseline 1700) | pass |
| `sniff/` | 2872 passed, 31 skipped (baseline 2858) | pass |
| `biscuit-terminal/` | 3324 passed, 55 skipped (baseline 3315) | pass |
| `worktree/` | 556 passed, 21 skipped (baseline 527) | pass |

- `cargo check -p worktree -p worktree-cli --all-targets --all-features` is clean.
- **Native Windows** (`./scripts/cross-check.sh worktree --os windows api_preference remote_head pull_requests`): 69 of 69 pass, including `the_file_resolves_under_userprofile_on_windows` and the directory-as-unreadable-file case.
- `just cross-check` with a quoted `-E` filterset fails with a bash syntax error, as the worktree skill warns. Call the script directly with substring filters.
- There are no pre-existing failures and no skipped gates. `just test-l2` is not part of Phase 2.

### Checkpoint 2

- All seven Phase 2 tasks are done, and `just test` and `just lint` pass in all four areas.
- `cargo check -p worktree-cli` compiles.
- The worktree skill's `remote_head`, `pull_requests`, and new `api_preference` entries are updated.

## Phase 3

Phase 3 builds the worker's update flow: the typed Git transport, `remote_update::run_attempt`, and the worker wiring (attempt id, `--force`, completion receipt). One agent did all three tasks in order, since Wave 2 depends on both Wave 1 APIs.

### Git transport (`worktree::live_remote`)

- `run_noninteractive` now sets `LC_ALL=C`. Removal's callers only display git's stderr as a reason; none parses it, so the change is safe for them (their reasons are now always English).
- **Typed failure (S3's recommendation):** `run_transport(base, args, deadline) -> Result<String, TransportError>`, where `TransportError { failure: GitFailure, reason }` and `GitFailure` is `Timeout | Credentials | Other`. The deadline is `Timeout` by construction, never matched from text. `run_noninteractive` is now a wrapper that keeps the `String` error for removal, and `LsRemote::head` is the typed form of `RemoteHeads::live_head`.
- `classify_git_failure(stderr)` uses exactly S3's six credential patterns; everything else is `Other`.
- `is_valid_branch_name(base, branch)`: `git check-ref-format --branch`, and the printed name must equal the input. This refuses names git would *expand* (`@{-1}`) as well as invalid ones. A leading `-` is refused before git runs.
- `fetch_argv(branch)` is the amended Rule 10 argv; `fetch_tracking_ref(base, branch, deadline)` validates first and never spawns the fetch for an invalid name.
- `tracking_ref_changed_at(base, branch)`: `git reflog -1 --format=%ct <ref> --`; `None` for a missing ref, a missing reflog (empty stdout, exit 0), or unparsable output.
- **Tests (8 new):** `an_unauthorized_origin_is_a_typed_credentials_failure` (loopback 401, both `ls-remote` and the fetch), `the_classifier_maps_each_recorded_sample_and_defaults_to_other` (every S3 sample), `the_fetch_argv_is_the_ruled_command_with_one_refspec_argument`, `a_fetch_updates_only_the_one_tracking_ref` (extra `remote.origin.fetch` mirror refspec, `fetch.prune=true`, a pushed tag, another branch moved, `FETCH_HEAD` sentinel; only `origin/main` changes), `a_remote_rewind_is_applied`, `an_invalid_branch_name_is_refused_before_any_request` (7 names, zero loopback connections), `a_fetch_past_its_deadline_is_a_typed_timeout`, and `the_tracking_ref_reflog_dates_its_last_change_or_is_none`.

### Update-flow core (`worktree::remote_update`)

- **API:** `run_attempt(AttemptRequest { store, main, id, ignore_api }, &Seams { api, git, now, monotonic }) -> AttemptEnd`.
    - `AttemptEnd` is `Finished(Outcome)`, `Contended` (nothing written), `NotStarted` (no origin, no default branch, or a lock file that cannot be opened), or `WriteFailed`. `AttemptEnd::head_status()` maps it to the receipt's `HeadStatus`.
    - Seams: `BranchHeadSource` (`branch_head`, `key_in_use`; production `SniffBranchHeads`), `GitRemote` (`live_head`, `fetch`; production `GitTransport { main }`), a Unix clock, and a monotonic clock for the budget.
    - `FETCH_DEADLINE` = 60 s. The check budget is the existing `REMOTE_HEAD_REFRESH_DEADLINE` (10 s), whose doc now says so.
- **Deviation: no `force` parameter.** The plan's sketch was `run_attempt(main, token, force, seams)`. Rule 7 gives the head half no freshness skip at all, so `force` would change nothing there; it lives in the PR half and the receipt instead.
- **Deviation: `refresh_remote_head` is removed**, not trimmed. With the `AlreadyFresh` recheck gone (Rule 7), it was a second, attempt-less writer of the same store. Its tests moved to `remote_update` (non-main default branch, digest-only storage, no origin/default, lock failure, contention, origin/branch change during the request, loopback 401) or became obsolete (`a_fresh_answer_skips_the_request`, `a_refresh_keeps_the_stored_attempt`). `select_cached_head` tests now seed with `publish_answer`.
- **Order:** lock → origin and default branch (none: `NotStarted`, nothing written) → `begin_attempt` → branch-name validation (invalid: `check-failed{other}`, no request) → check → re-read origin/branch → publish → `absent` / `in-sync` / `fetching` → fetch → re-read → publish the fetched tip → outcome.
- **Fallback mapping (Rule 9):** `CredentialsRequired` → `no-key`; `NotFoundOrNotPermitted` → `not-visible` without a key, `other` with one; `RateLimited` → `rate-limited`; `CredentialsRejected`/`Insufficient` → `rejected`; `Timeout`/`Network`/`Other` → `other` with no note; `Unsupported` → Git in phase `checking`, no note. The note's `fallback_answered` becomes `true` when `ls-remote` answers.
- **"Key set" for a 404** comes from `SniffBranchHeads::key_in_use`: the first of `credential_env`'s variables that is set and non-empty. It cannot see sniff's host-bound `SNIFF_*_TOKEN`, so a 404 sent with only that token reads as `not-visible`. Documented at the method.
- **Failed fetch (Rule 11):** when the tracking ref moved anyway, one recheck runs (own budget, no phase writes). Only when the recheck equals the new tip is the outcome `fetched`, and the recheck is published; otherwise `fetch-failed` and the first check stays. A fetch reason is `timeout` or `other` (a Git credentials failure during a fetch is `other`, since `FetchFailure` has no credentials reason).
- **Tests (24, `lib/src/remote_update/tests.rs`):** real bare origin and `pusher` (`TestRepo`), a scripted `Api`, Git real unless scripted, and a test clock.
    - `no_variance_is_in_sync_with_no_fetch`
    - `variance_fetches_and_publishes_the_fetched_tip` (FETCH_HEAD absent, local `main` unmoved, answer stamped at the fetch's start)
    - `a_remote_move_between_check_and_fetch_is_reported_from_the_fetched_tip`
    - `a_fetch_timeout_keeps_the_new_answer_and_the_tracking_ref`
    - `a_check_that_uses_the_whole_budget_fails_as_a_timeout_and_keeps_the_old_answer`
    - `the_api_call_and_the_fallback_share_one_budget` (API takes 7 s on the test clock; `ls-remote` gets exactly 3 s)
    - `an_unsupported_remote_is_checked_by_ls_remote_in_phase_checking` (real `SniffBranchHeads` on a local path)
    - `each_fallback_reason_and_condition_is_recorded` (10 cases; the phase is observed in the store *when `ls-remote` starts*)
    - `an_ignored_repository_makes_no_provider_request`
    - `only_ls_remote_proves_absence` (404 → `ls-remote` absence; 404 + Git failure → `check-failed{other}` with the old answer kept; Git credentials → `check-failed{credentials}`)
    - `an_origin_or_default_branch_change_during_the_check_publishes_nothing`, `an_origin_change_during_the_fetch_keeps_the_check_and_ends_unavailable`
    - `a_concurrent_fetch_that_reached_the_current_remote_head_counts_as_fetched`, `a_concurrent_fetch_of_an_older_head_is_not_labeled_current`
    - `the_attempt_is_recorded_before_any_request`, `a_contended_lock_writes_nothing_and_asks_nothing`, `a_lock_that_cannot_be_taken_records_nothing`, `without_an_origin_or_a_default_branch_nothing_is_recorded`, `an_invalid_default_branch_name_fails_the_check_without_a_request`, `a_store_that_cannot_be_written_ends_the_attempt_before_any_request`
    - `a_non_main_default_branch_is_checked_and_fetched`, `the_store_records_only_a_digest_of_the_origin`, `successive_attempts_replace_the_answer_read_write_read`, `an_unauthorized_origin_fails_the_check_fast_as_credentials_and_keeps_the_answer` (real loopback 401)

### Worker wiring (`worktree-cli`)

- `wt internal-refresh <main> [--attempt <id>] [--force]` (S4's proposal: `--attempt` is optional; without it the worker makes its own id).
- `run_halves` now returns each half's result (`None` for a panic). `run_and_record` writes the receipt after both halves join, only for `--force`, bound to the origin digest and default branch read **before** the halves start. A panicked PR half is `failed{other}`; a panicked head half is `failed`.
- PR half: `pr_status` maps `RefreshOutcome` to `PrStatus`; an ignored repository returns `PrStatus::Ignored` without any request.
- Head half: `run_attempt` with the production seams.
- **Lib changes this needed:**
    - `pull_requests::refresh` gains `force: bool` (skips only the freshness recheck, never the lock). `RefreshOutcome` is PR-only now: `NoDefaultBranch` and `DefaultBranchChanged` are gone, `Failed(PrFailure)` carries the failure (the Phase 2 handoff item), and it is no longer `Copy`.
    - `remote_head::PrStatus::Ignored` (new receipt value).
    - `api_preference::Preferences::ignores_origin(origin)`.
    - `remote_head::refresh_lock_held(store)`: a lock probe for tests and for Phase 4's "contended" check. **It takes the lock for an instant**, so a worker that tries to take it at that moment exits as `Contended`. See the message to Phase 4.
- **Tests:** the 7 existing worker unit tests now drive `run_attempt` with a scripted provider; new: `a_receipt_is_written_only_after_both_halves_finish`, `a_panicking_half_is_recorded_as_failed_in_the_receipt`, `an_unforced_run_writes_no_receipt`, `a_forced_pr_half_asks_even_when_the_answer_is_fresh`, `an_ignored_repository_makes_no_pr_request`; lib: `a_forced_refresh_asks_even_when_the_answer_is_fresh`, `an_added_repository_is_ignored_by_any_of_its_origin_spellings`, `the_lock_probe_sees_a_holder_and_releases_its_own_hold`.

### Existing tests migrated now (from S4's Phase 4 Wave 3 list)

The worker changed in this phase, so Checkpoint 3 ("L1 green") required migrating the tests it broke. `wt list`'s launch rule is unchanged, so this touches only tests that run a worker.

- `perf_support::FakeGitea` recognizes the branch-head path (`/branches/`): it answers 404 at once, counts it in `branch_requests()`, and never holds it, so `requests()`/`waiting()` count PR requests only (S4's "count by path"). This fixed 4 of the 7 `list_prs` failures with no assertion changes. `a_detached_workers_answer…` now also asserts one branch-head request.
- `list_prs::a_stale_store_shows_its_badges_at_once…`: through `ProxyStub` (HTTPS CONNECT) the two halves cannot be told apart, so the worker is 2 connections, the next list 4.
- `list_prs::the_worker_command_prints_nothing…` and `a_missing_or_stale_live_head_never_holds_up_the_listing`: the byte-equality of the live-head store became "the `answer` half is unchanged" plus the attempt record (`check-failed{other}`, resp. `checking` with no outcome while held).
- `list_remote_head`: `a_push_elsewhere_reads_as_a_difference…` became `a_push_elsewhere_is_fetched_by_the_worker_and_then_reads_as_behind_and_matched`; the recreated-branch half of `a_deleted_then_recreated…` now expects the tracking ref fetched back. New: `an_in_sync_check_fetches_nothing`, `a_forced_worker_records_the_given_attempt_and_a_receipt_for_both_halves`.
- `perf_support`: `probe_head_refresh` → `head_lock_held`; `NoRequest` lost its `RemoteHeads` impl.
- **L2 (S4's third proposal, done now):** `level2_list_verbose::DesignFixture` panes now run with `GIT_CONFIG_NOSYSTEM=1`, an empty `GIT_CONFIG_GLOBAL`, and `protocol.http(s).allow=never`. Before this, the worker's `ls-remote` fallback went through the hanging proxy as a third connection (and, with a user `insteadOf`, could have reached github.com). `level2_list_stale_pr_answer…` now waits for 2 connections and both locks; it dropped from 12.4 s to 1.8 s.

### Checkpoint 3 — by hand (macOS, isolated `HOME`, local bare origin)

| Run | `answer` | `attempt` | Refs |
|---|---|---|---|
| `wt internal-refresh <main> --attempt 0011…eeff`, in sync | `source: git`, sha = tip | `phase: checking`, `outcome: in-sync`, `api: null` | unchanged |
| after a push: `… --attempt ffee…1100 --force` | `source: fetch`, sha = pushed | `phase: fetching`, `outcome: fetched` | `origin/main` = pushed, `main` unmoved, no `FETCH_HEAD` |

The forced run's receipt: `{ attempt_id: ffee…1100, branch: main, head: ok, prs: { kind: failed, failure: { kind: other } } }` (a local origin is no PR provider).

### Requirement → test mapping (Phase 3 scope)

| Requirement | Tests |
|---|---|
| `LC_ALL=C`, typed deadline, classifier | `the_classifier_maps_each_recorded_sample_and_defaults_to_other`, `a_fetch_past_its_deadline_is_a_typed_timeout`, `an_unauthorized_origin_is_a_typed_credentials_failure` |
| Fetch touches only `origin/<default>`, no `FETCH_HEAD`, rewind | `a_fetch_updates_only_the_one_tracking_ref`, `a_remote_rewind_is_applied`, `variance_fetches_and_publishes_the_fetched_tip` |
| Branch validated before any request | `an_invalid_branch_name_is_refused_before_any_request`, `an_invalid_default_branch_name_fails_the_check_without_a_request` |
| Reflog present/absent | `the_tracking_ref_reflog_dates_its_last_change_or_is_none` |
| No variance / variance / move between check and fetch | `no_variance_is_in_sync_with_no_fetch`, `variance_fetches…`, `a_remote_move_between_check_and_fetch…` |
| Fetch failure keeps the new answer; check failure keeps the old | `a_fetch_timeout_keeps_the_new_answer…`, `a_check_that_uses_the_whole_budget…`, `only_ls_remote_proves_absence` |
| One 10 s budget | `the_api_call_and_the_fallback_share_one_budget`, `a_check_that_uses_the_whole_budget…` |
| Unsupported → `ls-remote` in `checking`; fallback phases | `an_unsupported_remote_is_checked_by_ls_remote_in_phase_checking`, `each_fallback_reason_and_condition_is_recorded` |
| Ignored repository: no provider request (both halves) | `an_ignored_repository_makes_no_provider_request`, `an_ignored_repository_makes_no_pr_request`, `an_added_repository_is_ignored_by_any_of_its_origin_spellings` |
| Absence only from `ls-remote` | `only_ls_remote_proves_absence` |
| Origin/branch change → `unavailable` | `an_origin_or_default_branch_change_during_the_check…`, `an_origin_change_during_the_fetch…` |
| Concurrent fetch rules | `a_concurrent_fetch_that_reached_the_current_remote_head…`, `a_concurrent_fetch_of_an_older_head_is_not_labeled_current` |
| Contended lock writes nothing | `a_contended_lock_writes_nothing_and_asks_nothing` |
| Receipt after both halves; panics recorded; force | `a_receipt_is_written_only_after_both_halves_finish`, `a_panicking_half_is_recorded_as_failed_in_the_receipt`, `an_unforced_run_writes_no_receipt`, `a_forced_*`, `a_forced_worker_records_the_given_attempt_and_a_receipt_for_both_halves` (real binary) |
| Independence and panic isolation still hold | the 7 migrated `refresh_worker` tests |

### Gates

| Gate | Result |
|---|---|
| `just test` (worktree, macOS) | 590 passed, 21 skipped |
| `just lint` (worktree) | pass; `cargo clippy -p worktree -p worktree-cli --all-targets --all-features -- -D warnings` also clean |
| `just test-l2` (worktree, macOS) | 20 passed |
| `just check-tier-coverage worktree` | nothing stranded |
| Native Windows, lib (`live_remote remote_update remote_head api_preference pull_requests`) | 99 of 99 |
| Native Windows, CLI (worker, `list_prs`, `list_remote_head` filters) | 41 of 42; the one failure passes alone (below) |
| WSL2 (Ubuntu), same lib and CLI filters | 99 of 99, 42 of 42 |
| build-linux | not run: the standing clone's `target/release/deps/*.rmeta` are not writable (host environment; nothing compiled). WSL2 stands in for Linux here |

- **Windows flake, not a regression:** `list_prs::an_origin_change_during_a_foreground_request_shows_no_badges_from_the_old_origin` failed once in the parallel run (`requests() == 1`, expected 2: the second list's 300 ms foreground PR request never reached the server under load; the test took 9.6 s) and passed alone (4.6 s). The test is unchanged in this phase, and the foreground path is unchanged. It is the same 300 ms-under-Windows-load fragility the stale-store test already documents; Phase 4/5 should keep it in mind when the foreground wait lands.
- No pre-existing failure was skipped.

## Phase 4

Phase 4 wires the foreground: the flags, the fast-forward, the renderer, the wait, and the listing orchestration, then migrates the tests the always-launch flow broke and adds the binary-level proofs. A subagent wrote `worktree/lib/src/fast_forward.rs` (a disjoint lib file); the orchestrator did everything else, because the renderer, the wait, and the orchestration share one contract.

### Flags (`args.rs`, `main.rs`)

- `Cli` gains `refresh` (`-r`/`--refresh`), `ignore_api` (`--ignore-api`), and `fast_forward` (`--fast-forward`, visible alias `--ff`), all global. `main.rs::reject_listing_flags` refuses any of them with `create`, `go`, `remove`, or the hidden worker through `Cli::command().error(ArgumentConflict, …)` (exit 2), before dispatch. Every combination is accepted for listing (Rule 15).
- They reach `commands::list` as `ListFlags { refresh, ignore_api, fast_forward }`.
- `WorktreeError::NoRepositoryIdentity(reason)` (exit 1 through the default arm): `--ignore-api` with no `origin`, a local-path `origin`, or no home directory. Nothing renders.
- There was no flag-completion snapshot anywhere (the `wrapper_protocol` snapshots are the shell wrappers). A new one, `list_flags__global_flag_completions`, records `COMPLETE=fish wt -- wt --`. clap's dynamic completion offers `--fast-forward` but not its visible alias `--ff`; help lists both.

### Fast-forward (`worktree::fast_forward`, subagent)

- `fast_forward_default(main, default) -> FfResult { Moved { from, to, checkout }, UpToDate, Refused(FfRefusal) }`, with `FfRefusal { DirtyCheckout, Diverged, MissingLocal(name), MissingTracking(name), Changed, Other }`.
- It re-reads both refs and their ancestry right before the move. No holder: `update-ref -m "wt: fast-forward" refs/heads/<d> <new> <old>`. A holder: re-verifies `symbolic-ref -q HEAD` there, confirms the ref still equals `<old>`, and runs `merge --ff-only --no-autostash -q <verified sha>` under `LC_ALL=C`; "would be overwritten" is `DirtyCheckout`. A holder that left the branch is re-resolved once.
- Deviations: it merges the verified SHA rather than `origin/<d>` (a concurrent fetch cannot change what is merged). `--no-autostash` is added so `merge.autoStash` cannot stash the user's changes. Two checkouts holding the branch (`worktree add --force`) is `Refused(Other)`. The one unclosable gap (a checkout switching branch between the check and the merge) is documented in the module.
- Tests (13, `fast_forward::tests`): `moves_the_ref_when_no_checkout_holds_the_branch`, `a_concurrent_change_makes_the_compare_and_swap_refuse`, `fast_forwards_a_clean_checkout_holding_the_branch`, `refuses_when_local_changes_touch_files_the_update_changes` (bytes, HEAD, and refs unchanged), `re_resolves_a_branch_that_moved_to_another_worktree`, `re_resolves_to_a_ref_update_when_the_old_holder_left_and_none_took_it`, `refuses_a_diverged_branch_without_changing_refs`, `reports_up_to_date_when_in_sync`, `reports_up_to_date_when_the_local_branch_is_ahead`, `names_a_missing_local_branch_and_creates_none`, `names_a_missing_tracking_ref_and_changes_nothing`, `fast_forwards_a_default_branch_not_named_main`, `refuses_an_invalid_branch_name_without_running_a_mutation`.

### Rendering (`list_table.rs`, Rule 16)

- `RemoteFacts { default_branch, tracking_tip, status: RemoteStatus }` replaces the stored `answer`. **Deviation:** Rule 16 adds `remote_status: Option<RemoteStatus>` beside `remote`; keeping it inside `RemoteFacts` keeps "`None` ⇔ no `origin`" in one field.
- `RemoteStatus` has one variant per §4 row (`CheckedNow`, `Fetched`, `FetchFailed { reason }`, `StillChecking { last }`, `StillPulling`, `CheckFailed { reason, last }`, `Absent`); `LastKnown` is `Answer { checked_at }`, `TrackingRefChanged { at }`, or `Never`, as Unix seconds judged at render time (a future timestamp reads as `Never`).
- `TableFacts` gains `credential_line`, `unfinished`, `ff_suggestion`, `fallback_notice`, and (**addition**) `ff_notice: Option<FfNotice>` for `--ff` refusals, which the spec requires to be printed but Rule 16 did not place. They sit in the closing notes, before the §8 notice.
- `caption_markup(caption, remote, now)`: the comparison (`local origin/<d>` only in the fetch-failed and still-pulling rows), then `<dim><i>(…)</i></dim>`. Wording choices the spec left open: a Git credentials failure reads "origin didn't accept Git's credentials"; a fetch failure other than the timeout reads "fetch failed"; without a comparison the sentence is "origin/main (…)", or "No local tracking ref origin/main (…)", or, pruned and absent, "main was absent on origin when checked just now". The diverged row keeps the spec's `(N ahead, M behind)` and adds the suffix as a second parenthetical.
- `render` prints the caption, then the §5 line (dim), a blank line, the table, the legend, and the PR age line. `render_hint` (§6) and `render_notes` (`--ff` refusal or §9, then §8) are separate, and `assemble(Sections { table, graph, hint, verbose, notes })` fixes the order (Rule 16). Notes are `UnorderedList` items, so long lines wrap with a hanging indent.
- The old caption snapshots were replaced, not kept (`caption_observation_rows`, `caption_comparison_states`, `caption_fresh_and_stale_answers`, `caption_age_boundaries_and_future_answers` deleted; `caption_missing_refs_and_failed_comparison` and `caption_trunk_default_branch` rewritten). New: `caption_every_row_in_every_comparison_state` (9 rows × 4 states), `caption_reasons`, `caption_age_boundaries_and_future_timestamps`, `credential_lines_every_condition_for_every_provider` (5 conditions × 4 providers), `closing_notes`, `output_order` (with and without a graph, with and without `--verbose`). Tests: `the_caption_is_one_sentence_with_a_dim_italic_suffix` (checks the SGR 2 and 3 bytes before the suffix), `only_rows_that_could_not_update_the_ref_call_it_local`, `an_absent_branch_never_reads_as_in_sync_with_origin`, `the_credentials_line_is_dim_and_directly_follows_the_caption`, `the_hint_appears_only_with_unfinished_work`, plus the existing table tests.

### Wait and spinner (`list/wait.rs`, Rule 6)

- `wait(WaitRequest { main, origin_digest, branch, force, budget }, &dyn WaitEnv, WorkerLaunch, on_phase) -> Waited { end: WaitEnd, worker }`. `WaitEnd` is `Finished { attempt, receipt }`, `TimedOut { last }`, or `Unavailable`. `WaitEnv` abstracts the store, the receipt, both lock probes, the clock, and the id generator; `StoreEnv` is production.
- Rules as implemented:
    - The worker's exit is read **before** the store, so an outcome written just before exit is never missed.
    - Only the followed id's attempt is observed, and only while `is_current_for(digest, branch, now)`.
    - Adoption only after our own worker has exited without recording an attempt (the Phase 3 message: probing earlier could steal our worker's lock), and only while the head lock is held; otherwise `Unavailable` at once.
    - A worker that recorded its attempt and exited without an outcome (a failed store write) is `Unavailable` at once, not a 3 s wait.
    - Forced: after the outcome, waits for this token's receipt; `prs: contended` waits for the PR lock (probed only after our worker exited); a worker that exits with no receipt ends the wait (bounded publication failure). A receipt saying `adopted-elsewhere` follows the current attempt, or, when the holder is for another origin or branch, waits for the head lock and relaunches with a new token (same budget).
- Budgets: `ORDINARY_BUDGET` 3 s, `FORCED_BUDGET` = `ATTEMPT_MAX_AGE` (75 s), polled every 25 ms.
- `Progress` wraps the biscuit-terminal `Spinner` (150 ms delay); `phase_text` maps phases to the §3 texts.
- `refresh_worker::launch(main, &LaunchArgs { attempt, force }) -> io::Result<WorkerHandle>` now passes `--attempt` and `--force`; `WorkerHandle::from_child` polls `try_wait` and never joins or kills.
- Tests (21, `list::wait::tests`, fake clock that advances only on sleep): `our_attempt_is_followed_through_every_phase_to_its_outcome` (also asserts zero early lock probes), `a_running_attempt_is_adopted_with_no_second_launch`, `a_contender_that_exits_early_is_never_a_finished_check` (3 stores), `a_spawn_failure_is_unavailable_at_once`, `a_worker_that_stops_mid_attempt_is_unavailable`, `the_budget_ends_the_wait_with_the_last_phase_seen`, `an_outcome_for_another_attempt_branch_or_origin_is_never_taken` (id, branch, origin, expired), `a_forced_wait_follows_the_outcome_to_the_receipt`, `a_forced_wait_waits_for_a_contended_pr_half_to_be_released`, `a_forced_worker_that_exits_without_a_receipt_ends_the_wait`, `a_forced_wait_adopts_a_matching_attempt_its_worker_reported`, `a_forced_wait_relaunches_after_a_holder_for_another_branch_finishes`, `an_ordinary_wait_never_relaunches_for_another_branchs_holder`, `a_forced_attempt_that_fails_its_fetch_still_waits_for_the_receipt`, `the_spinner_text_follows_the_phase`, `the_spinner_writes_nothing_when_its_output_is_not_a_terminal`, `the_spinner_draws_the_phase_text_and_clears_its_line_on_a_terminal`, `a_launched_process_reports_its_exit`.

### List orchestration (`list.rs`)

- Order: `parse_worktree_state` (main checkout and default branch) → `gather_remote` (origin; `--ignore-api` records first; stored PR answer; a PR miss requests in the foreground **before** the launch; launch and wait; the spinner cleared; the PR answer re-selected, since the worker may have published one) → `--ff` → `WorktreeList::reread_refs` (new lib method) → the local gather with the graph thread → facts → `assemble` → one `eprint!`.
- **Deviation (ordering):** the PR miss request (≤ 300 ms) now runs before the launch instead of beside the git work, so the worker's PR half finds the answer and never repeats the request (Rule 20's request counts stay exact). Under `-r`/`--ff` and for an ignored repository the foreground makes no PR request at all.
- **Deviation (seams):** `ListSeams { connect, launch, wait_budget, forced_budget }` has no clock. The wait's clock is `WaitEnv`'s, and the pure `wait` tests inject it; `gather_remote` tests use real time with a 300 ms budget and stub launchers that write the real store.
- `tracking_ref_changed_at` runs lazily, only for a row that needs `LastKnown` and has no stored answer. §5 and §8 read only this run's followed attempt's `ApiNote`, this run's foreground PR failure, or the receipt's PR failure under `-r`/`--ff`. The §9 suggestion needs `CheckedNow` or `Fetched`, a caption strictly behind, and no `--ff`.
- `unfinished` = the wait timed out, or the PR answer is not fresh and a refresh is running: our worker is still alive, or (after it exited) someone holds the PR lock. New lib probe `pull_requests::pr_lock_held`.
- `render_verbose` returns a `String`, so every section is assembled before printing.
- Perf stages: `pr gather`, `remote wait` (replaces `remote select`), `fast-forward`.
- Tests (`list::tests::gather`, 11): `every_listing_with_an_origin_launches_once_and_waits_for_the_outcome` (git calls = the origin lookup only), `a_stale_answer_is_shown_and_the_parent_never_writes_it`, `a_stale_empty_answer_is_still_an_answer`, `a_pr_answer_the_worker_publishes_during_the_wait_is_shown`, `a_pr_miss_settles_before_the_worker_is_launched`, `a_failed_miss_shows_no_badges_stores_nothing_and_is_this_runs_failure`, `a_forced_listing_leaves_a_pr_miss_to_its_forced_worker`, `a_changed_origin_never_shows_the_old_answers`, `a_worker_that_cannot_launch_is_unavailable_without_a_wait`, `a_silent_worker_is_waited_for_only_until_the_budget`, `without_an_origin_stored_answers_are_ignored_and_nothing_is_requested_or_launched`. `list::tests::observations` (6): `every_ending_maps_to_its_caption_row`, `only_a_row_without_a_current_answer_asks_what_was_last_known`, `each_confirmed_condition_gives_its_line_with_the_key_used_or_the_keys_accepted` (10 cases, incl. a 404 with a key and a no-key fallback that answered), `a_pr_failure_this_run_observed_gives_a_line_only_for_a_confirmed_condition` (and none when nothing was observed this run), `a_credentials_line_names_variables_and_never_their_values` (sentinel `GITHUB_TOKEN` value), `the_fallback_notice_follows_only_a_no_key_fallback_that_answered` (never for a rate limit).

### Wave 3: migration and new binary-level tests

Fewer tests broke than S4 predicted (12, not about 25), because Phase 3 had already migrated the worker-side ones.

- **Shared fixture.** `list_remote_head.rs`'s local-origin `Fixture` moved to `cli/tests/remote_fixture/mod.rs`, shared with the new `list_flags.rs`. `remote_fixture::UploadPackGate` points `remote.origin.uploadpack` at a `sh` script that counts runs and holds the Nth (0: the check's `ls-remote`, 1: the fetch) until a release file appears; its drop releases. This is how "still checking", "still pulling", and adoption are proven against a local origin without HTTP.
- **`list_remote_head.rs`** (14): rewritten for the new caption and flow, plus new `a_listing_fetches_a_variance_and_counts_from_the_fetched_tip` (counts, caption, and §9 from the post-fetch snapshot; `FETCH_HEAD` absent; `feature` unmoved), `a_check_still_running_at_the_deadline_is_still_checking_and_the_next_run_shows_it` (dated previous answer, hint, < 5 s, next run "checked just now"), `a_fetch_still_running_at_the_deadline_is_still_pulling_and_publishes_after_the_listing` (one coherent pre-fetch snapshot, no §9, the worker fetches after `wt list` exited), `a_second_listing_adopts_the_running_attempt_and_asks_origin_nothing` (one `upload-pack` run for two listings), `an_origin_that_cannot_be_read_keeps_the_previous_answer_dated` (the bare repository moved away). `a_fetch_newer_than_the_observation…` became `a_manual_fetch_before_the_listing_is_checked_and_never_reported_as_a_move`. Every listing asserts that no spinner glyph, `\r`, or clear-line reached captured stderr.
- **`list_prs.rs`**: `a_fresh_pr_store…` and `a_stale_store_shows_its_badges_and_its_workers_failed_request_stores_nothing` count one and two worker connections. A scoped `close_after` thread closes the held connections as soon as they arrive, because sniff's provider client has a **3 s connect timeout**, which races the listing's 3 s wait against a hanging proxy (the old "lock still held after `wt list` returned" assertion failed that way). The held-PR detachment proof stays in the `FakeGitea` tests. `a_changed_origin_…_and_its_worker_stores_nothing`; `an_origin_change_during_a_foreground_request…` now expects the worker's request for the **new** origin (3 requests) and only a new-origin answer stored; `a_missing_or_stale_live_head_never_holds_up_the_listing` became `a_held_live_head_check_holds_the_listing_only_until_its_deadline` (reflog-dated still-checking row, hint, < 5 s, `.output()` returns while `HoldingOrigin` still holds the request).
- **`perf_pr_request.rs`** (the minimum to keep `just test` green; Phase 5 owns the gates and `performance-testing.md`): new `ProxyStub::closing_after(hold)` (counts, then drops each connection after `hold`). `…when_the_pr_request_hits_its_deadline` holds 400 ms (the foreground still hits its 300 ms deadline) and bounds the full command at 1 s + 300 ms, since the miss request now precedes the wait. `…with_a_stale_answer_and_a_blocked_refresh` became `…and_a_failing_refresh` (fresh runs make only the live-head connection). `perf_remote_select_stays_under…` became `perf_a_held_live_head_check_costs_the_listing_only_its_wait` (`remote wait` in [3 s, 3.3 s), full < 3 s + 1 s; the second listing adopts the first one's held attempt).
- **`list_flags.rs`** (new, 19 incl. 2 shared helper tests): parsing on `wt` and `wt list`; refusal with `create`/`go`/`remove` in both flag positions (exit 2, nothing created); help; completion snapshot; `refresh_waits_for_both_halves_and_asks_again_despite_young_answers` (`FakeGitea`: a young PR answer is re-requested, the branch head is re-checked, the new PR badge shows in the same run, the receipt exists before render, no worker left); `refresh_on_a_local_origin_fetches_and_reports_like_a_listing`; `refresh_is_bounded_when_the_worker_can_publish_nothing` (Unix; read-only store directory; < 5 s); `ignore_api::{the_repository_is_recorded_before_the_run_and_no_provider_is_asked, a_corrupt_file_ignores_nothing_and_is_never_overwritten, without_an_identifiable_origin_the_flag_fails_and_records_nothing}` (Unix only: native Windows resolves home without `HOME`; the lib's `%USERPROFILE%` test covers it); `fast_forward_*` for a checked-out branch (tree moves), an unheld branch (`update-ref`), uncommitted touched files (refused, bytes kept), diverged (refused, no suggestion), in sync and ahead (silent), a failed check (moves to the local `origin/main`, keeps "couldn't check origin"), and `--ff -r` (one check, one fetch, one move).
- **L2** (`level2_list_verbose::level2_list_styles_follow_the_design_in_tmux`): only the caption **wording** was updated so the L2 suite stays green; the dim-italic suffix, hint, and warning style assertions remain Phase 5's L2 task.

### Requirement → test mapping (Phase 4 scope)

| Requirement | Tests |
|---|---|
| Every listing launches or adopts one attempt, waits ≤ 3 s, gathers after | `gather::every_listing_with_an_origin_launches_once…`, `a_listing_fetches_a_variance_and_counts_from_the_fetched_tip`, `a_fetch_still_running_at_the_deadline_is_still_pulling…`, `perf_a_held_live_head_check_costs_the_listing_only_its_wait` |
| Adopt, never mistake a contender for a finished check | `wait::tests::a_running_attempt_is_adopted…`, `a_contender_that_exits_early…`, `a_second_listing_adopts_the_running_attempt_and_asks_origin_nothing` |
| Spawn failure / early exit → unavailable at once | `a_spawn_failure_is_unavailable_at_once`, `gather::a_worker_that_cannot_launch…`, `a_worker_that_stops_mid_attempt_is_unavailable` |
| Never another attempt's, branch's, or origin's outcome | `an_outcome_for_another_attempt_branch_or_origin_is_never_taken` |
| §4 rows × comparison states, reasons, reflog, never, absent (±tracking ref), non-`main` | `caption_snapshot_*` (5), `an_absent_branch_never_reads_as_in_sync_with_origin`, `every_ending_maps_to_its_caption_row` |
| Dim italic suffix, one sentence, "local" only where ruled | `the_caption_is_one_sentence_with_a_dim_italic_suffix`, `only_rows_that_could_not_update_the_ref_call_it_local` |
| §5 per condition × provider, names never values, this run only | `credential_lines_snapshot_…`, `each_confirmed_condition_gives_its_line…`, `a_pr_failure_this_run_observed…`, `a_credentials_line_names_variables_and_never_their_values` |
| §6 hint only when unfinished, after graph/PR line, before verbose | `the_hint_appears_only_with_unfinished_work`, `output_order_snapshot`, the still-checking/still-pulling binary tests |
| §8 notice only for a no-key fallback that answered, never rate limit | `the_fallback_notice_follows_only_a_no_key_fallback_that_answered`, `closing_notes_snapshot`, `output_order_snapshot` |
| §9 only after a completed check/fetch and strictly behind | `a_listing_fetches_a_variance…` (shown), still-pulling test (not shown), `fast_forward_refuses_a_diverged_branch_and_suggests_nothing` |
| Flags parse, refuse, help, completion | `list_flags::every_listing_flag_*`, `help_lists_every_listing_flag`, `completion_offers_every_listing_flag` |
| `-r` waits for both halves, forces young caches, bounded on failures | `refresh_waits_for_both_halves…`, `refresh_is_bounded_when_the_worker_can_publish_nothing`, `a_forced_*` wait tests |
| `--ignore-api` before the run, no provider request, corrupt file, no origin | `list_flags::ignore_api::*`, `gather::a_forced_listing…` (no foreground request) |
| `--ff` §9 table, `--ff -r` | `fast_forward::tests` (13), `list_flags::fast_forward_*`, `a_failed_check_fast_forwards_to_the_local_tracking_ref…` |
| Spinner never on a non-terminal; clears its line | `the_spinner_writes_nothing…`, `the_spinner_draws_the_phase_text_and_clears_its_line…`, `assert_no_spinner` in every real-Git listing |
| No worker outlives a fixture | `remote_fixture::Fixture` drop, `ReleaseOnDrop`, `wait_until_unlocked`; `pgrep -f internal-refresh` empty after the full runs |

### Gates (macOS)

| Gate | Result |
|---|---|
| `just test` (worktree) | 678 passed, 23 skipped (Phase 3: 590) |
| `just lint` (worktree) | pass |
| `just test-l2` (worktree) | 20 passed |
| `just check-tier-coverage worktree` | nothing stranded |
| `cargo check -p worktree-cli --features terminal-tests --tests` | clean |
| Leaked workers after the runs | none (`pgrep -f internal-refresh`) |

`schematic/`, `sniff/`, and `biscuit-terminal/` were not changed in this phase, so their gates were not rerun.

### Cross-OS (Phase 4)

| Host | Run | Result |
|---|---|---|
| native Windows | `./scripts/cross-check.sh --os windows worktree-cli` (whole package) | 366 of 367 on the first run; the failure is below, fixed, and re-verified |
| native Windows | `./scripts/cross-check.sh --os windows worktree fast_forward` | 14 of 14 |
| WSL2 (Ubuntu, nextest archive) | `./scripts/cross-check.sh --os wsl worktree-cli` | 384 of 384 |
| build-linux | not run; WSL2 stands in for Linux, as in Phase 3 | — |

- **Windows fix:** `list_prs::a_changed_origin_hides_the_stored_badges_and_its_worker_stores_nothing` asserted "couldn't check origin". Native Windows spends about 2 s on each refused loopback connection, so under parallel load the worker's check sometimes outlived the 3 s wait and the row read "still checking". Both rows are truthful, so the test now asserts only "; never checked with origin)". It passes alone and in the full run on macOS and Windows. The fact is recorded in `.claude/skills/os/windows.md`.
- A first Windows run with substring filters (`list_flags`, `list_remote_head`, …) selected only 110 tests, because nextest filters match test **names**, not binary names; the whole-package run above is the evidence.
- `remote_fixture::UploadPackGate`'s `sh` script works under Git for Windows (the gate tests passed there).
- Only `worktree/` changed in this phase, so the other three areas' gates were not rerun.

## Phase 5

Phase 5 closes the fix: the performance gates, the L2 styling and spinner proofs, the documentation, a drift pass, cross-OS runs, and the final gates. A subagent wrote the `list.md` and README changes; the orchestrator did everything else and reviewed that diff against the code.

### Performance gates (Rule 19)

- **1 s bound with a worker that answers at once.** `perf_command_sla::perf_full_command_non_image_meets_sla` previously ran on a fixture with **no `origin`**, so no worker ran at all and it measured the pre-fix path. It now uses `MixedFixture::with_local_origin()` (new; a bare copy beside `HOME`, fetched, so the check answers from local Git and finds nothing to fetch) and `wt_command_direct()`. Every run must print `main is in sync with origin/main (checked just now)`, so a run that skipped or outwaited the worker cannot pass.
- **Held check and held fetch, ordinary listing.** The held check was already `perf_a_held_live_head_check_costs_the_listing_only_its_wait` (`HoldingOrigin`). New: `perf_a_held_fetch_costs_the_listing_only_its_wait` (`remote_fixture::Fixture` plus `UploadPackGate` holding run 1, the fetch): `remote wait` in [3 s, 3.3 s), full < 4 s, the row reads "pulling remote updates in the background", and, after `.output()` returned, the worker process still runs and `upload-pack` has started exactly twice (the no-join proof).
- **`-r` and `--ff` against a held origin.** `perf_refresh_against_a_held_check_reports_within_the_check_deadline` (`HoldingOrigin`, `wt list -r`): elapsed in [10 s, 13 s), "origin didn't answer within 10 s". `perf_fast_forward_against_a_held_fetch_reports_within_the_fetch_deadline` (`UploadPackGate` on the fetch, `wt --ff`): elapsed in [60 s, 63 s), "fetch didn't finish within 60 s", `main` unmoved, two `upload-pack` runs. Both import the deadlines from the library (`REMOTE_HEAD_REFRESH_DEADLINE`, `FETCH_DEADLINE`), so a changed deadline changes the bound.
- **nextest override (config change).** The 60 s test exceeded the default 30 s termination (`5s × 6`), so `.config/nextest.toml` gains one `[[profile.default.overrides]]` for exactly that test (`30s × 3`). The CI profile's global ceiling is already 90 s. `perf_` tests run only in `just test-perf` (serially), never in `just test`. `just ci-local --plan` classifies the file as configuration and schedules nothing for it.
- `worktree/docs/performance-testing.md`: the PR and Live Remote Head sections rewritten for the always-launch/wait flow (the old ones described "listing makes one launch decision", `remote select`, and a test that no longer exists); a new "full-command contract" table (1 s local gather plus render; 3 s plus 1 s stalled; 10 s and 60 s for `-r`/`--ff`); a 2026-09-27 measurement table. `hash` frontmatter refreshed with `md hash`.

Measured (`just -d worktree test-perf`, run alone, 26 passed):

| Gate | Result |
|---|---|
| Worker answering at once, full `wt list` (best of 5) | 143.4 ms (1 s) |
| Held check: `remote wait`, full (two listings) | 3.00 s, 3.11–3.13 s |
| Held fetch: `remote wait`, full | 3.00 s, 3.10 s |
| `-r`, held check | 10.19 s |
| `--ff`, held fetch | 60.20 s |
| Network down: cold / warm `list gather`, full | 22.1 ms / 12.1 ms / 161.4 ms |
| PR request past its deadline: full | 886.7 ms (1 s + 300 ms) |
| Fresh / stale answer, full; stale `pr gather` | 154.0 / 150.0 ms; 4.1–4.3 ms |

### L2 (`level2_list_verbose.rs`)

- **Styling as the spec rules it.** The plan's task reads "dim italic on the caption suffix, the hint, and a credentials warning". The spec makes only the caption suffix dim italic (§4); §5 and §6 are "one dim line", and the code renders them `<dim>` only. The tests assert the spec: suffix `dim && italic` (first and last word, across the wrap), the comparison word `behind` neither; warning and hint `dim && !italic`.
- **Credentials warning, deterministic.** Per the Phase 4 message, a seeded `ApiNote` is overwritten by the run's own attempt, so the run's own worker must observe it. `DesignFixture::with_gitea_origin()` (new; `build` takes the origin) points at `FakeGitea`, whose `/branches/` 404 without a key plus the refused `ls-remote` fallback gives `NotVisible`. `level2_list_credentials_warning_is_a_dim_line_beneath_the_caption_in_tmux` asserts the exact Gitea line naming `GITEA_TOKEN`, that it is the row directly after the caption's last row and before the table, no hint (the worker finished), no `GITEA_TOKEN=`, no PR request, and no worker left.
- **Spinner cleared, hint dim, deterministic.** A hanging HTTPS proxy races sniff's 3 s connect timeout against the 3 s wait (the Phase 4 message), so the check is held on plain HTTP instead: new `FakeGitea::hold_branch_heads()` holds branch-head requests until `release` (or drop). `level2_list_clears_the_spinner_before_the_caption_and_shows_a_dim_hint_in_tmux` uses the new `DesignFixture::start_in_pane` to see the pane **during** the wait (a spinner frame and "updating" must be visible, so the test cannot pass without a spinner), then asserts the finished pane has no frame glyph and no "updating", the caption starts its own row, the row is "still checking in the background", and the hint row after the legend is dim. It releases the hold and waits for the worker and the lock.
- The pane now also sets `HTTP_PROXY` (to the same stand-in) and unsets `GITEA_TOKEN`, `FORGEJO_TOKEN`, and `CODEBERG_TOKEN`.
- Run: `BISCUIT_TEST_REQUIRED_BACKENDS=tmux cargo nextest run -p worktree-cli --features terminal-tests -E 'binary(level2_list_verbose)'`: 11 of 11. tmux is headless; no window gained focus.

### Documentation

- `worktree/docs/cli/list.md` (subagent, reviewed): new "Checking origin" (API then `ls-remote`, 10 s; fetch only on variance with the **actual** `fetch_argv`, including `--no-recurse-submodules --refmap=`, 60 s; 3 s wait and spinner texts; gather after), the §4 table with the code's wording, reasons, last-known, missing refs; §5 table; §6 hint; closing notes (§9, `--ff` refusals, §8); the three flags and `~/.wt.json`. "never fetches" removed.
- `worktree/README.md` `wt list`: listing checks and fetches, one-sentence caption, notes, flags.
- `.claude/skills/worktree/SKILL.md`: already current from Phase 4 (store format 2, attempt and receipt, worker flow, wait/adopt, flags, fixtures). Added the new perf gates, `MixedFixture::with_local_origin`, `FakeGitea::hold_branch_heads`, `DesignFixture::with_gitea_origin`/`start_in_pane`, and the two new L2 tests.
- No OS fact was learned in this phase (S3's were recorded in Phase 1 and the Windows loopback fact in Phase 4), so `.claude/skills/os/` is unchanged.
- `docs/dependencies.md`: no crate was added on this branch (`git diff main -- '*Cargo.toml'` is empty); unchanged.

### Drift and comment pass

Reviewed the `//!`/`///` docs of `remote_head`, `live_remote`, `pull_requests`, `refresh_worker`, `list` (`gather_remote`, `remote_status`, `credential_line`, `fallback_notice`, `unfinished`), `list_table`, `remote_update`, and sniff `blocking` (`branch_head`, `branch_head_with`). Drift found, code taken as correct:

- `worktree::remote_head` module doc said the store lets "the caption say whether that tracking ref still matched the remote and how long ago that was checked", the old two-sentence model. Rewritten: the worker fetches on variance, and the store lets the listing follow its own attempt and date the last answer when the attempt has none.
- `worktree::default_target` module doc said "`wt` never fetches". Rewritten: `origin/<default>` is as of the last fetch, `wt list`'s own (only that ref, only on variance) or the user's.
- `worktree/docs/performance-testing.md` described the removed launch decision, the `remote select` stage, and a deleted test; rewritten (above).
- No other "never fetches" or "never waits" statement remains: `refresh_worker`'s "launches and never joins" is still true (the wait polls the store and `try_wait`, and never joins or kills).

### Cross-OS validation

| Host | Run | Result |
|---|---|---|
| native Windows (`build-win-native`) | `./scripts/cross-check.sh --os windows worktree-cli` | 367 of 367 (44 skipped) |
| native Windows | `./scripts/cross-check.sh --os windows worktree` | 276 of 276 |
| WSL2 (`build-win`, nextest archive) | `./scripts/cross-check.sh --os wsl worktree-cli` / `worktree` | 384 of 384 / 295 of 295 |
| build-linux | `./scripts/cross-check.sh --os linux …` | **not run**: the same host defect as Phases 3 and 4. The standing clone's `target/release/deps/*.rmeta` are not writable, so nothing compiles. WSL2 stands in for Linux |

The Windows runs cover the lock and polling (`wait` tests, `list_remote_head`, `list_flags`), `try_wait` on a detached child (`a_launched_process_reports_its_exit`, every listing test), `%USERPROFILE%` resolution (the `api_preference` lib test), and `LC_ALL=C` on Git for Windows (`fast_forward` and `live_remote` lib tests). No red result. The `perf_` tests were not run off macOS (`just test` and `cross-check` exclude them by design).

### Requirement → test mapping (Phase 5 scope)

| Requirement | Tests |
|---|---|
| Local gather plus render ≤ 1 s with an immediately answering worker | `perf_command_sla::perf_full_command_non_image_meets_sla` |
| Held check: ≤ 3 s + 1 s, `.output()` returns while held | `perf_a_held_live_head_check_costs_the_listing_only_its_wait`; `list_prs::a_held_live_head_check_holds_the_listing_only_until_its_deadline` |
| Held fetch: ≤ 3 s + 1 s, `.output()` returns while held | `perf_a_held_fetch_costs_the_listing_only_its_wait`; `list_remote_head::a_fetch_still_running_at_the_deadline_is_still_pulling_and_publishes_after_the_listing` |
| `-r` against a held origin within the 10 s check deadline | `perf_refresh_against_a_held_check_reports_within_the_check_deadline` |
| `--ff` against a held origin within the 60 s fetch deadline | `perf_fast_forward_against_a_held_fetch_reports_within_the_fetch_deadline` |
| Caption suffix dim italic in a real terminal | `level2_list_styles_follow_the_design_in_tmux` |
| §5 warning dim, directly beneath the caption | `level2_list_credentials_warning_is_a_dim_line_beneath_the_caption_in_tmux` |
| §6 hint dim, after the legend, only while unfinished | `level2_list_clears_the_spinner_before_the_caption_and_shows_a_dim_hint_in_tmux` (shown), the credentials test (absent) |
| Spinner drawn on a terminal and cleared before the caption | `level2_list_clears_the_spinner_before_the_caption_and_shows_a_dim_hint_in_tmux` |

### Final gates (macOS)

| Gate | Result |
|---|---|
| `just test` (worktree) | 678 passed, 26 skipped |
| `just lint` (worktree) | pass |
| `just test-l2` (worktree, `BISCUIT_TEST_REQUIRED_BACKENDS=tmux`) | 22 passed |
| `just test-perf` (worktree) | 26 passed |
| `just test` / `just lint` (schematic) | 1708 passed / pass |
| `just test` / `just lint` (sniff) | 2872 passed / pass |
| `just test` / `just lint` (biscuit-terminal) | 3324 passed / pass |
| `just check-tier-coverage worktree` | nothing stranded |
| Leaked workers after all runs (`pgrep -f internal-refresh`) | none |
| `just ci-local --plan` | 36 execute cells; worktree-cli L2 on ubuntu and macOS, the Windows and WSL2 L2 cells an accepted gap; `.config/nextest.toml` schedules nothing |

No pre-existing failure was skipped, and nothing failed.

### Checkpoint 5

Every Definition of Done item is checked. The spec's lifecycle directory is untouched: the spec stays in `fixes/`, not `_completed/`. Its frontmatter is updated only as this phase's instructions require (`implemented`, `human_review`, `message_to_agent`).

**Final state: implementation complete, ready for review.**

## Implementation of Review Findings #1

> **started at:** 2026-09-27T12:36:00-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/fix-wt-ux/worktree/fixes/2026-09-27-list-freshness-ux/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- starting the work on 'Forced refresh can finish without a pull request result after lock contention' at 12:36:09
        - confirmed: `Follow::run` accepted a `Contended` receipt once `pr_lock_held()` was false; `pull_requests::refresh` stores nothing on failure or `AlreadyFresh`, so a failed/skipped holder left `wt -r` showing the old answer
        - design: a contended PR half counts only if the PR store (current origin) then holds an answer with `fetched_at` newer than at launch (new `WaitEnv::pr_fetched_at`, `StoreEnv` now takes the origin); otherwise relaunch the forced worker once within the same budget; a second contended-without-answer ends as `PrStatus::Failed { Other }` in the returned receipt. "Newer than at launch" rather than ">= launch time" so a holder that began its request just before `wt -r` still counts
        - unit tests (`list/wait/tests.rs`): replaced the elapsed-only contention test with holder-published, holder-published-nothing (one relaunch), and relaunch-contended-too (PR failure) cases
        - binary tests (`cli/tests/list_flags.rs`): `refresh_shows_the_answer_a_contending_holder_published`, `refresh_asks_again_when_a_contending_holder_failed` (a real `internal-refresh` holder held at `FakeGitea`; the second fails without the fix: 1 request instead of 2)
        - updated `wait.rs` module/`WaitEnd` docs and the worktree skill's `wt list` paragraph
        - `just test` (684 passed), `just lint` clean
- work completed for 'Forced refresh can finish without a pull request result after lock contention' at 12:41:13
- starting the work on 'Spinner phase changes lack real-terminal verification' at 12:41:13
        - discovery: holding a fallback `ls-remote` and then a fetch in one pane needs an origin sniff recognizes as Gitea (`http://gitea.test/o/r.git`) *and* a real git transport; `url.*.insteadOf` cannot bridge them (`git remote get-url` expands it, so provider detection would see the local path), and `remote.origin.uploadpack` does not apply to HTTP
        - `cli/tests/perf_support`: `FakeGitea` now serves git smart HTTP through `git http-backend` (`serve_repositories(root)`), holds git requests with `hold_git(GitHold::All | Fetch)` (`Fetch` holds only the pack request, so an `ls-remote` passes), counts them apart (`git_requests`, `wait_for_git_waiting`), and can answer branch heads with another status (`answer_branch_heads_with(429)`); `release` also clears the git hold
        - `cli/tests/level2_list_verbose.rs`: `DesignFixture::with_gitea_repository` (bare `o/r.git` one commit past `origin/main`, the commit made only in the bare repo so the fetch sends a pack; the pane's git gets `http.proxy` at the stand-in instead of refused HTTP); new scenes under `wt list -r`: `level2_list_spinner_moves_from_the_fallback_to_the_fetch_on_one_line_in_tmux` (the `no API key, using fallback method` line with `ls-remote` held, then `pulling remote updates` with the fetch held, each exactly one `<frame> <text>` line with no other spinner text, then cleared before the caption, which shows the fetched state `main is 2 commits behind`) and `level2_list_spinner_shows_the_rate_limited_fallback_in_tmux` (429 on branch heads)
        - mutation check: dropping the erase-to-end-of-line `\x1b[K` from biscuit-terminal's spinner `frame` makes the transition scene fail (the fetch line keeps the fallback text's tail); reverted
        - updated the module doc and the worktree skill's `wt list` L2 paragraph
        - `just test-l2` 24 passed (both new scenes ran, about 2 s each, not skipped), `just test` 684 passed, `just lint` clean, `just check-tier-coverage` nothing stranded
- work completed for 'Spinner phase changes lack real-terminal verification' at 12:51:00
- starting the work on 'Ignored API preferences merge distinct SSH ports' at 12:51:00
        - policy: SSH to a known provider maps to 443 only on the standard SSH port (none spelled, or explicit 22); an explicit nonstandard SSH port is kept, so it differs from HTTPS and from other ports. sniff's `remote_identity` already reports an explicit `ssh://` port (the `url` crate knows no default for `ssh`), so sniff is unchanged
        - no scheme field added (format stays 1): an SSH and an HTTPS remote spelling the same explicit host and port name one server
        - added `a_known_provider_on_a_nonstandard_ssh_port_keeps_that_port` and `an_ignored_nonstandard_ssh_port_survives_a_round_trip_without_ignoring_other_ports`; the existing sharing test already covers `git@`, `ssh://` without a port, and `:22`
        - updated `RepoIdentity` / `from_origin` docs and the worktree skill's `api_preference` bullet
        - `just test` 686 passed, `just lint` clean
- work completed for 'Ignored API preferences merge distinct SSH ports' at 12:52:43
- starting the work on 'A failed remote check hides an available fast-forward suggestion' at 12:52:43
        - gate in `list.rs`: suggestion follows the post-wait comparison under every `RemoteStatus` except `StillChecking`/`StillPulling` (a timed-out check may still lead to a fetch, so both count as "rendered before the fetch finished"); check-failed, fetch-failed, unavailable, and absent-with-tracking-ref now suggest when strictly behind
        - `UploadPackGate::failing(fixture, n)` in `remote_fixture`; added `a_failed_check_still_suggests_…` and `a_failed_fetch_still_suggests_…` to `list_remote_head.rs` (the still-pulling no-suggestion test is unchanged)
        - updated `docs/cli/list.md` closing-notes wording and the worktree skill's `render_notes` note
        - `just test` 688 passed (26 skipped), `just lint` clean
        - gate: the suggestion follows the post-wait comparison (strictly behind, never under `--ff`) unless the listing rendered while the check was still running or the fetch was still pulling; checked-now, fetched, check-failed, fetch-failed, unavailable, and absent-with-tracking-ref all suggest when strictly behind
        - no L1 test exercises the still-checking exclusion with a behind branch: the existing still-checking test uses an in-sync branch
- work completed for 'A failed remote check hides an available fast-forward suggestion' at 12:57:58
- final verification (orchestrator)
        - `worktree/` `just test`: 688 passed, 26 skipped; `just lint`: clean; `just check-tier-coverage worktree`: 0 stranded
        - `just cross-check worktree-cli --os windows`: 375 passed, 46 skipped (native Windows)
        - `just test-l2` (24 passed) was run by the spinner-scene subagent after its change; the later changes touched no L2 test or tmux fixture

### Successful Completion

The implementation of review cycle 1 has completed successfully in 22 minutes. During this implementation all 4 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 4 were fixed, 0 were deferred (see reasons below):

- no findings were deferred

The files changed in this cycle are:

- `worktree/cli/src/commands/list.rs`
- `worktree/cli/src/commands/list/wait.rs`
- `worktree/cli/src/commands/list/wait/tests.rs`
- `worktree/cli/tests/level2_list_verbose.rs`
- `worktree/cli/tests/list_flags.rs`
- `worktree/cli/tests/list_remote_head.rs`
- `worktree/cli/tests/perf_support/mod.rs`
- `worktree/cli/tests/remote_fixture/mod.rs`
- `worktree/lib/src/api_preference.rs`
- `worktree/docs/cli/list.md`
- `.claude/skills/worktree/SKILL.md`

## Implementation of Review Findings #2

> **started at:** 2026-09-27T16:23:23-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/fix-wt-ux/worktree/fixes/2026-09-27-list-freshness-ux/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- starting the work on 'Forced refresh mistakes a same-second pull request update for no update' at 16:23:40
        - decision: every successful PR-store write now stamps a random `publication` id (`remote_head::new_attempt_id`, 32 hex) beside `fetched_at`, whose meaning (request start, whole seconds, shown as the age) is unchanged
        - decision: bumped `PR_STORE_FORMAT_VERSION` 2 -> 3 rather than an optional field, so every served answer has an id; an old store is a `Miss` and is refilled once (no users, so no migration)
        - both writers stamp a fresh id through one private `publish` (foreground `fetch_and_publish`, worker `refresh`); no OS randomness means nothing is stored (`refresh` returns `PublishFailed`, the foreground still shows its answer)
        - added `pull_requests::stored_publication(store, origin, now)`, which reads the id under exactly `select_cached`'s validity rules (shared private `usable`)
        - `WaitEnv::pr_fetched_at` replaced by `WaitEnv::pr_publication`; a contended holder counts as published when the id after its lock opens is present and differs from the id at launch
        - Level 1 tests added: `pull_requests::every_publication_stores_a_new_id_even_within_one_second`, `a_publication_id_is_read_only_where_the_answer_would_be_served`, a format-2 store as a `Miss`; `wait::a_forced_wait_accepts_a_holders_answer_published_within_the_same_second`, `a_forced_wait_accepts_a_holders_first_answer`; existing relaunch tests now script ids; `list_flags::refresh_shows_a_holders_answer_that_replaced_a_young_one_within_its_second`
        - the binary test seeds its young answer after the holder's request is held, so the seeded `fetched_at` is at or after the holder's stamp every time; with the old `fetched_at` comparison patched back in temporarily, it failed (two requests), and it passes with the fix
        - discovery: `cargo test` (shared process) intermittently reports `Contended` in the `pull_requests` refresh tests while nextest passes; it looks like lock fds inherited by other threads' git children. This predates the change and nextest is the repo's runner
        - PR store JSON fixtures (`perf_support::seed_pr_store`, `seed_empty_pr_store`, `level2_list_verbose` design fixture) now write `PR_STORE_FORMAT_VERSION` and a publication id
        - docs: the `pull_requests` and `wt list` wait sentences in `.claude/skills/worktree/SKILL.md` now describe format 3, `stored_publication`, and `WaitEnv::pr_publication`; `worktree/docs/cli/list.md` does not describe the contended-holder check, so it is unchanged
        - verification: `just test` 695 passed, 26 skipped; `just lint` clean; `just check-tier-coverage worktree` 0 stranded
        - blocker (outside this finding): `just test-l2` fails `level2_list_styles_follow_the_design_in_tmux` at line 637. `row_with` searches from the bottom and now matches the §9 note `main is 1 commit behind origin/main; run wt --ff ...`, which the uncommitted §9 change in `list.rs` prints after a failed check, rather than the caption. The PR store fixture is not involved: the caption and `PR #99` render correctly. The other 23 L2 tests pass when run serially (`-j 1`, tmux required)
        - orchestrator: resolved the L2 blocker. The failure was a test-matching regression from review cycle 1's §9 change, not a product defect: the caption lookup in `level2_list_styles_follow_the_design_in_tmux` now also requires `(`, which the caption carries and the `--ff` suggestion does not. `just test-l2` 24 passed; `just lint` clean
        - orchestrator: `just cross-check worktree-cli --os windows` passed (native Windows)
- work completed for 'Forced refresh mistakes a same-second pull request update for no update' at 16:37:02

### Successful Completion

The implementation of review cycle 2 has completed successfully in 14 minutes. During this implementation all 1 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 1 were fixed, 0 were deferred (see reasons below):

- no findings were deferred

The files changed in this cycle are:

- `worktree/lib/src/pull_requests.rs`
- `worktree/cli/src/commands/list/wait.rs`
- `worktree/cli/src/commands/list/wait/tests.rs`
- `worktree/cli/tests/list_flags.rs`
- `worktree/cli/tests/perf_support/mod.rs`
- `worktree/cli/tests/level2_list_verbose.rs`
- `.claude/skills/worktree/SKILL.md`

## Implementation of Review Findings #3

> **started at:** 2026-09-27T19:47:38-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/fix-wt-ux/worktree/fixes/2026-09-27-list-freshness-ux/review-3.md'
- this is iteration 3 of the review-to-implement cycle
- starting the work on 'An ordinary listing can replace a newer pull request answer' at 19:47:50
        - defect class: a shared store with more than one writer and no common ordering rule, so a writer holding an older answer (or an older snapshot) can replace newer data, and a reader that treats "the store changed" as proof that one particular writer succeeded can be fooled by another writer
        - design: every PR store write now happens under the persistent PR refresh lock. `refresh` already held it from the freshness recheck through publication; `fetch_and_publish` (the ordinary listing's 300 ms miss request) now reads the store's publication id before its request, then takes the lock nonblocking only to publish, and publishes only when the lock is free and the publication id is still the one it read. Contended, unopenable, or replaced: it stores nothing and returns its own answer for this run (which `gather_remote` replaces with the store's after the wait). The request itself stays unlocked and bounded, so a foreground miss never blocks or is blocked by a worker
        - design: each stored answer now records its writer (`writer: refresh | listing`, store format 4). The foreground writer can still publish while the lock is momentarily free (between `wt -r`'s launch and the holder taking the lock, or between the holder's release and the wait's reread), so "no unlocked writes" alone does not make the contended check sound; `WaitEnv::pr_publication` now returns only a refresh-written publication id, so a listing's answer can never read as the holder's success
        - store format 4 (`writer` added; a format-3 file reads as a miss, which costs one foreground request); test seeders in `cli/tests/perf_support/mod.rs` and `cli/tests/level2_list_verbose.rs` now write `"writer": "refresh"`
        - tests added (L1): `pull_requests::tests::a_foreground_answer_never_replaces_one_published_during_its_request` (a forced `refresh` publishes inside the listing's request; the listing's answer is shown but the refresh's survives), `…::a_foreground_answer_never_replaces_a_listings_answer_published_during_its_request`, `…::a_foreground_answer_is_shown_but_not_stored_while_a_refresh_holds_the_lock`; `a_refresh_that_cannot_open_its_lock_makes_no_request` now also covers the listing writer; `list::tests::…::a_miss_answer_overtaken_by_a_forced_worker_leaves_the_newer_answer_shown_and_stored` (through `gather_remote`: badges and store both keep the worker's PR); `list::wait::tests::a_forced_wait_never_takes_a_listings_answer_for_a_contending_holders` (real PR store: the holder's forced refresh fails inside an overlapping listing's request, the listing stores its answer, the wait relaunches once and ends `PrStatus::Failed { Other }`) and its control `…::a_forced_wait_takes_a_contending_holders_answer_that_an_overlapping_listing_left_alone`. Ordering is by nesting the writers inside the stub request, with no threads or sleeps
        - fail-before check: removing the unchanged-id check in `fetch_and_publish` fails the two library ordering tests and the wait control; removing the writer filter in `refresh_publication` fails the contended-wait test
        - docs: `.claude/skills/worktree/SKILL.md` (`pull_requests` format 4, the ordering rule, the listing writer, and the refresh-only publication check); `worktree/docs/cli/list.md` (overlapping listings keep the newer answer); `worktree/docs/performance-testing.md` (the foreground request never waits for the lock)
        - class sweep: a shared store with more than one writer and no common ordering rule, so a writer holding an older answer or snapshot replaces newer data, or a reader takes "the store changed" as proof a particular writer succeeded; sites checked: PR store (`refresh`, `fetch_and_publish`), PR publication readers (`WaitEnv::pr_publication`, `gather_remote`'s post-wait reread), remote-head store (`begin_attempt`, `set_phase`, `finish_attempt`, `publish_answer`, all only from `remote_update::run_attempt` under `<repo hash>.remote-head.lock`) and its readers (the wait follows its own attempt id), completion receipt (`write_receipt`, `load_receipt` keyed by attempt id, origin, and branch), `~/.wt.json` (`api_preference::add`, rereads under its lock), `~/.worktree.json` (`save_config`, one user-chosen value), comparison cache (`Cache::save_atomic` from a listing's snapshot), fork-origin store (`fork_origin::record` from `wt create`, the listing's prune), copy records (`create_worktree`'s write, the listing's `copy_record::prune`), hand-off records (one file per token); fixed here: PR store writers (one lock-plus-unchanged-publication rule), the contended forced wait's publication check (refresh-written only), the listing's fork-origin prune (saved its pre-wait snapshot and judged by a pre-wait ref read, so a `wt create` during the up-to-75 s wait lost its record), the listing's copy-record prune (judged by its pre-wait `worktree list`, so a worktree created during the wait lost its copy baseline); clean: remote-head store and its readers, `~/.wt.json`, `~/.worktree.json`, hand-off records, comparison cache (a lost entry is a pure function of its SHA-pair key, so it costs only a recomputation, never a wrong answer)
        - fork-origin fix: the listing now prunes the file as reread at prune time, and a record created at or after its ref read (`WorktreeList::refs_read_at`, set by `parse_worktree_state` and `reread_refs`) is dropped only when `git rev-parse --verify --quiet refs/heads/<branch>` finds no branch, so only that rare record costs a git call (`cli/tests/list_output.rs::create_from_records_the_parent_that_list_draws` creates and deletes a branch within one second and still sees it pruned). `ForkOriginStore::prune` now takes an `exists(branch, record)` predicate; after pruning, `WorktreeList::fork_origins` returns the pruned file. Test: `listing::repo_tests::a_fork_record_written_while_the_listing_waits_survives_its_prune`
        - copy-record fix: `copy_record::prune_from` deletes a record only when its worktree is missing from the listing **and** its admin directory no longer holds the record's marker (Git deletes that directory on `worktree remove`/`prune`). `separate_worktree_records_survive_concurrent_writes_and_prune` now actually removes the worktree it expects pruned, since a registered one is kept by design. Test: `copy_record::tests::a_record_for_a_worktree_missing_from_the_listing_survives_while_registered`. Both new tests fail with their fix reverted
        - not in this class, left as is (Minor): the completion receipt is one slot per repository, so two overlapping forced attempts can overwrite each other's receipt before the first run reads it. Readers never take a foreign receipt as theirs, so this can't produce a false success; the run that lost its receipt ends `Finished { receipt: None }` and skips its contended-holder check and its PR-failure line. Fixing it needs per-attempt receipts with cleanup, a separate change
        - cross-OS: the lock is the existing `fs4` nonblocking exclusive sidecar lock (`try_lock_sidecar`), already used by both halves and `~/.wt.json` on every OS; semantics are unchanged, the listing writer is just a new caller. `just cross-check worktree --os windows` 285 passed; `just cross-check worktree-cli --os windows` 386 passed, 46 skipped
        - verification: `just test` (worktree/) 706 passed, 26 skipped; `just lint` clean; `just check-tier-coverage worktree` 0 stranded; `just test-l2 level2_list` 11 passed (the L2 seed now writes `writer`). Every new test is an ordinary L1 name in the lib/bin unit-test targets
