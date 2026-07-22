---
prompt: "The `gitoxide` crate in Rust is written in pure Rust and is rapidly gathering adoption. It is often far faster than 'git2' (the traditional choice) and \n\nYour task is to do a deep dive into the `gitoxide` crate. Your research should be\nable to answer the following questions and cover the various topics:\n\n- Key URLS (docs, repo, etc.)\n- Functional overview\n- Architectural overview\n- Version history with dates and key changes for each release\n- Use Cases: for each use case give 2-3 variant examples of different variants of how this crate might achieve this operation. What gotchas are there, if any, are there for this operation? How expensive is this operation from a CPU and timing standpoint?\n    - git status\n    - git log\n    - git branch\n    - git tag --list\n    - git remote \n    - git grep\n    - git blame\n    - add others too"
last_updated: 2026-07-18
hash: c03cd691e18d1140-adf6181332f59a01
---
> **Naming note (read first).** The GitHub project is **`gitoxide`** (org [`GitoxideLabs`](https://github.com/GitoxideLabs/gitoxide)), but it publishes **two distinct crate streams** that move at different speeds and have *different version numbers*:
> 
> | What you add to `Cargo.toml` | Current stream | What it is                                                                          |
> |------------------------------|----------------|-------------------------------------------------------------------------------------|
> | **`gix`**                    | `0.84.x`       | The **library** crate. This is the one you depend on from application/library code. |
> | **`gitoxide`**               | `0.54.x`       | The **binary/CLI** meta-crate (ships the `gix` + `ein` binaries).                   |
> 
> When the prompt says "the `gitoxide` crate", the dependency that matters for *programming* is **`gix`**. Everything below uses the `gix` API surface (the same API the `gix`/`ein` CLIs are built on).

## Key URLs

| Resource                       | URL                                                                |
|--------------------------------|--------------------------------------------------------------------|
| Source repository              | https://github.com/GitoxideLabs/gitoxide                           |
| Library docs (`gix`)           | https://docs.rs/gix                                                |
| CLI crate (`gitoxide`)         | https://crates.io/crates/gitoxide                                  |
| Library crate (`gix`)          | https://crates.io/crates/gix                                       |
| CHANGELOG                      | https://github.com/GitoxideLabs/gitoxide/blob/main/CHANGELOG.md    |
| Crate status matrix            | https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md |
| Stability / MSRV guide         | https://github.com/GitoxideLabs/gitoxide/blob/main/STABILITY.md    |
| `git2` → `gix` migration hints | https://docs.rs/gix/latest/gix (search the docs for `git2`)        |
| Releases (binaries)            | https://github.com/GitoxideLabs/gitoxide/releases                  |

## Functional Overview

`gix` is an **idiomatic, pure‑Rust implementation of Git** — object database, refs, index, config, transport, pack files, commit‑graph, diff/merge, status, and blame — exposed behind a single `Repository` hub type. It targets four goals: pure‑Rust (no libgit2/libC dependency for core paths), **correctness** (on‑disk format 100% compatible with canonical `git`), **performance** (parallelism, memory‑mapped packs, fast zlib via `zlib-rs`), and a **pleasant, type‑safe DX** (the type system makes many misuses unrepresentable).

Capabilities include: `clone`/`fetch`/`push` (with sparse checkout from day one), `status`, blob & tree diff, three‑way merge (blobs/trees/commits), `commit` (with hooks), commit‑graph traversal, `rebase` (early), worktree checkout/stream/archive, `reset`, read/write objects & refs, read/write `.git/index`, config, pathspecs, revspecs, `.gitignore`/`.gitattributes`, and `blame`. Notable *non‑goals*: it does **not** try to be a drop‑in CLI clone of `git`, and it deliberately avoids async‑IO for the heavy CPU/decompression paths (using `blocking` + an interrupt system instead).

## Architectural Overview

### Workspace = ~60 plumbing crates behind one facade

`gix` is a **facade**. The real work lives in narrowly‑scoped `gix-*` plumbing crates; the top‑level `gix` crate re‑exports them and adds ergonomics. Selected layers:

- **Object storage:** `gix-hash`, `gix-hashtable`, `gix-object`, `gix-odb` (object DB), `gix-pack` (packfiles/multi‑pack‑index), `gix-commitgraph` (the `.git/objects/info/commit-graph` acceleration file).
- **Refs / config / index:** `gix-ref`, `gix-refspec`, `gix-config` (+ `gix-config-value`), `gix-index`, `gix-attributes`, `gix-ignore`, `gix-pathspec`.
- **Repo discovery & state:** `gix-discover`, `gix-worktree` (+ `gix-worktree-state`, `gix-worktree-stream`), `gix-sec` (trust), `gix-lock`, `gix-tempfile`, `gix-fs`, `gix-dir`.
- **Diffing / merging:** `gix-diff`, `gix-imara-diff` (line‑level diff engine), `gix-merge`, `gix-revision`, `gix-revwalk`, `gix-traverse`, `gix-negotiate`.
- **Network:** `gix-transport` (pluggable), `gix-packetline` (+ `_blocking`), `gix-protocol`, `gix-credentials`, `gix-url`.
- **Higher‑level features:** `gix-status`, `gix-blame`, `gix-submodule`, `gix-filter`, `gix-archive`, `gix-mailmap`, `gix-lfs`.
- **Shared infra:** `gix-features` (parallelism, progress, zlib, hashing backends), `gix-date`, `gix-actor`, `gix-glob`, `gix-quote`, `gix-utils`, `gix-validate`, `gix-error`, `gix-trace`, `gix-macros`.

Because everything is re‑exported into `gix::`, **most users only ever add `gix`** and reach for plumbing only to escape the facade's performance defaults.

### The `Repository` hub

`gix::Repository` (`ThreadSafeRepository` is the `Send + Sync` twin via `into_sync()`) is the entry point — open with `gix::open(path)`, `gix::discover(path)` (walks up parents), or `gix::init(...)`. It holds an object DB handle (`objects`) and a ref store (`refs`). By default it is `Send` but **not `Sync` unless the `parallel` feature is on** (which makes core structs `Sync` and turns on multi‑threaded algorithms).

### Trust model

On open, the repo is assigned a [`gix_sec::Trust`](https://docs.rs/gix-sec) level based on **ownership** vs. the current user. Untrusted config sections silently drop *sensitive* values (paths to executables, etc.), so reading a hostile repo won't execute attacker‑controlled programs. `open::Options::bail_if_untrusted()` makes `gix` behave like `git`/`git2` (refuse outright).

### Caching layers (the key to performance)

Object access goes through: (1) an optional **memory‑capped LRU object cache** (`Repository::object_cache_size(bytes)` — **off by default**), then (2) a small fixed‑size **pack delta‑base cache**. The docs are explicit: *the fastest object access is the one you don't do*, and a poorly‑sized object cache (low hit rate) can make you *slower*. `cache-efficiency-debug` prints hit/miss stats. `compute_object_cache_size_for_tree_diffs(&index)` picks a sane size (~10 MB per 10 k tracked files).

### Feature flags (compile‑time control)

`gix` ships "batteries included" but most users over‑compile. Organized into **Bundles** (`basic`, `extras`, `comfort`), **Components** (`status`, `blame`, `index`, `dirwalk`, `revision`, `revparse-regex`, `blob-diff`, `merge`, `worktree-stream`, `worktree-archive`, `mailmap`, `attributes`, `credentials`, `worktree-mutation`, `tree-editor`, `command`, `interrupt`), **Network** (mutually exclusive async/blocking, plus curl‑/reqwest‑based HTTPS with pluggable TLS), **Performance** (`parallel`, `pack-cache-lru-static/dynamic`, `max-performance`), **Hashes** (`sha1` default, `sha256`). Library authors should start with `default-features = false` and add only what they need. MSRV is **1.82** (raised in the `0.46` cycle).

### Stability tiers (from `crate-status.md`)

Only `gix-lock` (Tier 1) and `gix-tempfile` (Tier 2) are "production grade / 1.0‑ish". Stabilization candidates: `gix-mailmap`, `gix-chunk`, `gix-ref`, `gix-config`, `gix-glob`, `gix-actor`, `gix-hash`. **`gix` itself is still "initial development / usable"** — expect API churn; follow semver + the [STABILITY.md](https://github.com/GitoxideLabs/gitoxide/blob/main/STABILITY.md) guide.

## Version History

The `gitoxide` *workspace/CLI* crate (`0.54.0` latest, 2026‑05‑26) and the `gix` *library* crate (`0.84.x` latest) are versioned **independently**. The table below tracks the **`gitoxide` workspace** releases (the cadence `gix` follows), with the headline change per release. Full list (60+ releases since `0.1.0` on 2020‑07‑12) is on [crates.io](https://crates.io/crates/gitoxide/versions).

| Version         | Date               | Key changes                                                                                                                                                |
|-----------------|--------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **0.54.0**      | 2026‑05‑26         | Latest CLI/workspace release.                                                                                                                              |
| 0.53.0          | 2026‑04‑28         | —                                                                                                                                                          |
| 0.52.0 / .1     | 2026‑03‑22 / 04‑24 | `gix free trust` (probe trust level of a path; Windows‑friendly).                                                                                          |
| 0.51.0          | 2026‑02‑22         | —                                                                                                                                                          |
| 0.50.0          | 2026‑01‑22         | New `gix-error` / `Exn` exception‑tree error model (`Exn::into_inner()`).                                                                                  |
| 0.49.0          | 2025‑12‑31         | —                                                                                                                                                          |
| 0.48.0          | 2025‑12‑22         | —                                                                                                                                                          |
| 0.47.0          | 2025‑11‑22         | `gix credential fill` without a repo; removed `doc_auto_cfg` (fix docs.rs build).                                                                          |
| **0.46.0**      | 2025‑10‑22         | **`gix branch list`**, `gix commit sign` prototype; **MSRV → 1.82** (replaced `once_cell` with std).                                                       |
| 0.45.0          | 2025‑07‑15         | **`gix tag list`**, `gix revision list --long-hashes`, `commitgraph list from..to` (hide), multi‑range `gix blame -L`, precious‑file `.gitignore` parsing. |
| 0.44.0 / 0.43.0 | 2025‑04‑26 / 25    | CLI command docs cleanup.                                                                                                                                  |
| 0.42.0          | 2025‑04‑04         | **`gix diff file`**, revspec support for revisions/paths, blame respects diff algorithm + `--since`.                                                       |
| **0.41.0**      | 2025‑01‑18         | **`gix blame` CLI** (+ `-L start,end` range, stats), `gix env`.                                                                                            |
| 0.40.0          | 2024‑12‑22         | **`gix log` (debug)**, `gix merge --tree-favor`.                                                                                                           |
| 0.39.0          | 2024‑11‑24         | `gix merge tree`/`commits`/`commit --debug` (like `git merge-tree`).                                                                                       |
| **0.38.0**      | 2024‑10‑22         | **`time` → `jiff`**; `gix worktree list`, `gix cat`, `gix merge-file`, `gix merge-base`.                                                                   |
| 0.37.0          | 2024‑07‑23         | `mailmap check`, `gix clone --ref`.                                                                                                                        |
| **0.36.0**      | 2024‑05‑22         | **Security fix** (GHSA‑7w47‑3wg8‑547c, trampling‑herd index load race); `core.protectHFS`/`protectNTFS` checkout support.                                  |
| **0.35.0**      | 2024‑04‑13         | **`gix status`** with `--ignored`, `--index-worktree-renames`, submodule/rewrite support; `gix is-clean`/`is-changed`; `--dirty-suffix`.                   |
| 0.34.0          | 2024‑02‑25         | **`gix clean`** (basic).                                                                                                                                   |
| 0.33.0          | 2023‑12‑29         | `rev parse --reference`; **breaking:** `interrupt::init_handler()` marked `unsafe`.                                                                        |
| **0.32.0**      | 2023‑12‑06         | **Breaking:** renamed `GITOXIDE_*` env vars → `GIX_*`; `rev parse --format` (blob variants).                                                               |
| 0.25.0–0.31.0   | 2023               | Transport/protocol maturation, `ein tool`, status plumbing (`gix-status` crate landed).                                                                    |
| 0.20.0          | 2022‑12‑22         | Major `gix` crate restructure/facade stabilization wave.                                                                                                   |
| 0.13.0          | 2022‑07‑22         | Early clone/fetch porcelain.                                                                                                                               |
| 0.10.0          | 2021‑10‑19         | First broadly usable `gix` API.                                                                                                                            |
| **0.1.0**       | 2020‑07‑12         | First published release.                                                                                                                                   |

> Note: dates are the crates.io `created_at` (authoritative publish time). Minor UTC/local offsets vs. CHANGELOG prose exist (e.g. `0.53.0` shows 04‑27 on crates.io, 04‑28 in CHANGELOG).

## Use Cases

All examples assume `let repo = gix::open(".")?;` (use `gix::discover(".")?` to walk up from cwd). API is **pre‑1.0** — method names are stable in spirit but signatures drift between minor versions; treat snippets as the *shape* to confirm against your pinned `gix` version on docs.rs.

### `git status`

`Repository::status(progress)` returns a configurable `status::Platform` you turn into an iterator; `is_dirty()`/`is_pristine()` are the fast short‑circuit.

**Variant 1 — quick "is there any change?" (fast path):**

```rust
let dirty = repo.is_dirty()?;          // bails on first change; mirrors `git diff --quiet`
let pristine = repo.is_pristine()?;
```

**Variant 2 — full index↔worktree↔HEAD status:**

```rust
use gix::status::UntrackedFiles;
let platform = repo.status(gix::progress::Discard)?;
// platform.untracked_files(UntrackedFiles::Files);  // configure as needed
for item in platform.into_iter()? {       // -> status::Iter
    let item = item?;
    println!("{:?}", item);               // status::Item (Conflict / Change / ...)
}
```

**Variant 3 — submodule / rename-aware status:** the platform exposes submodule handling and separate index↔worktree rename tracking (`--index-worktree-renames`, something even `git` doesn't do by default). Use the plumbing `gix::status::plumbing` (= `gix-status`) for full control.

**Gotchas:** (1) `status()` **requires a progress argument** (pass `gix::progress::Discard` for none). (2) Needs the `status` feature (on by default) and a loaded `index`; `repo.index_or_empty()` if you want to tolerate no index. (3) Stat‑check options come from config — `core.trustCtime`/file‑system capabilities matter for correctness on network FS.
**Cost:** Cheap. Single directory walk + an index lookup per entry + per‑file `stat`. O(tracked files); negligible CPU. Parallelizable; the bottleneck is filesystem `stat` syscalls.

### `git log`

Revision walking via `Repository::rev_walk(tips)` → `revision::walk::Platform` → `.all()` (a `Walk` iterator).

**Variant 1 — walk from HEAD:**

```rust
let head = repo.head_id()?;
for commit_id in repo.rev_walk(Some(head)).all()? {
    let commit = commit_id?.object()?.into_commit();
    println!("{} {}", commit.id, commit.message_raw()?);
}
```

**Variant 2 — range / revspec (`A..B`, `A...B`):**

```rust
let spec = repo.rev_parse("v1.0..HEAD")?;   // revision::Spec (first/second endpoints)
for commit_id in repo.rev_walk(spec).all()? { /* ... */ }
```

**Variant 3 — first‑parent / topo‑sort / count:** configure the `revision::walk::Platform` (`.first_parent()`, sort options, `.count()` for just a tally like `git rev-list --count`).

**Gotchas:** (1) **Decode lazily** — the walk yields cheap IDs; only call `.object()` on the ones you actually print, or you pay full commit‑decompression for everything. (2) **Enable the commit‑graph**: `repo.commit_graph_if_enabled()` uses `.git/objects/info/commit-graph` for huge speedups on large histories; without it, walks do per‑commit object DB lookups. (3) **Set an object cache** for repeated access: `repo.object_cache_size_if_unset(...)`.
**Cost:** O(commits in range). Cheap per‑commit *if* commit‑graph + object cache are warm; CPU‑bound on object decompression otherwise. `--count`‑style tallies can be done without decoding bodies.

### `git branch` / `git tag --list` (refs)

Both are reference iteration: `Repository::references()` → `reference::iter::Platform` with `.local_branches()`, `.remote_branches()`, `.tags()`, `.all()`, `.prefixed(prefix)`.

**Variant 1 — list local branches:**

```rust
let branches: Vec<_> = repo.references()?
    .local_branches()?
    .map(|r| r.map(|r| r.name().as_bstr().to_string()))
    .collect::<Result<_, _>>()?;
```

**Variant 2 — list tags (≈ `git tag --list`):**

```rust
let tags: Vec<_> = repo.references()?
    .prefixed("refs/tags/")?           // or .tags()
    .map(|r| r.map(|r| r.name().as_bstr().to_string()))
    .collect::<Result<_, _>>()?;
```

**Variant 3 — look up / create:** `repo.find_reference("main")?`; create a lightweight tag `repo.tag_reference("v1.0", commit_id, gix_ref::transaction::PreviousValue::MustNotExist)?`; create a branch via `repo.reference("refs/heads/feature", id, PreviousValue::MustNotExist, "msg")?`.

**Gotchas:** (1) `branch_names()` reads *config* (`branch.*` sections), not the ref store — use `references().local_branches()` to enumerate actual branches. (2) Ref names are bytes (`BStr`), not UTF‑8 — non‑UTF‑8 names are silently skipped by some helpers.
**Cost:** Trivial. `gix-ref` may read a `packed-refs` file + loose ref files; O(number of refs). No decompression.

### `git remote`

**Variant 1 — list remote names:**

```rust
let names: Vec<_> = repo.remote_names().into_iter().collect();   // e.g. ["origin"]
```

**Variant 2 — look one up & read its URL/refspecs:**

```rust
let remote = repo.find_remote("origin")?;
println!("{:?}", remote.url(gix::remote::Direction::Fetch));
```

**Variant 3 — create/connect:** `let remote = repo.remote_at("https://...")?;` then `.fetch(...)` / `.connect(...)` / `.refspecs(...)`; `remote_default_name(Direction::Fetch)` resolves the configured default.

**Gotchas:** (1) **Network is opt‑in** — you must enable a `blocking-network-client` (+ an HTTPS transport like `blocking-http-transport-reqwest`/`-curl`) or `async-network-client` feature; without it `Remote::connect`/`fetch` aren't available. (2) TLS backend is chosen via features (rustls vs native‑tls vs openssl) — pick deliberately for licensing/FIPS. (3) Credential helpers need the `credentials` feature; `gix credential fill` now runs without a repo (0.47+).
**Cost:** Listing = trivial (config read). Connect/fetch = network‑bound + pack indexing (CPU‑heavy decompression).

### `git grep`

**There is no first‑class `gix grep`.** `gix` has no content‑search command/method; you build it from tree traversal + a regex. This is the single biggest "gotcha" vs. `git grep`.

**Variant 1 — search the *worktree* (fastest, recommended):** don't use `gix` for the search itself — honor `.gitignore` via `repo.excludes(...)`/`repo.dirwalk(...)` (feature `dirwalk`/`excludes`) to decide which paths to descend, then run your own `regex`/`aho-corasick` over file contents. This mirrors what `ripgrep` does, Git‑aware.

**Variant 2 — search a *committed tree* (no checkout):** iterate the tree's blobs and search each decoded blob:

```rust
let tree = repo.head_tree()?;
let mut walker = tree.iter();           // gix::object::tree::Iter
let re = regex::bytes::Regex::new("TODO")?;
loop {
    let Some(entry) = walker.next() else { break };
    let entry = entry?;
    if entry.mode().is_blob() {
        let blob = repo.find_blob(entry.id())?;
        for (i, line) in blob.data.lines().enumerate() {
            if let Ok(line) = line { if re.is_match(line.as_ref()) {
                println!("{}:{}: {}", entry.filename(), i + 1, String::from_utf8_lossy(line.as_ref()));
            }}
        }
    }
}
```

**Variant 3 — search across many blobs in parallel:** use the `parallel` feature (`gix::parallel`) to fan blob decode + match across threads; useful for "search all blobs at HEAD".

**Gotchas:** (1) No index (Git's grep has no special index either — it's a tree walk). (2) Decoding every blob is expensive; **filter by path first** (pathspecs / filename extension) before reading contents. (3) `gix` is not tuned to beat `ripgrep` at raw content search — for a CLI `grep`, combine `gix` (tree/ignore semantics) with a fast search crate.
**Cost:** O(total blob bytes in scope). Dominated by object decompression (zlib) + the regex itself; highly parallelizable. Much cheaper if you constrain to a subtree.

### `git blame`

`Repository::blame_file(path, suspect, options)` (feature `blame`, on by default) returns a `gix_blame::Outcome` — a list of consecutive `BlameEntry` hunks, each mapping a line range to its introducing commit.

**Variant 1 — blame a whole file at HEAD:**

```rust
use gix::repository::blame_file::Options;
let suspect = repo.head_id()?;
let blame = repo.blame_file("src/lib.rs".into(), suspect, Options::default())?;
for entry in blame.iter() {
    println!("L{}-{} → {} ({})",
        entry.range.start, entry.range.end,
        entry.commit.id, entry.commit.author.name);
}
```

**Variant 2 — blame only a line range (`-L`):** pass a range in `Options` (the CLI supports multiple `-L` ranges; 0.45+). Far cheaper than full‑file blame.
**Variant 3 — blame with rename/copy tracking / since‑filter:** `gix-blame` (0.42+) respects the configured diff algorithm and `--since` to cap history traversal.

**Gotchas:** (1) `blame` is marked **"very early"** in `crate-status.md` — semantics are close to `git` but edge cases (merges, copies, line‑ending normalization, filter pipelines) may differ. (2) It is the **single most expensive** operation here — it repeatedly diffs successive revisions of the file across history. **Constrain the range** and set an object cache. (3) `suspect` is the commit to start from; pass an earlier commit to blame only a slice of history.
**Cost:** High. O(history depth × file revisions). CPU‑bound on per‑revision line diffs + object decompression. Range‑limiting is the main lever; the commit‑graph helps.

### Others (brief)

- **`git diff` (tree↔tree / blob↔blob):** `repo.diff_tree_to_tree(...)`, or `diff::resource_cache()` for rapid in‑memory diffs. **`gix diff tree|file`** CLI exists (0.42+). Cost ∝ changed entries + (if line‑level) hunks.
- **`git merge`:** `repo.merge_trees(...)`, `repo.merge_commits(...)`, `repo.merge_file(...)` (3‑way, feature `merge`); `merge_base(one, two)` / `merge_bases_many(...)` for merge bases. CLI: `gix merge tree|commits|file` (0.39–0.40). For a hermetic committed-tip prediction, do not assume the high-level commit facade is isolated: the pinned `gix` resource cache may consult the live index for attributes, persist synthesized virtual-base objects, or honor configured drivers/filters. Enable `Repository::with_object_memory()` before merging, build the temporary index and attribute stack from the captured `ours` tree, use `gix::merge::plumbing::commit`, materialize unresolved stages into an in-memory index, and reject applicable external drivers, filters, or renormalization. Cost: tree merge O(entries); line merge O(file size).
- **`git clone`/`git fetch`:** `gix::prepare_clone(url, path)` / `prepare_clone_bare(...)`; or `Remote::fetch`. **Requires** a network feature. Cost: network + pack receive + index + (optional) checkout.
- **`git ls-tree`/`ls-files`:** `repo.head_tree()?.iter()`; `repo.index()?` for the staged set. Cost: O(entries).
- **`git rev-parse`:** `repo.rev_parse("HEAD~2")?` / `rev_parse_single(...)`. Cheap (graph/ref lookups only).
- **`git describe`:** `revision` feature; `Id::describe(...)`. Cost ∝ tags × depth.
- **`git clean`:** `gix clean` (0.34) on top of `gix-dir`. Cost: worktree walk.
- **`git archive`:** `repo.worktree_archive(...)` (feature `worktree-archive`; your app supplies `tar`/`zip`). Cost: stream tree → container.
- **`git commit`:** `repo.commit(...)` / `commit_as(...)` / `new_commit(...)` (writes objects + updates HEAD); `gix commit sign` (0.46).

## Cross‑Cutting Gotchas

1. **`gix` is pre‑1.0.** Expect breaking API changes between minor versions; pin precisely and regenerate docs against your version.
2. **CLI binaries are explicitly unstable** — `gix`/`ein` are development/validation tools; *do not script them* (per README).
3. **Feature flags are load‑bearing** for compile time *and* capability. Start from `default-features = false` as a library author; don't blindly enable `max-performance-safe` in a library.
4. **`Repository` is `!Sync` without `parallel`.** Use `into_sync()` → `ThreadSafeRepository` to share across threads, or enable `parallel`.
5. **Object cache defaults to OFF** and the wrong size *hurts*. Profile with `cache-efficiency-debug`; prefer "don't access" over "cache the access".
6. **Integrity checks differ from `git2`** — `git2` does strict hash/object verification by default that `gix` currently lacks.
7. **Async is not "everywhere"** — `gix` is sync at the core; bridge to async via `blocking`. Only transport can be async.
8. **Trust model** silently elides untrusted sensitive config; if you need `git`‑like strictness, set `bail_if_untrusted()`.

## Performance Notes (vs `git2` / canonical `git`)

- **Pure Rust, no libC for core paths** → no `git2`/libgit2 C dependency, easier cross‑compile/static‑link, memory‑safe.
- **`zlib-rs`** for decompression (the `zlib-*` features are now deprecated no‑ops; zlib‑rs is always used) — very fast pack decode.
- **Built‑in parallelism** (`parallel` feature) for status, pack, and traversal; mmap‑backed object DB.
- **commit‑graph** support makes history walks (log, merge‑base, blame) dramatically faster on large repos — enable it.
- In practice `gix`'s `status`/`object‑access`/`pack` paths are competitive with or faster than `git2`, while giving you a typed, idiomatic Rust API. The slow spots are **blame** (inherently), **unbounded grep**, and **uncached cold object access** — all addressable with the levers above.

## See Also

- [SHORTCOMINGS.md](https://github.com/GitoxideLabs/gitoxide/blob/main/SHORTCOMINGS.md) for known limitations.
- [STABILITY.md](https://github.com/GitoxideLabs/gitoxide/blob/main/STABILITY.md) for the churn‑expectation guide.
- The `git2` doc‑aliases: open `gix` docs and **search `git2`** to find the `gix` equivalent of a `git2` method.
