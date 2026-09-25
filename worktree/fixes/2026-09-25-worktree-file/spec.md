---
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
status: draft-spec
reviewed: false
review_iterations: 0
related:
    - 2026-09-24-ux-improvements
---

# `.worktreeinclude` support

This is an addendum to `2026-09-24-ux-improvements`, which is already implemented. That spec never covered the `.worktreeinclude` convention, and its ruling on ignored files (its Decision 18: every ignored entry needs consent before `wt remove` deletes it) is wrong without it. This spec adds the convention to both ends of a worktree's life, `wt create` and `wt remove`, and replaces that ruling.

## The convention

Several worktree tools read a `.worktreeinclude` file at the repository root: [Claude Code](https://code.claude.com/docs/en/worktrees) and [Worktrunk](https://worktrunk.dev/step/) among them. `wt` adopts the same file with the same meaning, so one committed file serves every tool (ruled 2026-09-25):

- **Syntax:** `.gitignore` syntax, one pattern per line.
- **Meaning:** a file belongs to the include set when it is gitignored **and** matches a `.worktreeinclude` pattern. Tracked files and untracked files that are not gitignored never belong to it: a new worktree already has tracked files from its checkout, and untracked files that are not ignored are ordinary work in progress.
- **Purpose:** these are the files a fresh checkout lacks but needs to work, typically `.env`, `.env.local`, or `config/secrets.json`. The file is committed; it lists names, not secrets.
- **Matching** uses git's own ignore engine (for example `git ls-files --others --ignored --exclude-from=.worktreeinclude`, intersected with the standard gitignore rules), never a reimplementation of the pattern syntax. The Claude Code documentation describes how `**/` patterns reach into wholly ignored directories; `wt` follows git's behavior and the same rule.

With no `.worktreeinclude`, the include set is empty: `wt create` copies nothing and `wt remove` treats every ignored entry as disposable.

## 1. `wt create` copies the include set

A new worktree gets a copy of every file in the include set of its **copy source** (ruled 2026-09-25):

- **Forked worktree:** the copy source is the worktree where the fork source is checked out: the `--from` branch, or the current branch by default (item 6 of `2026-09-24-ux-improvements`). When that branch has no worktree, the copy source is the base checkout. A worktree forked from `feat/theme` gets `feat/theme`'s `.env`, which may differ from the base checkout's.
- **Reused branch** (the branch already exists, so nothing is forked): the copy source is the base checkout.
- The `.worktreeinclude` that applies is the one in the copy source's working tree.

**Copying:**

- Files are copied, not linked: each worktree's `.env` can then change independently, and removing one worktree never touches another's files.
- Where the filesystem supports copy-on-write cloning (APFS on macOS, Btrfs and XFS on Linux, ReFS on Windows), a file is cloned instead of byte-copied, so even a large included folder is copied almost instantly without duplicating its data; elsewhere it is a normal copy. Whether `std::fs::copy` already clones on each platform is verified during implementation, not assumed.
- File permissions are preserved (a `0600` `.env` stays `0600` on Unix). A symbolic link is copied as a link, never followed.
- Directories are created as needed. A file that already exists at the destination is left alone; a brand-new worktree normally has none.
- Nothing is copied from outside the copy source's working tree.

**Copy record:** after copying, `wt create` stores one record per new worktree in the library's user-cache store, next to the fork-origin records. For each copied file it holds the repository-relative path, size, modification time, and `biscuit-hash` content digest, plus the copy source's path. The record is keyed by the worktree's canonical path, deleted when `wt remove` removes the worktree, and pruned for worktrees that no longer exist during the save `wt list` already performs. It answers the question removal needs: has this worktree's copy changed since it was copied?

**Report:** after the existing "created" message, one line lists what was copied and from where, e.g. "Copied from `feat-theme`: .env, config/secrets.json". Nothing is printed when the include set is empty.

**Failures:** a file that cannot be copied does not undo the worktree. `wt create` still succeeds (exit 0), and prints a warning naming each file that was not copied and why, so the caller knows the new worktree may be missing configuration.

## 2. `wt remove` consents only for included files that would lose something

This replaces Decision 18 of `2026-09-24-ux-improvements` (ruled 2026-09-25). Ignored entries split into two groups:

| Ignored entry | Needs consent? |
|---|---|
| In the removed worktree's include set, and **new or changed**: not in the copy record, or changed since it was copied | **Yes**, like a dirty file: interactive asks (default No); non-interactive needs `--force-worktree` and otherwise exits 3 |
| In the include set and **unchanged since it was copied** | No: deleting an unchanged copy loses nothing |
| Not in the include set (build output such as `target/`, caches) | No |

- **Which `.worktreeinclude`:** the removed worktree's own file. Without one, the base checkout's.
- **Changed since copied** is decided against the copy record, never against the copy source's current file. Comparing with the source would make an untouched copy look changed whenever someone edits the source's `.env` after the worktree was created. For each included file:
    - Not in the record: new.
    - In the record, file under 1 MiB: its digest is compared with the recorded digest. Small files are always read, so an edit that keeps the size and modification time cannot slip through.
    - In the record, file of 1 MiB or more: if its size and modification time match the record, it is unchanged without being read; otherwise its digest is compared with the recorded digest. This keeps removal fast when a pattern names a large folder.
    - A symbolic link is compared by its target.
- **Worktrees without a copy record** (created before this feature, or by another tool): each included file is compared by digest with the copy source's current file, the source being found as at creation from the branch's fork-origin record (the worktree where the fork parent is now checked out, else the base checkout; a branch with no record, or whose parent was deleted, uses the base checkout). A file absent at the source is new.
- Everything else in item 3 of `2026-09-24-ux-improvements` is unchanged: dirty files still need consent, and the safety tiers for the branch are untouched.

**Report** (step 1 of item 3's flow):

- Included files that need consent are listed with the dirty files, each marked "new" or "changed", and named in the confirmation question.
- Ignored entries that do not need consent get one dim summary line naming their top-level entries, e.g. "Also deletes ignored files: target/, .DS_Store". It names entries and does not count files, since counting would walk every file in folders like `target/`.

**Handoff fingerprint:** the move-first handoff fingerprint (item 3) covers the included files that need consent (path, whether new or changed, and content digest) instead of the whole set of ignored entries. A new or edited included file appearing between the two runs makes the second run refuse; a new build artifact does not.

## Out of scope

- `wt list`'s worktree dot stays git's dirty status; a changed included file does not change it.
- No command to re-copy the include set into an existing worktree (Worktrunk's `wt step copy-ignored`). A later spec can add one if it is wanted.
- Patterns that name large ignored directories (for example `node_modules/`) are honored. Cloning and the copy record keep copying and comparing fast; finding matches still makes git walk the ignored directory, so that step stays proportional to its size. Documentation recommends naming files.

## Acceptance criteria and testing

Levels follow the `rust-testing` skill: L1 against temporary repositories, L2 through `biscuit-test-harness` without taking focus. Every criterion holds on macOS, Linux, native Windows, and WSL2.

1. **Include set:** only files that are both gitignored and matched are selected; a tracked file or a non-ignored untracked file matching a pattern is never selected; no `.worktreeinclude` means an empty set; `**/` patterns into a wholly ignored directory follow git's behavior (L1).
2. **Create:** a forked worktree receives the fork source's included files, falling back to the base checkout; a reused-branch worktree receives the base checkout's; permissions and symbolic links are preserved; the copy record lists every copied file with its size, modification time, and digest; the copied-files line and the failure warning appear as ruled; a failed copy leaves the worktree created and exits 0 (L1). On a copy-on-write filesystem, a copied file shares its data with the source (L1 where the host filesystem supports it).
3. **Remove:** every row of the consent table gives the documented prompt, refusal, and exit code, interactive and non-interactive; an unchanged copied `.env` and a `target/` folder are removed without a question; a new or edited included file triggers the question, including a small file edited without changing its size or modification time; editing the copy source's `.env` after creation does not make the worktree's untouched copy ask; a large recorded file with matching size and modification time is not read; a worktree without a copy record falls back to comparing with the copy source; the summary line names the other ignored entries; the copy record is deleted with the worktree (L1, prompts in L2).
4. **Handoff:** editing an included file between the two runs makes the second run refuse; creating a new build artifact does not (L1).

## Packages

- `worktree` (library): include-set resolution through git, copy-source resolution (from the fork-origin record and `git worktree list`), copying (cloning where supported) at creation, the copy record in the user-cache store, the changed/new comparison, and the revised `Inventory` consent rule and fingerprint in `remove/inventory.rs`.
- `worktree-cli`: the create report and warning, and the remove report's list, marks, and summary line.
- Docs: the worktree README and the `worktree` skill describe `.worktreeinclude`.

## Decisions

1. Ruled 2026-09-25: `wt` reads `.worktreeinclude`, the file Claude Code and Worktrunk use, not a `wt`-specific name.
2. Ruled 2026-09-25: the copy source is the worktree of the fork source (the `--from` branch or the current branch), falling back to the base checkout; a reused branch copies from the base checkout.
3. Ruled 2026-09-25: an included ignored file needs consent only when it is new or changed since it was copied; an unchanged copy and every non-included ignored entry are removed without a question. This replaces Decision 18 of `2026-09-24-ux-improvements`.
4. Ruled 2026-09-25: ignored entries that need no consent are named in one dim summary line, without file counts.
5. Ruled 2026-09-25: "changed since copied" is decided against a copy record written by `wt create` (path, size, modification time, digest per file), not against the copy source's current file; files under 1 MiB are always hashed, larger ones only when their size or modification time differs. Worktrees without a record fall back to comparing with the copy source.
6. Ruled 2026-09-25: copying clones files where the filesystem supports copy-on-write, and falls back to a normal copy elsewhere.
