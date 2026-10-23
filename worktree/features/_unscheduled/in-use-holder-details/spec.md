---
status: draft
created: 2026-10-13
area: worktree
---

# Identify the Processes Behind a Paused Removal

## Problem

When `wt remove` pauses because another process is using the checkout, each
holder is shown by PID, name, executable, and the evidence that tied it to the
checkout. That is often not enough to decide what to stop:

- a shell script shows as `bash` or `sh`, not as the script it runs
- nothing says how long the process has been running, or whether it was
  leaked (its parent exited)
- a script's loop that starts a fresh child every second lists children whose
  PIDs are gone by the time the user reads them
- the user has to assemble the command that ends the holders themselves

## Prior Art

`main` solved this for the old refusal, which only checked working
directories (`f06c6d15e`, "name each program blocking a removal by command
line and age"). That refusal listed each program once with:

- its PID, its age (`started 3 days ago`), and `orphaned` when its parent had
  exited
- its full command line, cut at 160 characters with `…`, falling back to the
  executable name when the OS hides it
- a count, not a list, of the processes a listed program started
- a last line that ends them all: `kill …` on Unix, `Stop-Process -Id …` on
  Windows

`feat/filesystem-watchers` replaced that refusal with the Sniff path-usage
report (exit 3, "Removal paused"), so the merge of `main` into that branch
kept the new report and dropped this message. This spec carries the ideas
forward.

## Decided (2026-10-23)

The usage check no longer pauses anything. `wt remove` lists the processes
using the worktree in its report, before the question (`In use: …`), and
removes the worktree if the user agrees; the completion repeats them as a
warning. The facts this spec asks for belong in that report section
(`holders_markup` in `worktree/cli/src/commands/remove/usage.rs`), where they
help the user decide whether to answer yes. The "command that ends them all"
line still helps there, as a suggestion. The problem statement below still
describes the paused-removal message it was written against.

## Scope

- carry the same four facts into the holder lines of the report's `In use` section
- `Holder` today has no command line, start time, or parent PID; decide
  whether Sniff's path-usage query supplies them or `worktree` looks them up
  afterward, keeping the existing rule that a holder whose identity is
  uncertain never borrows facts from a reused PID
- fold child holders into a listed parent, as the old message did
- update `docs/cli/remove.md` and the README's remove section
