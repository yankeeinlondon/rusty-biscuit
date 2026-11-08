---
kind: canonical-documentation
---

# Schema Slots in Darkmatter

A schema can take some of its values from data instead of spelling them out.
A `{{ … }}` **slot** in an `enum(...)` or `suggest(...)` list, or as the whole
argument of `literal`, `default`, `min`, `max`, or `pattern`, is an
expression that Darkmatter evaluates to fill that position:

```yaml
kind: schema
$schema:
    os: enum({{ load_data("../../../os.yaml", "string[]") }}; required)
```

The grammar, the result shapes each position takes, and the substitution
rules belong to the SimplifiedSchema language and are described in
[Schema Slots](../../../../simplified-schema/docs/topics/language/slots.md).
This page covers what is Darkmatter's: which expressions a slot may contain,
where it evaluates, what it may read, and how a failure is reported.

## When a slot is evaluated

Reading a schema evaluates nothing. Parsing, querying, exporting, trigger
matching, completion, and hover all see the slot as text. A slot is
evaluated only when a schema is **resolved**: every slot of the schema is
first admitted (judged without running anything), then each is evaluated,
and its values are substituted into an ordinary concrete schema.

```mermaid
flowchart LR
    A[schema text] --> B[parse: slots kept as text]
    B --> C{resolve requested?}
    C -- no --> D[passive uses: query, export, matching]
    C -- yes --> E[admit every slot]
    E -- a slot refused --> F[nothing is evaluated; positioned error]
    E -- all admitted --> G[evaluate each slot]
    G -- a slot failed --> H[positioned error, typed cause kept]
    G -- all values --> I[concrete schema]
```

Composing a document resolves its templates: the document's `$schema` and
each schema trigger whose payload holds slots and matches the document.
[Resolving in a command](#resolving-in-a-command) covers how. Which other
surfaces (`md schema validate`, `md clean`, the editor) resolve is
**planned**; until then they report a typed "unresolved" error
(`dm.schema.slot_pending`) for a schema with slots.

## Where slots may appear

Slots belong in a schema a document selects: its `$schema`, a schema file
that names, and a schema trigger's `$schema` payload. Everywhere else a slot
is refused, after imports are expanded (so a slot reached through
`Name@file` counts):

| Where | Result |
| --- | --- |
| a kind definition (a `kind:` claim of a YAML document kind) | the claim is broken: `dm.schema.slot_not_allowed_in_kind` |
| the baseline, a global catalog, an editor extension layer | `dm.schema.slot_not_allowed_in_baseline` |
| a schema trigger's `match` conditions | the trigger is invalid (`dm.schema.invalid_trigger`), its message naming the slot |

A trigger whose payload holds slots is **pending**, not invalid: matching it
reads nothing, and `md schema triggers` lists it as `pending: true`. Its
slots evaluate only for a document it matches.

## What a slot may contain

A schema describes documents, so a slot may depend only on the schema's own
context, never on the document being validated. Admission walks the whole
expression, including both branches of `?:`, every `||` operand, and every
callback body, whether or not evaluation would reach them. One forbidden
read anywhere refuses the slot, and no slot of the schema is evaluated.

| Allowed | Refused (`dm.schema.slot_forbidden_binding`) |
| --- | --- |
| `ctx.<key>` (`ctx.today`, `ctx.repo_root`, …) | `fm.*`, and any bare name such as `title` (it reads the document) |
| `env.NAME`, `env`, `env[...]` | host globals such as `loop` or `err` |
| `null`, `true`, `false`, literals | `ctx.cwd` (the launch directory) |
| a callback's own parameters, and `current.*` / `local.*` inside a callback | `ctx.self`, `ctx.last_updated`, `ctx.hash` (the document's identity) |
| | `ctx.id`, `ctx.sid` (a fresh value every run) |
| | a bare `ctx` or a computed `ctx[...]`, which could reach the keys above |
| | `current.*` / `local.*` outside a callback |

For example, `{{ ctx.today }}` is admitted, while
`{{ env.CI ? ['a'] : fm.choices }}` is refused at `fm.choices` even when `CI`
is set.

### Functions

Every expression function declares what it may do in a slot:

| Class | Functions | In a slot |
| --- | --- | --- |
| pure | `length`, `lower`, `as_csv`, `map`, `is_date`, … | allowed |
| captured | the date functions (`is_today`, `date`, `older_than`, …), `has_command`, `has_binary`, `package`, `package_area`, `ipv4`, `ipv6`, `has_agentic_cli` | allowed; they read values the request captured once |
| input-reading | `load_data`, `frontmatter` and its family, `file_exists`, `validate_schema`, `find_files`, the path functions (`absolute`, `basename`, `join`, `link`, …), `has_skill` | allowed; every read is checked against the slot's approvals |
| volatile | `recent_commits`, `predict_conflicts`, `branch_exists_on_remote`, `remote_vendor` | allowed; never reused across commands |
| rejected | `as_markdown`, `pr`, `pr_list`, `cicd`, `cicd_list`, `ping`, `ping_under`, `has_alias`, `has_builtin_function`, `has_user_function`, `can_execute` | refused (`dm.schema.slot_forbidden_function`) |

A host function registered by an embedding program is pure: its handler
sees only its arguments.

## Where a slot evaluates

A slot evaluates in the context of the file it is written in. A relative
path in `load_data("../os.yaml", …)` is resolved from the schema file's
folder, and a slot inside a named type imported from another file
(`Os@../shared/types.yaml`) resolves from that file's folder. A slot in a
document's inline `$schema` evaluates in the document's folder. The context
is derived from the command's request context; nothing is re-read from the
process.

Each slot gets its own work budget, the default iteration limits (100 000
item visits, 16 MiB of expansion). A runaway expression stops with the
ordinary `dm.iteration.visit_limit` or `dm.iteration.expansion_limit`
failure, positioned at the slot.

A slot's `load_data` reads share the command's memo and are listed in its
read set like any other read (see
[Compose Requests](../compose-requests.md#reading-the-read-set)).

## What a slot may read

Every read a slot makes is checked before it happens: a content read or a
fetch is checked before the file is opened or the URL requested, and before
the command's shared `load_data` memo is consulted, so a file the document
itself already loaded is not handed to the slot. Any other input a function
observed (a probed path, a folder listing, a contract import) is checked
before the function's result reaches the slot. A refused read cannot be
turned into a value by `try_frontmatter` or `||`.

A read happens only when something approves it. The gate decides each read
in this order:

```mermaid
flowchart TD
    R[a slot is about to read a target] --> D{a deny grant covers it?}
    D -- yes --> X[refused: dm.schema.slot_denied]
    D -- no --> A{an allow grant covers it?}
    A -- yes --> OK[read]
    A -- no --> AU{the automatic grant covers it?}
    AU -- yes --> OK
    AU -- no --> H{an approval handler is present?}
    H -- "yes, and it allows" --> OK
    H -- "yes, and it denies" --> X
    H -- "no, or no answer" --> N[refused: dm.schema.slot_approval_required]
```

### The automatic grant

A slot may read a file of its own repository that is exactly what Git has
committed, with no grant at all. All of these must hold:

1. The schema file the slot is written in lies in the command's repository
   (the repository the command found when it started). A schema anywhere
   else gets no automatic grant for any read.
2. The target lies in that same repository after symlinks are resolved, so a
   link pointing out of the repository is judged by where it points.
3. For a read of a file's content: Git reports the file clean (tracked, with
   no staged or unstaged change and no `filter` attribute such as Git LFS),
   the bytes read hash to the committed blob, and no unsaved editor buffer
   for it differs from disk.

| Target | Automatic? |
| --- | --- |
| a clean, committed file in the schema's repository | yes |
| a modified, untracked, ignored, conflicted, or LFS-filtered file | no |
| a file in another repository, or with no repository at all | no |
| an existence check (`file_exists`) of a path in the repository, tracked or not | yes |
| a `load_data` of a file that does not exist, in the repository | yes: it fails with the ordinary not-found error |
| a folder listing or a glob (`find_files`, the `find_*_index` functions) | never |
| a URL | never |

The decision is made once per target for the whole run, so a file the run
itself changes later keeps the grant it had.

### Grants

A grant allows or denies reads of one target: a file, a folder (and
everything below it), an exact URL (query included), a URL whatever its
query, or a whole site (scheme, host, and port; no subdomains). A deny
always wins: if any deny covers a read, it is refused, whatever else allows
it.

A grant lasts for one run, or is **persisted** in the user's grant store,
`<home>/.config/darkmatter/schema-read-policy` on every OS. Nothing in a
repository or workspace can grant: the store is never read from a
repository, a document folder, or a workspace setting, and no setting can
move it. It is separate from the shell command whitelist and blacklist. The
file holds one grant per line; `#` lines and blank lines are ignored:

```text
allow file /home/me/data/people.yaml
allow dir /home/me/schemas
deny dir /home/me/schemas/private
allow url https://example.com/types.yaml?v=2
allow url-any-query https://example.com/types.yaml
allow site https://schemas.example.com
```

Paths must be absolute; URLs and sites are `http` or `https`. A line that
does not parse makes the whole file unusable for the run (a skipped line
could drop a deny), and the command stops with
`dm.schema.slot_grant_store_invalid` at the file and line. A missing or empty
file grants nothing.

`--allow host=HOST` also allows slot reads of `http` and `https` URLs on that
host for the run.

### Prompts

Before a composition runs any command, its pre-flight collects the reads its
document schema's slots will make, without evaluating them. A read whose
target is written as a literal (or built from literals and pure functions),
as a URL or as an absolute or `./`/`../` path, is known up front; both
branches of `?:` count. Every known read that no grant and no automatic
grant decides is asked about **in the same batch as the shell commands,
before them**. `md compose` on a terminal asks with a numbered prompt:

```text
  ⚠  Schema Read Approval Required
  Schema:   /repo/schemas/sandbox.yaml
  Read:     load_data() reads /repo/data/os.yaml

  1 │ Allow this file for this run          (/repo/data/os.yaml; this run only)
  2 │ Always allow this file                (saves "allow file /repo/data/os.yaml" to the schema read grants)
  3 │ Allow this file's folder for this run (/repo/data; this run only)
  4 │ Always allow this file's folder       (saves "allow dir /repo/data" to the schema read grants)
  5 │ Always deny this file                 (saves "deny file /repo/data/os.yaml" to the schema read grants)
  6 │ Deny for this run                     (the slot fails; this run only)
```

A URL offers the exact URL, the URL whatever its query (only when it has
one), and the site. Ended input is "deny for this run". A read whose target
is computed (from `env`, say) is asked about when the slot evaluates, through
the same handler. Without a terminal, and for a data output (`--output
json`, …), nothing is asked and an unapproved read is
`dm.schema.slot_approval_required`; a library caller that passes no approval
handler gets the same.

A slot's URL reads go through a remote runtime of their own, which may
fetch only the hosts its grants name; a slot grant never widens what the
document's own reads may fetch.

Handing pre-flight's run-only answers to the composition that follows it in
`md compose` is **planned**; today a library caller passes them with
`SlotSession::with_run_grants`.

## Resolving in a command

One command (an `md compose` run, one library composition) resolves its
templates through one **slot session**. A composition builds its own from
the request (its data runtime, repository, `--allow host=HOST` values, home,
and approval handler) unless the caller attached one with
`ComposeOptions::with_slot_session`, which a caller running several passes
(pre-flight, a reference check, lifecycle steps, then composition) does once
so every pass shares it:

```rust,ignore
use std::sync::Arc;
use darkmatter::markdown::compose::{ComposeOptions, DataRuntime};
use darkmatter::markdown::schemas::slots::SlotSession;

let data = DataRuntime::new();
let session = Arc::new(
    SlotSession::new(data.clone())
        .with_repository_root(Some(repository_root))
        .with_home(home)
        .with_run_grants(approvals.slot_grants),
);
let options = ComposeOptions::new().with_data_runtime(data).with_slot_session(session);
```

Within the session:

- **Grants are decided once per run.** Each read's decision (automatic,
  for this run, persisted, or denied) is kept for the whole run. When the
  run's own approved step later changes a file it read, the file keeps the
  grant it had, even though it is now dirty.
- **Each template is pinned.** Every later validation in the run gets the
  same concrete schema, so the passes of one command agree.
- **A pin follows the run's own writes.** After a shell command or lifecycle
  step runs, the next use rehashes what the pinned resolution read. An
  unrelated write keeps the pin; a changed input drops it, and the template
  resolves again under the grants already decided. If that fails, the
  failure (with its `dm.data.*` code) is reported; nothing is validated
  against a stale schema.
- **A pass that must read nothing** (a reference check, a pre-flight walk)
  uses only a schema already pinned, and otherwise leaves the template for
  composition to judge.

```mermaid
flowchart TD
    A[validate against a template] --> B{pinned in this command?}
    B -- yes, inputs unchanged --> P[use the pinned schema]
    B -- no, or an input changed --> C{cached by an earlier command?}
    C -- "yes: inputs, context, and this command's grants all still hold" --> Q[use it, replay its reads, pin it]
    C -- no --> R[admit and evaluate every slot, store and pin the result]
```

Across commands in one process, resolutions are kept in memory (never on
disk) and shared when they still hold. A result is reused only if:

- the template's schema files, its slots, the request context, the
  repository, and the Darkmatter build are the same;
- every file it read is unchanged. A file whose size and modification time
  are unchanged counts as unchanged without being reread; a file edited
  within the clock's resolution without changing size is not noticed;
- every `ctx` group and `env` value it read is the same;
- the requesting command's grants allow every read it made. A result that
  needed a grant for this run only is not reused by a command without that
  grant.

A resolution that read live state (git history, a network probe), a URL, or
a folder listing is never reused by another command. Failures are kept like
results, so creating the missing file or repairing the bad one is noticed;
a failure that depends on grants (a read needing approval, a denied read)
is never kept.

A slot whose `load_data` contract names a type holding slots resolves that
type's slots too, in the same session. When that leads back to a template
being resolved, the outer slot fails with `dm.schema.slot_cycle`. Outside a
slot (in a document's own `load_data`), a contract naming a type with slots
fails as unresolved.

## Failures

A failed slot is reported at the slot, in the file it is written in (an
imported schema's own file included), with a line and column only when the
file's text is known. The cause the expression failed with is kept:

| Failure | Code | Also carries |
| --- | --- | --- |
| the slot text is not an expression | `dm.schema.slot_syntax` | |
| a refused binding or function | `dm.schema.slot_forbidden_binding`, `dm.schema.slot_forbidden_function` | the refused read's position |
| a read no approval covers, or a denied one | `dm.schema.slot_approval_required`, `dm.schema.slot_denied` | |
| a malformed data file, a contract mismatch | the `load_data` code (`dm.data.malformed`, …) | `data_location`, the position inside the data file |
| a missing or unreadable file | `dm.schema.slot_failed` | the reference fields (`construct`, `reference`, `failure`) |
| an exhausted work budget | `dm.iteration.visit_limit`, `dm.iteration.expansion_limit` | |
| a value of the wrong shape, an empty list | `dm.schema.slot_wrong_shape`, `dm.schema.slot_empty` | |
| a slot pattern the linear regex engine must not take | `dm.schema.slot_pattern_rejected` | |
| a contract that leads back to a template being resolved | `dm.schema.slot_cycle` | |
| the user's grant store is invalid | `dm.schema.slot_grant_store_invalid` | the store's file and line in the message |
| the schema's slots were not resolved for this surface | `dm.schema.slot_pending` | |

The full family of slot codes is `SLOT_DIAGNOSTIC_CODES` in
`darkmatter::markdown::schemas::slots`.
