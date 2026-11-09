# Distribution verification: biscuit-hash-cli 0.1.0

Result: **incomplete** (6 passed, 0 failed, 10 pending, 2 disabled). Pending and disabled entries are not passes.

Host: `Darwin bolt.home 27.2.0 Darwin Kernel Version 27.2.0: Tue Sep 29 22:00:39 PDT 2026; root:xnu-13432.40.177.0.3~56/RELEASE_ARM64_T6000 arm64`; Docker engine: aarch64.

| Target | Route | Evidence class | Status | Notes |
| --- | --- | --- | --- | --- |
| aarch64-apple-darwin | archive | native-build-host | pass | [aarch64-apple-darwin/archive.txt](runtime/aarch64-apple-darwin/archive.txt) |
| aarch64-apple-darwin | installer | native-build-host | pass | [aarch64-apple-darwin/installer.txt](runtime/aarch64-apple-darwin/installer.txt) |
| aarch64-apple-darwin | clean-runtime | pending-infrastructure | pending | no clean macOS 15 VM without developer tools or Homebrew is provisioned |
| x86_64-apple-darwin | archive | pending-infrastructure | pending | no native x86_64 macOS host (this host is aarch64; Rosetta would be emulation) |
| x86_64-apple-darwin | installer | pending-infrastructure | pending | no native x86_64 macOS host (this host is aarch64; Rosetta would be emulation) |
| aarch64-unknown-linux-gnu | glibc-refusal | native-clean-runtime | pass | [aarch64-unknown-linux-gnu/glibc-refusal.txt](runtime/aarch64-unknown-linux-gnu/glibc-refusal.txt) |
| aarch64-unknown-linux-gnu | archive | native-clean-runtime | pass | [aarch64-unknown-linux-gnu/archive.txt](runtime/aarch64-unknown-linux-gnu/archive.txt) |
| aarch64-unknown-linux-gnu | installer | native-clean-runtime | pass | [aarch64-unknown-linux-gnu/installer.txt](runtime/aarch64-unknown-linux-gnu/installer.txt) |
| aarch64-unknown-linux-gnu | wsl2-archive | pending-infrastructure | pending | no WSL2 host to run the Linux artifact (WSL2 is not native Windows) |
| aarch64-unknown-linux-gnu | wsl2-installer | pending-infrastructure | pending | no WSL2 host to run the Linux artifact (WSL2 is not native Windows) |
| x86_64-unknown-linux-gnu | glibc-refusal | pending-infrastructure | pending | no native x86_64 Linux host (this Docker engine is aarch64; an emulated container is not runtime evidence) |
| x86_64-unknown-linux-gnu | archive | pending-infrastructure | pending | no native x86_64 Linux host (this Docker engine is aarch64; an emulated container is not runtime evidence) |
| x86_64-unknown-linux-gnu | installer | pending-infrastructure | pending | no native x86_64 Linux host (this Docker engine is aarch64; an emulated container is not runtime evidence) |
| x86_64-unknown-linux-gnu | wsl2-archive | pending-infrastructure | pending | no WSL2 host to run the Linux artifact (WSL2 is not native Windows) |
| x86_64-unknown-linux-gnu | wsl2-installer | pending-infrastructure | pending | no WSL2 host to run the Linux artifact (WSL2 is not native Windows) |
| * | channel:brew | disabled | disabled | docs/deployment.yaml: packages.biscuit-hash-cli.brew is not enabled |
| * | channel:binstall | disabled | disabled | docs/deployment.yaml: packages.biscuit-hash-cli.binstall is not enabled |
| * | binstall fixture-check | native-build-host | pass | cargo-binstall 1.26.2 against a 127.0.0.1 fixture release |

## Downloads

| File | Size (bytes) | SHA-256 (dist `.sha256`, `shasum -a 256 -c`) |
| --- | --- | --- |
| biscuit-hash-cli-aarch64-apple-darwin.tar.xz | 344728 | `44bf4b3fc4f9ef0cc689e4a6d13704c8a3bf819c82d1437ae0c22abbb2b6f0cb` |
| biscuit-hash-cli-aarch64-unknown-linux-gnu.tar.xz | 350416 | `17a55aba61c14fac9cde1f23f5e76eb4bcc767e590456ecdfb0a18c635a8d4d4` |
