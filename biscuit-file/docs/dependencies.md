# Biscuit File Dependencies

## File References And Remote Fetch

- `url` is enabled by `file-reference` so `FileReference` can classify HTTP(S)
  descriptors without applying local path normalization to URL syntax.
- `reqwest`, `bytes`, and `tokio` are gated behind the off-by-default `fetch`
  feature. They provide the shared policy-enforcing HTTP primitive used by
  Darkmatter compose and side-effect network paths.

The default feature set includes URL classification through `file-reference`,
but it does not compile the HTTP client stack unless `fetch` is enabled.
