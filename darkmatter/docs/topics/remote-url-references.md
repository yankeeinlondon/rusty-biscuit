# Remote URL References

Darkmatter can compose HTTP(S) resources through the same transclusion and
read-side expression surfaces used for local files.

## Supported Forms

- `::file https://example.com/doc.md` inserts the fetched response body.
- `::code https://example.com/snippet.rs` inserts the fetched response body as a
  code block.
- Read-side expression functions that accept file inputs, such as
  `frontmatter(url)`, `markdown_title(url)`, and `file_exists(url)`, can read
  HTTP(S) URLs through the remote fetch cache. The URL argument must be a
  quoted string literal, e.g. `{{ markdown_title("https://example.com/doc.md") }}`,
  because the interpolation expression parser only accepts a string literal
  there.
- **Remote URL arguments are body-interpolation only.** They are honored only on
  surfaces that carry a remote runtime, which is body interpolation. The
  **frontmatter** resolution context — both the pre-shell and post-shell
  interpolation passes, and the `$()` shell ternary condition/branch — is
  local-filesystem only, so a remote URL argument to a read-side function in
  frontmatter **fails loudly** rather than silently returning a default (e.g.
  `file_exists(url)` errors instead of returning `false`). In body
  interpolation, `file_exists(url)` is `true` once the URL is fetched; a
  denied host or a failed fetch is an error, not `false`.
- `absolute` and `relative` are **never remote**. They are local-only path
  transforms that require a resolution context for the base directory but never
  contact the network, and are not part of the remote-fetch discovery sweep.
- Ordinary rendered links and image URLs such as
  `[site](https://example.com)` are validated for URL shape and preserved in the
  output. They are not fetched by composition.

Unsupported v1 schemes, including `ftp:`, `s3:`, `ipfs:`, and `data:`, are not
remote compose inputs. Local file references continue to use the existing
`FileReference` path rules.

## Allowed Hosts

Remote reads use the shared `biscuit-file` fetch primitive and its
`FetchPolicy`. The default policy is deny-all. A host must be allowed before
any request is issued, in one of two ways:

- **For every run**, set `network: allow` in the `permissions` block of
  `~/.darkmatter.yaml`. Inside a repository the repository's
  `.darkmatter.yaml` must also say `allow`, both as committed at `HEAD` and in
  the working copy; an uncommitted edit cannot widen what `HEAD` says (see
  [Permissions](./permissions.md)). Every HTTP(S) host is then allowed.
- **For one run**, name the exact host:

  ```bash
  md compose doc.md --allow host=example.com
  ```

  This works under `network: deny` and changes nothing under `allow`.

| User file | Repository (`HEAD` / working copy) | `--allow host=example.com` | `https://example.com/a.md` | `https://other.test/a.md` |
| --- | --- | --- | --- | --- |
| absent or `deny` | any | no | refused | refused |
| absent or `deny` | any | yes | fetched | refused |
| `allow` | none (outside a repository) | no | fetched | fetched |
| `allow` | `allow` / `allow` | no | fetched | fetched |
| `allow` | `deny` / `allow` | no | refused | refused |

A schema slot that reads a URL takes the same decision: under `network:
allow` its read needs no approval, and under `deny` only the run's hosts are
read without one.

`network: allow` changes consent, not the transport rules. Only `http` and
`https` are fetched, a redirect is never followed (every redirect fails the
read as a policy refusal, whatever the target), and a cached body is served
only when the host is allowed now.

When the `permissions` block is invalid (any part of it: a bad `network`
value such as `null` or `sometimes`, an invalid `file-read` or
`shell-commands` entry, `permissions: null`, or a reference to a missing
file), no URL is fetched, not even one whose host the run names with
`--allow host=HOST`. The read fails with a message that names the
configuration file, line, and setting, and no request is sent. Only a valid
`deny` (or an absent setting, which means `deny`) lets a run's host through.

```bash
# ~/.darkmatter.yaml says `permissions: {network: null}`
md compose doc.md --allow host=example.com
# fails: remote read denied for host 'example.com': … the configuration is
# invalid: ~/.darkmatter.yaml:2:12 at `/permissions/network`: …
```

Library callers configure `ComposeOptions::with_remote_read_config(...)` for
per-run hosts; the configured `network` permission comes from the request's
configuration snapshot (`ComposeOptions::with_config_snapshot`, or the one
`ComposeRequest::prepare` loads). Attach the snapshot before
`with_shared_remote_fetch()`; a request whose snapshot decides differently
replaces the shared runtime. An allowed host or `network: allow` enables
read-side expression URL reads (`frontmatter(url)`, `markdown_title(url)`,
`file_exists(url)`, …) on its own. Remote `::file` / `::code` block
transclusion additionally requires the explicit
`with_allow_remote_transclusion(true)` opt-in, since it injects fetched bodies
into the document; `md compose` always sets it.

## Cache And Freshness

Raw remote response bodies (transport artifacts) can be stored in the remote
transport cache when a cache root is configured. The store lives at
`<cache-root>/.darkmatter/cache/v1/`; semantic results such as composed
documents are never persisted. A cache root never authorizes a host.

```bash
md compose doc.md --allow host=example.com --cache-root .
```

Response `Cache-Control` outranks everything below: a `no-store` response is
never written (an existing `no-store` entry is removed), and a `no-cache`
response is revalidated before every reuse.

Otherwise, freshness is resolved in this order:

1. `--remote-ttl <SECONDS>` overrides the server's `max-age`.
2. `Cache-Control: max-age=<SECONDS>` sets the remote artifact expiry.
3. No TTL means the artifact is revalidated according to the freshness mode.

Freshness modes:

- `fallback` (the default) revalidates stale artifacts but serves the stale
  cached body on network or HTTP failure, keeping CI/offline builds resilient.
  A `no-cache` body is never served stale, and a refused request is never
  covered: if revalidation is redirected to a host the run does not allow,
  the read fails even though a cached body exists.
- `strict` revalidates stale remote artifacts and fails when revalidation fails.
- `optimistic` serves an existing cached body without revalidation, except a
  `no-cache` body.

`--remote-refresh` forces revalidation even when a cached remote artifact is
still fresh. See [Caching](./caching.md#remote-transport-cache) for the full
contract.

Conditional revalidation uses `If-None-Match` and `If-Modified-Since` when the
server previously returned `ETag` or `Last-Modified`.

## Side Effects

The side-effect engine's `http_post` verb uses the same shared fetch policy for
host allowlist enforcement. It is separate from compose: composing a document
never runs side effects.
