# Phase 5 `dist plan` evidence

`dist plan --output-format=json` documents for the eight distributable CLI
packages, each checked with `scripts/ci/distribution.py validate-plan`.
Produced on 2026-11-09 on macOS (aarch64). Every selection is a
`verify-select` verification selection with a **fixture** Homebrew approval
(`--brew <command>`). No real approval was changed, `docs/deployment.yaml` was
not edited, and nothing was built or published.

## Result

All eight plans pass `validate-plan`. On the first pass `sniff-detection-cli`
failed because dist adds the package's `CHANGELOG.md` to every archive (see
[sniff-detection-cli](#sniff-detection-cli-sniff)); `validate-plan` and the
archive inspector now accept that one changelog, and the unchanged plan was
validated again.

| Package | Command | `dist plan` | `validate-plan` |
| --- | --- | --- | --- |
| biscuit-hash-cli | `bh` | exit 0 | valid |
| biscuit-file-cli | `bf` | exit 0 | valid |
| tree-hugger-cli | `hug` | exit 0 | valid |
| git-wt-cli | `wt` | exit 0 | valid |
| sniff-detection-cli | `sniff` | exit 0 | valid (after the changelog fix; first pass: invalid, exit 2) |
| darkmatter-cli | `md` | exit 0 | valid |
| darkmatter-lsp | `dmls` | exit 0 | valid |
| claudine-cli | `claudine` | exit 0 | valid |

Every `dist plan` run printed only this warning to stderr, which is expected
because no Homebrew publish job is configured:

```text
 WARN A Homebrew tap was specified but the Homebrew publish job is disabled
  consider adding "homebrew" to publish-jobs in Cargo.toml
```

## Files

- `dist-plan-<command>.json`: the `dist plan` stdout.
- `validate-<command>.json`: the `validate-plan --output` record.

## Inputs

### dist 0.34.0

The binary was downloaded from the `release_url` in
`.github/distribution/build-images.json` into a new, empty directory. Its
SHA-256 was checked against the pin in that file before extraction, then the
binary was extracted into a separate directory. It was not added to `PATH`.

```sh
curl --proto '=https' --tlsv1.2 -fsSL --retry 3 --max-time 300 \
  -o cargo-dist-aarch64-apple-darwin.tar.xz \
  https://github.com/axodotdev/cargo-dist/releases/download/v0.34.0/cargo-dist-aarch64-apple-darwin.tar.xz
echo "d1eec30e63eed23e5a5bd48fe96eae314799811bffca254ec1d2264638ba1890  cargo-dist-aarch64-apple-darwin.tar.xz" \
  | shasum -a 256 -c -     # cargo-dist-aarch64-apple-darwin.tar.xz: OK
tar -xJf cargo-dist-aarch64-apple-darwin.tar.xz -C /tmp/p5-dist-bin --strip-components=1
/tmp/p5-dist-bin/dist --version   # cargo-dist 0.34.0
```

Verified archive SHA-256:
`d1eec30e63eed23e5a5bd48fe96eae314799811bffca254ec1d2264638ba1890`.

### Source checkout

`stage` refuses a checkout with tracked modifications, and the repository's
working tree held uncommitted changes from earlier phases. The source was
therefore a throwaway clone outside the repository:

1. `git clone --quiet --no-hardlinks /Volumes/coding/rusty-biscuit /tmp/p5-src`
   (HEAD `dabb241075cf43f957109161e097b9c171727743`).
2. Every path from `git status --porcelain --untracked-files=all` (134 entries:
   added, modified, and one untracked file, `.github/distribution/rollout.json`;
   no deletions or renames) was copied from the working tree into the clone.
   Each copy was compared byte for byte (`cmp`) with the working-tree file.
   No `target/` directory was copied.
3. In the clone only, `user.name`, `user.email`, and `commit.gpgsign=false`
   were set in the local config, and `git add -A && git commit` produced
   **`e203e590ba9788d270a43fb41da63309bca9583e`**. That commit is the
   `commit` in every selection. It exists only in the deleted clone.

The clone and the staged trees were deleted afterward.

The snapshot reflects the working tree when the clone was made. Unstaged
edits that appeared in the working tree later (several package `README.md`
files, `docs/releases.md`, `docs/distribution.md`,
`.claude/skills/rust-devops/SKILL.md`) are not in it. The plans list asset
names and kinds, not file contents, so those edits do not change them.

## Commands

These commands ran from the repository root, once per package and one package
at a time. `PY=/tmp/distvenv/bin/python` is a venv with
`scripts/ci/requirements-distribution.txt`, and `W` is a new, empty directory
for each package. `<tag>` is the selection's `tag`, `<package>-v0.1.0` for
every package. The `stage`/`dist plan`/`validate-plan` steps match the
Prepare job in `.github/workflows/distribute-binaries.yml`.

```sh
$PY scripts/ci/distribution.py verify-select --package <package> --root /tmp/p5-src \
  --brew <command> --output "$W/selection.json"
$PY scripts/ci/distribution.py stage --selection "$W/selection.json" --source /tmp/p5-src \
  --dest "$W/staging" --notices scripts/distribution/fixtures/notices/header.md \
  --record-dir "$W/record" --output "$W/stage.json"
(cd "$W/staging" && /tmp/p5-dist-bin/dist plan --tag=<tag> --output-format=json) \
  > dist-plan-<command>.json
$PY scripts/ci/distribution.py validate-plan --selection "$W/selection.json" \
  --plan dist-plan-<command>.json --output validate-<command>.json
```

`--binstall` was not passed. The checked-in manifests carry the strategies
that disable binstall, so preflight correctly refuses a binstall approval.
The notices file is a fixture header, so the planned
`THIRD-PARTY-NOTICES.md` is a placeholder.

## Per package

Each package plans a release of version `0.1.0` with the tag `<package>-v0.1.0`
and GitHub hosting at `/yankeeinlondon/rusty-biscuit/releases/download/<tag>`.
Its release artifacts are:

- one `<package>-<target>.tar.xz` archive for each of the four targets:
  `aarch64-apple-darwin`, `x86_64-apple-darwin`, `aarch64-unknown-linux-gnu`,
  `x86_64-unknown-linux-gnu`;
- a `.sha256` file for each archive;
- `sha256.sum`;
- the shell installer `<package>-installer.sh`;
- the Homebrew formula `<command>.rb`.

Both installers cover all four targets. The "Bin targets" column lists the
package's `[[bin]]` targets from `cargo metadata --no-deps`. The
"Auxiliary excluded" column says whether every bin target other than the
command is absent from all four archives.

| Package | Command | validate-plan | Executables in each archive (all 4 targets) | Other archive files | Bin targets | Auxiliary excluded | Shell installer | Homebrew formula |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| biscuit-hash-cli | `bh` | valid | `bh` | LICENSE, README.md, SOURCE.txt, THIRD-PARTY-NOTICES.md | `bh` | n/a (none) | `biscuit-hash-cli-installer.sh` | `bh.rb` |
| biscuit-file-cli | `bf` | valid | `bf` | same four | `bf` | n/a (none) | `biscuit-file-cli-installer.sh` | `bf.rb` |
| tree-hugger-cli | `hug` | valid | `hug` | same four | `hug` | n/a (none) | `tree-hugger-cli-installer.sh` | `hug.rb` |
| git-wt-cli | `wt` | valid | `wt` | same four | `wt`, `render_dirty_tree` (`terminal-tests`) | yes: `render_dirty_tree` | `git-wt-cli-installer.sh` | `wt.rb` |
| sniff-detection-cli | `sniff` | valid (after fix) | `sniff` | CHANGELOG.md, LICENSE, README.md, SOURCE.txt, THIRD-PARTY-NOTICES.md | `sniff`, `render_cicd_fixture`, `render_git_status_fixture` (`test-fixtures`) | yes: both fixtures | `sniff-detection-cli-installer.sh` | `sniff.rb` |
| darkmatter-cli | `md` | valid | `md` | same four | `md`, `darkmatter-argv-recorder` (`test-fixtures`) | yes: `darkmatter-argv-recorder` | `darkmatter-cli-installer.sh` | `md.rb` |
| darkmatter-lsp | `dmls` | valid | `dmls` | same four | `dmls` | n/a (none) | `darkmatter-lsp-installer.sh` | `dmls.rb` |
| claudine-cli | `claudine` | valid | `claudine` | same four | `claudine`, `claudine-argv-recorder`, `claudine-fake-codex`, `claudine-fake-completion`, `claudine-fake-goose`, `claudine-fake-pi`, `claudine-fake-relay` (all `test-fixtures`) | yes: all six | `claudine-cli-installer.sh` | `claudine.rb` |

The archive names in each plan are `<package>-<target>.tar.xz`, for example
`darkmatter-cli-aarch64-apple-darwin.tar.xz`.

### sniff-detection-cli (`sniff`)

dist 0.34.0 automatically includes a `CHANGELOG.md` that it finds in the
package directory. `sniff/cli/CHANGELOG.md` is tracked, and it is the only
`CHANGELOG.md` in the eight package directories. Each sniff archive therefore
contains an asset of kind `changelog`, and `validate-plan` allows only
`license`, `readme`, `SOURCE.txt`, and `THIRD-PARTY-NOTICES.md` besides the
executable. Output of `validate-plan`, exit 2:

```text
distribution: dist plan: artifacts.sniff-detection-cli-aarch64-apple-darwin.tar.xz.assets: carries an unexpected changelog asset 'CHANGELOG.md'
```

The first-pass `validate-sniff.json` held that `invalid` result.

**Resolution.** The changelog is the package's release-plz changelog, which is
documentation, and more packages will gain one. `validate-plan` now accepts
exactly one asset of kind `changelog` named `CHANGELOG.md`, and
`inspect_archive.py` accepts that one optional non-executable member; any other
document name or an executable changelog is still refused. The same plan was
validated again against a `verify-select --package sniff-detection-cli --brew
sniff --checkout-commit e203e590ba9788d270a43fb41da63309bca9583e` selection,
and `validate-sniff.json` now holds `{"status": "valid", "tag":
"sniff-detection-cli-v0.1.0"}`. The plan is also a test fixture
(`scripts/ci/fixtures/distribution/dist-plan-sniff-detection-cli-brew.json`).

Apart from the changelog, the sniff
plan has the same shape as the other seven: `sniff` is the only executable,
and both fixture binaries are excluded.
