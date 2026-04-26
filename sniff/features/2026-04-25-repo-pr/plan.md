---
phases: 6
created: 2026-04-25
start_phase: 1
source_files_during_phase_1:
  - sniff/lib/src/remote/types.rs
  - sniff/lib/src/remote/github.rs
  - sniff/lib/src/remote/gitlab.rs
  - sniff/lib/src/remote/gitea.rs
  - sniff/lib/src/remote/bitbucket.rs
  - sniff/lib/tests/remote_providers.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - sniff/lib/src/remote/provider.rs
  - sniff/lib/src/remote/mod.rs
  - sniff/lib/src/remote/github.rs
  - sniff/lib/src/remote/gitlab.rs
  - sniff/lib/src/remote/gitea.rs
  - sniff/lib/src/remote/bitbucket.rs
  - sniff/lib/tests/remote_providers.rs
  - schematic/define/src/headers.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase2: []
packages:
  - sniff
  - sniff-cli
  - schematic-define
---

# Execution Plan: `sniff repo pr`

## Phase 1: Confirm API Surface and Data Model

1. Add a normalized `PullRequestState` enum in `sniff/lib/src/remote/types.rs` with variants `Open`, `Closed`, `Merged`, `Draft`, and `All`.
2. Implement parsing/display helpers for `PullRequestState` so CLI input accepts exactly `open`, `closed`, `merged`, `draft`, and `all`.
3. Expand `PullRequestInfo` in `sniff/lib/src/remote/types.rs` with `labels: Vec<String>` and `body: Option<String>`.
4. Update all existing `PullRequestInfo` construction sites in `sniff/lib/src/remote/{github,gitlab,gitea,bitbucket}.rs` and remote output tests to compile with the new fields.

Validation checkpoint:

- `cargo check -p sniff` reaches only expected follow-on errors from the trait signature change, or passes if providers are updated in the same pass.
- JSON serialization includes `labels` and `body` for `PullRequestInfo`.

Parallelizable:

- `PullRequestInfo` field propagation across providers can be done in parallel after the enum shape is fixed.

## Phase 2: Add Filtered Provider Support

1. Change `RemoteRepoProvider::list_pull_requests` in `sniff/lib/src/remote/provider.rs` to accept `state: PullRequestState`.
2. Update the `GitRemote` dispatcher in `sniff/lib/src/remote/mod.rs` to pass the state through to each concrete provider.
3. Update provider implementations:
   - GitHub: map `Open`, `Closed`, and `All` to GitHub `state` query values; handle `Merged` and `Draft` either through supported API parameters or post-filtering if the endpoint only returns open/closed/all.
   - GitLab: map to GitLab merge request states such as `opened`, `closed`, `merged`, and `all`; handle `Draft` through GitLab-supported filters or post-filtering on draft/work-in-progress flags.
   - Gitea: map supported states where available; post-filter `Draft` and `Merged` if the generated endpoint lacks exact query support.
   - Bitbucket: map supported states such as open/declined/merged/superseded where available; post-filter normalized values where needed.
4. Preserve `fetch_report` behavior by calling `list_pull_requests(owner, repo, PullRequestState::Open)` so existing remote reports continue listing open PRs by default.
5. If generated schematic request types do not expose state query parameters, update the relevant schematic definitions and regenerate `schematic/schema` using `--manifest-path schematic/schema/Cargo.toml` because that crate is excluded from the workspace.

Validation checkpoint:

- Provider unit tests in `sniff/lib/tests/remote_providers.rs` prove the requested state is encoded for GitHub/GitLab/Gitea/Bitbucket or that post-filtering returns the expected normalized subset.
- Existing `repo remote` behavior still returns open PRs by default.

Parallelizable:

- Provider-specific filter mapping and tests are independent once the trait signature is updated.

## Phase 3: Add CLI Parse Shape

1. Add a `Pr` variant to `RepoSubcommand` in `sniff/cli/src/args.rs`.
2. Add fields to the variant:
   - `status: PullRequestState` with `#[arg(long, default_value = "open")]`.
   - `verbose: bool` using `-v`/`--verbose` for the PR-only verbose block view.
3. Add a matching `RepoAction::Pr { status, verbose }` variant.
4. Extend `Commands::to_repo_action()` to normalize `RepoSubcommand::Pr` into `RepoAction::Pr`.
5. Add parse tests for:
   - `sniff repo pr`
   - `sniff repo pr --status merged`
   - `sniff repo pr --status draft --json`
   - invalid status error mentions all valid values.
6. Update `REPO_AFTER_HELP` and `sniff/cli/README.md` examples to include `sniff repo pr`.

Validation checkpoint:

- `cargo test -p sniff-cli args::` passes.
- `sniff repo pr --help` documents the default status and valid status values.

Parallelizable:

- Help/README updates can happen alongside parse tests after the CLI shape is chosen.

## Phase 4: Implement PR Command Dispatch

1. Add early handling for `RepoAction::Pr` in `sniff/cli/src/commands.rs` next to the existing `RepoAction::Remote` early return path.
2. Discover the current git repository from `base_dir` or `.` using `git2::Repository::discover`.
3. Resolve a preferred remote URL, using `origin` first when present and falling back to the first configured remote.
4. Parse the remote URL with `GitRemote::parse_url` and construct the provider with `GitRemote::from_url`.
5. Call `remote.list_pull_requests(&parsed.owner, &parsed.repo, status).await`.
6. For `--json`, emit the PR vector directly as pretty JSON, including `labels` and `body`.
7. For text output, call a dedicated PR renderer that supports default table and verbose block modes.
8. Translate common failures into explicit CLI errors:
   - not in a git repo: `No git repository found from <path>`
   - no remotes: `No git remotes found for this repository`
   - unsupported provider: name the remote URL/provider
   - 401/403/rate limit: include the provider credential env var, such as `GITHUB_TOKEN` or `GITLAB_TOKEN`
   - network/timeouts: preserve provider and status/message where available.

Validation checkpoint:

- Running inside this repository, `sniff repo pr --json` either returns JSON PR data or a clear provider/rate-limit/auth error.
- Running from a temporary non-repo directory fails with the no-repository message.
- Running from a repo with no remotes fails with the no-remotes message.

## Phase 5: Add PR Output Formatting

1. Add PR-only output helpers in `sniff/cli/src/output/remote.rs` or a new focused output module if the file is becoming too broad.
2. Default table output includes columns `ID`, `Title`, `Author`, and `State`.
3. Verbose block output includes number/title, author, normalized status with draft marker, source and target branches, labels, created date, URL if useful, and description body when present.
4. Ensure empty results render a clear one-line message such as `No open pull requests found`.
5. Keep existing `repo remote` PR table behavior compatible, or switch it to the shared table renderer if the output remains unchanged enough for snapshots.
6. Add unit tests for table rendering, verbose rendering with labels/body, empty rendering, and draft/merged state display.

Validation checkpoint:

- `cargo test -p sniff-cli output::remote::` passes.
- Snapshot/help output is updated only where command help changes.

Parallelizable:

- Output rendering tests can be built in parallel with command dispatch once the output function signatures are agreed.

## Phase 6: End-to-End Validation and Drift Updates

1. Run targeted library tests:
   - `cargo test -p sniff remote`
   - `cargo test -p sniff --test remote_providers`
2. Run targeted CLI tests:
   - `cargo test -p sniff-cli args`
   - `cargo test -p sniff-cli output`
3. Run checks for both touched packages:
   - `cargo check -p sniff`
   - `cargo check -p sniff-cli`
4. Manually exercise CLI behavior:
   - `sniff repo pr`
   - `sniff repo pr --status merged`
   - `sniff repo pr --status draft --json`
   - `sniff repo pr -v`
5. If schematic definitions changed, run the appropriate schematic/schema checks with `--manifest-path schematic/schema/Cargo.toml`.
6. Update drift-sensitive docs if behavior changed beyond the feature spec:
   - `sniff/cli/README.md`
   - `sniff/lib/README.md` if public library examples mention remote PR APIs
   - `.claude/skills/sniff/SKILL.md` only if the repo workflow or command catalog changes materially.

Final acceptance criteria:

- `sniff repo pr` defaults to open PRs from the current repository's upstream remote.
- `--status` supports `open`, `closed`, `merged`, `draft`, and `all`.
- `--json` includes every available `PullRequestInfo` field, including `labels` and `body`.
- `-v` renders the detailed PR block view.
- Auth, rate-limit, unsupported-platform, no-remote, invalid-status, and network failures produce actionable messages.
