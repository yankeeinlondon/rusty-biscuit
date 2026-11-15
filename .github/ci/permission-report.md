---
# The pull-request comment `permission-scope` posts when a change alters the
# repository's committed `permissions`. CI composes this file from the BASE
# commit with `darkmatter/lib/examples/permission_widening.rs --render`, which
# supplies `report` from `scripts/ci/permission_scope.py decide`. Every value
# that comes from the repository arrives as a literal token, so it is shown
# verbatim and never read as template syntax.
$schema:
  rule: "{ form: string(required), code: string(required) }"
  shell_changes: "{ changed: boolean(required), before_count: number(required), before_default: boolean(required), added: rule[](required), removed: rule[](required), excluded: rule[](required), unexcluded: rule[](required) }"
  read_changes: "{ changed: boolean(required), before_count: number(required), before_default: boolean(required), added: string[](required), removed: string[](required), excluded: string[](required), unexcluded: string[](required) }"
  network_change: "{ changed: boolean(required), before: string(required), after: string(required) }"
  policy: "{ title: string(required), scope_note: string(required), shell: shell_changes(required), file_read: read_changes(required), network: network_change(required) }"
  unreadable: "{ input: string(required), detail: string(required) }"
  report: "{ verdict: string(required), sources: string[](required), policies: policy[](required), unreadable: unreadable[](required) }(required)"
---
::block when="report.verdict == 'widens'"
_This pull request **widens** what Darkmatter and Claudine may do without asking, during composition, on every clone of this repository._
::end-block
::block when="report.verdict == 'narrows'"
_This pull request **narrows** what Darkmatter and Claudine may do without asking, during composition, on every clone of this repository._
::end-block
::block when="report.verdict == 'unknown'"
_This pull request changes a default that this check cannot read, so it cannot tell whether Darkmatter and Claudine may do more without asking. Review the files under "Could not be compared" yourself._
::end-block

These are **not** the permissions of an AI agent (Claude Code, Codex, …) working in this repository. They only govern what Darkmatter (`md compose`) and Claudine do while composing a document:

1. **Shell commands** in `::shell`, `::shell-block`, frontmatter `$( … )`, and lifecycle actions. A command that matches a rule runs without asking; any other command asks first. Dangerous commands (`rm`, `dd`, `git push --force`, …) are always refused, whatever the configuration says. A pre-approved command can do anything that command does, including writing files.
2. **Schema-slot file reads.** A `{{{ … }}}` inside a schema may read a file to fill itself in. Matching files are read without asking, and nothing here ever writes. A document's own reads (`load_data`, `frontmatter()`, `::file`) are not affected by this setting.
3. **Network.** Either every URL may be fetched without asking, or none may (the default). A single host can still be allowed for one run with `--allow host=…`.

These rules are read from these files at the repository root:

::map report.sources "s => s"
- {{ s }}
::end-map

## Changes

::map report.policies "p => p"
### {{ p.title }}

::block when="p.scope_note"
_{{ p.scope_note }}_
::end-block

::block when="p.shell.changed"
#### Shell commands

::block when="p.shell.before_count == 0"
Before this change there were no shell rules, so every command asked first.
::end-block
::block when="p.shell.before_count == 1"
Before this change there was **1** shell rule.
::end-block
::block when="p.shell.before_count > 1"
Before this change there were **{{ p.shell.before_count }}** shell rules.
::end-block

::block when="p.shell.added"
Added:

::map p.shell.added "r => r"
::block when="r.form == 'prefix'"
- Commands **starting with** {{ r.code }} will run without asking
::end-block
::block when="r.form == 'exact'"
- The {{ r.code }} command (exact command, no variation) will run without asking
::end-block
::block when="r.form == 'template'"
- Commands **shaped like** {{ r.code }} will run without asking; each `{{{ … }}}` placeholder accepts only a value of its type
::end-block
::end-map
::end-block

::block when="p.shell.unexcluded"
No longer excluded, so another rule may now let them run without asking:

::map p.shell.unexcluded "r => r"
::block when="r.form == 'prefix'"
- Commands **starting with** {{ r.code }}
::end-block
::block when="r.form == 'exact'"
- The {{ r.code }} command (exact command, no variation)
::end-block
::block when="r.form == 'template'"
- Commands **shaped like** {{ r.code }}
::end-block
::end-map
::end-block

::block when="p.shell.removed"
Removed, so these will now ask first:

::map p.shell.removed "r => r"
::block when="r.form == 'prefix'"
- Commands **starting with** {{ r.code }}
::end-block
::block when="r.form == 'exact'"
- The {{ r.code }} command (exact command, no variation)
::end-block
::block when="r.form == 'template'"
- Commands **shaped like** {{ r.code }}
::end-block
::end-map
::end-block

::block when="p.shell.excluded"
Now excluded, so these always ask first, even when another rule allows them:

::map p.shell.excluded "r => r"
::block when="r.form == 'prefix'"
- Commands **starting with** {{ r.code }}
::end-block
::block when="r.form == 'exact'"
- The {{ r.code }} command (exact command, no variation)
::end-block
::block when="r.form == 'template'"
- Commands **shaped like** {{ r.code }}
::end-block
::end-map
::end-block
::end-block

::block when="p.file_read.changed"
#### Schema-slot file reads

::block when="p.file_read.before_default"
Before this change these were the defaults: common text and data files (`md`, `txt`, `csv`, `json`, `yaml`, `toml`, …) anywhere in the repository.
::end-block
::block when="!p.file_read.before_default && p.file_read.before_count == 0"
Before this change no file was pre-approved.
::end-block
::block when="!p.file_read.before_default && p.file_read.before_count == 1"
Before this change there was **1** file rule.
::end-block
::block when="!p.file_read.before_default && p.file_read.before_count > 1"
Before this change there were **{{ p.file_read.before_count }}** file rules.
::end-block

In a pattern, `&` is the repository root and `./` is the folder of the schema doing the read.

::block when="p.file_read.added"
Added, so a schema slot may read these without asking:

::map p.file_read.added "f => f"
- Files matching {{ f }}
::end-map
::end-block

::block when="p.file_read.unexcluded"
No longer excluded, so a schema slot may now read these without asking when another rule allows them:

::map p.file_read.unexcluded "f => f"
- Files matching {{ f }}
::end-map
::end-block

::block when="p.file_read.removed"
Removed, so a schema slot will now ask before reading these:

::map p.file_read.removed "f => f"
- Files matching {{ f }}
::end-map
::end-block

::block when="p.file_read.excluded"
Now excluded, so a schema slot always asks before reading these, even when another rule allows them:

::map p.file_read.excluded "f => f"
- Files matching {{ f }}
::end-map
::end-block
::end-block

::block when="p.network.changed"
#### Network

::block when="p.network.after == 'allow'"
- Changed from **no URLs** to **every URL** fetched without asking
::end-block
::block when="p.network.after != 'allow'"
- Changed from **every URL** to **no URLs** fetched without asking
::end-block
::end-block
::end-map

::block when="report.unreadable"
### Could not be compared

::map report.unreadable "u => u"
- {{ u.input }}: {{ u.detail }}
::end-map
::end-block

## When this takes effect

For anyone who checks out a commit that contains it: today that is this branch, and everyone on `main` once it is merged. Your own `~/.darkmatter.yaml` can still exclude anything, and an exclusion always wins.

These settings help people with good intentions avoid running something destructive or exposing something sensitive by accident. They are not a defense against a hostile repository.
