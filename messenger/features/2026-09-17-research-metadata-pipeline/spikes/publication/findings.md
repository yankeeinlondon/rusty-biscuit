---
title: Publication Spike - Multi-Artifact Snapshot Publication
date: 2026-09-17
status: completed
---

# Publication Spike

This spike answers one question: how does `messenger` publish the accepted research snapshot so that readers see either the complete previous set or the complete next set? The snapshot is the five platform documents, `catalog.json`, the summary, the CHANGELOG, review artifacts, and the repo-root `.claude/skills/messenger/platform-metadata.md`. All of these live at fixed, committed paths. The mechanism must work on macOS, Linux, native Windows, and WSL2 without symlinks.

It supports spec verification criteria 7 (an interrupted generation keeps the prior catalog), 30 (partial refresh), and 34 (an interrupted publication still leaves a usable snapshot).

**Decision: strategy A, a committed manifest plus a local journal.** Replacing the committed manifest file is the single selection point. Every generation and reporting read verifies each fixed artifact against that manifest. A local journal with staged copies and backups drives rollback before the selection point and roll-forward after it.

The prototype is in [`prototype/`](prototype/). It is a standalone crate with an empty `[workspace]` table. The root `Cargo.toml` lists its members explicitly and has no glob that would pick this crate up.

## Strategies compared

| | A: manifest + journal (chosen) | B: generation dirs + pointer | C: git as the transaction |
|---|---|---|---|
| Atomic selection | One `rename` of the committed manifest. The file replacements before it are atomic per file. | One `rename` of the local `CURRENT` pointer. | `git commit`. The spec forbids automatic commits, so this is not available. |
| Input and schema hashes | Recorded in the manifest (schema version and hash, input hashes, per-artifact xxh64 and length, `snapshot_id`). | Same manifest, stored inside each generation. | None of its own. |
| Rollback | Restore from `tx/<txid>/old/`. Artifacts that did not exist before are removed. | Point `CURRENT` back to an older generation. Requires keeping old generations. | `git checkout`. |
| Startup recovery | A deterministic rule decided by the manifest hash; see [Recovery algorithm](#recovery-algorithm). | Re-mirror the selected generation onto the fixed paths. | Manual. |
| What a crash leaves at the fixed paths | Old, new, or a mixture that verification refuses. Recovery always converges to old or new. | Can be a mixture, and it disagrees with the pointer: the pointer says New while the fixed paths still read Old (measured). | Unchanged. |
| Git-friendliness | Adds one small committed file (`publication.json`) that doubles as provenance. All state is gitignored. A fresh clone is self-verifying. | The generations must stay gitignored, because committing them duplicates the whole snapshot on every publication. A fresh clone therefore has no pointer, and B's reader fails (`fresh_clone_has_no_generation_state`). It falls back to A's verifier. | Native, but not permitted to automate. |
| Windows | File renames only. Needs a bounded retry against holders that do not share delete. | The same file renames, plus directory garbage collection. Renaming a directory that contains an open file fails with error 5. | Not applicable. |

B is strictly more machinery than A. The committed fixed paths are what git, humans, and agents read, so the mirror step has the same multi-file problem that A solves, and B still needs A's manifest verification for fresh clones. C is ruled out by the spec. Recovery in all three can fall back on `git checkout -- <artifact set>` to restore the last committed snapshot, because the manifest is committed in the same commit as the artifacts.

## Chosen protocol

### On-disk layout

Committed, relative to the repository root:

    messenger/docs/research/publication.json      # manifest = selection point
    messenger/docs/research/platforms/{discord,slack,telegram,whatsapp,signal}.md
    messenger/docs/research/platforms/catalog.json
    messenger/docs/research/summary/platforms.md
    messenger/docs/research/CHANGELOG.md
    messenger/docs/research/reviews/*.json        # review artifacts (path TBD by planning)
    .claude/skills/messenger/platform-metadata.md

Local state is gitignored and per worktree. The proposal is `messenger/.research-state/`, which can also hold the spec's other local working records such as candidates and failed runs. The precedent is root `.gitignore`'s `.claudine/tmp/`; there is no existing messenger state directory. Proposed `.gitignore` additions: `messenger/.research-state/` and `*.publish-tmp`.

    messenger/.research-state/publication/
      lock                          # File::try_lock (flock / LockFileEx); OS releases on process death
      journal.json                  # exists iff a transaction is pending
      tx/<txid>/new/<repo path>     # staged artifacts + staged manifest
      tx/<txid>/old/<repo path>     # backups of every touched path that existed (incl. manifest)

Each file replacement writes a sibling temp named `.<name>.<txid>.publish-tmp` in the target's own directory. Renames therefore never cross a volume, even for the repo-root `.claude/` path. Recovery removes any leftover temps.

### Manifest (`publication.json`)

The manifest is deterministic: keys are sorted, it is pretty-printed with LF line endings and a trailing newline, and it contains no timestamps or host paths. Republishing identical content produces byte-identical output and touches no file (tested).

    { "format": "messenger-research-publication/1",
      "schema": { "version": "1", "xxh64": "<schema file hash>" },
      "inputs": [ { "path": "messenger/docs/platforms.yaml", "xxh64": "..." } ],
      "artifacts": [ { "path": "<repo-relative, />", "xxh64": "<16 hex>", "len": 1234 } ],
      "snapshot_id": "<xxh64 of the canonical body above>" }

- Hashes are xxh64 with seed 0. This is byte-identical to `biscuit_hash::xx_hash_bytes`.
- Production should record the Darkmatter frontmatter and body hashes for Markdown inputs, as the spec requires, in `inputs` or in `catalog.json`. The artifact integrity hash stays a raw-bytes xxh64.
- The `.gitattributes` rule `* text=auto eol=lf` keeps the committed bytes identical on Windows checkouts. The generator must emit LF only.

### Journal (`journal.json`)

    { "format": "messenger-research-publication-journal/1", "txid": "...", "state": "prepared",
      "old_manifest_xxh64": "<hash|null>", "new_manifest_xxh64": "<hash>",
      "entries": [ { "path": "...", "old_xxh64": "<hash|null>", "new_xxh64": "<hash|null>" } ] }

There is only one durable journal state, `prepared`. Whether a transaction committed is decided by the committed manifest, not by the journal, so no second journal write can be lost or reordered. The journal's absence means no transaction is pending.

### Publish sequence

1. Acquire the lock. If a journal exists, refuse with `RecoveryRequired`.
2. Stage the new artifacts and the new manifest under `tx/<txid>/new/`, and back up every touched existing path under `tx/<txid>/old/`. Every file is written with `sync_all`, and on Unix the directory is fsynced.
3. Atomically write `journal.json`.
4. Replace each fixed artifact in sorted order through the sibling temp and a rename. Unchanged artifacts, such as a failed platform's carried-over document, are skipped. Artifacts absent from the new snapshot are removed.
5. **Selection:** atomically replace `publication.json` with the staged manifest.
6. Remove `journal.json`, then `tx/`, then any leftover temps.

### Reader contract

Everything that feeds generation or reporting goes through `read_verified`:

1. Read the manifest.
2. Read every listed artifact into memory.
3. Accept only if every length and xxh64 matches, and the schema version is supported.

A verified set equals exactly one published snapshot, even when a publication runs concurrently. A reader that races a publication gets a refusal and may retry. A pending journal alone does not cause a refusal. A manual edit or a partial `git checkout` is refused (`Inconsistent`), which also enforces the spec's rule that generated artifacts are not edited by hand.

### Recovery algorithm

Writers such as `generate` and `publish` run recovery at startup under the lock:

    if no journal:            delete tx/ and *.publish-tmp; done (Clean)
    parse journal             (unparseable -> Corrupt; it was written atomically)
    m := xxh64(publication.json) or null
    if m == new_manifest:     roll forward: for each entry, if fixed hash != new, replace from
                              tx/new (verify staged hash first); then ensure manifest == new
    elif m == old_manifest:   roll back: for each entry, if old == null remove path, else if
                              fixed hash != old, replace from tx/old (verify backup hash)
    else:                     Corrupt -> refuse; fix by `git checkout` of the artifact set
    remove journal, then tx/ and temps

Every step is guarded by a hash, so recovery is idempotent. A crash during rollback or during roll-forward is handled by running recovery again (both cases are tested).

Roll-forward also repairs the power-loss case where the manifest rename persisted but an earlier file rename was lost. The test simulates this by rewriting one artifact after the selection point.

## Interruption results

Every injection point was run two ways. In-process runs return an error, so destructors still run. Real child-process runs call `std::process::abort()`, so there are no destructors and the OS releases the lock. Results were identical in both modes on macOS, Linux, and native Windows. The child exit status was `SIGABRT` on Unix and `0xc0000409` on Windows.

The first 10 entries of the new snapshot are the changed artifacts plus one new review artifact. Entry 1 is the repo-root skill file.

| Injection point | Verified read before recovery | Recovery | Verified read after |
|---|---|---|---|
| Before staging | Old | Clean | Old |
| During staging (1 of 10 staged) | Old | Clean (orphan `tx/` removed) | Old |
| Staged, no journal | Old | Clean | Old |
| Before selection (journal durable, no fixed path touched) | Old | RolledBack | Old |
| Mid-replacement after k = 1 to 10 artifacts | **Refused** | RolledBack | Old; the new review artifact is removed |
| All artifacts replaced, manifest not | **Refused** | RolledBack | Old |
| After selection (manifest replaced) | New | RolledForward | New |
| Journal removed, `tx/` not cleaned | New | Clean | New |

In every case, recovery left no journal, `tx/`, or `*.publish-tmp` behind, and a follow-up publication succeeded.

Additional results:

- **Interrupted initial publication:** reads returned `NoSnapshot` before and after rollback, and no artifacts were left behind.
- **Second publisher:** gets `Locked`.
- **Publishing during a pending journal:** refused with `RecoveryRequired`.
- **Strategy B:** pointer reads were always Old or New. After the "after selection" crash, however, the pointer read New while the fixed paths still verified as Old. After a crash mid-mirror, the fixed paths were refused.

## Per-OS evidence

Hosts declared on this machine were `BUILD_LINUX=build-linux`, `BUILD_WIN=build-win-native`, and `BUILD_WSL=build-win`. All results below are behavioral runs of the full suite: 16 tests on the first runs, and 17 after the rename-primitive test was added.

| OS | Command | Result |
|---|---|---|
| macOS (Darwin 27.0.0, APFS, rustc 1.97.1) | `CARGO_TARGET_DIR=$TMPDIR/publication-spike-target cargo test -- --nocapture` in `prototype/`; also `cargo clippy --all-targets` (clean) and `cargo check --tests --target x86_64-pc-windows-gnu` (clean) | All pass |
| Linux (build-linux, kernel 7.0.14-pve, ZFS, rustc 1.97.1) | The crate was piped over `ssh -o BatchMode=yes` with tar into `~/scratch/publication-spike-20260917`, then `bash -lc 'cargo test -- --nocapture'` ran there; the directory was removed afterward | All pass |
| Native Windows (build-win-native, Windows NT 10.0.26200, NTFS, rustc 1.97.1 msvc) | `scp` to `C:\Users\ken\scratch\publication-spike-20260917`, then `cmd /c "cargo test -- --nocapture"`, then `--test open_handles -- --test-threads=1`; the directory was removed afterward | All pass |
| WSL2 | `ssh -o BatchMode=yes -o ConnectTimeout=8 build-win` failed twice (`kex_exchange_identification: Connection reset by peer`, then `Connection closed`). The distro listed as Running, but `wsl.exe -e sh -c "df -h ~"` produced no output within 60 s. | **Gap: not run** |

Notes on these runs:

- **Linux and WSL2:** WSL2 follows Linux code paths, and the Linux run passed. Under the os skill's standing rules, however, it is not WSL2 evidence.
- **Windows storage:** W: reported 0 GB free, and 8 KB free after the run. The os skill documents the WSL resets as the same storage condition, because the guest's VHDX lives on W:. The spike therefore built on C:, which had about 27 GB free, in a scratch directory with the default in-crate target. `CARGO_TARGET_DIR` was not overridden and no shared target was touched.
- **WSL2 follow-up:** WSL2 remains owed once W: is freed.

## Windows replacement and open-handle behavior

Measured on build-win-native. The Linux and macOS rows are included for contrast.

| Case | Windows | macOS / Linux |
|---|---|---|
| `std::fs::rename` over a file held by a **std default handle** (`File::open`: share read, write, and delete) | **Ok**. The held handle keeps reading the old bytes. | Ok, same behavior |
| `tempfile::NamedTempFile::persist` over the same holder | **Err: error 5, ERROR_ACCESS_DENIED** | Ok |
| Either primitive over a holder **without delete sharing** (share mode `0x3`, as used by CRT `_wopen`, editors, and scanners) | **Err: error 5** (ERROR_SHARING_VIOLATION, error 32, was not observed) | Ok |
| Publication while a no-delete-share holder stays open (retry of 5 attempts every 20 ms) | Publish fails partway; verified read is Refused; after the handle closes, recovery rolls back to Old and republishing succeeds | Publish succeeds |
| Holder released 150 ms into the default 40 × 25 ms retry | Publish succeeds after 2 transient retries | No retries |
| 60 alternating publications, with 2 verifying readers and one no-delete-share reader in a tight loop | 60 of 60 succeeded, 30 transient retries, 0 publish failures, and no reader accepted a mixture. The foreign reader's own open failed 168 times out of about 115,000. | 60 of 60, 0 retries |
| Directory rename while a file inside it is open | **Err: error 5** | Ok |
| `remove_dir_all` while a file inside is open | Ok | Ok |

Interpretation:

- **Correction to the brief's premise:** Rust std opens files on Windows with `FILE_SHARE_DELETE` by default. A reader using `std::fs::File::open` therefore does not block replacement.
- **Why the two rename primitives differ:** the difference between `std::fs::rename` and `tempfile::persist` is consistent with std's rename using POSIX-semantics rename (`FileRenameInfoEx`), which unlinks the old file while handles remain. `tempfile` uses legacy `MoveFileExW(MOVEFILE_REPLACE_EXISTING)`. The API internals were not traced. The measured behavior is what matters.
- **Legacy fallback risk:** POSIX-semantics rename needs NTFS on Windows 10 1607 or later. On filesystems without support, for example some network shares or FAT, expect std to fall back to legacy behavior. That fallback was not measured.
- **Retry scope:** the protocol retries only errors 5 and 32, for a bounded period (default 40 × 25 ms, matching `playa::detached::replace_path`). It never deletes the destination first.
- **After retries are exhausted:** the protocol aborts the publication, which leaves the journal pending. Verified reads refuse the mixture, and the next writer start-up rolls back.
- **Error 5 is ambiguous:** it can also mean a real permission problem. Exhaustion therefore reports the path and the OS error rather than looping.
- **Transient scanners:** Defender, the search indexer, and cloud-sync clients open files briefly without delete sharing. The os skill already records hosted-runner scanners causing this with Playa's journal. The bounded retry absorbs them, as the concurrency run showed.
- **Durability:** std cannot open a directory handle on Windows, so there is no directory fsync. The spike syncs file data (`FlushFileBuffers` through `sync_all`) before each rename. NTFS journals the rename metadata, but the rename is not guaranteed durable across power loss until the log is flushed. Playa passes `MOVEFILE_WRITE_THROUGH` to `MoveFileExW`.
- **Why a lost rename is safe:** a lost rename can only make the fixed paths older than the manifest, or leave the manifest old. Verification detects both, and recovery repairs them while the journal and staging exist. Journal removal happens after the manifest rename. If power loss reorders the two, the reader refuses rather than accepting a mixture, and `git checkout` restores the committed snapshot.
- **Directory renames are fragile on Windows**, which is one more reason not to select generations by renaming directories (a variant of B).
- **Readers:** production readers should read each file fully and close it immediately, as `std::fs::read` does. They must not hold artifact handles across a publication.

## Existing helpers

| Helper | Fit | Why |
|---|---|---|
| `claudine/lib/src/config/atomic.rs` `atomic_write` | Pattern only | It is well built: unique sibling temp, `sync_all`, Unix directory fsync, and a bounded Windows retry on errors 2, 5, and 32. However, it goes through `tempfile::persist`, which fails even against std-default holders (measured). Messenger also must not depend on Claudine. |
| `darkmatter/lib/src/effects/fs_write.rs` `atomic_write_guarded` | No | `pub(crate)`, no `sync_all`, no Windows retry. |
| `darkmatter/.../compose/cache/store.rs` `atomic_write` | No | Private cache internals, no retry. |
| `worktree/lib/src/cache.rs` `atomic_write` | Closest primitive | It uses `std::fs::rename`, the better Windows behavior. It has no `sync_all`, no retry, and its temp names are not recoverable. |
| `playa/lib/src/detached/mod.rs` `replace_path` | Retry policy only | `MoveFileExW(REPLACE_EXISTING, WRITE_THROUGH)` with a 40 × 25 ms retry, but private and legacy-semantics. |
| `biscuit-file` | Not today | It has no atomic-write API. The spec already makes it a messenger dependency for `FileReference` and format conversion. |
| `biscuit-hash` `xx_hash_bytes` | **Use** | xxh64 with seed 0, identical to the spike's hashes. |

**Recommendation for production.** Add one shared file-replacement function, `replace_file(dest, bytes, tag, retry)`. It should:

- write to a unique sibling temp, `sync_all`, then `std::fs::rename`;
- retry errors 5 and 32 on Windows for a bounded period;
- fsync the parent directory on Unix;
- take a temp-name tag, so recovery can find leftovers.

Home it in `biscuit-file`, which messenger already depends on under the spec. Four near-duplicate helpers already exist, so this is a later consolidation target, but migrating them is out of scope. If adding to `biscuit-file` is out of scope for this feature, keep the function private to messenger's maintenance module.

Put the journal protocol itself (`publish`, `recover`, `read_verified`) in the `messenger` library behind the opt-in maintenance feature. Have `messenger-cli`'s `research generate`, `--check`, and `report` call `recover` (for writers) and `read_verified` (for everything) first. `generate --check` should compare against `read_verified` output plus a fresh projection. Its drift report should name the first mismatching path.

## Risks and open items

- **WSL2 evidence is missing.** Re-run the suite on the guest once W: has space.
- **Windows storage:** the Windows build ran on C:, not the documented W: target volume, because W: was full.
- **Readers outside the protocol can see a mixture.** Humans, editors, git, and agents reading the published skill file will see a mixture during the window between the first replacement and the manifest replacement, or after a crash until recovery. This is inherent to fixed multi-file paths without symlinks. The requirement concerns eligibility for generation and reporting, which `read_verified` enforces.
- **The spike simulated power loss logically, not physically.** The os skill should record the POSIX-rename-versus-`MoveFileExW` difference; it is a new, measured Windows fact.
- **A long-held no-delete-share reader,** such as an editor keeping `catalog.json` open, makes publication fail cleanly after about 1 s and requires recovery once the file is closed. The CLI error should say which path is held. Production might pre-check each destination with a trial open before staging, but that only narrows the window.
- **Lock semantics:** `File::try_lock` requires Rust 1.89 or later; the workspace uses 1.97. It is an advisory, process-scoped lock and does not protect against a second worktree, because each worktree has its own state and its own committed files. That is intended.
- **Commit timing:** the committed `publication.json` must be committed together with the artifacts. If they are committed separately, a checkout between the two commits is refused until the other commit is present. That refusal is correct behavior, but worth documenting in the maintenance README.
