# When not to use kache

kache is very good at a specific shape of problem. Recommending it outside that shape wastes disk
and adds moving parts.

## 1. When you depend on incremental compilation

**The single most important tradeoff.** While kache wraps rustc, cargo's incremental compilation is
turned off (`CARGO_INCREMENTAL=0`) — the two solve the same problem, and running both can corrupt
artifacts on filesystems like APFS.

This makes kache a **dependency and cross-machine optimizer, not an inner-loop optimizer**:

- Editing one crate repeatedly and rebuilding — incremental is designed for exactly this, and kache
  removes it. Cache restores help with the *dependencies* you aren't touching, not the crate you
  are.
- Large workspace, small edits, tight edit-compile-test loop → measure before adopting. It may be
  slower for that specific motion even while being faster for clean builds.
- Clean builds, CI, worktree switching, machine switching → kache wins clearly.

If your day is dominated by iterating on one crate in one checkout, kache may be the wrong tool.

## 2. Single worktree on a non-reflink filesystem

On ext4/NTFS, populating the store costs a real second copy, and with one worktree there's nothing
to dedup against. You're paying storage for a cache whose flagship benefit — one blob, many links —
has no consumers.

Still worth it if you get value from: cheap re-cleaning (a warm store makes `clean` non-destructive
in cost terms), CI/multi-machine sharing via S3, or you're about to start using worktrees.

Not worth it if the machine is disk-constrained and does one thing.

## 3. Link-dominated builds

kache **does not cache** binary crates, dynamic libraries, proc-macros (by default), link and
whole-program steps, multi-source or multi-arch invocations, response-file (`@file`) invocations,
precompiled headers, modules, coverage, or split-DWARF.

A workspace whose build time is mostly linking many binaries and test executables — a big `nextest`
suite, for example — sees a much smaller share of its work cached. `KACHE_CACHE_EXECUTABLES=1`
extends coverage but those outputs are linker- and platform-sensitive (macOS code signing), so
verify results before relying on it.

## 4. C/C++ projects needing remote sharing

C/C++ object caching is **local-only**. Only Rust artifacts sync to S3. A C/C++-heavy CI fleet gets
no cross-machine benefit today.

## 5. Disk-constrained hosts where the store competes with `target/`

The store is bounded (good), but it's still a second consumer of the same volume. On a filesystem
sized close to the build output, adding a store can turn "occasionally full" into "regularly full".
Size `local_max_size` against the volume, or don't adopt it there.

## 6. Where the real problem is one unbounded `target/`

kache reduces what accumulates (incremental is off; deps come back as links) and makes cleanup
cheap. It does **not** shrink an individual artifact or stop cargo from letting one build tree grow.
If a single `target/debug` is eating a disk, the levers are:

- `[profile.dev] debug = "line-tables-only"` (or scoped to `[profile.dev.package."*"]`) — less
  debug info produced in the first place
- `cargo sweep` / `kache clean` on a schedule — bounded growth
- Capping or isolating the filesystem the build tree lives on

kache complements all three. It substitutes for none of them.

## Comparison to the alternatives

**vs sccache** — same `RUSTC_WRAPPER` idea; kache adds content-addressed dedup with reflink/hardlink
restores, so identical artifacts occupy space once across worktrees. sccache avoids recompiles but
leaves duplicate artifacts in each `target/`. Both disable/bypass incremental.

**vs cargo-sweep** — different layers, and *not* fully redundant. `cargo sweep` prunes stale
artifacts inside `target/`; `kache gc` bounds the store. Adopting kache moves *retention policy* to
the store, but you still want target hygiene, because kache's per-crate keying slows dramatically on
huge build trees (~18 s/crate on a 957k-file `target/deps` vs ~30–170 ms clean). Keep a sweep or
clean job — for speed, not disk.

**vs `cargo clean`** — normally a last resort, since it forces full rebuilds. With a warm kache
store the rebuild is mostly link-restores, so blunt cleaning becomes reasonable. `kache clean` is
the same idea with better discovery across worktrees.

**vs a shared `CARGO_TARGET_DIR`** — a single shared target dir also avoids duplication, but
serializes builds behind cargo's lock and mixes artifacts across branches. kache keeps independent
target dirs that *share storage*, which is what you actually wanted.

## Quick disqualifiers

Don't adopt if **all** of these hold:

- One machine, one worktree, no CI
- Non-reflink filesystem
- Inner loop dominated by editing one crate
- Disk already tight

Do adopt if **any** of these hold:

- Multiple worktrees (especially agent-driven, one per task)
- CI runners, or several machines on the same target triple
- Reflink filesystem plus a heavy dependency graph
- You want cleaning `target/` to stop being expensive
