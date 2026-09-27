---
total_phases: 5
created: 2026-09-27
phase: 3
agent: claude/opus
yolo: true
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
packages:
    - schematic-definitions
    - schematic-schema
    - sniff
    - biscuit-terminal
    - worktree
    - worktree-cli
---

# Plan: `wt list` should know, not guess, whether `origin/<default>` is current

Spec: `2026-09-27-list-freshness-ux` (`spec.md` in this directory). Related: `2026-09-26-stale-remote-caption`.

## Summary and Definition of Done

### Work required

Today `wt list` shows a background `ls-remote` answer from an earlier run and never updates `origin/<default>`. After this fix, every listing checks the remote, fetches the one tracking ref when it differs, waits up to 3 s for that work, and renders a one-sentence caption that states what it actually knows. The work spans four package areas:

1. **schematic.** Add one branch-head endpoint per provider (GitHub `GetBranchReference`, Gitea/GitLab/Bitbucket `GetBranch`) to `schematic-definitions`, then regenerate `schematic-schema`.
2. **sniff.** Add `remote::blocking::branch_head`, `BranchHead`, and `credential_env`. Split `PrUnavailable::Auth` into the confirmed credentials variants `CredentialsRequired`, `CredentialsRejected`, `CredentialsInsufficient`, and `RateLimited { authenticated }`, and keep `NotFoundOrNotPermitted` ambiguous.
3. **biscuit-terminal.** Add a `Spinner` component. It writes to stderr only, only when stderr is a terminal, starts after a delay, accepts replacement text, and clears its line.
4. **worktree (lib and CLI).**
    - Remote-head store format 2, with an `attempt` record and a completion receipt.
    - A worker update flow: provider check, then `ls-remote` fallback, publish, then fetch on variance.
    - A foreground launch/adopt/wait with the spinner, and local facts gathered *after* the wait.
    - The new caption, credentials warning, refresh hint, fast-forward suggestion, and fallback notice.
    - The `-r/--refresh`, `--ignore-api` (`~/.wt.json`), and `--ff/--fast-forward` flags.
    - The fast-forward itself.
    - A revised performance contract.

Code locations this plan relies on (verified 2026-09-27):

| Concern | Location |
|---|---|
| Provider definitions (no branch endpoints yet; GitHub has `GetTagReference`) | `schematic/definitions/src/{github,gitea,gitlab,bitbucket}/mod.rs` |
| Generated clients; path params already percent-encoded via `urlencoding` | `schematic/schema/src/*/requests.rs`, `schematic/gen/src/codegen/request_structs/shared.rs` |
| PR lookups, `classify`, `client_for_url`, `run_with_deadline` | `sniff/lib/src/remote/blocking.rs` |
| Anonymous retry and `MissingCredentials` mapping per provider | `sniff/lib/src/remote/{github,gitea,gitlab,bitbucket}.rs` |
| Existing `Auth` assertions | `sniff/lib/tests/l1/{open_pull_requests,pr_for_branch}.rs` |
| Other `PrUnavailable` consumers | `worktree/lib/src/pull_requests.rs` (maps to `String`), `worktree/lib/src/remove/safety.rs` (`to_string()`) |
| Live-head store (format 1), lock, refresh | `worktree/lib/src/remote_head.rs` |
| Non-interactive git transport | `worktree/lib/src/live_remote.rs` (`run_noninteractive`, `LsRemote`) |
| Existing user config (`~/.worktree.json`, `dirs::home_dir`) | `worktree/lib/src/config.rs` |
| Store paths (`dirs::cache_dir`) | `worktree/lib/src/cache.rs` (`repo_cache_file`) |
| Global flags (`--width`, `--verbose`, `--perf`), hidden worker subcommand | `worktree/cli/src/args.rs` |
| Worker (two halves on scoped threads) | `worktree/cli/src/commands/refresh_worker.rs` |
| Listing orchestration, `ListSeams { connect, launch }` | `worktree/cli/src/commands/list.rs`, `list/tests.rs` |
| Pure renderer | `worktree/cli/src/commands/list_table.rs`, `cli/tests/list_table.rs` |
| Tests that change | `cli/tests/{list_prs,list_remote_head,list_output,perf_pr_request,perf_command_sla,level2_list_verbose}.rs`, `cli/tests/perf_support/` |

### Definition of done

- [ ] Every `wt list` with an `origin` and a resolvable default branch launches, or adopts, one worker attempt. It waits for a terminal outcome for at most 3 s, then gathers refs, counts, and the graph *after* the wait.
- [x] The worker checks through the provider API (or `ls-remote` when the remote is unsupported, the API call fails, or the repository is ignored), all within one 10 s budget. It publishes the check before fetching, and fetches only `refs/remotes/origin/<default>` with the exact spec command and a 60 s deadline. `FETCH_HEAD`, tags, and other refs stay unchanged.
- [x] A 404 is never treated as absence. Only a complete `ls-remote` answer that lacks the ref proves absence. No failure erases the last successful answer, and a fetch failure keeps the newly checked answer.
- [ ] The caption is one sentence in every §4 row, with a dim italic suffix. §5 warnings, the §6 hint, the §9 suggestion, and the §8 notice appear only under their stated conditions and in their stated positions.
- [ ] `-r`, `--ignore-api`, and `--ff` work as `wt …` and `wt list …`, are rejected with `create`, `go`, and `remove`, and appear in help and in the completion snapshot.
- [ ] The spinner is never written when stderr is not a terminal, and its line is cleared before the caption is drawn (L2).
- [ ] The performance contract holds: local gather plus render stays within 1 s, and ordinary listing with a stalled worker returns within 3 s plus render time. `worktree/docs/performance-testing.md` states the new full-command bound.
- [ ] `just test` and `just lint` pass in `schematic/`, `sniff/`, `biscuit-terminal/`, and `worktree/`. `just test-l2` passes for `worktree/` on macOS. No detached worker outlives any test fixture.
- [ ] The docs and skills listed in the spec's "Packages and documentation" section are updated.
- [ ] The spec's frontmatter and lifecycle directory are left for the author. The terminal state is "implementation complete, ready for review".

## Phase 1 — Rulings, Spikes, and Baseline

### Necessary Rules

These rulings resolve the spec's ambiguities and fix the cross-task contracts, so parallel waves agree. Implementers follow them as written. If a spike contradicts a ruling, stop and record the conflict in `implementation-log.md` before changing course.

1. **Module layout.**
    - **Lib (`worktree`):**
        - Extend `remote_head.rs` with the format-2 store (`attempt`, receipt).
        - Add `remote_update.rs` for the worker's check → publish → fetch flow, with the injectable seams described below.
        - Add `api_preference.rs` for the `~/.wt.json` reader and writer.
        - Add `fast_forward.rs`.
        - Add `fetch_tracking_ref`, `classify_git_failure`, and `tracking_ref_changed_at` (the reflog read) to `live_remote.rs`.
    - **CLI (`worktree-cli`):**
        - Add `commands/list/wait.rs` for launch/adopt/poll and the spinner.
        - Extend `list.rs` (orchestration), `list_table.rs` (rendering), `refresh_worker.rs` (attempt and receipt), and `args.rs` (flags).
    - Do not create a generic store abstraction. Three stores still do not justify one (Rule 2 of `CLAUDE.md`).
2. **`~/.wt.json` beside `~/.worktree.json`.** The spec names `~/.wt.json` explicitly, and the existing `WorktreeConfig` (`base_dir`, required) is a different concern with a setup flow. So follow the spec: a separate file, `api_preference::preference_path()` = `dirs::home_dir()/.wt.json`. That is the same mechanism `config.rs` uses, and it resolves to `%USERPROFILE%` on native Windows.
    - Schema: `{ "format_version": 1, "ignore_api": [ { "host": "github.com", "port": 443, "path": "owner/repo" } ] }`.
    - Writes go through `cache`'s `atomic_write` under a persistent `~/.wt.json.lock` (`fs4`, **blocking** with a 2 s bound; this is a user-initiated write, not a background race).
    - Record the two-file coexistence as a follow-up in the implementation log. Merging the two files is out of scope.
3. **Repository identity for `--ignore-api`.** *(Amended in Phase 1 by S2; see `implementation-log.md`: sniff adds a public raw `remote_identity` → `RemoteIdentity { scheme, host, port, path }`, and the port policy below stays in `api_preference`.)* Use sniff's existing `parse_remote_identity` / `provider_url` parsing on the **fetch** URL of `origin`. The normalized host is lowercase with no userinfo. The effective port is 443 for HTTPS, and 443 for SSH to a known provider host, because SSH and HTTPS then share one API identity. Otherwise it is the explicit port, or the scheme default. The path is trimmed of `.git` and slashes, with case kept. Expose the identity as `api_preference::RepoIdentity::from_origin(url) -> Option<RepoIdentity>`. When the result is `None`, `--ignore-api` is rejected, as the spec requires. Spike S2 confirms that the sniff helpers are reachable from `worktree`.
4. **Store format 2** (`remote_head.rs`, `REMOTE_HEAD_FORMAT_VERSION = 2`):
    ```text
    { format_version: 2,
      answer:  { origin_digest, branch, sha: Option<oid>, checked_at, source: "api"|"git"|"fetch" } | null,
      attempt: { id, origin_digest, branch, started_at, phase, outcome: Option<Outcome>, api: Option<ApiNote> } | null }
    ```
    - `phase` ∈ `checking` | `checking-fallback` (with a `reason`: `no-key` | `rate-limited` | `not-visible` | `rejected` | `other`) | `fetching`.
    - `Outcome` ∈ `in-sync` | `fetched` | `absent` | `fetch-failed{reason}` | `check-failed{reason}` | `unavailable{reason}` (discarded answer, origin or branch changed).
    - Check reasons: `timeout` | `credentials` | `other`. Fetch reasons: `timeout` | `other`.
    - `ApiNote` records the §5 condition observed (`CredentialsRequired`, `CredentialsRejected`, `CredentialsInsufficient`, `RateLimited{authenticated}`, `NotFoundOrNotPermitted`), the variable name used (`key: Option<String>`, a name never a value), and whether the fallback answered. The foreground needs `ApiNote` to render §5 and §8 for this run.
    - A format-1 file reads as `answer` with `source: "git"` and `attempt: null`.
    - An attempt is discarded when:
        - any field is invalid;
        - `started_at` is in the future;
        - its origin or branch does not match;
        - `now - started_at > 10 + 60 + 5` s (`ATTEMPT_MAX_AGE`).
    - A discarded attempt never touches `answer`.
5. **Attempt identity and worker invocation.**
    - The foreground generates the token with the same random generator `remove::handoff` uses for its tokens (hex, 128 bit).
    - It launches `wt internal-refresh <main> --attempt <token> [--force]`.
    - The worker takes the remote-head lock **nonblocking**. When it wins, it writes `attempt { id: token, phase: checking, outcome: None }` before any request. When it loses, it exits without writing anything.
    - The PR half keeps its own lock and its own freshness rule (Rule 7).
6. **Foreground wait protocol** (`list/wait.rs`, pure core over an injected clock and store reader):
    - Launch, keeping the `Child` handle, and never join. Then poll the store every 25 ms:
        - `attempt.id == token`: this is **ours**. Follow its phase until it reaches an outcome.
        - A different, valid, non-terminal attempt for the same origin digest and branch while the lock is `Contended`: **adopt** it and follow that id.
        - A terminal outcome is accepted only for the followed id. An outcome from any other id, repository, or branch is never shown.
        - `Child::try_wait` reports an exit while no attempt carries our token and no attempt is being adopted: render at once with `unavailable` ("couldn't check origin"). A contender that exits early therefore never counts as a completed check.
        - A spawn failure takes the same immediate `unavailable` path.
    - Budgets:
        - Ordinary listing: 3 s, then the "still checking" or "still pulling" row, according to the last seen phase.
        - `-r` and `--ff`: wait for the receipt (Rule 8), bounded by `10 s + 60 s + 5 s`. If that expires, it renders as a 3 s timeout would, with no hang.
7. **Freshness.**
    - Ordinary listing has **no** freshness skip for the head check (Decision 1). Remove the `AlreadyFresh` recheck from the head half, keeping the lock.
    - The PR half keeps its 60 s window and the existing 300 ms foreground miss request.
    - `--force` makes the worker ignore the PR window as well.
    - In forced mode, a contended head lock held by an attempt for the same origin and branch is adopted. A contended lock held for another origin or branch makes the foreground wait for that holder to finish, then relaunch with `--force` within the same budget.
8. **Completion receipt** (`-r`/`--ff` only): `<repo hash>.refresh-receipt.json`, written atomically by the worker after **both** halves join:
    ```text
    { format_version: 1, attempt_id, origin_digest, branch, finished_at,
      head: ok|failed|adopted-elsewhere, prs: ok|failed{PrFailure}|skipped-fresh|contended }
    ```
    - For a half that was `contended`, the foreground waits for that lock to be free, then reads the half's store before reporting completion.
    - A mismatched `attempt_id`, origin, or branch, or a receipt older than the attempt, is ignored.
    - Last writer wins, which is safe because readers key on the token.
9. **Check chain** (`remote_update::check`, one `Instant` budget of 10 s shared by the API call and the fallback):
    - If the repository is ignored (`~/.wt.json`), run `ls-remote` directly, with no provider request.
    - If `sniff` reports `Unsupported`, run `ls-remote` in phase `checking`. This is not a "fallback" and has no notice.
    - Otherwise call `branch_head`:
        - `Ok` is the answer.
        - `CredentialsRequired` → fallback `no-key`.
        - `NotFoundOrNotPermitted` without a key → fallback `not-visible`. It counts as "no key" for §8 when Git answers.
        - `RateLimited` → fallback `rate-limited`.
        - `CredentialsRejected` and `CredentialsInsufficient` → fallback `rejected`.
        - `Timeout`, `Network`, and `Other` → fallback `other`, when budget remains.
    - The spinner text for each phase:
        - `no-key` and `not-visible`: "no API key, using fallback method".
        - `rate-limited`: "rate limited, using fallback method".
        - Every other phase: "updating".
    - Absence comes only from `LsRemote::live_head` returning `Ok(None)`.
10. **Git commands.** *(Amended in Phase 1 by S3; see `implementation-log.md`: the fetch argv adds `--no-recurse-submodules --refmap=`, the classifier adds `unable to get password from user`, and the deadline is a typed error, never a matched string.)*
    - Both `ls-remote` and the fetch run through `run_noninteractive` with `LC_ALL=C` added.
    - The fetch argv is the spec's exact command. Spike S3 decides whether to add `--no-recurse-submodules`. If S3 shows that a default `fetch.recurseSubmodules`/`submodule.recurse` setting can touch other refs, add the flag and record the deviation. Otherwise keep the spec argv verbatim.
    - The branch is validated with `git check-ref-format --branch` before the refspec is built. An invalid name ends as `check-failed{other}` without a request.
    - `classify_git_failure(stderr)` maps only known `LC_ALL=C` patterns (authentication failed, permission denied (publickey), could not read Username, deadline) to reasons. Everything else is `other`. Raw stderr is never stored or rendered.
11. **Post-fetch observation.**
    - After a successful fetch, re-read `refs/remotes/origin/<default>`. Publish `answer { sha: <that tip>, checked_at: <fetch start>, source: "fetch" }` and set the outcome to `fetched`. This is how a remote move between check and fetch is reported from the fetched tip.
    - On a failed fetch, re-read the tracking tip. If it changed, it may be a concurrent fetch. Run one more check (same chain, with its own 10 s budget). When that check equals the tracking tip, the outcome is `fetched`, and the re-check becomes the published answer. Otherwise the outcome is `fetch-failed`, and the first check stays published.
12. **Worker order and publication.** `answer` is published (atomic write, still holding the lock) **before** the phase becomes `fetching`. Origin and the default branch are re-read after each network step. A change makes the outcome `unavailable` and publishes nothing new. Every write in the worker is best effort: a failed write ends the worker, and the foreground's bounded wait covers it.
13. **sniff API** (`remote::blocking`): *(Amended in Phase 1 by S2; see `implementation-log.md`: key names follow `credentials::provider_token`'s order, not `env_auth`'s, and the 403 split happens in `focused.rs`.)*
    ```rust
    pub struct BranchHead { pub sha: String }
    pub fn branch_head(remote_url: &str, branch: &str, deadline: Duration) -> Result<BranchHead, PrUnavailable>;
    pub fn branch_head_with(client: &FocusedProviderClient, branch: &str, deadline: Duration) -> Result<BranchHead, PrUnavailable>;
    pub struct CredentialEnv { pub provider: String, pub variables: Vec<String> }
    pub fn credential_env(remote_url: &str) -> Option<CredentialEnv>;
    ```
    - `PrUnavailable` gains `CredentialsRequired { key: None, .. }`, `CredentialsRejected { key }`, `CredentialsInsufficient { key }`, and `RateLimited { authenticated, key }`, and loses `Auth`. `key` is the name of the variable the client used. Spike S2 decides how sniff learns that name.
    - A 403 maps to `CredentialsInsufficient` only when the provider's response establishes it. Otherwise it maps to `RateLimited` (rate-limit headers or body) or `NotFoundOrNotPermitted`.
    - `PrUnavailable` stays `#[non_exhaustive]`.
14. **worktree PR mapping.**
    - `pull_requests` keeps storing no failures. The foreground miss request returns a typed `PrFailure` (a mirror of the credentials variants, plus `Other`) instead of a `String`, so this run's PR failure can produce a §5 line.
    - The detached PR half reports its failure only through the receipt, which only `-r`/`--ff` wait for.
    - `remove::safety` keeps `to_string()` and needs a compile check only.
15. **Flags.**
    - `Cli` gains three flags:
        - `refresh: bool` (`-r`, `--refresh`, global)
        - `ignore_api: bool` (`--ignore-api`, global)
        - `fast_forward: bool` (`--fast-forward`, alias `--ff`, global)
    - clap global args cannot conflict with subcommands, so `main.rs` rejects them **after parsing**, before dispatch, for `create`, `go`, and `remove` (and the hidden worker). The rejection is `Cli::command().error(ErrorKind::ArgumentConflict, …)`, which exits 2, as clap does.
    - `--ff` with `-r` is accepted: one forced update, then one fast-forward. `--ignore-api` combines with either. No combination is rejected.
    - `--ignore-api` without an `origin`, or with an unidentifiable one, exits 1 with a message and renders nothing.
16. **Renderer contract** (the agreed interface for the parallel Phase 4 wave). `TableFacts` gains:
    ```rust
    pub remote_status: Option<RemoteStatus>,   // None ⇔ no origin (existing rule)
    pub credential_line: Option<CredentialLine>, // §5, already resolved to provider + key names
    pub unfinished: bool,                        // §6 hint
    pub ff_suggestion: Option<FfSuggestion>,     // §9, only when rendered after a completed fetch or no-variance check
    pub fallback_notice: Option<Vec<String>>,    // §8 key names
    ```
    - `RemoteStatus` has one variant per §4 row: `CheckedNow`, `Fetched`, `FetchFailed{reason}`, `StillChecking{last: Option<age>}`, `StillPulling`, `CheckFailed{reason, since: LastKnown}`, `Absent{tracking_ref_present}`. `LastKnown` is `Answer(age) | TrackingRefChanged(age) | Never`.
    - Wording:
        - "local origin/main" appears only in the rows the spec spells that way.
        - "Just now" means this run's attempt produced it.
        - Otherwise the existing `age_text` applies.
    - Output order:
        1. caption
        2. §5 line
        3. table and legend
        4. PR age line
        5. graph
        6. §6 hint (after the graph, or after the PR age line when no graph is drawn)
        7. verbose section
        8. a blank line, then the §9 suggestion and the §8 notice as the final notes
    - When a graph is drawn, the hint therefore comes after it and before verbose.
17. **Spinner** (`biscuit-terminal`, `components/spinner.rs`):
    ```rust
    Spinner::new(text).with_delay(Duration::from_millis(150)).start_on_stderr() -> SpinnerHandle
    ```
    - `SpinnerHandle::set_text`, `finish()` (which clears the line), and `Drop` (which also clears).
    - A background thread draws frames about every 80 ms. It is a no-op when `!std::io::stderr().is_terminal()`.
    - The frame and clear sequences are pure functions (`frame(i, text, width)`, `CLEAR_LINE`), so they can be L1 tested with an injected writer (`start_on(writer, is_terminal)`).
    - It is not a `TerminalRenderable` tree node, because it is a live, time-driven widget. The docs page says so.
    - Clearing when output is not a terminal writes nothing.
18. **Test seams.**
    - The check runs out of process, so `remote_update` takes its seams in the lib:
        - `trait BranchHeadSource` (production: sniff, with `credential_env`)
        - `trait GitRemote` (production: `LsRemote` plus `fetch_tracking_ref`)
        - a clock
    - L1 update-flow tests call `remote_update::run_attempt` in-process against a real local bare origin and a `pusher` clone.
    - `ListSeams` becomes `{ connect, launch: fn(&Path, &LaunchArgs) -> io::Result<WorkerHandle>, clock, wait_budget }`. Tests can use a short budget and stub launch outcomes (spawn failure, early exit, adopt).
    - Binary-level tests prove the Git-only paths (file-path origin, `HoldingOrigin`). Provider paths are proven in-process, because sniff's hosts cannot be redirected without MITM.
19. **Performance contract.**
    - `perf_command_sla`'s 1 s bound applies to "local gather + render" with a fresh stubbed attempt (the worker answers `in-sync` at once).
    - A new bound is `3 s + render` with a stalled worker. `perf_list_meets_sla_with_a_stale_answer_and_a_blocked_refresh` becomes "returns within 3 s + 1 s while the worker is still held".
    - The `pr gather < 300 ms` assertion is unchanged.
20. **Existing tests.** Every `wt list` with an origin now launches a worker and may wait for up to 3 s.
    - Fixtures with a file-path origin answer in milliseconds.
    - Fixtures with an `example.invalid` or refused origin fail fast.
    - Any test that would sit through the full 3 s is migrated to a seeded or stubbed path. Each listing test must finish in under 5 s.
    - Cleanup still waits for the worker process and both locks (`MixedFixture::wait_until_unlocked`).

### Spikes

- [x] **S1 — Provider branch-head endpoints and path encoding** (about 45 min; read-only anonymous `GET`s against public repositories only)
    - For GitHub, GitLab.com, Codeberg (Gitea/Forgejo), and Bitbucket Cloud, request the branch-head endpoint for a branch without `/`, and for one with `/` percent-encoded as `%2F` (the encoding schematic generates).
    - Record the status, the response field path (`object.sha`, `commit.id`, `target.hash`), and the rate-limit response shape (GitHub 403 plus `x-ratelimit-remaining: 0`, Bitbucket 429, GitLab 429). Record also what each provider returns for a private or missing repository without a token.
    - Output: a table in the implementation log. If a provider rejects `%2F`, stop and record a ruling proposal. Do not hand-roll the path around schematic.
- [x] **S2 — Credential key metadata and identity parsing** (about 30 min, read-only)
    - Determine whether schematic's generated client reports which `env_auth` variable it used. If it does not, the ruling is that sniff resolves the name itself, in the same order as `env_auth`, and attaches it to the error.
    - Confirm how each sniff provider module maps 401, 403, 404, and 429 today (`github.rs`, `gitea.rs`, `gitlab.rs`, `bitbucket.rs`), including the anonymous retry.
    - Confirm that `parse_remote_identity` (or `provider_url`) is public and can give Rule 3's `(host, port, path)`. If it cannot, the fallback is a small `pub fn remote_identity` in sniff.
- [x] **S3 — Fetch side-effect and platform audit** (about 45 min, macOS; Linux through Docker if available; load the `os` skill first)
    - Run the spec's fetch against a local bare origin under these configurations:
        - `fetch.prune=true`
        - `fetch.recurseSubmodules=true` with a submodule
        - a custom `remote.origin.fetch`
        - `core.logAllRefUpdates=false`
    - Confirm that only `refs/remotes/origin/<default>` changes, that `FETCH_HEAD` is untouched, that tags are untouched, and that a rewind works with `+`.
    - Record `LC_ALL=C` stderr for four cases: lock contention (a concurrent fetch holding `origin/main.lock`), an authentication failure over loopback HTTP 401, an SSH `BatchMode` failure, and a timeout kill.
    - Record the minimum Git version for `--no-write-fetch-head` against the CI images (`os` skill runner table).
    - Confirm `git reflog -1 --format=%ct refs/remotes/origin/<default>` output with and without a reflog.
    - Output: the Rule 10 decision on `--no-recurse-submodules`, and the classifier's pattern list.
- [x] **S4 — Affected-test inventory** (about 30 min, read-only)
    - List every test that runs `wt list` or the worker with an origin configured: `list_prs`, `list_remote_head`, `list_output`, `perf_*`, `level2_*`, `cache_*_path`, and the `list/tests.rs` units. For each, record the expected new wait (fast answer, fast failure, or a 3 s stall), and whether it asserts "no worker", a request count, or caption text.
    - List every sniff test asserting `PrUnavailable::Auth`.
    - Output: a table in the implementation log that Phase 4 Wave 3 works through.

### Tasks

- [x] **Baseline green**
    - Run `just test` and `just lint` in `schematic/`, `sniff/`, `biscuit-terminal/`, and `worktree/` on the unmodified branch. Record any pre-existing failures in `implementation-log.md`.
- [x] **Log setup**
    - Create `implementation-log.md` in this fix directory with the accepted rulings, the spike results, and any ruling amended by a spike.

**Checkpoint 1:** S1–S4 are recorded; Rules 10, 13, and 3 are confirmed or amended in the log; the baseline is known.

## Phase 2 — Provider, Component, and Store Foundations

### Wave 1 (parallel; disjoint files)

- [x] **Schematic endpoints** (`schematic/definitions/src/{github,gitea,gitlab,bitbucket}/mod.rs`, then regenerate `schematic/schema`)
    - Add GitHub `GetBranchReference` (`GET /repos/{owner}/{repo}/git/ref/heads/{branch}`, beside `GetTagReference`), Gitea `GetBranch`, GitLab `GetBranch`, and Bitbucket `GetBranch`, each with the response type fields from the spec table. Use the existing `env_auth` and `env_mapping`, and add no new variables.
    - Update each module's endpoint table (`//!` docs) and `schematic/definitions/README.md`.
    - Add a definition test per endpoint (path, method, auth) that matches the existing per-provider tests. Run `just generate` (schematic), and confirm the generated client compiles and `just check-drift` is clean.
- [x] **sniff credentials split** (`sniff/lib/src/remote/blocking.rs`, the provider modules as needed, `sniff/lib/tests/l1/{open_pull_requests,pr_for_branch}.rs`)
    - Replace `Auth` per Rule 13, and implement the key-name metadata per the S2 outcome.
    - Add `CredentialEnv` and `credential_env(remote_url)`, reading the names from each definition's `env_auth` and `env_mapping` (never hard-coded in `worktree`).
    - Update the existing `Auth` assertions to the split variants. Add wiremock cases for 401 and 404 with and without a token, 403 as insufficient, 403 as a rate limit (GitHub), and 429. Assert that no variable **value** appears in any `Display` output.
- [x] **Spinner component** (`biscuit-terminal/lib/src/components/spinner.rs`, the prelude export, `biscuit-terminal/docs/components/spinner.md`, the `docs/components/index.md` entry)
    - Implement Rule 17. L1 tests with an injected writer:
        - nothing is written before the delay;
        - nothing is written when the output is not a terminal;
        - `set_text` changes the next frame;
        - `finish` and `Drop` write the clear sequence exactly once;
        - frames are truncated to the width.
    - Add the skill entry in `.claude/skills/biscuit-terminal/`.
- [x] **API preference store** (`worktree/lib/src/api_preference.rs`)
    - Implement Rules 2 and 3: `RepoIdentity::from_origin`, `load(path) -> Preferences`, where a missing, corrupt, or unreadable file is empty for reading. Also `is_ignored(&identity)`, and `add(path, identity) -> Result<(), WorktreeError>`.
    - `add` refuses to overwrite an unreadable or corrupt file (it reports a write error), and uses the lock plus `atomic_write`.
    - Tests:
        - the stored identity never contains userinfo or a password;
        - distinct ports stay separate;
        - SSH and HTTPS identities for one repository unify;
        - two concurrent `add` threads both persist;
        - a corrupt file reads as empty, and `add` refuses it;
        - a Windows-only test resolves under `%USERPROFILE%`.
- [x] **Remote-head store format 2** (`worktree/lib/src/remote_head.rs`)
    - Implement Rule 4 (schema, format-1 read, `ATTEMPT_MAX_AGE`) and Rule 8 (receipt types, path `<repo hash>.refresh-receipt.json`, load and validate).
    - Add store writers that preserve `answer` when only the attempt changes: `begin_attempt`, `set_phase`, `publish_answer`, and `finish_attempt`. They are called with the lock held.
    - Tests:
        - a format-1 file upgrades without losing its answer;
        - each discard rule drops the attempt but not the answer;
        - a failed or stale attempt never replaces an answer;
        - receipt mismatch by id, origin, or branch.

> Shared file: the two worktree tasks each add one `pub mod` line to `worktree/lib/src/lib.rs`. The orchestrator adds both lines (plus `remote_update` and `fast_forward`) before the wave starts.

### Wave 2 (after Wave 1)

- [x] **sniff `branch_head`** (`sniff/lib/src/remote/blocking.rs`, a new `sniff/lib/tests/l1/branch_head.rs`)
    - Build it like `open_pull_requests` (`client_for_url`, `run_with_deadline`, the same anonymous fallback, `classify`), using the fetch URL identity. Validate the SHA as 40 or 64 lowercase hex. A 404 stays `NotFoundOrNotPermitted`.
    - Add wiremock tests per provider:
        - SHA parsing;
        - an invalid SHA is rejected;
        - the deadline;
        - branch path encoding (`/`, a space, Unicode), where the recorded request path shows the repository path unchanged;
        - the anonymous fallback succeeds;
        - every §5 status for that provider;
        - `credential_env` returns each definition's names.
    - Update the sniff skill's remote section and `sniff` docs.
- [x] **Typed PR failure in worktree** (`worktree/lib/src/pull_requests.rs`, a compile check of `remove/safety.rs`)
    - Implement Rule 14. Update the `pull_requests` tests that used string failures, and add a case per credentials variant.

**Checkpoint 2:** `just test` and `just lint` pass in `schematic/`, `sniff/`, `biscuit-terminal/`, and `worktree/`. `cargo check -p worktree-cli` compiles.

## Phase 3 — Worker Update Flow

### Wave 1 (parallel; disjoint files)

- [x] **Git transport additions** (`worktree/lib/src/live_remote.rs`)
    - Add `LC_ALL=C` to `run_noninteractive`'s environment. Confirm that removal's parsers do not read localized text; S3 reads the transport.
    - Add `fetch_tracking_ref(base, branch, deadline)`, which returns the exact Rule 10 argv as one refspec argument after `check-ref-format --branch`. Add `classify_git_failure(stderr) -> GitFailure` with S3's patterns, and `tracking_ref_changed_at(base, branch) -> Option<u64>` (the reflog; `None` when missing or disabled).
    - Tests use a real bare origin:
        - a fetch updates only the one ref;
        - `FETCH_HEAD` is absent or unchanged;
        - a rewind is applied;
        - an invalid branch name is refused before any spawn;
        - the classifier maps each S3 sample and defaults to `other`;
        - reflog present and absent.
- [x] **Update-flow core** (`worktree/lib/src/remote_update.rs`)
    - Implement `run_attempt(main, token, force, seams) -> Outcome` per Rules 5, 7, 9, 11, and 12, using the Phase 2 store writers, `BranchHeadSource`, `GitRemote`, and an injected clock.
    - Unit tests with stub sources and a real bare origin with a `pusher` clone:
        - no variance → `in-sync`, with no fetch;
        - variance → `fetched`, with the answer taken from the post-fetch tip;
        - a movement between check and fetch is reported from the fetched tip;
        - a fetch timeout → `fetch-failed{timeout}`, keeping the new answer and leaving the tracking ref untouched;
        - a check timeout at 10 s → `check-failed{timeout}`, keeping the old answer;
        - an API failure followed by an `ls-remote` answer shares one 10 s budget, measured with the injected clock;
        - an unsupported remote uses `ls-remote` in phase `checking`;
        - each fallback reason's phase is recorded;
        - an ignored repository makes zero `BranchHeadSource` calls;
        - absence only from `ls-remote`, with 404 plus a Git failure giving `check-failed`;
        - an origin or default change mid-request → `unavailable`, with nothing published;
        - a concurrent fetch that reached the current remote SHA → `fetched`;
        - a different SHA equal to the earlier check is not labeled current;
        - a contended lock writes nothing.

### Wave 2 (after Wave 1)

- [x] **Worker wiring** (`worktree/cli/src/commands/refresh_worker.rs`, `worktree/cli/src/args.rs` for the hidden subcommand's `--attempt`/`--force`)
    - The head half calls `remote_update::run_attempt` with the production seams. The PR half honors `--force` (Rule 7) and the ignored-repository rule (it skips the PR request entirely, so no badges appear).
    - After both halves join, write the receipt when `--force` is set (Rule 8).
    - The existing independence and panic tests must still pass. Add: a receipt is written only after both halves finish, and a panicking half is recorded as `failed`.

**Checkpoint 3:** `worktree` L1 is green. Running `wt internal-refresh <main> --attempt t` by hand against a local bare origin gives the expected store transitions, recorded in the log.

## Phase 4 — Foreground Listing, Flags, Fast-forward, and Rendering

### Wave 1 (parallel; disjoint files)

- [ ] **Flags** (`worktree/cli/src/args.rs`, `worktree/cli/src/main.rs`, the completion snapshot)
    - Implement Rule 15, and thread `refresh`, `ignore_api`, and `fast_forward` into the listing entry point.
    - Tests:
        - `wt -r`, `wt list -r`, `--ignore-api`, `--ff`, and `--fast-forward` parse;
        - each is rejected with `create`, `go`, and `remove` (exit 2);
        - help lists them;
        - the completion snapshot is updated in `cli/tests/wrapper_protocol.rs` or wherever it lives (not a unit snapshot; see the skill's twin-target note).
- [x] **Fast-forward** (`worktree/lib/src/fast_forward.rs`)
    - `fast_forward_default(main, default) -> FfResult` implements the spec's §9 table:
        - it re-reads both refs and verifies ancestry immediately before the move;
        - when the default branch is not checked out, it runs `update-ref refs/heads/<d> <new> <old>`;
        - when it is checked out, it re-resolves which worktree holds it, verifies that the checkout's current branch is still the default, then runs `git -C <wt> merge --ff-only origin/<d>`;
        - it classifies "would be overwritten" (under `LC_ALL=C`) as `DirtyCheckout`.
    - Tests on a real bare origin with a `pusher` clone:
        - not checked out → the ref moves;
        - a concurrent CAS change is refused;
        - checked out and clean → the working tree moves;
        - dirty touched files → refused, with the files byte-identical;
        - the branch moved to another worktree → re-resolved, or refused;
        - diverged, in sync, and ahead → no change;
        - a missing default branch or tracking ref → names the missing ref, and nothing is created.
- [x] **Rendering** (`worktree/cli/src/commands/list_table.rs`, `cli/tests/list_table.rs`, snapshots)
    - Implement Rule 16:
        - one-sentence captions without "local tracking ref";
        - a dim italic suffix;
        - §5 lines per condition, with `{provider}` and `{key}` from `CredentialLine`;
        - the §6 hint;
        - the §9 suggestion;
        - the §8 notice;
        - the ordering and blank line.
    - Snapshots:
        - every §4 row × four comparison states (behind, ahead, in sync, diverged);
        - each reason;
        - the reflog row and "never checked";
        - absent with the tracking ref present and absent;
        - a non-`main` default name;
        - every §5 row × four provider names, asserting that no variable value appears;
        - the hint/notice/suggestion placement with and without a graph and with `--verbose`.
    - Replace the old caption snapshots rather than keeping both.
- [x] **Wait and spinner** (`worktree/cli/src/commands/list/wait.rs`)
    - Implement Rule 6 over `ListSeams`: launch, adopt, poll, the budget, and `try_wait` early exit. The spinner (Rule 17) starts only when stderr is a terminal, and its text follows the phase.
    - Unit tests with a stub launcher, a fake clock, and a scripted store:
        - ours → outcome;
        - adopt a matching attempt with no second launch request;
        - an early-exiting contender → `unavailable`, not success;
        - a spawn failure → immediate `unavailable`;
        - a timeout reports the last phase;
        - an outcome from another id, branch, or origin is ignored;
        - forced mode waits for the receipt and the contended half's lock;
        - forced mode against a mismatched holder relaunches after it finishes;
        - the spinner writes nothing with a non-terminal stderr.

### Wave 2 (after Wave 1)

- [ ] **List orchestration** (`worktree/cli/src/commands/list.rs`, `list/tests.rs`)
    - The new order:
        1. Resolve `origin` and the default branch cheaply.
        2. `--ignore-api`: add the preference first, before the launch.
        3. Launch or adopt, and wait (`wait.rs`).
        4. Clear the spinner.
        5. `--ff`: fast-forward.
        6. Gather `parse_worktree_state` / `fill_worktree_statuses` / the graph (the existing PR thread still runs in parallel with the local gather).
        7. Build `TableFacts` per Rule 16 and render.
    - Supporting logic:
        - `tracking_ref_changed_at` is called only when no answer is stored.
        - §5 and §8 come only from this run's `ApiNote` or foreground PR failure (or the receipt under `-r`/`--ff`).
        - The §9 suggestion appears only when rendered after a terminal outcome, with the default branch strictly behind.
        - `unfinished` = the wait timed out, or the PR lock is contended at render time.
    - Update the perf stage names in `perf.rs` for the new stages ("remote wait", "fast-forward").
    - `list/tests.rs`: the launch count is one per run; a snapshot gathered after a fetch that completes during the wait agrees with the caption; a fetch still running at the deadline leaves one coherent snapshot.

### Wave 3 (after Wave 2; parallel by test file)

- [ ] **Existing-test migration** (per the S4 table: `list_prs.rs`, `list_output.rs`, `cache_*_path.rs`, `perf_support/`)
    - Apply Rule 20: seed or stub where a test would stall, keep cleanup waits for the process and both locks, and update caption text assertions.
- [ ] **Real-Git update-flow tests** (`cli/tests/list_remote_head.rs`)
    - Spec acceptance 4 at the binary level with a local bare origin and a `pusher` clone:
        - no variance → "checked just now", with no fetch;
        - variance → new counts, with `FETCH_HEAD` unchanged;
        - a held `HoldingOrigin` check → the "still checking" row plus the hint within 3 s, and the next run shows the result;
        - a held fetch → the "still pulling" row, and the worker publishes after `wt list` has exited;
        - a second concurrent `wt list` adopts, with a single request recorded by `HoldingOrigin`;
        - refused and black-holed origins keep the previous answer, dated;
        - an ignored repository (`~/.wt.json` in the test `HOME`) → no request through a counting `ProxyStub`, and no notice;
        - the spinner is never written to captured stderr.
- [ ] **Flags end to end** (`cli/tests/list_flags.rs`, new)
    - `-r` waits for both halves and forces fresh requests with young caches. `-r` with a spawn failure or a publication failure stays bounded.
    - `--ignore-api` writes the entry before the run; a corrupt file is treated as empty; an unreadable file is not overwritten; there is no `origin` → exit 1.
    - `--ff` covers acceptance 9 at the binary level, including a failed remote check that fast-forwards to the local `origin/<default>` while keeping the failure reason. `--ff` with `-r` performs one update and one move.

**Checkpoint 4:** `just test` in `worktree/` is green with no worker leaks. Every listing test finishes in under 5 s, except the tests that deliberately wait for `-r`.

## Phase 5 — Performance, L2, Documentation, and Final Validation

### Wave 1 (parallel)

- [ ] **Performance gates** (`cli/tests/perf_command_sla.rs`, `cli/tests/perf_pr_request.rs`, `worktree/docs/performance-testing.md`)
    - Implement Rule 19:
        - local gather plus render within 1 s, with an immediately answering worker;
        - with the check held and, separately, with the fetch held, the command returns within 3 s plus the render bound, and the captured `.output()` returns while the worker is still held (the `list_prs` no-join pattern);
        - `-r` and `--ff` against a held origin finish or report within the 10 s and 60 s deadlines.
    - Record the measurements and the new full-command bound in `performance-testing.md`.
- [ ] **L2** (`cli/tests/level2_list_verbose.rs`)
    - Update the caption assertion. Assert dim italic on the caption suffix, the hint, and a credentials warning (seeded `ApiNote` store), using `styled_capture`. Assert that the spinner line is cleared before the caption: with a held origin, the captured pane contains no spinner glyph and no "updating" text once the command finishes.
    - Run with `BISCUIT_TEST_REQUIRED_BACKENDS=tmux cargo nextest run -p worktree-cli --features terminal-tests -E 'binary(level2_list_verbose)'`. Windows must not gain focus.
- [ ] **Documentation** (keep each file's existing style and US English)
    - `worktree/docs/cli/list.md`: the §4 table, §5, §6, §8, §9, the flags, and `~/.wt.json`.
    - The `worktree/README.md` `wt list` description: listing now checks and fetches.
    - `.claude/skills/worktree/SKILL.md` `wt list` section:
        - store format 2, the attempt and receipt, the worker flow, the wait/adopt protocol, the flags, and the new test fixtures;
        - remove the "never fetches" and "no foreground wait" claims.
    - Add any OS fact learned in S3 to `.claude/skills/os/`.
    - `docs/dependencies.md` for any new crate. None is expected, and any new crate must be justified in the log.

### Wave 2 (after Wave 1)

- [ ] **Drift and comment pass**
    - Review the `///`/`//!` docs of every behavior-changed symbol:
        - `remote_head`, `live_remote`, `pull_requests`, `refresh_worker`, `list`, `list_table`, and sniff `blocking`;
        - drop statements that the listing never fetches or never waits.
    - Record detected drift and its resolution in the log.
- [ ] **Cross-OS validation** (load the `os` skill)
    - Run `just test` for `worktree` on Linux and on native Windows (`./scripts/cross-check.sh --os windows worktree-cli`, and `worktree`), covering the lock and polling, `try_wait` on a detached child, `%USERPROFILE%` resolution, and `LC_ALL=C` on Git for Windows.
    - Record the results in the log. A red result on one OS is fixed forward here, not deferred.
- [ ] **Final gate**
    - Run `just test` and `just lint` in `schematic/`, `sniff/`, `biscuit-terminal/`, and `worktree/`, and `just test-l2` in `worktree/`. Review `just ci-local --plan`.
    - Tick the Definition of Done, and set the log's final state to "implementation complete, ready for review". Do not edit the spec's lifecycle, and do not move the directory.

**Checkpoint 5:** every Definition of Done item is checked, with the evidence recorded in `implementation-log.md`.
