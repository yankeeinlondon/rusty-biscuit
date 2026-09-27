# Provenance: bazel-8.4.2/bzlmod

- Tool: Bazel 8.4.2 via `npx -y @bazel/bazelisk` with `USE_BAZEL_VERSION=8.4.2`
- Host: macOS 27.2 (build 26B5091g), Darwin 27.2.0 arm64 (`uname -a`: Darwin shazam.home 27.2.0 ... RELEASE_ARM64_T6041 arm64)
- Date: 2026-09-26
- Generated in a scratch directory (`/tmp/lockfile-fixtures/...`), then copied here.

## Commands (in order)

```sh
printf 'module(name = "fixture_root", version = "0.1.0")\n' > MODULE.bazel
USE_BAZEL_VERSION=8.4.2 npx -y @bazel/bazelisk mod deps --lockfile_mode=update
USE_BAZEL_VERSION=8.4.2 npx -y @bazel/bazelisk shutdown
```

## Trimmed

- Nothing to trim: `mod deps` created no `bazel-*` convenience symlinks. Only MODULE.bazel and MODULE.bazel.lock exist.

## Expected membership

- Declared by the manifest (relative to the workspace root, root excluded, sorted):
- none (bzlmod has no workspace-member concept)
- Recorded by the lockfile:
- JSON, top-level keys `lockFileVersion` (18), `registryFileHashes` (132 BCR URL -> sha256),
    `selectedYankedVersions` ({}), `moduleExtensions` (4 entries). No membership data. The file is ~165 KB
    even with zero declared deps because Bazel's implicit builtin modules are recorded.
