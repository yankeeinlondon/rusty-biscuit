# Remote object storage

## What it is

The local store dedups artifacts on one machine. The **remote** shares them across machines: the
daemon pushes and pulls blobs to S3-compatible object storage, so a build on one host can be
restored on another. Cache keys deliberately exclude absolute paths and machine identity, which is
what makes this portable.

**Supported backends:** any S3-compatible endpoint — AWS S3, MinIO, Ceph, Cloudflare R2. There is no
separate proprietary object store; self-hosted MinIO/Ceph and R2 are configured exactly like S3 with
a custom endpoint.

**Important limitation:** only **Rust** artifacts travel to the remote. C/C++ object caching is
local-only. Don't plan a C/C++ CI strategy around remote sharing.

## What it buys you

The local cache mostly catches cheap, frequently rebuilt crates. The expensive ones — `tokio`,
`kube`, `tauri`, anything with a long single-crate compile — are exactly what you want to *not*
build twice across machines. That's the remote's value: it turns "every machine pays the expensive
first build" into "one machine pays it".

Concretely it helps when:

- **CI runners start cold every job.** The highest-value case by far.
- **Several machines build the same target triple** — laptop plus build server, or a fleet of
  agents. Point them at one bucket.
- **A team shares a dependency graph.** Dependencies rarely change; their artifacts are highly
  reusable.
- **Ephemeral environments** — containers, cloud dev boxes, worktrees on fresh hosts.

It helps little when a single long-lived machine builds one project — the local store already has
everything, and you're adding network round-trips and credentials for nothing.

## Sync model

```bash
kache sync                 # pull + push
kache sync --pull          # download only
kache sync --push          # upload only
kache sync --dry-run       # show what would transfer
kache sync --all
kache sync --manifest-path path/to/Cargo.toml
```

Two distinct strategies, and picking the wrong one is the common mistake:

**Warm prefetch (preferred).** The daemon selectively downloads only *expensive* artifacts named by
a build manifest. Cheap crates are faster to compile than to fetch, so skipping them is a win.
Requires a manifest recorded by `kache save-manifest`, and is tuned by `min-compile-ms` — crates
that compile faster than the threshold aren't prefetched at all.

**Full sync (`sync --pull`).** Downloads the entire remote store up front. Simple, thorough, and
slow. Use it for a genuinely cold environment where you want everything resident, not as a default.

## Manifests, namespaces and prefetch warming

```bash
kache save-manifest                          # key defaults to the host target triple
kache save-manifest --manifest-key <key>     # scope builds explicitly
kache save-manifest --namespace <ns>         # also upload content-addressed per-dependency shards
```

A **manifest** records what a build actually needed, so a later run can prefetch precisely that set
instead of guessing. `--manifest-key` scopes manifests — default is the target triple, so cross-OS
or cross-arch builds don't pollute each other. Setting `--namespace` additionally uploads
per-dependency shards (when `Cargo.lock` exists), enabling finer-grained warming than the monolithic
manifest.

Sequence in practice: build → `save-manifest` → `sync --push`. Next environment: daemon warms from
the manifest → build → repeat.

## GitHub Actions

```yaml
- uses: kunobi-ninja/kache-action@v1
  with:
    s3-bucket: my-build-cache
    s3-region: eu-west-1
    s3-access-key-id: ${{ secrets.S3_ACCESS_KEY_ID }}
    s3-secret-access-key: ${{ secrets.S3_SECRET_ACCESS_KEY }}
```

Inputs and defaults:

| Input | Default | Notes |
| --- | --- | --- |
| `version` | latest release | |
| `s3-bucket` | — | **Setting this enables the S3 backend** |
| `s3-region` | `us-east-1` | |
| `s3-prefix` | `artifacts` | |
| `s3-endpoint` | — | Custom endpoint: MinIO, Ceph, R2 |
| `s3-access-key-id` / `s3-secret-access-key` | — | Use repo/org secrets |
| `github-cache` | `true` | Used when S3 is not configured |
| `cache-key-prefix` | `kache` | GitHub cache key prefix |
| `cache-executables` | `false` | Also cache bin/dylib/proc-macro outputs |
| `sync` | `false` | Pull the entire remote on setup — **S3 only** |
| `warm` | `true` | Selective prefetch of expensive artifacts — **S3 only** |
| `manifest-key` | — | Scope builds; defaults to target triple — **S3 only** |
| `namespace` | `manifest-key` | Content-addressed shards — **S3 only** |
| `min-compile-ms` | `1000` | Don't prefetch crates faster than this — **S3 only** |
| `max-size` | `50GiB` | Local store cap before LRU eviction |
| `token` | `${{ github.token }}` | Releases + PR comments |
| `pr-comment` | `true` | Sticky PR comment with cache stats |

**Backend choice:** with no `s3-bucket`, the action persists the store via GitHub Actions cache —
fine for a single repo, and it restores the whole store in one shot (which is why the S3-only tuning
inputs are ignored). Choose S3 when you need sharing **across repos, across runners, or with
developer machines**.

The action's post-build step records the manifest, uploads shards when `namespace` is set, then
pushes — so CI populates the cache your laptop later pulls from.

## Self-hosting the bucket

MinIO or Ceph with an `s3-endpoint` works the same as AWS. For a homelab or on-prem fleet this is
usually the better answer: no egress charges, and the bucket sits on the same network as the
builders, so warm prefetch is fast. Sizing follows the same logic as the local store — headroom
above the working set, with a lifecycle policy for cold objects.

## Security notes

- Credentials are ordinary S3 keys — use CI secrets, never commit them.
- Prefer **scoped credentials** limited to the bucket and prefix; the cache is a build-artifact
  store, not a place to grant broad object-store access.
- A shared cache is a **supply-chain surface**: anyone who can write to the bucket can serve
  artifacts to everyone who reads it. Keep write access limited to trusted CI, give developer
  machines read-mostly credentials, and don't share a bucket across trust boundaries.
- Content addressing means a fetched blob's key must match its content, which protects against
  accidental corruption — it is not by itself an authorization control.

## Planner (preview)

A remote planner service can prefetch based on workspace manifests, dependency history and build
intent. The daemon calls it when `KACHE_PLANNER_ENDPOINT` is set, with `KACHE_PLANNER_TOKEN` for
bearer auth; it returns `use_fallback` when it has no matching candidates. It lives in
`crates/kache-service` and persists state in an embedded SurrealDB. The hosted service is preview —
treat as experimental.
