# `wt remove` facts and warnings (`remove::facts`, `remove::submodules`)

What the single confirmation question shows. `facts::gather(&Sources)` makes
one pass before anything changes and returns `Facts`: the `Target` at the
path, ordered `Warning`s (lock first), `Note`s (not risks, no `--force`
needed), an optional `Blocker` (Windows R15 only), the `git status`
`Inventory` (the questioned set for the late-change line) and the one usage
report. `Facts::needs_force()` is "any warning" (N8): a non-interactive run
refuses without `--force` when it is true. User-facing wording is in
`worktree/docs/cli/remove.md`; the CLI renders the list in
`cli/src/commands/remove/report.rs`.

## Rules that bite

- **One usage query, one `git status`.** `gather` never queries twice; the
  report is kept for later enrichment. A failed query is a
  `UsageIncomplete` warning, except a query that failed because the checkout
  vanished, which is `Target::Vanished` (R17, "already removed").
- **On a very large checkout the usage query runs out of budget.** No
  tree index is supplied, so Sniff walks the whole checkout inside its one
  2 s budget. Measured on macOS: a 500,000-entry checkout needs about 3.5 s,
  so the walk exhausts the budget, no process is enumerated, and the
  question always carries `UsageIncomplete` (a non-interactive run then
  needs `--force`). Raising the deadline or supplying an index changes how
  long the question takes to appear; it is a product decision, not a test
  tweak.
- **Only a real miss warns (`facts::missed`).** Discovery unavailable, the
  budget run out, or a mechanism `Failed`/`NotAttempted` is
  `UsageIncomplete`. A mechanism the platform never supports (macOS FSEvents
  subscriptions, polling watchers) and `Partial` coverage (processes of
  other users) are on every run on a normal host: counting them made every
  non-interactive removal on macOS refuse.
- **The caller's own shell is not a program using the worktree.** A holder
  whose PID is `Caller::parent` and whose evidence only stands in a
  directory (`WorkingDirectory`, or an `OpenHandle` on a directory: that is
  how Sniff sees a shell's working directory on Windows) is dropped; the
  shell-inside warning or the wrapper's move covers it. The CLI fills
  `parent` with `std::os::unix::process::parent_id` on Unix and one
  `sysinfo` lookup on Windows, which also names the shell (`Caller::shell`).
- **Included copies warn too.** `gather` runs `classify_included` after
  `git status`: copies that are new, changed, or uncomparable are
  `Warning::IncludedFiles`; rules that can't be read are
  `Warning::IncludedUnchecked` (no longer a failure before removal); the
  assessment's own messages become `Note::IncludedCheck`.
- **Inject everything through `Sources`.** Usage, mounts, repair Git, the
  caller (`cwd`, wrapper present, shell name) and `Host` (synced-folder
  roots; `Host::detect()` reads the OneDrive variables on Windows only).
  Tests never depend on the host's processes or mounts.
- **Warning `Display` leaves untrusted text raw.** Process names, lock
  reasons, paths and Git messages are not escaped in the library; the CLI
  escapes the whole rendered string. Never invent a count: a failed
  `git status` has no count.
- **A locked worktree whose directory is gone is not `prunable`.** Git never
  marks a locked entry prunable, so `availability::classify` calls it
  healthy. `gather` checks the path itself and treats it as missing; the
  lifecycle's record removal must do the same.
- **A link counts as the worktree's own checkout only when** its target's
  `.git` leads to the link's own record, that record names the link, and the
  target is neither another listed checkout (the main one included) nor an
  ancestor of one. Anything unverifiable is "not own": only the link and the
  record are deleted, and nothing behind the link is inspected.
- **Process context (R27)** is rendered in `facts::describe_process`, the one
  place a holder becomes text: Sniff's app name leads when it differs from
  the process name, then PID, start time (local `HH:MM`, with a date when not
  today), and "run from" the nearest ancestor on `KNOWN_ORIGINS`, else the
  nearest ancestor with an app name, else nothing. An open handle reads "has
  … open" unless that handle was observed writing (`HolderEvidence::writing`).

## Submodule inspector

- Lists gitlinks (mode 160000) from the worktree's own index
  (`<admin>/index`, through `git::staged_entries_of`), then each submodule's
  index, recursively. A gitlink with no `.git` in its directory is
  uninitialized and gets no entry. Two Git processes per initialized
  submodule: `status --porcelain=v1 -z --untracked-files=normal
  --ignore-submodules=dirty` and `rev-list --count HEAD --branches --not
  --remotes` (N6). Never fetches. Any failure is `Finding::Unchecked`.
- `GitDirOwner::External` is report-only (a `Note`); the deletion engine
  deletes the `.git` file, never what it points to.

## Fixture traps

- In a linked worktree, `git submodule add` puts the module's Git data in
  `<common>/worktrees/<id>/modules/<name>`, whose config does not inherit an
  identity: append `user.*` and `commit.gpgsign=false` there with
  `test_support::append_config` before committing inside the submodule.
- A local-path submodule needs `-c protocol.file.allow=always` on `submodule
  add` and on any later `submodule update`.
- Git's wording for a `.git` file that points nowhere varies by version
  ("not a git repository" vs "gitfile does not point to a valid
  repository"); assert on `fatal:`, not a phrase.
- **gix-index 0.52 panics on an index shorter than its checksum.**
  `git_metadata::staged_entries` hands any file under 32 bytes to Git, which
  reports it as corrupt. A test that "corrupts the index with junk" must not
  rely on gix to fail gracefully on other malformed shapes either.
