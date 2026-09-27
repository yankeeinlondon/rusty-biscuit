# Provenance: go-1.27.1/workspace

- Tool: go version go1.27.1 darwin/arm64 (GOTOOLCHAIN=local)
- Host: macOS 27.2 (build 26B5091g), Darwin 27.2.0 arm64 (`uname -a`: Darwin shazam.home 27.2.0 ... RELEASE_ARM64_T6041 arm64)
- Date: 2026-09-26
- Generated in a scratch directory (`/tmp/lockfile-fixtures/...`), then copied here.

## Commands (in order)

```sh
export GOTOOLCHAIN=local
(cd packages/alpha && go mod init example.com/fixture/alpha && printf 'package alpha\n\nimport _ "golang.org/x/text/language"\n' > alpha.go)
(cd packages/beta && go mod init example.com/fixture/beta && printf 'package beta\n' > beta.go)
go work init ./packages/alpha ./packages/beta
(cd packages/alpha && go get golang.org/x/text@v0.3.8)
go list -m all        # this is what wrote go.work.sum
```

## Trimmed

- Nothing.

## Expected membership

- Declared by the manifest (relative to the workspace root, root excluded, sorted):
- `packages/alpha`
  - `packages/beta`
- Recorded by the lockfile:
- `go.work.sum` records NO membership: it holds `<module> <version>[/go.mod] h1:<hash>` lines only
    (here the go.mod hashes of x/mod, x/sys, x/tools that no member go.sum contained). Membership lives in
    `go.work` `use` directives, which is the manifest, not a lockfile.

## Notes

`go work init` + `go get` did NOT write go.work.sum; it appeared only after a workspace-level command
(`go list -m all`) needed checksums absent from every member go.sum. go.work.sum has no version field.
