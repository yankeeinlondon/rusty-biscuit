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
| `FakeGitea` | Plain-HTTP provider. Answers `/branches/` 404 at once (or, after `hold_branch_heads`, once released), counted apart (`branch_requests`), so `requests()` counts PR requests only. `hold` stalls PR requests. `serve_repositories` answers git smart HTTP via `git http-backend`; `hold_git(GitHold::All \| Fetch)`; `answer_branch_heads_with(429)`; `answer_branch_heads_at(sha)` makes the API check succeed | holding a PR request or check; a successful (anonymous unless the command sets a token) API answer |
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
- Seeded PR stores carry `"credentials": {"state": "unknown"}` (required
  since format 6), so a seeded answer can never produce the keyless notice.
- Keyless regressions through the binary (`list_prs::*keyless*`): PR success
  with head 500, head success (`serve_gitea_origin_one_commit_ahead` +
  `answer_branch_heads_at`) with PR 500, both, a keyed run (`GITEA_TOKEN` on
  the command; its value in neither store nor output), and coexistence with
  the fallback notice (head 401 + answering `ls-remote`).
- `seed_empty_pr_store(age)` isolates the live-head path from PR requests;
  `isolated_cache_file(home, xdg, real)` resolves a store path the way `wt` will.

### The pipeline overlap seam (`lib/src/list/tests.rs`, `lib/src/list/tests/pipeline.rs`)

- `tests::overlap` is a `#[cfg(test)]` seam inside `worktree::list::gather`: each
  local gather reports `arrive`/`finished`, the calling thread
  `remote_finished`. Without `overlap::Installed` it does nothing. Modes:
  `Rendezvous` (list and graph each wait for the other to start), `Observe`
  (record only), `HoldListUntilRemote` (the list gather starts after the
  wait). Every wait is bounded (10 s), so a non-overlapping pipeline fails
  instead of hanging; no sleeps or elapsed-time asserts.
- `pipeline.rs` drives `gather` with a **scripted launch**: it runs on
  the calling thread inside the wait, so blocking in it (on
  `overlap::await_both_started` / `await_both_finished`) holds the worker's
  outcome. Released, it runs its ref `moves` (as a fetch would), then records
  a finished attempt and receipt in the real per-user stores, or stays silent
  (`finishes: false`) for timeout cases. `Repo` removes every
  `<repo hash>.*` cache file on drop.
- Assert through the returned `Listing` and counters: `git status` walks,
  `for-each-ref` reads, `merge-tree` calls (via `recorder`), and
  `Listing::regathered`; never a timing. The timing shape per path, the
  `git_calls` coverage, and timings on/off doing the same work live in
  `pipeline/timings.rs` (stage paths, decoded through `Timings::from_json`).
  `assert_describes_the_final_state` compares caption, target, tree, counts,
  dirtiness, graph, and verbose labels with a from-scratch gather after the
  run.
- Fail the Nth ref read with `recorder::fail_matching` plus an `AtomicUsize`
  (`for-each-ref` is the only call it matches in a listing).
- `recorder` and `git::calls` are different counts. The recorder logs every
  **requested** call into one process-global log, *before* spawning, so an
  injected failure or a failed spawn is still logged; it needs `count-git`
  (or `cfg(test)`) and `#[serial_test::serial]`. `calls::CallScope` counts
  only **started** processes, per thread, is always compiled, and needs no
  serialization. With no injection and every spawn succeeding, the two agree:
  `listing::repo_tests::a_counting_scope_sees_every_call_of_a_threaded_local_gather`
  asserts exactly that, which is how a missing `TaskHandle` at a spawn site
  shows up. `live_remote::run_transport` is the exception: its own Git
  process is never recorded, so it counts one more than the recorder.
- `run_pipeline_gathers_the_graph_while_list_gather_is_unfinished` has no
  `origin`: it proves list-versus-graph overlap only. Overlap with the wait is
  `pipeline::the_local_gathers_start_while_the_worker_outcome_is_held`.

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
  rows are proven with a local origin. The hold loop also ends when the gate
  directory disappears: a test killed before the gate's `Drop` (nextest
  timeout, Ctrl-C) once left a `git ls-remote` + `upload-pack.sh` pair
  orphaned under launchd for hours. A long-lived `upload-pack.sh` in `ps`
  points to a gate written without that exit.
- A held-worker test that finds the worker with `refresh_workers` after
  `gate.wait_for_runs` (e.g.
  `a_failed_assertion_while_upload_pack_is_held_still_reaps_the_worker_before_the_fixture_goes`)
  can see zero workers under heavy host load (load average 40+): once 10 s
  pass from the worker's launch, its check deadline kills the held
  transport and the worker exits. A run near 10 s rather than the usual
  3.4 s is load, not a regression; rerun before investigating.
- Binary tests find receipts by prefix beside the store
  (`list_flags::receipts_beside`), since the wait deletes them.
- Library transport tests reuse `live_remote::tests::Loopback` (the module is
  `pub(crate)`); their repositories set an empty `http.proxy` and
  `credential.helper`.

### Performance gates

Measurements: `worktree/docs/performance-testing.md`.

- Run `wt list --perf=json` and read it with `perf_support::perf_timings`
  (the final nonempty stderr line, LF or CRLF, prefix `WT_PERF_JSON `; never
  earlier text), then `stage_at(&timings, &[Stage::…])` or `local_gather`.
  Never read the human report: labels are display only. The old scrapers
  (`perf_rows`, `stage_from_perf`, `list_gather_from_perf`) have no
  consumers left and are due for deletion.
- An L1 test that bounds how long a held worker keeps `wt list` waiting reads
  `[RemoteAndLocal, RefreshWorker]`, not the whole command's elapsed time. The local
  gather overlaps the wait and may outlast it, so a whole-command bound fails
  under suite load without a wait regression (`list_prs.rs` held-request
  tests). Whether the command returned while the worker is still held, that is,
  never joined it, is proven by `.output()` returning, not by a duration.
- `perf_support`'s own unit tests sit under a `perf_`-prefixed module, so they
  run only in `just test-perf`; the L1 parser and report-shape tests are in
  `cli/tests/perf_flag.rs`.

- `perf_pr_request::perf_list_meets_sla_with_a_stale_answer_and_a_failing_refresh`
  is the stale timing gate: reseeds a stale store per sample and asserts every
  sample's foreground PR reads (`pr gather` + `pr reread`) < 300 ms (the 1 s full-command bound alone would hide a
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
| `level2_list_credentials_warning_is_a_closing_note_in_tmux` | `DesignFixture::with_gitea_origin` (404 without a key plus the refused fallback) | undimmed §5 closing note after the legend; it outranks the anonymous PR answer in the same run |
| `level2_list_keyless_notice_is_a_closing_note_after_the_spinner_in_tmux` | `with_gitea_origin`, `answer_branch_heads_with(500)` held until the spinner draws, PR answered anonymously | undimmed keyless closing note after the legend, spinner frames and text gone |
| `level2_list_clears_the_spinner_before_the_caption_and_shows_a_dim_hint_in_tmux` | `hold_branch_heads`, `DesignFixture::start_in_pane` | spinner seen mid-wait, then no frame or spinner text remains; dim hint follows the legend |
| `level2_list_spinner_moves_from_the_fallback_to_the_fetch_on_one_line_in_tmux`, `level2_list_spinner_shows_the_rate_limited_fallback_in_tmux` | `DesignFixture::with_gitea_repository` (bare `o/r.git` one commit past `origin/main`, pane's `http.proxy` at the stand-in) + `FakeGitea::serve_repositories` + `hold_git(GitHold::All \| Fetch)`; `answer_branch_heads_with(429)` for rate-limited | under `wt list -r` each phase is exactly one `<frame> <text>` line, fetch text replaces the longer fallback text with no remnant, no spinner before the caption when finished |

The caption suffix's dim italic is asserted in the design test.

### Unavailable rows at L2 (`DesignFixture::break_worktree`)

- **A deterministic `?` row:** overwrite the linked worktree's
  `<base>/.git/worktrees/<name>/index` with junk. `git worktree list` still
  lists it without `prunable`, but `git status` in it exits 128 ("index file
  smaller than expected") on every OS, so `wt` shows a dim `?`. Deleting the
  directory or the `.git` file makes Git mark the entry `prunable` instead
  (`✕`); a directory link in place of a moved checkout is unmarked but `✕`
  (Unix-only fixture: Windows symlinks need a privilege).
- Notes print after the legend, so waiting for `parent deleted` can capture
  before them. `DesignFixture::list_to_end` runs `wt list; echo wt-list-""done`
  (the quotes keep the typed line from matching), waits for the marker, and
  captures with `capture-pane -J`, which joins each soft-wrapped line into
  one row and keeps the spaces at the break; without `-J` a narrow pane splits
  a soft-wrapped line into rows and drops the space at the break.
- `break_worktree` puts its worktrees under `LONG_DIR`, a name longer than a
  note line at 80 columns, so every host (Linux temp paths are short) proves
  that a path or command is shown whole rather than broken by the wrap. The
  60-column pass checks rows and notes only: the table's `-> parent` header
  and the legend lines overrun the pane there with or without broken rows.
