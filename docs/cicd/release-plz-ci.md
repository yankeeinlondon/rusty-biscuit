# release-plz in CI

A guide to [release-plz](https://release-plz.dev), the Rust release-automation tool, for someone
who knows Rust and the basics of GitHub Actions but has never used it.

## What release-plz is

release-plz answers one question on every push to your default branch: *given the commits since
the last release, what should each crate's next version be, and what changed?* It then writes that
answer down as a **pull request**.

That PR contains nothing but release bookkeeping — bumped `version` fields in `Cargo.toml`,
regenerated `CHANGELOG.md` entries, and updated internal dependency requirements. You read it like
any other PR. Merging it is the act of releasing: on merge, release-plz creates git tags, publishes
GitHub releases, and runs `cargo publish`.

So there are two commands, and they are the two halves of that loop:

| Command | Runs when | Does |
|---|---|---|
| `release-plz release-pr` | after each push to the default branch | opens or updates the release PR |
| `release-plz release` | after the release PR merges | tags, publishes to the registry, creates the forge release |

A third, `release-plz update`, does only the local half of `release-pr` — it computes versions and
rewrites files in your working tree and stops. It creates no branch, no PR, and contacts no forge.
That makes it the safe command for experimentation, and it matters more than its billing suggests;
see [Testing the workflow](#testing-the-workflow-before-you-merge-it).

### How it decides

Three inputs, in rough order of influence:

1. **Did the package change at all?** release-plz compares the files that would be *packaged* into
   the crate against the last released version. Changes to files excluded from the package (tests,
   CI config, docs outside the crate) do not trigger a release. This is a content comparison, not
   a commit-message one — release-plz will propose a release for a change described by a commit
   message it cannot parse.
2. **What do the commits say?** [Conventional commits](https://www.conventionalcommits.org)
   (`feat:`, `fix:`, `feat!:`) drive the size of the bump and group the changelog. They are
   *recommended, not required*: unparseable commits still count as changes and land in an "Other"
   changelog section.
3. **Did the public API break?** release-plz optionally runs
   [`cargo-semver-checks`](https://github.com/obi1kenobi/cargo-semver-checks) and bumps the major
   version when it detects a breaking change. This is its most distinctive feature — no other Rust
   release tool does it — and it means an accidental breaking change shows up as a surprising major
   bump in a PR you can reject, rather than as a broken build for your users.

### Where the "last released version" comes from

This is the single most consequential thing to understand, and the docs are quiet about it.

By default, release-plz asks **the cargo registry** — crates.io — what version each package is
currently at. It downloads the published crate and diffs your source against it.

That default is right for a crate you actually publish. It is wrong, and quietly so, for a crate
you do not, because *registry lookups are keyed on the crate name alone*. If your workspace
contains a crate called `queue` or `research` or `worktree`, and somebody unrelated published a
crate under that name, release-plz will download **their** crate and compute your next version
relative to **their** version history. Nothing warns you; you just get a version baseline that
belongs to a stranger's project. See [`git_only`](#git_only-and-the-registry-lookup) for the fix.

## Who uses it, and when it fits

Adopters include `aws-lambda-rust-runtime`, SurrealDB, `windows-drivers-rs`, `tokei`,
`cargo-generate`, `octocrab`, and `pavex`. The project maintains a
[list of public users](https://release-plz.dev/docs/release-plz-in-the-wild).

### Reach for it when

- **You publish Rust crates to a registry and want releases to be reviewable.** The release PR is
  the feature. It turns "did I remember to bump the version and write the changelog?" into a diff
  someone approves.
- **You have a workspace.** release-plz handles the tedious part of multi-crate releases: when a
  library changes, its dependents need version bumps too, and it works that graph out for you.
- **Breaking changes are expensive for you.** The `cargo-semver-checks` integration is the reason
  to pick release-plz over anything else in this space.
- **You want releases to originate in CI, not from a maintainer's laptop.** This is release-plz's
  explicit design centre, and the main axis separating it from the alternatives.

### Look elsewhere when

- **Your repository is not Rust, or not only Rust.** release-plz understands Cargo and nothing
  else. A polyglot monorepo wants [release-please](https://github.com/googleapis/release-please),
  which does the same PR-driven thing across many languages.
- **You want to run releases by hand.** [`cargo-release`](https://github.com/crate-ci/cargo-release)
  is the mature CLI for a human-triggered release, and it supports lifecycle hooks that release-plz
  does not.
- **Your workspace graph is gnarly and your changelogs are hand-curated.**
  [`cargo-smart-release`](https://github.com/GitoxideLabs/cargo-smart-release) (built for
  `gitoxide`) is designed for deeply interdependent workspaces with semi-handcrafted changelogs.
- **You are not publishing to a registry at all.** This is the subtle one. release-plz *can* run in
  a registry-free mode and is genuinely useful there — it still tags, still writes changelogs,
  still cuts GitHub releases. But several of its mechanisms assume packageable crates, and you will
  spend configuration effort switching those off. If all you want is a changelog, `git-cliff` (which
  release-plz uses internally) on its own is less machinery.
- **Every release needs human judgement about version numbers.** release-plz proposes versions from
  rules. You can edit the PR before merging, but if you are overriding it most of the time, the
  automation is costing you more than it saves.

## Configuring the workflow

Configuration lives in two places, and confusing them is a common early mistake:

- **`release-plz.toml`**, beside your root `Cargo.toml` — *what* to release and how to version it.
- **The workflow YAML** in `.github/workflows/` — *when* and *with what credentials* to run.

Both are optional in principle; release-plz's pitch is that it needs no configuration. That is true
for a single published crate and gets less true with every crate you add.

### The token decides whether your release PR gets tested

The default `GITHUB_TOKEN` **cannot trigger other workflows**. This is a GitHub security rule, not a
release-plz limitation, and its consequence is specific and easy to miss: *the release PR that
release-plz opens will have no CI checks on it.* Your tests do not run on the one PR whose merge
publishes to the world.

Three ways out:

| Approach | Author shown | Setup | Notes |
|---|---|---|---|
| `GITHUB_TOKEN` (default) | `github-actions[bot]` | none | release PR gets no CI; fine if you don't gate on it |
| Personal access token | the PAT's owner | low | use a machine account or your name is on every release |
| GitHub App token | `your-app[bot]` | moderate | most control; what release-plz itself uses |

If you do nothing else from this document, decide this one deliberately.

### Trigger design: don't race your own CI

The obvious trigger is `on: push: branches: [main]`. It works, and it means release-plz starts
computing a release *at the same time* your test suite starts running — so a release PR can be
opened for a commit that is about to go red.

The stricter pattern gates release on validation:

```yaml
on:
  workflow_run:
    workflows: ["ci"]      # the name: of your test workflow
    types: [completed]
    branches: [main]
```

…plus a job condition of `github.event.workflow_run.conclusion == 'success'`. Now the release
calculation only happens for commits that passed.

This buys real safety and costs you something specific, covered under
[Testing the workflow](#testing-the-workflow-before-you-merge-it).

### Checkout: do not detach HEAD

release-plz needs to know which branch it is releasing, and it works that out by running:

```
git rev-parse --abbrev-ref --symbolic-full-name @{upstream}
```

A detached HEAD has no upstream, so that command fails and release-plz aborts with
`failed to determine next versions` before computing anything. **`actions/checkout` detaches HEAD
whenever you give it a raw commit SHA** as `ref`.

This bites hardest in the `workflow_run` pattern above, where the natural instinct is to check out
exactly the commit that CI validated:

```yaml
# Broken: a bare SHA detaches HEAD.
- uses: actions/checkout@v5
  with:
    ref: ${{ github.event.workflow_run.head_sha }}

# Works: a branch ref, which also restores upstream tracking.
- uses: actions/checkout@v5
  with:
    ref: main
    fetch-depth: 0
```

If you needed the SHA for correctness, keep it as an assertion rather than as the checkout target —
compare `git rev-parse HEAD` against the validated SHA and stand down if the branch has moved on.
A branch that advanced is not an error: the newer commit gets its own validation run and its own
release calculation.

Always set `fetch-depth: 0`. release-plz reads tag and commit history; a shallow clone starves it.

### `git_only` and the registry lookup

`publish = false` does **not** stop release-plz reading crates.io. It only skips `cargo publish`.
Version *detection* still queries the registry, with the name-collision hazard described earlier.

```toml
[workspace]
publish  = false   # don't push to the registry
git_only = true    # and don't read from it either
```

`git_only = true` moves version detection onto your own git tags, matched against the `git_tag_name`
pattern. No registry traffic at all. If no tag matches, the package is treated as a first release.
It cannot be combined with `publish = true` — publishing and registry-free detection are mutually
exclusive by definition.

Use it whenever the registry is not your source of truth: internal crates, workspaces with names
you have not reserved, private registries you'd rather not poll, or air-gapped CI.

### `semver_check` is a compiler, not a linter

`cargo-semver-checks` builds each crate to compare its public API. It is the best reason to adopt
release-plz, and it is also, on a large workspace, the dominant cost of every release run — and it
can perturb build artifacts and lockfiles as a side effect. Leave it on when you publish for other
people. Turning it off is defensible when nothing external consumes your API, and your release runs
will get dramatically faster.

### The path-dependency trap in workspaces

release-plz runs `cargo package` as part of working out whether a crate changed. `cargo package`
enforces a rule that surprises people who have never published:

> all dependencies must have a version requirement specified when packaging

An intra-workspace dependency declared only by path —

```toml
my-other-crate = { path = "../other" }
```

— has no version requirement, and `cargo package` refuses it. The reasoning is sound: when a crate
is packaged, `path` is meaningless to whoever unpacks it, so cargo strips it and needs a version to
put in its place. The fix is to state both:

```toml
my-other-crate = { path = "../other", version = "0.1.0" }
```

On a workspace of any size, do this once centrally rather than in every manifest:

```toml
# root Cargo.toml
[workspace.dependencies]
my-other-crate = { path = "other", version = "0.1.0" }

# member Cargo.toml
[dependencies]
my-other-crate = { workspace = true }
```

**This requirement applies even if you never publish**, because it comes from the packaging step
release-plz uses for change detection, not from publishing. It is the most common reason a
registry-free release-plz setup fails on a workspace, and there is no configuration flag to skip
that step. Budget for it before adopting release-plz on a workspace whose internal dependencies are
path-only.

### Testing the workflow before you merge it

`workflow_run` workflows **always execute the definition from the default branch**, never from the
triggering commit or a PR branch. This is deliberate on GitHub's part — such a job holds
`contents: write` and your registry token, so honouring the triggering commit would let any commit
rewrite the privileged workflow. It is not a setting you can turn off.

The practical consequence: if you use the `workflow_run` trigger, **you cannot test changes to this
workflow on a pull request**. Every iteration has to land on the default branch first, and every
mistake is a red run on your main branch.

Add a `workflow_dispatch` escape hatch, because `workflow_dispatch` *does* honour the ref you
select:

```yaml
on:
  workflow_run: { workflows: ["ci"], types: [completed], branches: [main] }
  workflow_dispatch:

jobs:
  dry-run:
    if: github.event_name == 'workflow_dispatch'
    permissions:
      contents: read          # cannot release, by construction
    steps:
      - uses: actions/checkout@v5
        with: { fetch-depth: 0 }
      # ... install release-plz ...
      - run: release-plz update          # local worktree only
      - run: git status --porcelain      # exactly what a real run would rewrite
```

`release-plz update` computes versions into the working tree and stops — no branch, no push, no PR,
no registry. With write permissions dropped at the job level, it cannot release even if you got the
condition wrong. This turns a class of bug that could previously only be found on your main branch
into a four-minute experiment on a topic branch, and the resulting `git status` is a preview of the
release PR's contents.

One caveat: the official action installs a pinned release-plz version. If your dry-run job installs
release-plz itself, pin the **same** version, or the dry run stops predicting what the real job
does.

### Concurrency

Give the `release-pr` job a concurrency group so two pushes don't race to write the same release
branch:

```yaml
concurrency:
  group: release-plz-${{ github.ref }}
  cancel-in-progress: false
```

`cancel-in-progress: false` matters — a cancelled release is worse than a slow one. The `release`
job conventionally has *no* concurrency group, so a burst of merges can't cause a publish to be
skipped.

## Example configurations

### 1. Single published crate — the default case

Almost no configuration. This is release-plz at its best.

```toml
# release-plz.toml — optional; these are the defaults, shown for orientation
[workspace]
semver_check     = true
changelog_update = true
```

```yaml
name: Release-plz
on:
  push:
    branches: [main]
permissions:
  contents: write
  pull-requests: write

jobs:
  release-plz:
    runs-on: ubuntu-latest
    concurrency:
      group: release-plz-${{ github.ref }}
      cancel-in-progress: false
    steps:
      - uses: actions/checkout@v5
        with:
          fetch-depth: 0
      - uses: dtolnay/rust-toolchain@stable
      - uses: release-plz/action@v0.5
        env:
          GITHUB_TOKEN:         ${{ secrets.GITHUB_TOKEN }}
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}
```

**What the choices buy you.** Omitting `command:` runs both halves in one job: it publishes anything
unpublished, then refreshes the release PR. Full semver checking is on, which is the right default
when strangers depend on your API. The plain `push` trigger means the release PR may be opened before
tests finish — acceptable when the PR itself is the review gate. Using `GITHUB_TOKEN` means the
release PR gets no CI; for a single crate with a fast test suite, many maintainers accept that and
rely on the checks that ran on `main`.

### 2. Workspace with tags and GitHub releases, no registry

For internal crates, or a workspace whose names you have not reserved on crates.io. You still get
versioning, changelogs, tags and releases — you just never publish.

```toml
[workspace]
publish  = false            # no cargo publish
git_only = true             # and no registry reads either — versions come from tags

git_tag_enable   = true
git_tag_name     = "{{ package }}-v{{ version }}"   # required: names must not collide in a workspace
git_release_enable = true

semver_check = false        # nothing external consumes these APIs; saves a full build per crate
pr_draft     = true         # release PRs land as drafts for deliberate review
pr_labels    = ["release", "automated"]

[[package]]
name           = "my-lib"
changelog_path = "crates/my-lib/CHANGELOG.md"

[[package]]
name    = "internal-scratch"
release = false             # excluded from releases entirely
```

**What the choices buy you.** `git_only` is the load-bearing line: without it, `publish = false`
still leaves release-plz computing versions against whatever crates.io happens to have under each of
your crate names. `git_tag_name` must include `{{ package }}` in a workspace, or every crate fights
over `v1.2.3`. Disabling `semver_check` is the single biggest speed win available and is defensible
precisely because you are not publishing. `release = false` per package is how you keep scratch or
fixture crates out of the release graph.

**Before adopting this shape**, check your internal dependencies for the
[path-dependency trap](#the-path-dependency-trap-in-workspaces). A registry-free workspace whose
crates depend on each other by bare `path` will fail at `cargo package`, and the fix is a change to
every manifest.

### 3. Release gated on a green test suite

For a project where publishing a broken version is expensive.

```yaml
name: Release-plz
on:
  workflow_run:
    workflows: ["ci"]
    types: [completed]
    branches: [main]
  workflow_dispatch:          # the only way to test this workflow from a branch

permissions:
  contents: write
  pull-requests: write

jobs:
  release-pr:
    if: >-
      github.event_name == 'workflow_run' &&
      github.event.workflow_run.conclusion == 'success' &&
      github.repository == 'my-org/my-repo'
    runs-on: ubuntu-latest
    concurrency:
      group: release-plz-${{ github.ref }}
      cancel-in-progress: false
    steps:
      - uses: actions/checkout@v5
        with:
          ref: main            # a BRANCH — a bare SHA would detach HEAD
          fetch-depth: 0

      - name: Stand down unless main is still the validated commit
        id: freshness
        run: |
          if [ "$(git rev-parse HEAD)" != "${{ github.event.workflow_run.head_sha }}" ]; then
            echo "main advanced; the newer commit owns its own release calculation"
            echo "stale=true" >> "$GITHUB_OUTPUT"
          fi

      - uses: dtolnay/rust-toolchain@stable
        if: steps.freshness.outputs.stale != 'true'
      - uses: release-plz/action@v0.5
        if: steps.freshness.outputs.stale != 'true'
        with:
          command: release-pr
        env:
          GITHUB_TOKEN: ${{ secrets.RELEASE_PLZ_TOKEN }}   # PAT or App token, so the PR gets CI
```

**What the choices buy you.** The `workflow_run` trigger means no release is ever computed from a
commit that failed its tests. The repository check stops forks from running release logic. Splitting
out `command: release-pr` separates proposing a release from performing one, so publication is
driven by the PR merge rather than by a push. The freshness check exists because `workflow_run` jobs
are queued, not instantaneous — `main` can move between validation finishing and this job starting,
and treating that ordinary situation as a failure produces alarming red runs for no reason. A
non-default token is what gets CI to run on the release PR; with a plain `GITHUB_TOKEN`, gating the
release on a green suite and then opening an untested PR is self-defeating.

The cost is that this workflow can only be changed by merging to the default branch — hence the
`workflow_dispatch` trigger, and a dry-run job as sketched
[above](#testing-the-workflow-before-you-merge-it).

## A short adoption checklist

1. Decide the token first — it determines whether your release PR is tested.
2. Decide whether the registry is your source of truth. If not, set `publish = false` **and**
   `git_only = true`.
3. On a workspace, set `git_tag_name` to include `{{ package }}`.
4. On a workspace, audit internal dependencies for missing `version` alongside `path` before you
   need it at 3am.
5. Use `fetch-depth: 0`, and never check out a bare SHA.
6. Add `workflow_dispatch` plus a `release-plz update` dry run, especially if you use
   `workflow_run`.
7. Decide about `semver_check` with your eyes open: correctness for your users, or speed for you.

## References

- [release-plz documentation](https://release-plz.dev/docs) — [configuration reference](https://release-plz.dev/docs/config) · [GitHub Action quickstart](https://release-plz.dev/docs/github/quickstart) · [token options](https://release-plz.dev/docs/github/token) · [why yet another release tool](https://release-plz.dev/docs/why) · [users](https://release-plz.dev/docs/release-plz-in-the-wild)
- [release-plz on GitHub](https://github.com/release-plz/release-plz)
- [cargo-semver-checks](https://github.com/obi1kenobi/cargo-semver-checks)
- [git-cliff](https://git-cliff.org) — the changelog engine release-plz uses
- Alternatives: [cargo-release](https://github.com/crate-ci/cargo-release) · [cargo-smart-release](https://github.com/GitoxideLabs/cargo-smart-release) · [release-please](https://github.com/googleapis/release-please)
- Cargo's path-dependency rule: [rust-lang/cargo#11133](https://github.com/rust-lang/cargo/issues/11133)
