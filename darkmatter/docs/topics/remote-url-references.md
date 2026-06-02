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
- Ordinary rendered links and image URLs such as
  `[site](https://example.com)` are validated for URL shape and preserved in the
  output. They are not fetched by composition.

Unsupported v1 schemes, including `ftp:`, `s3:`, `ipfs:`, and `data:`, are not
remote compose inputs. Local file references continue to use the existing
`FileReference` path rules.

## Allowed Hosts

Remote reads use the shared `biscuit-file` fetch primitive and its
`FetchPolicy`. The default policy is deny-all. A host must be explicitly
allowed before any request is issued.

```bash
md compose doc.md --allow-host example.com
```

The CLI currently accepts exact host names. Library callers configure
`ComposeOptions::with_remote_read_config(...)`.

## Cache And Freshness

Remote response bodies can be stored in the persistent compose cache when a
cache root is configured:

```bash
md compose doc.md --allow-host example.com --cache-root .darkmatter/cache/v1
```

Freshness is resolved in this order:

1. `--remote-ttl <SECONDS>` overrides server cache headers.
2. `Cache-Control: max-age=<SECONDS>` sets the remote artifact expiry.
3. No TTL means the artifact is revalidated according to the freshness mode.

Freshness modes:

- `strict` revalidates stale remote artifacts and fails when revalidation fails.
- `fallback` revalidates stale artifacts but serves the stale cached body on
  network or HTTP failure.
- `optimistic` serves an existing cached body without revalidation.

`--remote-refresh` forces revalidation even when a cached remote artifact is
still fresh.

Conditional revalidation uses `If-None-Match` and `If-Modified-Since` when the
server previously returned `ETag` or `Last-Modified`.

## Side Effects

The side-effect engine's `http_post` verb uses the same shared fetch policy for
host allowlist enforcement. It is separate from compose: composing a document
never runs side effects.
