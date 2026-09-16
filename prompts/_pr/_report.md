## The PR Report File

Every stage of the PR flow shares its state through one report file:

- **`{{report}}`**

Claudine's lifecycle hooks read this file's **frontmatter** to decide what to tell the caller and which prompt runs next. The frontmatter is therefore a contract: use exactly the property names and values below, and replace stale values from a previous run rather than leaving them behind.

| Property | Type | Meaning |
|---|---|---|
| `status` | `pending` \| `passed` \| `local_failed` \| `error` \| `pr_opened` | where the flow stands (see below) |
| `branch` | string | the branch being proposed |
| `head` | string | the full SHA that was pushed or tested |
| `base` | string | the branch the pull request targets |
| `commits` | string[] | `<short sha> <subject>` for every commit in `origin/{{base}}..HEAD` |
| `summary` | string | one or two sentences a person can take in from a phone notification or a spoken message; no Markdown tables, no raw logs |
| `failures` | object[] | one entry per failing test or gate (only when `status` is `local_failed`) |
| `issue` | string | a self-contained description of the problem for a fixing agent (only when `status` is `local_failed`) |
| `fix_requested` | boolean | whether the caller asked for the failures to be fixed (set only by the triage stage) |
| `pr_url` | string | the pull request URL (set only by the open stage) |

`status` values:

- `pending` — set by the flow before the agent starts; an agent must never leave it here
- `passed` — local testing passed and the branch is on the remote
- `local_failed` — local testing ran and one or more gates failed; the push was blocked
- `error` — local testing could not produce a verdict (for example the hook crashed before any test ran, the remote rejected the push for a non-test reason, or the tree was in a state the flow cannot push)
- `pr_opened` — the pull request exists and CI/CD has been handed the work

Each `failures` entry has:

```yaml
- gate: "L1 claudine-cli"          # the gate name as the hook printed it
  test: "shipped_prompt_contract::shipped_prompts_have_parseable_schemas_and_expressions"
  attribution: branch               # branch | working-tree | base | environment | unknown
  commit: "61f085043"               # the introducing commit, when attribution is `branch` and it was found
  confidence: high                  # high | medium | low
  cause: "one sentence naming the root cause, not the symptom"
```

The body of the report (below the frontmatter) is free-form Markdown for the detailed write-up.
