## The PR Report File

Every stage of the PR flow shares its state through one report file, created fresh for each run:

- **`{{report}}`**

Claudine's lifecycle hooks read this file's **frontmatter** to decide what to tell the caller and which prompt runs next. The frontmatter is therefore a contract: use exactly the property names and values below, and never remove a property an earlier stage wrote.

| Property | Type | Set by | Meaning |
|---|---|---|---|
| `status` | see below | every stage | where the flow stands |
| `base` | string | the flow | the branch the pull request targets |
| `branch` | string | push | the branch being proposed; **not always the branch the flow started on** (see below) |
| `head` | string | push | the full SHA that was pushed or tested |
| `commits` | string[] | push | `<short sha> <subject>` for every commit in `origin/{{base}}..HEAD` |
| `log` | string | push | path of the captured `git push` output, when the push was blocked |
| `summary` | string | every stage | one or two sentences a person can take in from a phone notification or a spoken message; no Markdown tables, no raw logs |
| `failures` | object[] | diagnose | one entry per failing test or gate |
| `issue` | string | diagnose, triage | a self-contained briefing for the fixing agent |
| `fix_requested` | boolean | triage | whether the caller asked for the failures to be fixed |
| `staged` | string[] | fix | the repository-relative paths the fix staged for commit |
| `pr_url` | string | open | the pull request URL |

`status` values:

- `pending` — set by the flow before any agent starts; an agent must never leave it here
- `passed` — local testing passed and `branch` is on the remote at `head`
- `local_failed` — the pre-push hook ran and one or more gates failed; the push was blocked and nothing has been diagnosed yet
- `diagnosed` — every failure has a root cause and an attribution, and `failures`, `summary`, and `issue` are filled in
- `fixed` — the chosen problems are repaired, the reproducing tests pass, and the changed files are **staged but not committed**
- `error` — the stage could not produce a verdict (for example the hook crashed before any test ran, the remote rejected the push for a non-test reason, or there was nothing to propose)
- `pr_opened` — the pull request exists and CI/CD has been handed the work

### `branch` is a deliverable, not an assumption

The flow starts on whatever branch the caller happened to be on. That is usually, but not always, the branch that gets pushed: a caller on `{{base}}` with unpushed commits needs a new branch created for them. The push stage decides, records the answer in `branch`, and every later stage reads `branch` from this report instead of assuming the checkout's current branch.

Each `failures` entry has:

```yaml
- gate: "L1 claudine-cli"          # the gate name as the hook printed it
  test: "shipped_prompt_contract::shipped_prompts_have_parseable_schemas_and_expressions"
  attribution: branch               # branch | working-tree | base | environment | unknown
  commit: "61f085043"               # the introducing commit, when attribution is `branch` and it was found
  confidence: high                  # high | medium | low
  cause: "one sentence naming the root cause, not the symptom"
  fix_branch: "fix/ci-worker-budget" # where the repair belongs; omit when nothing in the repository needs repairing
```

`fix_branch` follows from the attribution. A `branch` or `working-tree` failure is repaired on `branch`. A `base` failure is not this pull request's to repair: its fix belongs on its own branch cut from `{{base}}`, so name one (for example `fix/<short-topic>`). An `environment` failure has no `fix_branch`.

The body of the report (below the frontmatter) is free-form Markdown for the detailed write-up.
