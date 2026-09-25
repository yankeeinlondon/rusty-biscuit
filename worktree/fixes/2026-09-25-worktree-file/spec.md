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
reviewed: true
reviewed_by: codex/default
reviewed_on: 2026-09-25
review_iterations: 0
related:
    - 2026-09-24-ux-improvements
    - 2026-09-25-list-remove-performance
---

# `.worktreeinclude` support

This is an addendum to `2026-09-24-ux-improvements`, which is already implemented. That spec requires consent before deleting any ignored entry. This addendum intentionally changes that policy: `.worktreeinclude` identifies ignored files whose new or changed contents need consent, while other ignored entries become disposable. This spec adds the convention to both ends of a worktree's life, `wt create` and `wt remove`, and replaces that ruling.

## The convention

Several worktree tools read a `.worktreeinclude` file at the repository root: [Claude Code](https://code.claude.com/docs/en/worktrees) and [Worktrunk](https://worktrunk.dev/step/) among them. `wt` adopts the shared filename and pattern-based selection convention (ruled 2026-09-25). This does not promise identical lifecycle behavior: Worktrunk copies all ignored files by default unless configured to require the include file; `wt` requires it. Removal consent is a `wt` policy, not a property of the shared convention. See the linked [Worktrunk copy documentation](https://worktrunk.dev/step/#wt-step-copy-ignored).

- **Syntax:** `.gitignore` syntax, one pattern per line.
- **Meaning:** a file belongs to the include set when it is gitignored **and** matches a `.worktreeinclude` pattern. Tracked files and untracked files that are not gitignored never belong to it: a new worktree already has tracked files from its checkout, and untracked files that are not ignored are ordinary work in progress.
- **Purpose:** these are the files a fresh checkout lacks but needs to work, typically `.env`, `.env.local`, or `config/secrets.json`. The file is committed; it lists names, not secrets.
- **Matching** uses Git's own ignore engine, never a reimplementation. From the source root, enumerate candidates with `git ls-files --others --ignored --exclude-from=.worktreeinclude -z`, then test those candidates against the standard ignore rules separately, using a batched, NUL-delimited Git operation. Combining `--exclude-standard` and `--exclude-from` in one listing does not express the required intersection. Standard rules include nested `.gitignore` files, `.git/info/exclude`, and user excludes. Preserve native path identities; never split filenames on newlines or round-trip them through lossy text. See [Git's listing options](https://git-scm.com/docs/git-ls-files).
- **Pattern behavior:** blank lines, comments, escaping, root anchoring, directory patterns, ordered negation, and `**/` follow [Git's ignore rules](https://git-scm.com/docs/gitignore). A negated child cannot undo a parent directory that Git has already excluded; document this with an example. Only the root `.worktreeinclude` is read, not nested include files.
- **Boundaries:** do not descend into submodules, nested repositories, nested worktrees, or linked directories, and never copy Git administrative files. Empty directories are not included files. Unsupported file kinds are reported rather than opened as regular files.

With no applicable `.worktreeinclude` (including the removal fallback below), the include set is empty: `wt create` copies nothing and `wt remove` treats every ignored entry as disposable.

An existing empty file deliberately selects nothing. A missing file permits the documented fallback; an unreadable file, directory in its place, or Git discovery failure does not mean an empty set. Creation warns and skips copying; removal aborts with exit 1 before mutation when it cannot determine the set. Do not follow a `.worktreeinclude` symlink outside the checkout.

## 1. `wt create` copies the include set

Here, **base checkout** means the main Git worktree, not the configured directory that holds linked worktrees. Resolve the source before creating the destination; if several checkouts have the fork branch, fail source selection with a warning instead of choosing arbitrarily. Existing validation of `--from`, detached HEAD, and already checked-out destination branches remains unchanged.

A new worktree gets a copy of every file in the include set of its **copy source** (ruled 2026-09-25):

- **Forked worktree:** the copy source is the worktree where the fork source is checked out: the `--from` branch, or the current branch by default (the branch-source behavior specified in `2026-09-24-ux-improvements`). When that branch has no worktree, the copy source is the base checkout. A worktree forked from `feat/theme` gets `feat/theme`'s `.env`, which may differ from the base checkout's.
- **Reused branch** (the branch already exists, so nothing is forked): the copy source is the base checkout.
- The `.worktreeinclude` that applies is the one in the copy source's working tree.

**Copying:**

- Files are copied, not linked: each worktree's `.env` can then change independently, and removing one worktree never touches another's files.
- Where the filesystem supports copy-on-write cloning (APFS on macOS, Btrfs and XFS on Linux, ReFS on Windows), a file is cloned instead of byte-copied, to avoid duplicating file data; elsewhere it is a normal copy. Verify the actual copy API during implementation. Attempt cloning per file where supported; unsupported operations or cross-volume copies fall back to byte copying. Cloning still requires directory traversal, and the required initial content digest reads every copied regular file, so this spec promises no constant-time copy of a large folder.
- File permissions are preserved (a `0600` `.env` stays `0600` on Unix), including while temporary files exist. Native Windows permissions must not be represented as Unix mode guarantees. A symbolic link is copied as a link with its target text unchanged, never followed, even if dangling or pointing outside the worktree. If link creation is unavailable, warn and skip it; never substitute a copy of its target. Do not traverse Windows junctions or other directory reparse points.
- Directories are created as needed. Any existing destination entry, including a dangling link, is left alone and reported as skipped. Also skip paths tracked in the destination index, even when absent on disk, and reject ancestor file/link conflicts. Check both source and destination ancestors without following links. Never overwrite a concurrently created destination entry. Publish a completed temporary copy without replacement, and remove partial temporary files on failure.
- Nothing is copied from outside the copy source's working tree.

**Copy record:** after copying, `wt create` stores one record per new worktree in the library's user-cache store, next to the fork-origin records. For each copied file it holds the repository-relative path, size, and content digest, plus the copy source's path. Store entry kind and link target for links. Regular-file digests are BLAKE3 through `biscuit-hash`, streamed with `biscuit_hash::blake3_hash_reader` so no file is loaded into memory whole. Metadata and digest describe the completed destination copy, not a later reread of the source. If concurrent mutation prevents a consistent baseline, warn and leave that entry without a trusted baseline. The record is keyed by the worktree's canonical path, deleted when `wt remove` removes the worktree, and pruned using a successful worktree listing during `wt list`. Extend the existing maintenance step; the current fork-origin prune only knows branches and cannot establish which worktree records are stale. An unavailable checkout or failed listing must not be treated as proof of deletion. It answers the question removal needs: has this worktree's copy changed since it was copied?

**Record lifecycle:** use a versioned, atomically written record per worktree so concurrent creates cannot overwrite another worktree's record. Bind it to the Git worktree registration as well as its canonical path, and invalidate any old record before reusing a destination path; a recreated checkout must not inherit deletion permission from its predecessor. Record only successful copies, never destination conflicts or failed copies. Delete the record after directory removal succeeds, even if a later branch or remote deletion fails. Record cleanup failures warn and do not reverse a successful removal.

Missing, corrupt, incompatible, or identity-mismatched records are untrusted. Report unreadable/corrupt records, then use the no-record policy below. Record write failures preserve creation's exit 0 and warn that later removal may require consent. A cache is not a backup and contains no file contents. Restrict record access to the current user where supported.

**Report:** after the existing "created" message, one line lists what was copied and from where, e.g. "Copied from `feat-theme`: .env, config/secrets.json". Nothing is printed when the include set is empty. Send reports and warnings to stderr through biscuit-terminal renderable components, escaping filenames and branch names as literal text. Preserve stdout exclusively for the existing shell protocol. Wrap long lists and escape control characters; never print secret contents.

**Failures:** a file that cannot be copied does not undo the worktree. `wt create` still succeeds (exit 0), and prints a warning naming each file that was not copied and why, so the caller knows the new worktree may be missing configuration.

## 2. `wt remove` requires consent for new, changed, or unknown included files

This replaces the all-ignored-files consent rule in `2026-09-24-ux-improvements` (ruled 2026-09-25). Ignored entries split into two groups:

| Ignored entry | Needs consent? |
|---|---|
| In the removed worktree's include set, and **new or changed**: not in the copy record, or changed since it was copied | **Yes**, like a dirty file: interactive asks (default No); non-interactive needs `--force-worktree` and otherwise exits 3 |
| In the include set and **unknown** because comparison failed | **Yes**, under the same consent policy; unverifiable handoff content refuses deletion |
| In the include set and **unchanged since it was copied** | No: the copy has no local edits. This does not prove another copy still exists: if the source was later deleted or edited, this may be the last copy of the original contents, and it is still removed without a question (ruled 2026-09-25, Decision 8). The README says so and never claims deletion "loses nothing". |
| Not in the include set (build output such as `target/`, caches) | No |

- **Which `.worktreeinclude`:** the removed worktree's own file. Only when absent, use the base checkout's file, evaluated relative to the removed worktree with that worktree's standard ignore rules. An empty file suppresses fallback. Never read the copy source's rules for this decision. Changing or deleting patterns can intentionally make previously protected ignored files disposable; the README must explain this consequence.
- **Changed since copied** is decided against the copy record, never against the copy source's current file. Comparing with the source would make an untouched copy look changed whenever someone edits the source's `.env` after the worktree was created. For each included file:
    - Not in the record: new.
    - In the record, size differs from the recorded size: changed, without being read. A size difference proves a change, so this shortcut can never wrongly clear a file.
    - In the record, same size: its digest (`biscuit_hash::blake3_hash_reader`) is compared with the recorded digest, whatever the file's size. Modification time is never used: tools such as `cp -p`, `rsync -t`, and archive extraction restore it after writing, so it cannot prove a file unchanged (ruled 2026-09-25, Decision 7).
    - A symbolic link is compared by its target text and entry kind, without reading its referent. A kind change is changed. Missing destination entries require no consent because removal cannot delete their contents. Read failures are unknown, never unchanged: they require explicit consent, and a handoff that cannot verify content must refuse.
- **Worktrees without a copy record** (created before this feature, or by another tool): each included file is compared by digest with the copy source's current file, the source being found as at creation from the branch's fork-origin record (the worktree where the fork parent is now checked out, else the base checkout; a branch with no record, or whose parent was deleted, uses the base checkout). A file absent at the source is new. Require a distinct source checkout: comparing a worktree with itself must never establish permission to delete it. A missing, unreadable, or ambiguous source makes the included file unknown and requires consent. Compare regular files by full digest regardless of size and links by target and kind; do not write this comparison back as a creation baseline.
- Other removal behavior from `2026-09-24-ux-improvements` is unchanged: dirty files still need consent, and the safety tiers for the branch are untouched.

**Report** (the initial removal inventory):

- Included files that need consent are listed with the dirty files, each marked "new", "changed", or "unknown", and named in the confirmation question.
- Ignored entries that do not need consent get one dim summary line naming their top-level entries, e.g. "Also deletes ignored files: target/, .DS_Store". It names entries and does not count files, since counting would walk every file in folders like `target/`.

**Handoff fingerprint:** the move-first handoff fingerprint covers the included files that need consent (path, whether new or changed, and content digest) instead of the whole set of ignored entries. A new included file or a detectable edit between the two runs makes the second run refuse; a new build artifact does not. Preserve the existing dirty-file and full-index coverage. Also bind the effective include-file location, presence, and contents, and the baseline record identity and contents, into the handoff. For a no-record comparison, retain the chosen source identity and comparison result so a changed source cannot silently change the second run’s consent decision. Re-resolve membership during verification; rules or baseline changes must refuse even when a protected file would disappear from the selected set. The handoff format must be versioned so older records cannot bypass this check. Content or policy changes refuse with exit 3 before deletion, even with `--force-worktree`; discovery errors exit 1. Existing token expiry, branch checks, remote protections, and exit 4 environment checks remain unchanged.

Included files in the handoff follow the same rule as removal: a size difference is a change, and every same-size included file is hashed in full, at any size (Decision 7). Share this comparison contract between creation records, removal classification, and handoff verification. The size-and-modification-time shortcut in `2026-09-25-list-remove-performance` applies only to dirty files, whose fingerprint spans the minute between the two handoff runs.

## Out of scope

- `wt list`'s worktree dot stays git's dirty status; a changed included file does not change it.
- No command to re-copy the include set into an existing worktree (Worktrunk's `wt step copy-ignored`). A later spec can add one if it is wanted.
- Patterns that name large ignored directories (for example `node_modules/`) are honored. Cloning reduces data duplication, but removal hashes every same-size included file and finding matches makes git walk the ignored directory, so both stay proportional to its size. Documentation recommends naming files.

## Acceptance criteria and testing

Levels follow the `rust-testing` skill: L1 against temporary repositories, L2 through `biscuit-test-harness` without taking focus. Every criterion holds on macOS, Linux, native Windows, and WSL2.

1. **Include set:** only files that are both gitignored and matched are selected; a tracked file or a non-ignored untracked file matching a pattern is never selected; no applicable `.worktreeinclude`, after the removal fallback, means an empty set; `**/` patterns into a wholly ignored directory follow git's behavior (L1).
2. **Create:** a forked worktree receives the fork source's included files, falling back to the base checkout; a reused-branch worktree receives the base checkout's; permissions and symbolic links are preserved; the copy record lists every copied file with its size and digest; the copied-files line and the failure warning appear as ruled; a failed copy leaves the worktree created and exits 0 (L1). Use injected copy-operation results to prove clone success, unsupported fallback, and failure cleanup at L1; use a real filesystem check for supported cloning, with unavailable capability reported explicitly. Writing either copy must leave the other unchanged.
3. **Remove:** every row of the consent table gives the documented prompt, refusal, and exit code, interactive and non-interactive; an unchanged copied `.env` and a `target/` folder are removed without a question; a new included file or an edit detectable under the stated comparison rule triggers the question, including an edit of a file of any size that keeps its size and modification time; editing the copy source's `.env` after creation does not make the worktree's untouched copy ask; an included file whose size differs from the record is classified changed without being read, and every same-size included file is hashed regardless of size or modification time; a worktree without a copy record falls back to comparing with the copy source; the summary line names the other ignored entries; the copy record is deleted with the worktree (L1, prompts in L2).
4. **Handoff:** editing any included file between the two runs, including an edit that keeps its size and restores its modification time, makes the second run refuse; creating a new build artifact does not (L1).
5. **Failure and boundary coverage:** prove empty versus missing rules, unreadable rules, ordered negation, global and nested excludes, filenames with spaces/newlines, destination index conflicts, dangling links, ancestor links/junctions, nested repositories, partial-copy cleanup, corrupt records, cache write failure, path reuse, unavailable sources, and self-comparison refusal at L1. Scope platform-specific assertions to native capabilities and test the documented warning fallback where a capability is unavailable.
6. **Consent verification:** changing the effective include rules or copy record between handoff runs refuses; an unchanged build artifact does not invalidate approval. Dirty-file and staged-index changes still refuse. Test that failures leave files, registration, and branches intact. Exercise representative prompts through the existing focus-preserving L2 harness; use pure policy tests for the full matrix.
7. **Cost and reporting:** count file reads to prove that a size difference is classified without reading and that same-size included files are always read; do not use a wall-clock threshold for it. Verify stderr reports and unchanged stdout protocol, including partial success. Run the package area's `just test`, `just test-l2`, and `just lint` during implementation, with fixture-owned configuration/cache and no live remote services. No CI scope change is required by this spec review.

## Packages

- `worktree` (library): include-set resolution through git, copy-source resolution (from the fork-origin record and `git worktree list`), copying (cloning where supported) at creation, the copy record in the user-cache store, the changed/new comparison, and the revised [Inventory](../../lib/src/remove/inventory.rs), which determines whether local contents need deletion consent and fingerprints approved work. Extend [create_worktree](../../lib/src/worktree.rs), which creates the checkout, to return structured copy outcomes for library callers and CLI reporting.
- `worktree-cli`: the create report and warning, and the remove report's list, marks, and summary line.
- Docs: the worktree README and the `worktree` skill describe `.worktreeinclude`, including that an unchanged copy may be the last copy of its original contents (Decision 8).

## Decisions

1. Ruled 2026-09-25: `wt` reads `.worktreeinclude`, the file Claude Code and Worktrunk use, not a `wt`-specific name.
2. Ruled 2026-09-25: the copy source is the worktree of the fork source (the `--from` branch or the current branch), falling back to the base checkout; a reused branch copies from the base checkout.
3. Ruled 2026-09-25: an included ignored file needs consent only when it is new or changed since it was copied; an unchanged copy and every non-included ignored entry are removed without a question. This replaces the all-ignored-files consent rule in `2026-09-24-ux-improvements`.
4. Ruled 2026-09-25: ignored entries that need no consent are named in one dim summary line, without file counts.
5. Ruled 2026-09-25: "changed since copied" is decided against a copy record written by `wt create` (path, size, and digest per file), not against the copy source's current file. Worktrees without a record fall back to comparing with the copy source. How files are compared is Decision 7, which replaces this decision's earlier 1 MiB size-and-modification-time rule.
6. Ruled 2026-09-25: copying clones files where the filesystem supports copy-on-write, and falls back to a normal copy elsewhere.
7. Ruled 2026-09-25 (was open question 1): included files are never judged unchanged from metadata. A size difference is a change without reading; every same-size included file is hashed in full with `biscuit_hash::blake3_hash_reader`, at removal and in the handoff. Modification time is not recorded or used.
8. Ruled 2026-09-25 (was open question 2): creation-baseline semantics stay. An included file unchanged since it was copied is removed without a question even if it may be the last copy of its original contents; the README documents this.
