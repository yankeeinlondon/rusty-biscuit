The `sniff` CLI's `--json` output is governed by a principle documented at [`sniff/docs/topics/json-output.md`](../../docs/topics/json-output.md): **scope-complete JSON**. At any command node, `--json` returns exactly that node's scope — no more, no less. Parent scopes are the keyed aggregate of their children's scopes; leaves return only their own data; default subcommands affect terminal output only. This feature brings the CLI implementation into compliance with that principle and adds a new flag needed for one of the rules.

### Background

An audit of the current CLI found that most of the surface already complies (all program-family commands aggregate correctly at `None`-action; file-list leaves return scoped JSON; `repo structure`, `repo packages`, `repo git-status`, `files`, `docs`, `blast-radius`, `just`, `filesystem` all compliant). The non-compliant surface is concentrated in the `repo` command tree and in one missing CLI flag.

### Concrete changes

#### 1. Fix `sniff repo --json` to aggregate (HIGH)

Today: `sniff repo --json` dispatches to the default subcommand `name` and returns `RepoIdentity` fields.

Target: `sniff repo --json` returns the aggregate of every direct `repo` child, keyed by subcommand name:

```json
{
  "name": "rusty-biscuit",
  "is-monorepo": true,
  "package-count": 63,
  "version": null,
  "language": "Rust",
  "structure": { ... },
  "packages": [ ... ],
  "package-areas": [ ... ],
  "deps": { ... },
  "worktrees": [ ... ],
  ...
}
```

Mechanism: in `args/mod.rs` `to_repo_action()` (line ~686), the `None => RepoAction::Name` mapping is human-output-only. The JSON path must take a different branch when `cli.json && repo_subcommand.is_none()` and build the aggregate by invoking every child's scope builder.

Keys are kebab-case strings matching the subcommand names users would type (e.g. `is-monorepo`, not `is_monorepo`). This makes the JSON keys round-trip back to drillable subcommands.

#### 2. Fix `sniff repo name --json` to be leaf-only (HIGH)

Today: `sniff repo name --json` serializes the full `RepoIdentity` struct.

Target: `sniff repo name --json` returns `{ "name": "rusty-biscuit" }` and nothing else.

Mechanism: the `name_outcome()` builder at `output/repo_json.rs:317` already produces the focused shape. Route the Name leaf through it. Do not serialize `RepoIdentity` directly from the leaf handler.

#### 3. Add three new repo leaves

To preserve user access to the information currently bundled into `repo name`'s output, split each into its own leaf so `repo`'s aggregate JSON has somewhere to draw from:

- `sniff repo is-monorepo` → text: `yes` / `no`; JSON: `{ "is-monorepo": true }`
- `sniff repo package-count` → text: the count as plain text; JSON: `{ "package-count": 63 }`
- `sniff repo version` → text: the version if present, blank otherwise; JSON: `{ "version": "0.1.0" }` or `{ "version": null }`

`sniff repo language` already exists and is compliant — no change.

The text-form behavior of these leaves follows existing repo-leaf conventions (`sniff repo package` etc.): single-value plain text suitable for scripting, with the standard `--no-error` / `--on-error` opt-ins where they make sense.

#### 4. Wire phase-6 commit families with focused JSON builders (MEDIUM)

Today: `sniff repo recent-commits --json`, `sniff repo source-code-changes --json`, and `sniff repo documentation-changes --json` fall through to `fallback_repo_value` at `output/repo_json.rs:287-288`, which serializes the entire `RepoInfo`. This is a Rule 4 violation (leaves leaking parent scope).

Target: each leaf returns only its own commit data, following the same focused-builder pattern as the other repo leaf families. Shape proposal (subject to refinement during implementation):

```json
{
  "period": "1w",
  "commits": [
    { "sha": "...", "subject": "...", "action": "feat", "package-area": "sniff", "files-changed": 12 },
    ...
  ]
}
```

#### 5. Add `--with-network` global flag

Today: the flag does not exist in the CLI args layer. The principle doc references it as the gate for any network-touching supplemental data — applies to **both terminal and JSON output**.

Target: add `--with-network` to the global `Cli` struct in `args/mod.rs` alongside `--json`, `--plain`, etc. No current child requires it (sniff is host detection; network usage is the exception), but adding the flag now:

- Keeps the principle doc and the CLI in lockstep
- Provides the well-known surface for any future child that opts into network supplements
- Allows the aggregator in change #1 to filter children by their network-requirement marker (children with no network needs always included; network-touching children included only when `--with-network` is set)

The flag has no positional or value form — it is a pure bool toggle. Behavior when no children currently honor it: the flag is accepted, has no observable effect, and is documented as future-proofing in `--help`.

### Out of scope

- Changing the human/terminal output of any compliant command. Only `repo` (default-subcommand dispatch) and the three commit families' JSON branches change. Their text outputs are not part of this work.
- Restructuring `RepoIdentity`. It can stay as-is internally; we only change which fields the `name` leaf surfaces.
- Adding network-touching children. This work adds the flag; the first network-touching child is a separate feature.
- Renaming any existing command or flag.

### Test considerations

- `sniff repo --json` should return an object with the kebab-case keys listed in change #1, and each top-level key must match a subcommand the user could invoke directly. Add a test that walks the keys and parses each as `sniff repo <key>` successfully.
- `sniff repo name --json` should return exactly `{ "name": "<repo-name>" }` — no other keys. Tests should assert key set, not just value.
- `sniff repo is-monorepo --json`, `sniff repo package-count --json`, `sniff repo version --json`: each returns a single-key object whose key matches the subcommand name (kebab-case).
- `sniff repo recent-commits --json` and the two sibling commit-family leaves: assert the returned object does NOT contain any of `RepoInfo`'s top-level keys outside the commit-data scope.
- `--with-network` parsing test: `sniff --with-network repo --json` parses, `cli.with_network == true`.

### Documentation

The principle is already documented at `docs/topics/json-output.md`. After implementation, the following docs need terminology/example refreshes (covered by the docs-rollout follow-up, not this spec):

- `.claude/skills/sniff/SKILL.md` — reference the principle and the `--with-network` flag
- `sniff/cli/README.md` (if it discusses output modes)
- The repo CLI's `REPO_AFTER_HELP` (`args/mod.rs:1104`) — add a JSON-modes section noting the new leaves
