# Spawning the Binary Under Test (L1)

Part of the `rust-testing` skill; [SKILL.md](SKILL.md) is the index.

An L1 test that runs its crate's own binary must run it against a workspace the
test built — never against the checkout the suite was compiled from. Make that
hold **by construction**, through one shared command builder per package area,
rather than per-test `.env(...)` chains.

Inheriting the runner's environment costs twice:

- **Cost.** The ambient working directory under `cargo nextest` is the package
  directory inside the monorepo, so anything that discovers a repository walks
  all 35 members. That is ~220 ms on an idle 16-core Mac and 20–77 s per test in
  a WSL2 guest reading its workspace over the Windows disk.
- **Correctness.** The child also sees the checkout's git state and root
  `system-prompt.md`, the developer's `$HOME`, and — with a full host `PATH` —
  every real CLI installed on the machine. Assertions then pin a snapshot of
  whichever machine ran them.

The shape that fixes it, with claudine's `claudine-cli` L1 suite as the
reference implementation:

| Piece | Where | Contract |
|---|---|---|
| Fixture | `claudine/cli/tests/common/mod.rs` and `darkmatter/cli/tests/common/fixture.rs` — `CliProcessFixture` | Per-test temp `cwd`/`home`/`bin` plus area-owned config/cache/temp policy; platform home variables point inside the fixture |
| Builder | `CliProcessFixture::command()` / `command_builder()` | The one supported spawn. `current_dir` pinned to the fixture `cwd`; child-local `PLAYA_DRY_RUN=1` and a private `PLAYA_SPOOL_DIR` so shipped `say:`/`effect:` lifecycle actions stay silent (`detached_audio.rs` opts out per key) |
| Raw surface | `command_std()` / `command_builder()…build_std()` | The same policy on a `std::process::Command`, for a test that has to keep the child — a signal, a deadline, a streaming read, an `expectrl` session |
| Guard | `cli/tests/l1/spawn_site_guard.rs` in claudine, darkmatter, and sniff | Source scan; a raw `Command::cargo_bin("<bin>")`, isolation escape, or stale exemption fails the suite |

**Two command surfaces, one policy.** `assert_cmd::Command` has no `spawn`, so a
live-child test needs a `std::process::Command` — and hand-building one
re-derives the isolation at the call site, which is the per-test `.env(…)` chain
the fixture replaced. Do not give the two surfaces separate builders: compute
the policy once as data (clear flag, ordered removes, ordered sets,
`current_dir`) and apply it through a small trait each command type implements.
Then assert the two produce the **same effective environment** against a
recording stub, so a policy change that reaches only one of them fails there
rather than in a platform-only test months later.

**Default `PATH` is the fixture `bin` plus a minimal system set** — `/usr/bin:/bin`
on Unix, `%SystemRoot%\System32` on Windows, `PATHEXT` untouched so `.cmd` stubs
resolve. Fake-only is stricter but breaks every test whose subject shells out
by bare name (`sh`, `cmd`, `git`, `sleep`), so the opt-out would become the
norm; the minimal set still excludes the Homebrew, npm, cargo, and
`~/.local/bin` prefixes real tools install into. Escapes are named methods, and
each requires a call-site comment naming the tool or the proof it needs:
`fake_only_path()`, `host_path()`, and `ambient_context(dir)` — the last pins
the launch CWD to a repository the test built *inside its own workspace* and
panics on anything outside it.

The tool roster is platform-specific: the Windows system set does not provide
Git for Windows or Unix utilities. Use native fixture scripts or explicit
fixture tools and `std::env::join_paths`; do not assume a Unix roster on Windows.
This PATH convention bounds lookup, not security: tools installed in the
allowed directories remain visible. Use fake-only lookup when absence is the
assertion. Reject fixture roots inside the checkout, including through symlinks,
so ancestor discovery cannot silently undo isolation.

The builder must also control inherited application variables, Git plumbing
such as `GIT_DIR`/`GIT_WORK_TREE`/`GIT_INDEX_FILE`, and rendering inputs such as
width and forced color. That includes selectors for third-party tools the
application launches (claudine: every provider overlay selector such as
`CODEX_HOME`, read from provider metadata, plus profile-owned state such as
`CODEX_SQLITE_HOME`) — a suite run from a wrapped agent session exports them.
Scrub inherited values before applying intentional test overrides. Keep cache roots and platform home variables inside the
fixture where the application uses them. Choose a documented deny list or a
cleared environment based on actual runtime needs; Windows command stubs may
need `SystemRoot`, `COMSPEC`, and `PATHEXT` restored.

**Also disable the side effects that outlive the process.** Two of claudine's
defaults exist only because the child can start work that the child's own exit
does not end:

- `CLAUDINE_RENDEZVOUS_REPORT=false` — absence means *enabled*, so a test that
  merely forgets the key reports a live session to the developer's daemon.
- `PLAYA_DRY_RUN=1` plus a fixture-local `PLAYA_SPOOL_DIR` — a lifecycle audio
  effect makes the CLI re-exec **itself** as playa's detached spool worker,
  which deliberately survives the command that enqueued the job. Without the
  default, an L1 test that composes a prompt carrying such an effect leaves two
  orphaned binaries per run and plays a sound through the developer's speakers.
  `just test-leaks` is what surfaces this; no assertion in the test will.

Both live inside namespaces the builder sweeps by prefix, so the *ordering* is
part of the contract: scrub first, then apply defaults. Assert them against a
parent that exported the opposite value — a set-equality comparison between two
command surfaces cannot catch a policy that is identically wrong on both.

Give the guard an **explicit allowlist of `(file, one-line reason)` entries with
stale-entry failure**: an entry matching no live site fails too, so the list
burns down instead of becoming a grandfather table. Same mechanics as
`dispatch_inventory.rs`; sanitize comments and string literals before searching,
as `test_placement.rs` does, so prose mentions don't false-positive.
