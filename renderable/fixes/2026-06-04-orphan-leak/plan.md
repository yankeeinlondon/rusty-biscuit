---
status: ready for implementation
date: 2026-06-04
owner: ken
area: biscuit-test-harness
parent: .claude/skills/rust-testing/apple-terminal-harness-pitfalls.md
---

# Fix: Apple Terminal orphan-window leaks (Pitfall 2)

## Problem

`AppleTerminalHarness` leaks Terminal.app windows. Killed/timed-out L2 runs (and
any panic before `Drop`) leave their spawned windows open, and they accumulate
across runs into dozens of idle login-shell windows. Beyond clutter, a large
window set makes `osascript` window lookups slower and ambiguous and worsens the
`do script` reuse hazard.

The harness *has* a reaper —
`cleanup_stale_apple_terminal_windows()` (`apple_terminal.rs:308`) — but it
**cannot see the leaks**. It identifies harness windows by a custom title tag
`biscuit-test-terminal-<pid>-<seq>` (`WINDOW_TITLE_PREFIX`, `unique_window_tag()`)
and closes only tagged windows whose owner pid is dead. Three things defeat it:

1. **The title tag does not survive.** `spawn_shell` runs `exec <shell> -l`; the
   login shell's prompt (`zsh precmd` / `bash PROMPT_COMMAND`) emits OSC 0/2
   title escapes that overwrite Terminal's `custom title`. Leaked windows show
   `custom title = "Terminal"`, so the prefix scan matches **nothing**
   (empirically: 60 leaked windows, 0 matched the prefix).
2. **No workspace isolation.** WezTerm quarantines harness panes in a dedicated
   `biscuit-bg` workspace (`wezterm.rs:33`) and can therefore bulk-sweep it
   safely (`BISCUIT_TEST_HARNESS_SWEEP_LEGACY_WEZTERM`). Terminal.app has no
   equivalent — harness windows and the developer's windows share one global
   window list, so "close everything that looks idle" is unsafe in general.
3. **macOS window restoration.** "Reopen windows when reopening an app" can
   restore closed Terminal windows on the next launch with **new** window ids and
   fresh login shells, re-leaking them and defeating any id-based reaping done in
   a prior process.

## Goals

- Leaked harness windows are reaped reliably **regardless of window title**.
- Reaping is **user-safe**: it never closes a window the harness did not create
  (the developer's windows are untouched) by default.
- A spawn proactively cleans prior-run orphans, so leaks cannot accumulate.
- The fix is **focus-free** (no `activate` / `System Events keystroke`) and does
  not regress the [`apple-terminal-harness-pitfalls.md`](../../../.claude/skills/rust-testing/apple-terminal-harness-pitfalls.md)
  invariants (never steal focus, never close a window we didn't create).

## Non-Goals

- The `do script` window-reuse corruption (Pitfall 1) — already fixed
  (ownership guard + lifecycle skip).
- Other backends (WezTerm/tmux/Kitty) — their cleanup already works.
- Eliminating macOS window restoration globally (we mitigate, not abolish).

## Design

Three layers, in priority order. Layer 1 is the primary fix; 2 and 3 are
defense-in-depth.

### Layer 1 — title-independent window-id registry (primary)

Stop relying on the window title for identity. Each spawn records the window it
created in a small on-disk registry; the reaper consumes it.

- **Registry file:** `${TMPDIR:-/tmp}/biscuit-test-terminal-registry.jsonl`,
  one JSON object per line: `{ "window_id": i64, "owner_pid": u32, "seq": u64 }`.
  Append-only writes (`OpenOptions::append`) keep concurrent spawns from
  clobbering each other (single-line `O_APPEND` writes are atomic on macOS for
  small records).
- **On spawn (`spawn_shell`, after the window id is captured):** append
  `{window_id, owner_pid = current_process_id(), seq}`. Do this only when the
  window was genuinely created (`owned == true` after the Pitfall-1 ownership
  guard) — never register a reused window we don't own.
- **Reaper (`cleanup_stale_apple_terminal_windows`, rewritten):**
  1. Read + parse the registry (best-effort; ignore malformed lines).
  2. For each entry whose `owner_pid` is **dead** (`!process_is_alive`) and not
     our own pid: if a Terminal window with that id still exists, **and** it
     passes the idle-shell safety check below, close it `saving no`.
  3. Rewrite the registry keeping only entries whose window still exists **and**
     whose owner is still alive (prune closed/dead rows). Guard the
     read-modify-write with a sidecar lock file
     (`…-registry.lock`, `O_CREAT|O_EXCL`, stale-after-N-seconds) so two
     concurrent reapers don't corrupt it; on lock contention, skip (best-effort).
- **On `Drop` / `close_window`:** also remove the window's row from the registry
  (best-effort) so the common clean path leaves no residue.
- **Window-id reuse safety:** Terminal can recycle a closed window's id. Before
  closing a registry id, verify it still looks like a harness window via the
  idle-shell check (Layer 3's predicate), so a recycled id now hosting real work
  is never closed.

### Layer 2 — make the title tag survive (secondary, complementary)

Make the existing title-based path work again as a backstop and to aid manual
debugging, by stopping the spawned shell from overwriting `custom title`.

- **Option A (preferred): suppress shell title escapes.** Spawn the marker shell
  without rc/profile title hooks. The harness already injects `PATH`, so the
  login profile is not load-bearing for `bt` discovery. Evaluate
  `exec <shell> --noprofile --norc` (bash) / `--no-rcs` (zsh) for the harness
  shell, or inject `PROMPT_COMMAND=` / `precmd_functions=()` + `DISABLE_AUTO_TITLE=true`.
  Risk: a no-rc shell changes the environment; gate behind a characterization
  test that the existing image/color/prose L2 assertions still hold.
- **Option B: re-assert the title.** Set `custom title` *after* `wait_for_prompt`
  and confirm it sticks; if the shell re-overwrites every prompt, Option B is
  insufficient and we rely on Layer 1 — document that.

Layer 2 is optional: Layer 1 already makes reaping title-independent. Land it
only if Option A proves env-safe.

### Layer 3 — opt-in signature sweep for restored / legacy windows (tertiary)

Registry-by-id cannot catch windows **restored by macOS** (new ids, not in the
registry) or leaked by harness versions predating the registry. Mirror WezTerm's
`SWEEP_LEGACY` gate, but stricter because Terminal has no workspace isolation.

- **Predicate `looks_like_harness_window(w)`** (the idle-shell signature, also
  reused by Layer 1's reuse-safety check): `busy of w is false` **and** every
  tab's `processes` ⊆ `{login, <shell>}` (idle login shell, no foreground
  program) **and** default geometry (e.g. 80×24) **and** title is empty/`Terminal`.
- **Gate:** sweep matching windows **only** when
  `BISCUIT_TEST_HARNESS_SWEEP_LEGACY_APPLE=1` is set. **Never on by default** —
  an idle login-shell window can belong to a developer who *does* use
  Terminal.app. (Unlike WezTerm, do **not** auto-trigger on a count threshold.)
- Document that this is safe to enable on CI / on machines where Terminal.app is
  not the interactive terminal.

### macOS window-restoration mitigation

- Document `defaults write com.apple.Terminal NSQuitAlwaysKeepsWindows -bool false`
  as the opt-in way to stop restoration on relaunch; do **not** mutate the
  developer's defaults from the harness.
- Restored windows that do reappear are idle login shells with dead owners → they
  are caught by Layer 3 when the sweep is enabled.

## Phases

### Phase 0 — Characterize (no code)
- Reproduce the leak under a dedicated broker window: spawn, `kill -9` the test
  process mid-run, confirm the window survives and the prefix reaper misses it.
- Confirm the title-overwrite mechanism: print `custom title` immediately after
  spawn vs. after the prompt settles. Records the exact shell hook responsible.

### Phase 1 — Registry write path
- Add `registry_path()`, `register_window(id)`, `unregister_window(id)` to
  `apple_terminal.rs`. Append on spawn (owned-only); remove on `close_window`.
- Unit-test the JSONL round-trip + append concurrency with `tempfile`.

### Phase 2 — Registry-driven reaper
- Rewrite `cleanup_stale_apple_terminal_windows()` to consume the registry
  (dead-owner + still-exists + idle-shell predicate) and prune it under a lock.
- Keep the existing title-prefix scan as a secondary pass (cheap, helps if
  Layer 2 lands). Call the reaper from `spawn_shell` (already via `CLEANUP_ONCE`)
  and export it through `cleanup_stale_terminal_harness_resources()`.

### Phase 3 — Title survival (optional, only if env-safe)
- Prototype Option A; run the full apple-terminal L2 subset to prove no
  capability/rendering regression. If anything drifts, drop Layer 2 and rely on
  Layer 1.

### Phase 4 — Opt-in sweep + restoration docs
- Implement `looks_like_harness_window` and the
  `BISCUIT_TEST_HARNESS_SWEEP_LEGACY_APPLE` gate.
- Document the restoration `defaults` and the sweep env in the harness README +
  skill topic page.

### Phase 5 — Verify
- Broker-window harness, `-j 1`: run the apple-terminal subset, `kill -9` a run
  mid-flight, then run again and assert the leaked window is reaped on the next
  spawn and the registry is pruned. Assert windows-before == windows-after across
  N repeats (zero net leak) and the shared window is never touched.
- Confirm focus-free (grep the spawn AppleScript: no `activate` / `keystroke`).

### Phase 6 — Docs
- Flip Pitfall 2 to "resolved (registry + opt-in sweep)" in
  `apple-terminal-harness-pitfalls.md`; update the `biscuit-test-harness` README
  "Defensive cleanup" section; regenerate the `rust-testing` SKILL.md `hash:`
  if its text changes (`md hash --save`).

## Acceptance Criteria

1. After a `kill -9` mid-run leak, the **next** `spawn_shell` reaps the orphan
   (window closed) without any title match.
2. A full apple-terminal L2 subset run is **net-zero** windows
   (`count windows` before == after) across ≥3 repeats.
3. The reaper **never** closes a window not created by the harness with the sweep
   gate off (verified by leaving a hand-made idle Terminal window open across a
   run and asserting it survives).
4. Spawning remains focus-free; both Pitfall-1 invariants still hold.
5. Registry is pruned (no unbounded growth) and concurrent spawns/reapers do not
   corrupt it.

## Risks & Mitigations

- **Window-id reuse** closing an unrelated window → gate every registry close
  behind the idle-shell predicate (AC-3).
- **No-rc shell changes env** (Layer 2) → characterization tests; Layer 2 is
  optional and Layer 1 stands alone.
- **Opt-in sweep closes a developer's idle window** → strictly env-gated, never
  auto-triggered, documented; predicate also requires default geometry + empty
  title to narrow the blast radius.
- **Registry file contention** across parallel `cargo` invocations → append-only
  writes + lock-guarded rewrite + best-effort skip on contention.

## Effort

- Layer 1 (registry + reaper): ~120–160 LoC + unit tests. **Core deliverable.**
- Layer 2 (title survival): ~10–20 LoC, gated on a characterization pass. Optional.
- Layer 3 (opt-in sweep) + restoration docs: ~60–80 LoC + docs.

## References

- `biscuit-test-harness/src/apple_terminal.rs` — `cleanup_stale_apple_terminal_windows`
  (308), `unique_window_tag` (681), `WINDOW_TITLE_PREFIX` (63), `spawn_shell`,
  `close_window`, `CLEANUP_ONCE`.
- `biscuit-test-harness/src/lib.rs` — `pid_from_tag`, `process_is_alive`,
  `current_process_id`, `cleanup_stale_terminal_harness_resources` (461).
- `biscuit-test-harness/src/wezterm.rs` — `BACKGROUND_WORKSPACE` (33) +
  `SWEEP_LEGACY` gate (678): the precedent this plan adapts (workspace isolation
  is the capability Apple Terminal lacks, hence the registry).
- `.claude/skills/rust-testing/apple-terminal-harness-pitfalls.md` — Pitfall 2.
