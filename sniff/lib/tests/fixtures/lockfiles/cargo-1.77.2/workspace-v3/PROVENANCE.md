# Provenance: cargo-1.77.2/workspace-v3

- Tool: cargo 1.77.2 (e52e36006 2024-03-26), docker image `rust:1.77-slim` (linux/arm64)
- Host: docker `rust:1.77-slim` on the macOS host above
- Date: 2026-09-26
- Generated in a scratch directory (`/tmp/lockfile-fixtures/...`), then copied here.

## Commands (in order)

```sh
rsync -a --exclude Cargo.lock ../../cargo-1.98.1/workspace/ ./
sed -i '' 's/edition = "2024"/edition = "2021"/' local-lib/Cargo.toml   # 1.77 does not know edition 2024
tar -cf - . | docker run -i --rm rust:1.77-slim sh -c 'mkdir /w && cd /w && tar xf - && cargo generate-lockfile >&2 && cat Cargo.lock' > Cargo.lock
```

## Trimmed

- Nothing (the project was streamed into the container with tar; only Cargo.lock was streamed back).

## Expected membership

- Declared by the manifest (relative to the workspace root, root excluded, sorted):
- `.tools/hidden`
  - `crates/alpha`
  - `crates/beta` (crate `itoa`)
- Recorded by the lockfile:
- `version = 3`; same package set as the v4 fixture; no membership or path data (see `cargo-1.98.1/workspace`)

## Notes

Docker could not bind-mount `/private/tmp`, hence the tar-over-stdin approach.
