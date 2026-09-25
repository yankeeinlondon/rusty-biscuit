# Parallel-walker parity probe (Phase 1 spike)

- **Date:** 2026-09-25
- **Host:** macOS 27.2 (26B5091g), Apple M4 Max, 16 logical CPUs (12P + 4E), `aarch64-apple-darwin`
- **Base commit:** `1634e6e55b7bc1d42989e058e87eb7ac59d0e2f6` (plus the uncommitted Phase 1 working tree)
- **`ignore`:** 0.4.25 (default worker policy: `available_parallelism().min(12)`, i.e. 12 here)
- **Status:** the scaffolding is discarded. It was a temporary `#[ignore]`d test module
  (`spike_probe`) appended to `sniff/lib/src/filesystem/repo/nested.rs`, run once, then removed.

## Method

Both sides used the exact production builder settings: `hidden(false)`,
`git_ignore(true)`, `git_global(true)`, `git_exclude(true)`, the
`should_skip_directory_name` `filter_entry` prune, and all other defaults.
Both sides kept the production filter, `!file_type().is_some_and(is_dir)`,
and dropped walk errors.

- **Serial:** `WalkBuilder::build()`, i.e. the current `walk_for_nested_markers`.
- **Parallel:** `WalkBuilder::build_parallel().run(..)`. Each callback also
  recorded `std::thread::current().id()` so we could count how many threads
  actually ran callbacks.

The two sides were compared as `BTreeSet<PathBuf>` of every admitted
non-directory entry. The comparison was not restricted to markers.

Command:

```sh
SPIKE_CORPUS=/Volumes/coding/wt/rusty-biscuit/fix-sniff \
  cargo test -p sniff --lib spike_probe -- --ignored --nocapture
```

## Results

| Case | Serial entries | Parallel entries | Markers | Errors (s/p) | Callback threads | Equal |
|---|---:|---:|---:|---|---:|---|
| Small fixture (plain root) | 6 | 6 | 5 | 0/0 | 1 | yes |
| Symlinked starting root | 7 | 6 | 5 | 0/0 | 1 | **no: root entry** |
| This checkout, run 0 | 12,889 | 12,889 | 116 | 0/0 | 12 | yes |
| This checkout, run 1 | 12,889 | 12,889 | 116 | 0/0 | 12 | yes |
| This checkout, run 2 | 12,889 | 12,889 | 116 | 0/0 | 12 | yes |

The small fixture contained:
- markers at depths 1 and 2
- a hidden directory holding a marker (kept)
- markers under `node_modules/` and `target/` (pruned: the prune held on both sides)
- a marker-named directory `d/package.json/` (a directory, so not evidence)
- a `.sln` file
- a root marker

The checkout root was spelled `/Volumes/coding/wt/rusty-biscuit/fix-sniff`, a
linked worktree whose `.git` is a file. Git ignore rules applied.

## Findings

1. **Parity holds for descendants.** On a plain root and on this checkout
   (three repeats), the entry sets are identical. The `filter_entry` prune,
   hidden-directory admission, and marker-named-directory exclusion behave
   the same on both sides.
2. **Root-entry divergence with a symlinked starting root (Phase 2 must
   handle this deliberately).** When `root` is a symlink to a directory:
   - the serial walker yields the depth-0 root entry with the *link's* file
     type (not a directory), so the production non-directory filter admits
     the root path itself;
   - the parallel walker yields the root as a directory, so the same filter
     drops it.

   All descendant paths keep the link spelling on both sides; neither walker
   canonicalizes.

   Consequence for candidates: `candidates_from_marker_paths` keys on the
   entry's *parent*. For the root entry, the parent is the directory that
   *contains* the link, which is outside the walked tree.
   - Normally the root's basename is not a marker name, so the extra entry
     has no effect.
   - If the symlinked root is itself **named like a marker** (probe: a link
     named `package.json` pointing at the fixture), today's production walk
     registers `<parent of link>` as a nested candidate with
     `[BunWorkspaces, NpmWorkspaces, YarnWorkspaces]`. That is a directory
     *outside* the repository. The parallel walk does not register it.

   Observed with the unmodified production `walk_for_nested_markers`:

   ```text
   production_walk = [<tmp>, <tmp>/package.json/a, <tmp>/package.json/a/b,
                      <tmp>/package.json/c/.hidden, <tmp>/package.json/e]
   parallel        = [<tmp>/package.json/a, <tmp>/package.json/a/b,
                      <tmp>/package.json/c/.hidden, <tmp>/package.json/e]
   ```

   **Recommended handling (default for Phase 2; the author may override in
   review):** treat the depth-0 root entry as never being marker evidence.
   This matches the module's "nested discovery is non-root only" contract,
   and the supplied-evidence path already excludes it: the shared
   observation walk only records descendants. Concretely:
   - the parallel callback skips `entry.depth() == 0`. This is redundant
     with the directory filter on the parallel side, but it is explicit and
     holds on every platform;
   - the Phase 3 test-only serial reference applies the same depth-0 skip,
     and documents it as the one deliberate difference from the pre-change
     loop;
   - a Phase 3 test pins that a marker-named symlinked root registers no
     candidate outside the root.

   The alternative, preserving the quirk exactly, would mean deliberately
   re-admitting a symlink root entry in the parallel walker to reproduce a
   candidate outside the repository. We judged that not worth preserving.
3. **Worker count.** The default policy starts 12 workers here, not 16: the
   cap is 12. On the corpus, all 12 threads ran callbacks. On the tiny
   fixtures only one thread ever ran a callback, so a small tree pays thread
   start-up cost without any parallel speedup. Phase 4's tiny-tree probe
   should quantify that cost.
4. **Walk errors.** There were none on any case. Error handling is exercised
   in Phase 3 (missing root, and permission denial where constructible).

## Corpus facts versus historical figures

The historical figures were 11,290 paths and 92 markers. This checkout now
yields 12,889 non-directory entries and 116 markers, so the corpus has grown
since the original investigation. Timing comparisons must use the figures
recorded in `baseline/`, not the historical ones.

## Assertions promoted to Phase 3

- Serial reference and parallel walk agree on the full ordered candidate
  list for:
  - plain roots;
  - hidden, pruned (`node_modules`, `target`), and marker-named-directory
    fixtures;
  - a symlinked starting root (Unix; native Windows only where symlink
    creation needs no elevation).
- Collected candidate roots keep the caller's root spelling; there is no
  canonicalization.
- A marker-named symlinked root registers no candidate outside the root
  (subject to the author's ruling on finding 2).
- Worker propagation is checked with a multi-worker configuration on a wide
  fixture, because the default policy may run a small fixture on a single
  thread.
