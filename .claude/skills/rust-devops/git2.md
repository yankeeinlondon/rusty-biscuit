---
prompt: |-
    The `git2` crate in Rust is the old standby for interacting with git in Rust
    programs. It's technical approach is to bind to the C-based `libgit2` library
    to achieve it's functional aims. It is battle tested and mature.

    Your task is to do a deep dive into the `git2` crate. Your research should be
    able to answer the following questions and cover the various topics:

    - Key URLS (docs, repo, etc.)
    - Functional overview
    - Architectural overview
    - Version history with dates and key changes for each release
    - Use Cases: for each use case give 2-3 variant examples of different variants of how this crate might achieve this operation. What gotchas are there, if any, are there for this operation? How expensive is this operation from a CPU and timing standpoint?
        - git status
        - git log
        - git branch
        - git tag --list
        - git remote 
        - git grep
        - git blame
        - add others too
last_updated: 2026-06-06
---
# `git2` — Deep Dive

`git2` (repo: `git2-rs`) is the long-standing Rust binding to [libgit2](https://libgit2.org/), a portable, pure-C reimplementation of Git's core methods. It is the most mature library for programmatically reading and writing Git repositories from Rust, maintained under the `rust-lang` organization by Alex Crichton, Josh Triplett, and Eric Huss.

---

## Key URLs

| Resource                      | URL                                                         |
|-------------------------------|-------------------------------------------------------------|
| crates.io                     | https://crates.io/crates/git2                               |
| API docs (docs.rs)            | https://docs.rs/git2                                        |
| Source repository             | https://github.com/rust-lang/git2-rs                        |
| CHANGELOG                     | https://github.com/rust-lang/git2-rs/blob/main/CHANGELOG.md |
| Upstream libgit2              | https://libgit2.org/                                        |
| libgit2 reference             | https://libgit2.org/docs/reference                          |
| libgit2 source                | https://github.com/libgit2/libgit2                          |
| Companion `libgit2-sys` crate | https://crates.io/crates/libgit2-sys                        |
| `git2-curl` transport         | https://github.com/rust-lang/git2-rs/tree/main/git2-curl    |

**Owners:** `alexcrichton`, `joshtriplett`, `ehuss`, `rust-lang-owner`. License: MIT OR Apache-2.0.

---

## Functional Overview

`git2` exposes nearly the entire libgit2 surface as safe, idiomatic Rust:

- **Repository lifecycle** — `open`, `open_ext`, `discover`, `init`, `init_bare`, `clone`, `clone_recurse`, worktrees, bare-repo handling.
- **Object model** — `Commit`, `Tree`, `Blob`, `Tag`, `Object` (polymorphic), with typed lookups (`find_commit`, `find_tree`, …) and prefix lookups (`find_commit_by_prefix`, …).
- **References & branches** — `Reference`, `Branch`, `Branches` iterator, symbolic refs, HEAD management, reflogs.
- **History traversal** — `Revwalk` (commit-graph walker), `revparse` / `revparse_single` / `revparse_ext` (revision-spec parsing), `merge_base*`, `graph_ahead_behind`, `graph_descendant_of`.
- **Diffing** — tree↔tree, tree↔index, index↔workdir diffs with `Diff`, `DiffDelta`, `DiffHunk`, `DiffLine`, rename/copy detection, patch formatting, and `Patch` extraction.
- **Status** — `statuses()` / `status_file()` mirroring `git status`.
- **Blame** — `blame_file()` producing line-level authorship with copy/move tracking.
- **Index & working tree** — `Index` manipulation, `checkout_*`, `reset`, `apply` (patch application).
- **Merging & rebasing** — `merge_analysis`, `merge`, `merge_trees`, `merge_commits`, full `Rebase` state machine, cherry-pick, revert, conflict resolution.
- **Remotes & networking** — `Remote`, fetch/push, refspecs, credential callbacks, certificate checks, transport customization. *(Networking requires the `ssh`/`https`/`cred` features.)*
- **Configuration** — `Config` multi-level key/value store, `ConfigEntries` iteration.
- **Submodules, notes, stash, tags, packfiles, ODB** — low-level object database (`Odb`), `PackBuilder`, `Indexer`, mailmap, message trailers, describe, email/mbox formatting.
- **Globals** — `opts::` module for caching, TLS, timeouts, extensions, and SSL cert paths.

**Feature flags (0.21+):** By default `git2` builds with **only local-repo support**. `ssh`, `https`, and `cred` are **no longer default** as of 0.21 — enable them explicitly when you need cloning/pushing over the network:

```toml
[dependencies]
git2 = { version = "0.21", features = ["ssh", "https"] }
```

---

## Architectural Overview

`git2` is a **thin safe-wrapper FFI crate**. Its design has three layers:

```text
┌─────────────────────────────────────────────┐
│  your Rust app                               │
├─────────────────────────────────────────────┤
│  git2 (safe Rust API)                        │  ← strong types, RAII, Result<T, Error>
├─────────────────────────────────────────────┤
│  libgit2-sys (raw `-sys` bindings)           │  ← `#[link]`, `extern "C"`, pointer types
├─────────────────────────────────────────────┤
│  libgit2 (C library, vendored or system)     │  ← the actual git implementation
└─────────────────────────────────────────────┘
```

### How the layers fit together

1. **`libgit2-sys`** (`libgit2_sys`) is the `-sys` companion crate. It exposes the C `git_*` functions and structs via `extern "C"`. It vendors the libgit2 source (via a git submodule) and builds it with `cc`/`cmake`, **unless** `LIBGIT2_NO_VENDOR=1` is set and a suitable system `libgit2` is found. The current release tracks **libgit2 1.9.x**.
2. **`git2`** wraps every `git_*` handle in a Rust newtype that owns the resource and frees it on `Drop`. The `Binding` trait (`from_raw` / `raw`) is the bridge between safe Rust structs and raw `*mut` pointers.

### Key design properties

- **RAII / automatic resource management.** Every `Repository`, `Commit`, `Diff`, etc. owns a heap-allocated libgit2 object. When it goes out of scope the C struct is freed. `Repository::open`/`init`/`clone` are the entry points; nearly all other objects borrow from a `Repository`'s lifetime (`'repo`).
- **`Result<T, Error>` everywhere.** A single `Error` type carries libgit2's error code (`ErrorCode`), class (`ErrorClass`), and message. There is no panicking on expected failure paths.
- **Thread-safety.** libgit2 is built thread-safe; `Repository` is `Send` but **`!Sync`** — you cannot share a single `&Repository` across threads. The common pattern is one `Repository` per thread, or wrapping access in a `Mutex`.
- **Bitflags via `bitflags` 2.x.** Status flags, open flags, diff options, etc. are strongly-typed `bitflags!` structs (e.g. `Status`, `RepositoryOpenFlags`, `Sort`).
- **Callbacks as closures.** Network operations, progress, credential acquisition, certificate checks, and tree walks accept Rust closures that are translated into libgit2 `git_*_cb` function pointers (stored in `RemoteCallbacks`, `DiffOptions`, etc.).
- **Lifetime discipline.** Borrowed handles (`Reference<'repo>`, `Commit<'repo>`) cannot outlive their `Repository`. The 0.15 release famously removed the `Iterator` impl for `ConfigEntries` because the borrow pattern was unsound; safe `next`/`for_each` methods replaced it.

### Build & linking

- libgit2 is **statically linked (vendored) by default**. The `vendored-libgit2` Cargo feature forces it; `LIBGIT2_NO_VENDOR=1` forces use of a system libgit2. On macOS with the `ssh` feature, you also pull in libssh2 → OpenSSL, requiring the openssl crate's environment setup.
- `libgit2-sys` version numbers encode the bundled libgit2 version, e.g. `0.18.4+1.9.3`.

---

## Version History

The crate has been continuously maintained since November 2014 (0.0.1). Below are the significant releases; patch releases within a minor series are consolidated where they only bump `libgit2-sys`.

### Modern era (libgit2 1.4+ — current)

| Version    | Date       | Key changes                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
|------------|------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **0.21.0** | 2026-05-18 | **Breaking:** `ssh`/`https`/`cred` no longer default features; edition 2021; many string accessors now return `Result` to distinguish missing-vs-non-UTF-8. **Added:** experimental SHA-256 (`unstable-sha256` feature), `ObjectFormat` enum, `merge_file()`, `Refdb` type + `refdb_compress()`, `BlameHunk` committer/summary accessors, `Repository::set_config()`, `author_from_env`/`committer_from_env`, `Clone` for `Reference`. Bumped to libgit2 1.9.3. **Fixed:** panics on non-UTF-8 branch names & missing blame signatures. |
| **0.20.4** | 2026-02-02 | Fixed UB dereferencing empty `Buf`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| **0.20.3** | 2025-12-06 | Bumped to libgit2 1.9.2.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| **0.20.2** | 2025-05-05 | Added `Status::WT_UNREADABLE`; fixed missing `ErrorCode` variants; fixed `Indexer::new` init.                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| **0.20.1** | 2025-03-17 | Added `branch_upstream_merge`, `Index::conflict_get/remove`, `opts::set_cache_object_limit`, `merge_file_from_index`. Fixed empty-URL panic in `Remote::url_bytes`; fixed lifetimes on `Patch` accessors. libgit2-sys 0.18.1 (advapi32 link fix).                                                                                                                                                                                                                                                                                       |
| **0.20.0** | 2025-01-04 | **Breaking:** libgit2 **1.9.0**; removed unused `ssh_key_from_memory` feature; `Tree::walk` errors now propagated; `trace_set` callback takes `&[u8]`; `Error::last_error` returns `Error` not `Option`. Added `merge_base_octopus`, `PackBuilder::write`, restored `Ord`/`Hash` on bitflags.                                                                                                                                                                                                                                           |
| **0.19.0** | 2024-06-13 | **Breaking:** libgit2 **1.8.1**. Added server timeout `opts` + `ErrorCode::Timeout`; shrank `Error` struct.                                                                                                                                                                                                                                                                                                                                                                                                                             |
| **0.18.3** | 2024-03-19 | `opts` mwindow get/set functions.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| **0.18.2** | 2024-02-07 | **Security:** libgit2 **1.7.2** fixing CVE-2024-24575 & CVE-2024-24577. Added `opts::set_ssl_cert_file/dir`, `find_*_by_prefix`, `TreeIter::nth`.                                                                                                                                                                                                                                                                                                                                                                                       |
| **0.18.1** | 2023-09-20 | `FetchOptions::depth` (shallow clone support). Bug fixes.                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| **0.18.0** | 2023-08-28 | **Breaking:** libgit2 **1.7.0/1.7.1**; bitflags 1→2.x; `Revwalk::with_hide_callback` signature change; `FusedIterator` impls. Added `Blame::blame_buffer`.                                                                                                                                                                                                                                                                                                                                                                              |
| **0.17.2** | 2023-05-28 | Stashing with options (`StashSaveOptions`, partial stashing).                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| **0.17.1** | 2023-04-16 | libgit2 1.6.4.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| **0.17.0** | 2023-04-02 | **Breaking:** libgit2 **1.6.3** (min); libssh2-sys 0.2→0.3 (SHA-2 RSA); `certificate_check` callback changed. Added `Indexer`, `push_negotiation` callback, `Index::find_prefix`, `discover_path`, group-writeable blob mode.                                                                                                                                                                                                                                                                                                           |
| **0.16.1** | 2023-01-20 | libgit2-sys 0.14.2+1.5.1.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| **0.16.0** | 2023-01-10 | **Breaking:** `certificate_check` callback gained SSH host-key access; libgit2 1.5.0.                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| **0.15.0** | 2022-07-28 | libgit2 1.5.0. Added `Email`/`EmailCreateOptions`, `tag_annotation_create`, `ErrorCode::Owner`, `opts::set_verify_owner_validation`. **Soundness:** removed `Iterator` for `ConfigEntries` (use-after-free) → `next`/`for_each`.                                                                                                                                                                                                                                                                                                        |
| **0.14.4** | 2022-05-19 | `Commit::body`, `Tree::get_name_bytes`; libgit2 1.4.2.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| **0.14.3** | 2022-04-27 | libgit2 1.4.2; fixed `Remote::create_detached` lifetime.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| **0.14.2** | 2022-03-10 | `Odb::exists_ext`; libgit2 1.4.2.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| **0.14.1** | 2022-02-28 | libgit2 1.4.2.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| **0.14.0** | 2022-02-24 | **Breaking:** libgit2 **1.4.1**. Added `opts::get/set_extensions`, `PackBuilder::name`, redirect options, `StatusOptions::rename_threshold`.                                                                                                                                                                                                                                                                                                                                                                                            |

### Earlier eras

| Version    | Date       | Key changes                                                                                                                                                                      |
|------------|------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **0.13.0** | 2020-03-13 | libgit2 1.0/1.1 era; long patch series (25 patches through 0.13.25, Dec 2021) adding submodule, stash, blame, and credential features incrementally. Targeted Rust 2018 edition. |
| **0.12.0** | 2020-02-26 | libgit2 1.0.0 bindings; large API expansion (worktrees, describe, reflog).                                                                                                       |
| **0.11.0** | 2019-12-12 | libgit2 0.99/1.0-pre; `RepositoryInitOptions`, `Pathspec`, `Submodule` improvements.                                                                                             |
| **0.10.0** | 2019-08-20 | libgit2 0.28; `PackBuilder`, `Odb` writer streams, `RemoteConnection`.                                                                                                           |
| **0.9.0**  | 2019-06-04 | libgit2 0.27; `Rebase` support, `ApplyOptions`, mailmap.                                                                                                                         |
| **0.8.0**  | 2018-12-13 | libgit2 0.26; first edition-2018; `AnnotatedCommit`, merge analysis.                                                                                                             |
| **0.7.0**  | 2018-02-27 | libgit2 0.25; `Transaction`, `Mempack`, custom transports.                                                                                                                       |
| **0.6.0**  | 2016-11-08 | libgit2 0.24; `ReferenceFormat`, `TreeBuilder`, worktrees foundations.                                                                                                           |
| **0.5.0**  | 2016-10-05 | libgit2 0.23; `Revwalk` sorting, `DiffStats`.                                                                                                                                    |
| **0.4.0**  | 2016-02-22 | libgit2 0.22; `StatusOptions`, stash, notes.                                                                                                                                     |
| **0.3.0**  | 2015-07-28 | libgit2 0.21; diff API, blame, config.                                                                                                                                           |
| **0.2.0**  | 2015-02-25 | libgit2 0.20; core commit/tree/blob/reference API.                                                                                                                               |
| **0.1.0**  | 2014-12-16 | Initial release; libgit2 0.19 bindings.                                                                                                                                          |
| **0.0.1**  | 2014-11-14 | First published version.                                                                                                                                                         |

**Notable inflection points:** the 0.13→0.14 jump aligned libgit2 with its 1.x line and reset minimum versions; the 0.18.x series carried critical CVE fixes; 0.21 is the biggest API-hygiene release (non-UTF-8 safety, SHA-256 groundwork, and the de-defaulting of network features that affects every `cargo add git2` consumer).

---

## Use Cases

Each use case below maps a `git` CLI command to its `git2` equivalents, lists variants, gotchas, and performance characteristics. Unless noted, operations work on a `Repository` you have already opened:

```rust
use git2::Repository;
let repo = Repository::discover(".")?;
```

---

### `git status`

Gather working-tree and index changes. The central API is `Repository::statuses(Option<&mut StatusOptions>) -> Statuses`.

#### Variant A — full status (`git status`)

```rust
use git2::{StatusOptions, StatusOptions as O};

let mut opts = StatusOptions::new();
opts.include_untracked(true)
    .renames_head_to_index(true)   // detect staged renames
    .sort(O::SORT_INDEX);          // deterministic order

for entry in repo.statuses(Some(&mut opts))?.iter() {
    let s = entry.status();
    println!("{:?}  {}", s, entry.path().unwrap_or("?"));
}
```

#### Variant B — single-file status (`git status -- <path>`)

```rust
let flags = repo.status_file(std::path::Path::new("src/main.rs"))?;
// `flags` is a `Status` bitflag; check e.g. flags.is_wt_modified()
```

#### Variant C — scoped, index-only (`git diff --cached`-style)

```rust
let mut opts = StatusOptions::new();
opts.set_show(git2::StatusShow::Index)   // ignore workdir changes
    .include_untracked(false)
    .pathspec("src/**/*.rs");
```

**Gotchas**

- `StatusOptions` must be passed as `Some(&mut opts)` and is consumed during the call; reuse requires re-construction. Pass `None` for defaults.
- **Rename detection + pathspec conflict:** if you supply a `pathspec`, rename-detection results may be inaccurate because libgit2 needs the full file set to match moves. Do rename detection with *no* pathspec.
- `Statuses` borrows from the `Repository`; collect what you need before dropping the repo.
- `WT_UNREADABLE` (added 0.20.2) covers permission-denied/unreadable files — handle it or it silently shows as unchanged on some platforms.

**Performance**

- Cost is proportional to the number of files in the workdir and index, plus a `lstat` per file. On a typical checkout it is **milliseconds**. Rename detection (`renames_*`) roughly doubles cost because it diffs candidate pairs. Status is a **single pass** over the index and workdir — far cheaper than running a full `git diff`.

---

### `git log` (history traversal)

There is no single "log" function; you compose a `Revwalk` over the commit graph.

#### Variant A — linear log from HEAD (`git log`)

```rust
let mut walk = repo.revwalk()?;
walk.push_head()?;
walk.set_sorting(git2::Sort::TIME)?;

for oid in walk {
    let commit = repo.find_commit(oid?)?;
    println!("{} {}", &oid.to_string()[..7], commit.summary().unwrap_or(""));
}
```

#### Variant B — range / `git log A..B` (`git rev-list --reverse`)

```rust
let mut walk = repo.revwalk()?;
walk.push_ref("refs/heads/feature")?;     // include
walk.hide_ref("refs/heads/main")?;        // exclude (merge base boundary)
walk.set_sorting(git2::Sort::TOPOLOGICAL)?;
```

#### Variant C — first-parent only (`git log --first-parent`)

```rust
let mut walk = repo.revwalk()?;
walk.push_head()?;
walk.simplify_first_parent()?;   // follow only first parents — very fast on merge-heavy histories
```

**Gotchas**

- `Revwalk` yields **`Oid`s**, not `Commit`s. You must `repo.find_commit(oid)` per step — this is the dominant cost. Batch lookups benefit from libgit2's internal object cache (`opts::set_cache_max_size` in 0.21).
- You can also use `revparse` to resolve `HEAD~3`, `main@{1}`, etc., then seed the walk.
- Sorting matters: `Sort::TIME` is cheapest; `Sort::TOPOLOGICAL` requires extra bookkeeping; combine with `Sort::REVERSE` to invert direction.
- `hide`/`push` on the same walk create the `^` exclusion set exactly like `git rev-list`.

**Performance**

- Graph walking itself is cheap (pointer-chasing in the object DB). The `find_commit` per OID decompresses a zlib object — **microseconds each**, dominated by I/O on cold caches. On huge repos, `simplify_first_parent()` can cut work by an order of magnitude. Walkers are **lazy iterators**; you pay only for what you consume.

---

### `git branch`

Branches are references under `refs/heads/` (and `refs/remotes/`). Use `Branch` / `Branches` / `find_branch`.

#### Variant A — list branches (`git branch` / `git branch -a`)

```rust
use git2::BranchType;
for (branch, bt) in repo.branches(Some(BranchType::Local))?.flatten() { /* ... */ }
// BranchType::Remote for remotes; `None` for all.
let name = branch.name()?.unwrap();   // returns Result<Option<&str>> in 0.21
```

#### Variant B — create / move / delete (`git branch <name>`)

```rust
use git2::BranchType;
let target = repo.head()?.peel_to_commit()?;
repo.branch("feature", &target, false)?;        // create
// delete:
let mut b = repo.find_branch("feature", BranchType::Local)?;
b.delete()?;
```

#### Variant C — upstream tracking (`git rev-parse --abbrev-ref @{u}`)

```rust
let branch = repo.find_branch("feature", BranchType::Local)?;
let upstream = branch.upstream()?;              // refs/remotes/origin/feature
let ahead_behind = repo.graph_ahead_behind(
    branch.get().target().unwrap(),
    upstream.get().target().unwrap(),
)?;
```

**Gotchas**

- `Branch::name()` returns `Result<Option<&str>, Error>` since 0.21 — a missing value (unborn) is `Ok(None)`; a non-UTF-8 name is an `Err`. Older versions returned `Option<&str>` and **panicked** on non-UTF-8 names.
- Branches are a thin wrapper over `Reference`; `branch.get()` gives you the underlying `Reference` for raw OID access.
- `BranchType::Remote` names include the `origin/` prefix; the local name does not.

**Performance**

- Listing reads only the loose/packed ref files — **very fast** (sub-millisecond on typical repos). `graph_ahead_behind` walks the graph and is O(commits-in-range); cheap for small ranges, can be slow for divergent long histories.

---

### `git tag --list`

Tags live under `refs/tags/`. Two listing paths: `tag_names` (lightweight string list) or `references_glob("refs/tags/*")` for full refs.

#### Variant A — list tag names (`git tag`)

```rust
let names = repo.tag_names(None)?;     // None = all; Some("v*") for a glob
for name in names.iter() {
    println!("{name}");
}
```

#### Variant B — list with objects (`git tag -n`)

```rust
for (branch, _) in repo.references_glob("refs/tags/*")?.flatten() {
    let name = branch.name()?;
    if let Some(oid) = branch.target() {
        let obj = repo.find_object(oid, None)?;
        // peel annotated tags → commit; obj.as_tag() for the Tag object
    }
}
```

#### Variant C — `tag_foreach` callback (efficient bulk walk)

```rust
repo.tag_foreach(|oid, name_bytes| {
    // called per tag without materializing a Vec first
    true // continue
})?;
```

#### Creating tags

```rust
// Lightweight
repo.tag_lightweight("v1.0", &commit, false)?;
// Annotated (0.15+ preferred API)
repo.tag_annotation_create("v1.0", &commit, &signature, "release", false)?;
```

**Gotchas**

- Annotated tags are **objects** (a `Tag` wrapping signature/message/target); lightweight tags are just refs pointing at a commit. `find_tag` returns the annotated object; to always reach a commit, `find_tag(...)?.?.target_id()` then `find_commit`, or use `Object::peel(ObjectType::Commit)`.
- `tag_names` returns owned `StringArray` borrowed from the result — iterate within its lifetime.
- `Tag::is_valid_name()` validates against git's ref-name rules.

**Performance**

- Tag listing is ref-file I/O only — **negligible**. `tag_foreach` avoids allocations versus building a `Vec`. Peeling annotated tags adds one object lookup each (microseconds).

---

### `git remote`

Remotes are config-driven. `repo.remotes()` lists names; `find_remote` / `remote` / `remote_anonymous` load them; fetch/push go through `Remote`.

#### Variant A — list remotes & URLs (`git remote -v`)

```rust
let remotes = repo.remotes()?;            // StringArray of names
for name in remotes.iter().flatten() {
    let r = repo.find_remote(name)?;
    println!("{}  {}", name, r.url().unwrap_or(""));
}
```

#### Variant B — add / rename / delete / set URL (`git remote add`)

```rust
repo.remote("upstream", "https://github.com/upstream/repo")?;
repo.remote_set_url("origin", "git@github.com:owner/repo.git")?;
repo.remote_rename("old", "new")?;
repo.remote_delete("old")?;
```

#### Variant C — fetch (`git fetch`)

*(Requires the `ssh` and/or `https` feature.)*

```rust
use git2::{FetchOptions, RemoteCallbacks, Cred, CredentialType};

let mut callbacks = RemoteCallbacks::new();
callbacks.credentials(|_url, username, allowed| {
    if allowed.contains(CredentialType::SSH_KEY) {
        Cred::ssh_key_from_agent(username.unwrap_or("git"))
    } else { Err(git2::Error::from_str("no creds")) }
});

let mut fo = FetchOptions::new();
fo.remote_callbacks(callbacks).prune(git2::FetchPrune::On);

let mut remote = repo.find_remote("origin")?;
remote.fetch(&["refs/heads/*:refs/remotes/origin/*"], Some(&mut fo), None)?;
```

**Gotchas**

- **Feature gating (0.21+):** networking silently does nothing / errors without `ssh`/`https` features enabled. This is the #1 "why can't I clone?" question.
- `Remote::url()` returns `Result<&str>` (0.21+); pre-0.21 it returned `Option<&str>` and panicked on empty URLs (fixed 0.20.1).
- Credential callbacks must handle the **allowed** `CredentialType` mask and may be called **multiple times** (retry on auth failure). Returning the same cred repeatedly causes an auth loop.
- `remote_anonymous` creates an in-memory remote (e.g. a one-off URL) that **cannot be persisted**.
- Fetch refspec direction matters: `refs/heads/*:refs/remotes/origin/*` maps remote branches to remote-tracking branches; don't fetch directly into local `refs/heads` unless you mean it.
- SSH on macOS requires libssh2/OpenSSL setup; the `ssh_key_from_agent` path needs `SSH_AUTH_SOCK`.

**Performance**

- Listing/adding remotes is config-file I/O — **instant**. **Fetch/push cost is entirely network + pack negotiation**, identical to `git fetch`/`git push` (libgit2 implements the same protocol). Use `FetchOptions::depth` (0.18.1+) for shallow clones to cut transfer dramatically.

---

### `git grep`

There is **no dedicated `grep` API**. libgit2 has `git_grep` only experimentally; `git2` historically exposes search via two routes: tree walking + substring match, or the more efficient `Pathspec` + blob scan.

#### Variant A — search HEAD tree (most common workaround)

```rust
let head = repo.head()?.peel_to_tree()?;
head.walk(git2::TreeWalkMode::PreOrder, |root, entry| {
    if entry.kind() == Some(git2::ObjectType::Blob) {
        let blob = repo.find_blob(entry.id()).ok();
        if let Some(b) = blob {
            if b.content().windows(needle.len()).any(|w| w == needle) {
                println!("{}{}", root, entry.name().unwrap());
            }
        }
    }
    git2::TreeWalkResult::Ok
})?;
```

#### Variant B — search a single commit's tree (targeted)

```rust
let commit = repo.find_commit(oid)?;
let tree = commit.as_object().peel_to_tree()?;
// walk as above, scoped to a subdir:
repo.find_tree(tree.id())?
    .walk(/* ... */, |path, entry| { /* ... */ git2::TreeWalkResult::Ok })?;
```

#### Variant C — `Pathspec` to filter, then scan

```rust
use git2::{Pathspec, PathspecFlags};
let ps = Pathspec::new(&["**/*.rs"])?;
let matches = ps.match_tree(&head, PathspecFlags::DEFAULT)?; // list of paths
// then find_blob each match and substring-search
```

**Gotchas**

- Every blob you inspect is decompressed from the object DB on demand — **memory pressure** on large trees. Avoid holding all blobs; scan and drop.
- `TreeWalkMode::PreOrder` visits directories before children; `PostOrder` is the reverse. Return `TreeWalkResult::Skip` to prune a subtree (perf win).
- Binary files: check the buffer for a NUL byte before treating as text.
- For *regex* search, pair with the `regex` or `regex-lite` crate over the bytes.

**Performance**

- Cost = `O(blobs × blob_size)`. Walking the tree is cheap (one `TreeEntry` lookup each); reading every blob's content is the expense. On a monorepo this is **seconds**; on a small repo, **milliseconds**. There is no inverted index, so this is fundamentally a linear scan per invocation — far slower than `git grep` which streams the same way but is C-tight. For repeated queries, consider building your own index or shelling out.

---

### `git blame`

Line-level authorship via `Repository::blame_file` → `Blame` (an iterator of `BlameHunk`s).

#### Variant A — blame a file at HEAD (`git blame <file>`)

```rust
use git2::BlameOptions;
let blame = repo.blame_file("src/main.rs", Some(&mut BlameOptions::new()))?;
for hunk in blame.iter() {
    let sig = hunk.final_signature();   // Option in 0.21+
    println!("L{}-L{}  {}",
        hunk.final_start_line(),
        hunk.final_start_line() + hunk.lines_in_hunk() - 1,
        sig.map(|s| s.name().unwrap_or("")).unwrap_or_default());
}
```

#### Variant B — blame a line range (`git blame -L 10,20`)

```rust
let mut opts = BlameOptions::new();
opts.min_line(10).max_line(20);
let blame = repo.blame_file("src/main.rs", Some(&mut opts))?;
```

#### Variant C — blame with copy/move tracking (`git blame -M -C`)

```rust
let mut opts = BlameOptions::new();
opts.track_copies_same_file(true)              // -M (within file)
    .track_copies_same_commit_moves(true)      // -C within commit
    .track_copies_any_commit_copies(true)      // -C across all commits (expensive!)
    .use_mailmap(true)
    .ignore_whitespace(true);
let blame = repo.blame_file("src/main.rs", Some(&mut opts))?;
```

#### Variant D — blame in-memory buffer (`git blame` against uncommitted edits)

```rust
let base = repo.blame_file("src/main.rs", None)?;
let blame = base.blame_buffer(&modified_bytes)?;   // 0.18.0+
```

**Gotchas**

- `BlameHunk::final_signature` / `orig_signature` return `Option` since **0.21** (previously could segfault when signature info was missing). Always handle `None`.
- `blame_file` takes a **path relative to the repo root**, not the worktree-relative path in all cases — verify with a known file.
- `track_copies_any_commit_copies(true)` is the libgit2 equivalent of `-C -C -C` and is **dramatically slower** — it re-runs blame across every blob in history.
- `first_parent(true)` mirrors `--first-parent` and is a major speedup on merge-heavy histories at the cost of accuracy.
- Blame is **read-only** and never modifies the repo.

**Performance**

- Default blame is roughly `O(file_lines × commits_touching_file)` with diff machinery per step. Small files: **milliseconds**. Large files with deep history: **hundreds of ms to seconds**. Copy-tracking across commits can push it to **minutes** on big repos. The libgit2 in-memory cache helps repeat runs in the same process.

---

### Other operations

#### `git diff` / `git show`

Six diff entry points on `Repository`: `diff_tree_to_tree`, `diff_tree_to_index`, `diff_index_to_workdir`, `diff_tree_to_workdir`, `diff_tree_to_workdir_with_index`, `diff_index_to_index`, plus `diff_blobs`. Configure via `DiffOptions`; extract with `Diff::print`, `Diff::deltas`, `DiffStats`, or `Patch::from_diff`.

**Gotchas:** renaming large trees with `DiffFindOptions::rename_threshold` recomputes pairwise similarity — quadratic-ish; set `rename_limit`. Binary files yield `DiffBinary` (literal/delta); don't assume text.

**Performance:** diff cost ∝ changed files × content size; rename detection is the multiplier.

#### `git commit` / index staging

Stage with `Index::add_path` / `add_all` (the `IndexAddOption` flags), build a `Tree` via `index.write_tree()`, then `repo.commit(...)`. `commit_create_buffer` + `commit_signed` supports signed commits without GPG.

**Gotchas:** `Index` must be `write()`'d to disk to persist; `write_tree` returns a tree OID. You must supply a valid `Signature` (use `repo.signature()` for config-derived identity, or `author_from_env`/`committer_from_env` in 0.21).

#### `git merge` / rebase / cherry-pick

`merge_analysis` first (decide fast-forward vs real merge), then `merge`/`merge_trees`/`merge_commits`. Full `Rebase` state machine: `repo.rebase` → `Rebase::next`/`commit` loop. Cherry-pick via `repo.cherrypick`.

**Gotchas:** always check `repo.state()` for an in-progress merge/rebase before starting another. Conflicts leave the index in a conflict state — use `IndexConflicts` iterator and resolve before committing.

#### `git checkout` / reset

`CheckoutBuilder` (in `git2::build`) drives `checkout_head`/`checkout_index`/`checkout_tree`. `repo.reset(target, ResetType::Hard, Some(&mut checkout))` mirrors soft/mixed/hard reset.

**Gotchas:** `checkout_head` behavior is subtle (documented caveat added 0.21) — it does *not* recreate files deleted from the index by default. Use explicit `CheckoutBuilder` flags (`force`, `update_index`).

#### `git stash`

`repo.stash_save` / `stash_save_ext` (0.17.2+, supports partial stashing via `StashSaveOptions`), `stash_apply`, `stash_pop`, `stash_drop`, `stash_foreach`. Stashes live in `refs/stash` and the reflog.

#### `git config`

`repo.config()` returns a merged `Config` (system → global → local priority). `get_string`/`get_bool`/`get_i64`, or iterate `entries()` via `ConfigEntries::next`/`for_each` (the safe API that replaced the removed `Iterator`).

**Gotchas:** `ConfigEntries` is **single-pass**; calling `next` after exhaustion is fine but you cannot rewind. Open `Config::open_default()` for the global-only view.

#### `git rev-parse` / revisions

`repo.revparse("HEAD~3")`, `revparse_single("main^2")`, `revparse_ext` (also returns the intermediate `Reference`). Supports the full gitrevisions syntax (`@{upstream}`, `:path`, ranges).

**Gotchas:** ranges return a `Revspec` with `from`/`to`; the `^` exclusion is represented by the direction. A miss returns `ErrorCode::NotFound`.

#### `git describe`

`repo.describe(DescribeOptions)` → `Describe` → `.format(...)` for `git describe`-style names (`v1.2-4-gabc`). Configure tags, abbrev length, dirty flag, and `DescribeFormatOptions`.

---

## Cross-cutting notes & gotchas

- **`Repository` is `!Sync`.** Share across threads via `Mutex<Repository>` or open one per thread. libgit2 is thread-safe at the C level but `git2` marks the handle `!Sync` to prevent aliased mutation.
- **Object cache is process-global.** libgit2 caches parsed objects; repeated lookups in one process are fast, but the cache has a default size (tunable via `opts::set_cache_max_size` in 0.21 / `set_cache_object_limit` in 0.20.1). Long-running daemons should bound it.
- **`Error` carries rich context.** Match on `ErrorCode` (`NotFound`, `Conflict`, `Auth`, `Timeout`, `Owner`, …) rather than string-matching messages. `0.21`'s `From<Utf8Error> for Error` lets `?` propagate non-UTF-8 paths cleanly.
- **Ownership validation (safe.directory).** libgit2 1.5+ checks directory ownership; mismatched owners yield `ErrorCode::Owner`. Disable with `opts::set_verify_owner_validation(false)` for containers/CI.
- **Vendored vs system libgit2.** Default build vendors a static libgit2 (adds build time + ~1MB). For smaller binaries or distro packaging, set `LIBGIT2_NO_VENDOR=1` and install a system libgit2 ≥ the required version.
- **Security/CVE history.** Always track the latest 0.x for libgit2 security fixes — 0.18.2 fixed CVE-2024-24575/24577; the 0.16/0.17 series fixed SSH host-key handling.
- **When to prefer `gix` instead.** `git2`/libgit2 is the pragmatic default for read-mostly work and mature features (blame, merge). For pure-Rust auditing, no C build step, partial-clone, or the newest Git features (libgit2 lags canonical git), the `gix` crate is the modern alternative.
