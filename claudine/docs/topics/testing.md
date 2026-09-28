# Testing

Claudine's default verification path is the package-area `just` recipes, which run package-scoped nextest when `cargo-nextest` is installed and fall back to `cargo test` otherwise:

```bash
cargo --version
rustc --version
just test
just lint
```

The root nextest config marks tests as slow after 5 seconds in the default profile and after 10 seconds in CI. CI also writes JUnit output to `test-results.xml`.

Additional expectations for the Claudine test suite:

- New and modified tests should prefer `#[rstest]` when fixtures or parameterized cases make the setup clearer; do not bulk-migrate unrelated tests.
- Tests that mutate process-global state, especially environment variables, should stack `#[serial_test::serial]` directly on the test.
- Environment setup/teardown should use `test_toolkit::EnvGuard` instead of local guard types. `EnvGuard::set` and `EnvGuard::remove` are unsafe and require the test to serialize environment access.
- Use `test_toolkit::trace_phase!` only for meaningful setup, body, or teardown boundaries where a tracing span helps diagnose hangs or fixture failures.
- PTY coverage stays manual-only and lives under the L2 filter set: `cargo test -p claudine-cli --test level2_pty_tests -- --ignored` (or `just test-l2`)
- Snapshot updates should be reviewed with `cargo insta review` and accepted with `cargo insta accept`
- Benchmarks are opt-in and non-gating: `cargo bench -p claudine --bench runtime_hot_paths`
- CLI integration helpers live under `claudine/cli/tests/common/mod.rs`
- Inline unit tests are preferred for private library logic such as harness, sequence, dispatch, and TUI reducers

## Silent audio fixtures

Use clear speech such as “This is a test message.” and explicit zero volume for
real playback. Pin test providers and voices; these tests do not validate the
operator's personal Claudine speech configuration. A fake agent does not disable
lifecycle TTS or sound effects in a real prompt template.

`CliProcessFixture::command()` already sets child-only `PLAYA_DRY_RUN=1` and a
private `PLAYA_SPOOL_DIR` (`fixture.audio_spool()`); a test that executes a
shipped prompt asserts that directory is never created. This preserves normal
lifecycle evaluation while suppressing audio publication. `detached_audio.rs`,
whose subject is publication, removes the dry-run key on its built command.
Publication tests retain real handoff with the workspace-shared
`test_toolkit::LockedAudioSpool` fixture: it holds `worker.lock` for its whole
lifetime and takes `queue.lock` before scanning and removing pending jobs, so a
publisher or preparation helper cannot commit behind the scan. Destruction runs
the same cleanup while unwinding, and releases `worker.lock` while still holding
`queue.lock` — the order `run_scheduler_with` uses — so no publisher can commit
between the final scan and the release of worker ownership. A cleanup failure is
reported on stderr and retains worker ownership instead of exposing what it
could not remove; it also panics unless the thread is already panicking.
Tests that execute helpers must release blocked fixtures and observe completion
before removing their spool. Never use the operator's real queue for test cleanup.

## Messaging fixtures

A test that needs a real outbound message uses the loopback webhook listener in
`claudine/cli/tests/common/webhook_listener.rs`, never a real Discord or Slack
endpoint. For example:

```rust
write_webhook_route(fixture.home());
let listener = WebhookListener::start(ListenerMode::WithholdUntilReleased);
let mut command = fixture.command_std();
listener.apply_route_env(&mut command);
// spawn, wait for listener.wait_for_request(..), then listener.release()
```

- `write_webhook_route` writes an active `discord_webhook` route that reads its
  URL from an environment variable. The inline URL validator accepts only
  production Discord hosts, and the environment-backed route keeps that
  validation in the path under test.
- `apply_route_env` points that variable at the listener, removes inherited
  proxy variables, and sets `NO_PROXY`.
- The URL's token is `dummy-token`. Redaction only recognizes
  `https://discord.com/...` URLs, so an error string can print the loopback URL
  verbatim; assert the token is absent where a test checks redaction.
- `ListenerMode::Reply(status)` answers at once, `WithholdUntilReleased` holds
  every reply until `release()`, and `NeverReply` never answers. One polling
  thread serves every connection, so several held deliveries can be open at
  once, and a watchdog stops the listener even if a test forgets to.
- `write_config_with_webhook_route(home, json)` writes the same route next to
  other config keys. A `claudine handle` test uses it to add hook `actions`,
  for example `{"actions": {"session_end": [{"type": "message", "message": "…"}]}}`.
- A test of a desktop `notify` must stay silent without a production
  override. Make every backend fail fast instead: on Unix, spawn with
  `command_builder().fake_only_path()` so neither a notification helper nor
  `osascript` is on `PATH`, and set `DBUS_SESSION_BUS_ADDRESS` to a missing
  socket for Linux. Windows fails before the toast API because Claudine
  configures no AppUserModelID.
- A test that waits out the 10-second exit drain stays in L1 under an ordinary
  name. Claudine does not declare `l1-include-slow`, so a `slow_` prefix
  would leave the test running in no recipe at all.
