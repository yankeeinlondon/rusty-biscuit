# Spawning the Binary Under Test (L1)

Part of the `rust-testing` skill; [SKILL.md](SKILL.md) is the index.

An L1 test that runs its crate's own binary must run it against a workspace the
test built, never the checkout the suite was compiled from. Make that hold **by
construction**, through one shared command builder per package area, not
per-test `.env(...)` chains.

Inheriting the runner's environment hurts twice:

- **Cost.** Under nextest the ambient working directory is inside the monorepo,
  so anything that discovers a repository walks the whole workspace; on a slow
  filesystem (a WSL2 guest) that is seconds per test.
- **Correctness.** The child sees the checkout's git state and root files, the
  developer's `$HOME`, and every real CLI on a full host `PATH`, so assertions
  pin whichever machine ran them.

## The shape

`claudine-cli`'s L1 suite is the reference implementation.

| Piece | Where | Contract |
|---|---|---|
| Fixture | `CliProcessFixture` in `claudine/cli/tests/common/mod.rs` and `darkmatter/cli/tests/common/fixture.rs` | Per-test temp `cwd`/`home`/`bin` plus area-owned config, cache, and temp policy; platform home variables point inside the fixture |
| Builder | `CliProcessFixture::command()` / `command_builder()` | The one supported spawn, with `current_dir` pinned to the fixture `cwd` |
| Raw surface | `command_std()` / `command_builder()…build_std()` | The same policy on a `std::process::Command`, for a test that must keep the child (a signal, a deadline, a streaming read, an `expectrl` session) |
| Guard | `cli/tests/l1/spawn_site_guard.rs` in claudine, darkmatter, and sniff | Source scan; a raw `Command::cargo_bin("<bin>")`, an isolation escape, or a stale exemption fails the suite |

**Two command surfaces, one policy.** `assert_cmd::Command` cannot `spawn`, so
live-child tests need `std::process::Command`. Do not give the two separate
builders: compute the policy once as data (clear flag, ordered removes, ordered
sets, `current_dir`), apply it through a small trait each command type
implements, and assert both produce the **same effective environment** against
a recording stub (`claudine/cli/tests/l1/cli_process_fixture.rs`).

## PATH

**Default `PATH` is the fixture `bin` plus a minimal system set**: `/usr/bin:/bin`
on Unix, `%SystemRoot%\System32` on Windows, `PATHEXT` untouched so `.cmd` stubs
resolve. Fake-only would break every subject that shells out by bare name
(`sh`, `cmd`, `git`, `sleep`); the minimal set still excludes the Homebrew,
npm, cargo, and `~/.local/bin` prefixes.

- Escapes are named methods, each needing a call-site comment naming the tool
  or proof it needs: `fake_only_path()`, `host_path()`, and
  `ambient_context(dir)`, which pins the launch cwd to a repository the test
  built inside its own workspace and panics on anything outside it.
- The Windows system set has no Git for Windows or Unix utilities: use native
  fixture scripts or explicit fixture tools with `std::env::join_paths`.
- This bounds lookup, not security. Use fake-only lookup when absence is the
  assertion.
- Reject fixture roots inside the checkout, including through symlinks, so
  ancestor discovery cannot undo isolation.

## Environment scrubbing

The builder also controls inherited application variables, Git plumbing
(`GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE`), rendering inputs (width, forced
color), and selectors for third-party tools the application launches (in
claudine, every provider overlay selector such as `CODEX_HOME`, read from
provider metadata, plus profile-owned state such as `CODEX_SQLITE_HOME`); a
suite run from inside an agent session exports them.

- Scrub inherited values first, then apply intentional overrides.
- Keep cache roots and platform home variables inside the fixture.
- Choose a documented deny list or a cleared environment from actual runtime
  needs; Windows command stubs may need `SystemRoot`, `COMSPEC`, and `PATHEXT`
  restored.

## Side effects that outlive the process

Default off anything the child can start that its own exit does not end.
Claudine's builder sets:

- `CLAUDINE_RENDEZVOUS_REPORT=false`: absence means *enabled*, so a test that
  forgets the key reports a live session to the developer's daemon.
- `PLAYA_DRY_RUN=1` plus a fixture-local `PLAYA_SPOOL_DIR`: a lifecycle audio
  effect re-execs the CLI as playa's detached spool worker, which survives the
  command, leaves orphans, and plays sound. `claudine/cli/tests/l1/detached_audio.rs`
  opts out per key. `just test-leaks` surfaces such leaks; no assertion will.

Both keys sit in namespaces the builder sweeps by prefix, so scrub-then-default
ordering is part of the contract. Assert the defaults against a parent that
exported the opposite value; comparing the two command surfaces cannot catch a
policy that is identically wrong on both.

## The spawn-site guard

Give the guard an **explicit allowlist of `(file, one-line reason)` entries
that fails on a stale entry**, so the list burns down instead of becoming a
grandfather table (same mechanics as `claudine/cli/tests/l1/dispatch_inventory.rs`).
Strip comments and string literals before searching, as
`claudine/cli/tests/l1/test_placement.rs` does, so prose mentions do not
false-positive.
