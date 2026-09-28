# Provenance: cargo-1.98.1/workspace

- Tool: cargo 1.98.1 (797e8a9bc 2026-08-05)
- Host: macOS 27.2 (build 26B5091g), Darwin 27.2.0 arm64 (`uname -a`: Darwin shazam.home 27.2.0 ... RELEASE_ARM64_T6041 arm64)
- Date: 2026-09-26
- Generated in a scratch directory (`/tmp/lockfile-fixtures/...`), then copied here.

## Commands (in order)

```sh
# write root Cargo.toml (package fixture-root + [workspace] members/exclude + [workspace.package] version = "0.3.0")
cargo new --vcs none --lib crates/alpha --name alpha
cargo new --vcs none --lib crates/beta --name itoa
cargo new --vcs none --lib .tools/hidden --name hidden-tool
cargo new --vcs none --lib local-lib --name local-lib
# rewrite member Cargo.toml files (alpha: version.workspace = true, depends on
#   itoa = { path = "../beta" } and itoa_registry = { package = "itoa", version = "1" })
cargo generate-lockfile   # network: crates.io sparse index
```

## Trimmed

- Nothing removed (no `target/` was created; `--vcs none` created no `.git`). `src/` stubs from `cargo new` kept so every manifest has a target.

## Expected membership

- Declared by the manifest (relative to the workspace root, root excluded, sorted):
- `.tools/hidden` (crate `hidden-tool`)
  - `crates/alpha` (crate `alpha`, version from `version.workspace = true` -> 0.3.0)
  - `crates/beta` (crate `itoa`)
- Recorded by the lockfile:
- `version = 4`; Cargo.lock does NOT record membership or paths. Every path package (members, root,
    and the non-member `local-lib`) is a `[[package]]` with no `source` key; registry packages carry
    `source = "registry+..."` and `checksum`.
  - sourceless packages: `alpha 0.3.0`, `fixture-root 0.1.0`, `hidden-tool 0.1.0`, `itoa 0.1.0`, `local-lib 0.1.0`
  - registry `itoa 1.0.18` shares the name of member `itoa 0.1.0`; dependency strings become `"itoa 0.1.0"` / `"itoa 1.0.18"`

## Notes

Cargo accepted a member crate named `itoa` plus a renamed registry `itoa` dependency without complaint.
`local-lib` is excluded via `exclude = ["local-lib"]` and is indistinguishable from a member in the lockfile.
