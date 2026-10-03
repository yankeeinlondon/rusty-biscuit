# Testing the Worktree Area

Load before writing or debugging worktree tests. Repo-wide test rules live in
the `rust-testing` skill; graph tests are in [git-graph.md](git-graph.md#tests).

## Rules that apply everywhere

- **No test touches the network.** PR and live-head requests go to local stubs
  (`ProxyStub`, `FakeGitea`, `HoldingOrigin`, local bare origins).
- Test origins use `example.invalid` hosts so a user `insteadOf` rule cannot
  rewrite them.
- Tests inject the source, so they count requests with a stub.
- `wt` colors stderr even when captured; set `NO_COLOR=1` when asserting text.
- Every fixture that can spawn a detached worker must release it in a drop guard
  (`list_prs::ReleaseOnDrop`), so a failed assertion still closes the
  connection and waits for the process and both locks.
- On Windows the user cache ignores `HOME`, so tests that seed a store use the
  real per-user path there. `--ignore-api` binary tests are Unix-only (native
  Windows resolves the home directory without `HOME`).
- Inside tmux `wt` cannot query the background: pass `COLORFGBG` (`15;0` dark,
  `0;15` light), or the row highlight follows the host's appearance.

## Running one L2 binary

`just test-l2 <substring>` matches test names only, and a quoted
`-E '<filterset>'` breaks the recipe's bash. Run one binary directly:

```sh
BISCUIT_TEST_REQUIRED_BACKENDS=tmux cargo nextest run -p worktree-cli \
  --features terminal-tests -E 'binary(level2_remove)'
```

The variable turns a missing-backend skip into a failure.

## `wt remove`

- Lib unit tests use `remove::test_support::TestRepo` (local bare `origin`, plus
  a `pusher` clone for other people's pushes).
- `cli/tests/remove.rs` sets `NO_COLOR=1`. Its zero-network tests point origin
  at `http://gitea.test/…` with every proxy variable aimed at a local counting
  listener, since a file-path origin never reaches the PR lookup.
- `cli/tests/level2_remove.rs` drives bash, zsh, and fish wrappers in tmux, with
  a `wt` shim that runs `WT_TEST_BETWEEN` before the handoff call.
- `cli/tests/powershell_wrapper_exec.rs` is Windows-only L1.
- `cli/tests/level2_powershell_remove.rs` is the Windows L2:
  - PowerShell 5.1 in a ConPTY pseudoconsole (`xpty`, no window, no focus),
    launched *inside* the worktree, the prompt answered through the console;
  - assertions read the console's own screen buffer
    (`$Host.UI.RawUI.GetBufferContents`), since ConPTY's output stream is a
    repaint, not text in order;
  - a third case runs `wt remove --force-worktree` from the base repo while a
    windowless `ping` stands in the worktree, and asserts exit 4 with files,
    registration, and branch intact.
  - It needs no terminal backend, so **CI never runs it**: `windows-latest`
    hosts none of `l2-backends` and the L2 cell is an accepted gap. Run it with
    `./scripts/cross-check.sh --os windows worktree-cli --features terminal-tests level2_powershell`
    and read the durations (about 5 s each; a skip is 0.0x s).

## `wt list`

### Network stand-ins (`cli/tests/perf_support`)

| Stand-in | What it does | Use when |
| -------- | ------------ | -------- |
| `ProxyStub` | `HTTPS_PROXY` to a hanging or refused local port (sniff's reqwest honors it). `closing_after(hold)` counts connections and drops each after `hold`. | HTTPS PR paths; one connection per worker half |
| `FakeGitea` | Plain-HTTP provider. Answers `/branches/` 404 at once (or, after `hold_branch_heads`, once released), counted apart (`branch_requests`), so `requests()` counts PR requests only. `hold` stalls PR requests. `serve_repositories` answers git smart HTTP via `git http-backend`; `hold_git(GitHold::All \| Fetch)`; `answer_branch_heads_with(429)` | holding a PR request or check |
| `perf_support::HoldingOrigin` | Loopback HTTP origin that records each request line and holds git's `ls-remote` until `close_held` | live-head path with real git HTTP |

- **Hold PR requests at `FakeGitea`, never at a hanging `ProxyStub`**: CONNECT
  races sniff's 3 s connect timeout against the listing's 3 s wait.
  `ProxyStub::closing_after(hold)` exists for the same reason.
- `hold_branch_heads` holds the check with no connect timeout racing the 3 s
  wait.

### `MixedFixture` and helpers

- Its network-free command refuses git's own HTTP transports (`GIT_CONFIG_*`
  `protocol.http(s).allow=never`), because git honors the same proxy variables
  and the live-head `ls-remote` would otherwise be counted as a PR request.
- Tests that count PR requests also seed a fresh head (`seed_remote_head_store`);
  that only dates the caption — the worker and its check still run.
- `MixedFixture::with_local_origin`: a bare copy in sync (worker answers at
  once).
- `MixedFixture::with_origin(url)` + `wt_command_direct()` (no proxy variables,
  no user or system git config) to make git's HTTP transport actually reach a
  `HoldingOrigin`.
- `MixedFixture::serve_gitea_origin_one_commit_ahead` with
  `wt_command_via_gitea_git`: a real `origin` behind `FakeGitea`
  (`list_prs::a_pr_failure_never_blocks_a_permitted_fast_forward`).
- `seed_empty_pr_store(age)` isolates the live-head path from PR requests;
  `isolated_cache_file(home, xdg, real)` resolves a store path the way `wt` will.

### Tearing down a detached worker

A stale-store test's worker must not outlive the fixture: wait for its
connection, then `close_held()` and wait until neither lock is `Contended`
**and** no `internal-refresh` process runs (`MixedFixture::wait_until_unlocked`,
used by `list_prs::finish_worker`). A free PR lock alone no longer proves the
worker is gone.

### Real-Git live-head proofs

- `cli/tests/list_remote_head.rs`: local bare `origin`, a `pusher` clone, and
  either `wt list` or `wt internal-refresh <main>` run as a direct child and
  waited for.
- Its fixture is shared with `cli/tests/list_flags.rs` (`-r`, `--ignore-api`,
  `--ff` end to end) as `cli/tests/remote_fixture/`.
- `remote_fixture::UploadPackGate` points `remote.origin.uploadpack` at a script
  that counts runs and holds the Nth one (0: the check's `ls-remote`, 1: the
  fetch) until released — how the still-checking, still-pulling, and adoption
  rows are proven with a local origin.
- Binary tests find receipts by prefix beside the store
  (`list_flags::receipts_beside`), since the wait deletes them.
- Library transport tests reuse `live_remote::tests::Loopback` (the module is
  `pub(crate)`); their repositories set an empty `http.proxy` and
  `credential.helper`.

### Performance gates

Measurements: `worktree/docs/performance-testing.md`.

- `perf_pr_request::perf_list_meets_sla_with_a_stale_answer_and_a_failing_refresh`
  is the stale timing gate: reseeds a stale store per sample and asserts every
  `pr gather` < 300 ms (the 1 s full-command bound alone would hide a
  reintroduced wait).
- `perf_a_held_live_head_check_costs_the_listing_only_its_wait`,
  `perf_a_held_fetch_costs_the_listing_only_its_wait`, and
  `perf_a_held_pr_request_costs_the_listing_only_its_wait` (`FakeGitea::hold`)
  bound a stalled worker at 3 s plus the 1 s bound.
- `perf_command_sla` holds the 1 s bound with a worker that answers at once.
- `-r` against a held check and `--ff` against a held fetch are bounded by the
  worker's 10 s and 60 s deadlines (the 60 s test has a 90 s nextest override in
  `.config/nextest.toml`).
- The deterministic no-join proof is the `list_prs` tests whose captured
  `.output()` returns while the worker's request is still held.

### Styling at L2 (`cli/tests/styled_capture/`)

Styling is proven on **cells, not bytes**: `styled_capture` parses a
`tmux capture-pane -e` frame into per-cell SGR state (tolerating tmux's merged
params and 256-color downgrade).

- `level2_list_verbose::level2_list_styles_follow_the_design_in_tmux` and
  `level2_dirty_tree` assert each dot, badge, connector, count (`+N` dim green,
  `-N` dim red, in a pane asserted ≥ 100 columns), file name, and the
  highlighted row with the same values as L1 `styles_follow_the_design`.
- The width gate is proven in real panes resized to exactly 99 and 100 columns
  (`TmuxHarness::resize`, via `DesignFixture::list_at`):
  `level2_list_hides_the_counts_in_a_99_column_pane` and
  `level2_list_shows_target_and_parent_counts_in_a_100_column_pane` use
  `DesignFixture::with_child` (child's fork parent is `feature-test`, so its
  `-> parent` cell carries counts and a PR badge). At 100 that badge wraps
  between `PR` and `#105`, because a badge's inner space is a break opportunity
  for `Table`.
- `DesignFixture` panes run with no user or system git config and git's HTTP
  transports refused (`GIT_CONFIG_*`), so the live-head `ls-remote` fallback can
  reach neither the proxy nor a real host (the L2 binary includes
  `perf_support` for that).

| Test | Fixture | Proves |
| ---- | ------- | ------ |
| `level2_list_stale_pr_answer_shows_a_dim_age_line_in_tmux` | `DesignFixture::with_gitea_pr_age` (stored answer names `o/r`); PR request held at `FakeGitea` past the 3 s wait | dim `- PRs as of` item directly beneath the legend (tmux draws no graph), dim hint after it, spinner gone, caption not "still checking" |
| `level2_list_failed_pr_refresh_shows_a_dim_couldnt_refresh_item_in_tmux` | | dim `(couldn't refresh)` item and no hint |
| `level2_list_credentials_warning_is_a_dim_line_beneath_the_caption_in_tmux` | `DesignFixture::with_gitea_origin` (404 without a key plus the refused fallback) | dim §5 line directly beneath the caption |
| `level2_list_clears_the_spinner_before_the_caption_and_shows_a_dim_hint_in_tmux` | `hold_branch_heads`, `DesignFixture::start_in_pane` | spinner seen mid-wait, then no frame or spinner text remains; dim hint follows the legend |
| `level2_list_spinner_moves_from_the_fallback_to_the_fetch_on_one_line_in_tmux`, `level2_list_spinner_shows_the_rate_limited_fallback_in_tmux` | `DesignFixture::with_gitea_repository` (bare `o/r.git` one commit past `origin/main`, pane's `http.proxy` at the stand-in) + `FakeGitea::serve_repositories` + `hold_git(GitHold::All \| Fetch)`; `answer_branch_heads_with(429)` for rate-limited | under `wt list -r` each phase is exactly one `<frame> <text>` line, fetch text replaces the longer fallback text with no remnant, no spinner before the caption when finished |

The caption suffix's dim italic is asserted in the design test.
