---
ready: false
agent: codex
model: ""
---

# Review: More Repo

## Findings

### High: repo test-runner manifest detection is substring-based, so it reports false positives

The strategy requires exact manifest dependency names for repo-declared test-runner usage (`test-runner-strategy.md:189`), and the catalog distinguishes dependency keys from other manifest content (`test-runner-strategy.md:204`). The implementation instead scans every raw manifest blob with `blob.contains(key)` (`sniff/lib/src/filesystem/repo/test_runner_usage.rs:356`). That makes unrelated strings look like runner dependencies.

Concrete repro:

```bash
tmp=$(mktemp -d)
printf '{"name":"jest-helper"}\n' > "$tmp/package.json"
cargo run --quiet --color=never -p sniff-cli --bin sniff -- --base "$tmp" repo test-runner --json
```

Output includes Jest even though there is no `devDependencies.jest`, `dependencies.jest`, Jest config file, or package.json `"jest"` config key:

```json
{
  "test_runners": [
    { "runner": "Jest", "source": { "key": "jest", "kind": "manifest" } },
    { "runner": "NodeTest", "source": { "kind": "ecosystem_default" } }
  ]
}
```

The same matcher is also too loose for colon-separated coordinates: `junit:junit` is split into duplicate halves and matches any manifest containing `junit` once (`sniff/lib/src/programs/test_runner_spec.rs:367`, `sniff/lib/src/filesystem/repo/test_runner_usage.rs:357`). For .NET, xUnit/NUnit/MSTest are modeled as independent alternative keys (`sniff/lib/src/programs/test_runner_spec.rs:386`), while the spec describes framework package combinations such as `xunit` + `xunit.runner.visualstudio` (`test-runner-strategy.md:250`).

Verification level: strongest present is Level 1 unit coverage, which is the right level for manifest semantics, but it lacks negative/exact-key cases. Add L1 tests for package names/scripts/config text that contain runner words without declaring dependencies, plus JVM/.NET combination semantics.

### High: repo test-runner config detection stores globs but only checks literal paths

The catalog includes config patterns such as Codeception `*.suite.yml` (`sniff/lib/src/programs/test_runner_spec.rs:302`), and the strategy explicitly lists `*.suite.yml` as a Codeception signal (`test-runner-strategy.md:227`). `config_glob_matches` treats every catalog entry as a literal `pkg_dir.join(glob)` and calls `exists()` (`sniff/lib/src/filesystem/repo/test_runner_usage.rs:214`), so glob patterns never match.

Concrete repro:

```bash
tmp=$(mktemp -d)
printf '{"name":"x"}\n' > "$tmp/composer.json"
printf 'actor: ApiTester\n' > "$tmp/api.suite.yml"
cargo run --quiet --color=never -p sniff-cli --bin sniff -- --base "$tmp" repo test-runner --json
```

Output is `{"test_runner":null}`, but the spec says `api.suite.yml` is enough to report Codeception. The same literal-only approach also leaves several documented convention/config signals unimplemented, such as RSpec `spec/**/*_spec.rb` and Behat `features/*.feature` (`test-runner-strategy.md:233`, `test-runner-strategy.md:236`).

Verification level: strongest present is Level 1 unit coverage, which is appropriate, but it only covers literal config filenames. Add L1 tests for glob/convention-only signals and use a real glob or shallow/depth-bounded matcher as appropriate for each catalog entry.

### Medium: hard-break documentation audit still has an active stale `repo deps` reference

The feature requires in-repo hard-break references to be migrated from `sniff repo deps` to `sniff repo package-dependencies` in the same change (`sniff/features/2026-06-14-more-repo/spec.md:420`). `rg --hidden` still finds an active, not-completed spec table using `repo deps` in `sniff/fixes/2026-05-07-repo-package-consistency/spec.md:34`. Historical completed feature/review docs can reasonably stay historical, but active specs are part of the working repo guidance and should not teach a removed command name.

Verification level: L1 grep/audit is sufficient; no L2/L3 terminal coverage is needed.

## Test Rigor Notes

- `sniff repo test-runner` manifest/config semantics are library/JSON behavior; Level 1 unit and CLI integration tests are the right tier. Current Level 1 tests are not strong enough for exact dependency matching, glob config matching, and negative false-positive cases.
- The `sniff repo --json` aggregate shape now includes `package_manager` and `test_runner`; Level 1 aggregate tests are appropriate for that JSON contract.
- Removed command paths are covered at Level 1 through clap/CLI failure tests. No Level 2 or Level 3 coverage is required because the feature does not assert terminal styling, key input, paste, mouse, or real terminal encoder behavior.

## Verification

- Passed: `cargo test --color=never -p sniff --lib filesystem::repo::test_runner_usage -- --nocapture`
- Passed: `cargo test --color=never -p sniff-cli repo_json::tests -- --nocapture`
- Passed: `cargo test --color=never -p sniff-cli test_old_programs_command_fails -- --nocapture`
- Passed: `cargo test --color=never -p sniff-cli test_repo_deps_is_not_an_alias -- --nocapture`
- Also ran the two manual `repo test-runner --json` repros above; both demonstrate the blocking detection gaps.
