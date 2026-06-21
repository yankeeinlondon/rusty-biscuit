---
ready: false
agent: codex
model: ""
---

# Review: More Repo

## Findings

### High: single-package repos still report an empty package catalog/count in the aggregate

The iteration fixed root-package facts for `repo package-manager`, `repo dependencies`, and the aggregate dependency projections, but the same standalone package still disappears from the package catalog. In a plain Cargo repo with one root `Cargo.toml`, `sniff repo --json` now reports `"package_manager": "cargo"` and one external dependency, while simultaneously reporting `"package_count": 0` and `"packages": []`. The focused `sniff repo package-count --json` also returns `{ "package-count": 0 }`, and `sniff repo packages --json` still errors with `Not inside a recognized repository`.

That violates the aggregate redesign's "complete, but each fact once" contract for the group-A repo-wide facts: `packages`, `package_count`, `package_manager`, `dependencies`, and `package_dependencies` should describe the same package universe. The fallback `RepoInfo` is already present in the aggregate builder, but `packages` is populated through `collect_repo_package_names`, which intentionally returns empty for `!repo.is_monorepo`, and `package_count` is copied from `RepoIdentity`, whose count is only populated for monorepos. See `sniff/cli/src/output/repo_json.rs:745`, `sniff/cli/src/output/repo_json.rs:795`, `sniff/cli/src/output/filesystem/packages.rs:42`, and `sniff/lib/src/filesystem/repo/identity.rs:94`.

Fix this by making the synthesized root-package `RepoInfo` the package source of truth for the aggregate and focused package-count path, or by changing the library identity/count model so a recognized standalone root package counts as one package. Add Level 1 CLI tests that assert a standalone Cargo/Node/Python package has `package_count == 1`, `packages == ["..."]`, and matching package dependency/dependency facts in bare `repo --json`.

Verification level: Level 1 CLI integration is appropriate and currently incomplete for this requirement.

### Medium: README documents an invalid software subcommand

The spec rehomes `audio-players` under `sniff software audio-players`, but `sniff/cli/README.md` documents `sniff software audio`, which the CLI rejects with an "unrecognized subcommand" error and a suggestion for `audio-players`. See `sniff/cli/README.md:221`.

This is not a runtime bug, but this feature is a hard CLI break and the repo instructions require README updates when public behavior changes. Update the README example and add a cheap help/assertion test or doc audit check if this command list is maintained manually.

Verification level: documentation/static check is sufficient.

## Test Rigor Notes

- The remaining issues are data-contract and documentation problems; Level 1 unit/CLI integration coverage is the right level.
- I did not find a requirement in this feature that requires Level 2 real-terminal capture or Level 3 OS keyboard injection. The branch/software/repo commands expose ordinary command output and JSON contracts, not emulator input encoding or keypress behavior.
- The previous high-severity branch JSON issue now has appropriate Level 1 coverage: `test_repo_branches_json_shape` asserts the nullable tracking fields are present.

## Verification

- Ran `cargo test -p sniff-cli --test cli single_package --color=never` — passed.
- Ran `cargo test -p sniff-cli --test cli test_repo_branches_json_shape --color=never` — passed.
- Manually reproduced the remaining single-package catalog/count gap with a temporary one-crate Cargo repo and `target/debug/sniff --base "$tmp" repo --json`.
- Manually confirmed `sniff programs`, `sniff editors`, and `sniff repo deps` are removed, and `sniff software` / `sniff repo package-dependencies` / `sniff repo dependencies` are accepted.
