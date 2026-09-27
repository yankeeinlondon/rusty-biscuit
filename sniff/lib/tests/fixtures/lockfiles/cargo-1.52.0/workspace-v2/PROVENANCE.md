# Provenance: cargo-1.52.0/workspace-v2

- Tool: cargo 1.52.0 (69767412a 2021-04-21), docker image `rust:1.52-slim` (linux/arm64)
- Host: docker `rust:1.52-slim` on the macOS host above
- Date: 2026-09-26
- Generated in a scratch directory (`/tmp/lockfile-fixtures/...`), then copied here.

## Commands (in order)

```sh
rsync -a --exclude Cargo.lock ../../cargo-1.77.2/workspace-v3/ ./
sed -i '' 's/edition = "2021"/edition = "2018"/' Cargo.toml crates/*/Cargo.toml .tools/hidden/Cargo.toml local-lib/Cargo.toml
sed -i '' 's/^version.workspace = true/version = "0.3.0"/' crates/alpha/Cargo.toml   # workspace inheritance needs 1.64+
# removed the [workspace.package] table from the root Cargo.toml
tar -cf - . | docker run -i --rm rust:1.52-slim sh -c 'mkdir /w && cd /w && tar xf - && cargo generate-lockfile >&2 && cat Cargo.lock' > Cargo.lock
```

## Trimmed

- Nothing (only Cargo.lock was streamed back from the container).

## Expected membership

- Declared by the manifest (relative to the workspace root, root excluded, sorted):
- `.tools/hidden`
  - `crates/alpha`
  - `crates/beta` (crate `itoa`)
- Recorded by the lockfile:
- v2 format signature: NO top-level `version` key, `checksum` inline on each registry `[[package]]`
    (v1 would instead have a `[metadata]` table of checksums). Same package set; no membership data.

## Notes

The image tag says 1.52 (rustc 1.52.1) but its cargo reports 1.52.0; the directory uses the cargo version.
