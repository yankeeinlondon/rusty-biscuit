# Reasoning level: what the research found

Every table in this document is produced from the research documents'
frontmatter by `prototypes/reasoning_level_tables.py`, with no reading or
interpretation. That is what narrow types make possible.

## The runs

Both ran on 2026-09-29.

| | First run | Second run |
| --- | --- | --- |
| Contract | Revision 1 | Revision 2 |
| Gates | Shape | Shape and relations |
| Started with | Three commands typed by hand | `just research reasoning-level` |
| Providers confirmed | 8 of 10 | 10 of 10 |
| Rejected and retried | Kilo, once | None |
| Not researched | Qwen and Antigravity, after OpenCode reached its usage limit | None |
| Wall-clock | 52 minutes, until the limit | 62 minutes |

Time per document in the second run, in minutes:

| Researcher | Documents | Times |
| --- | --- | --- |
| Claude, `sonnet`, high effort | 3 | 6, 7, 12 |
| Codex, `gpt-6-luna`, high effort | 3 | 8, 10, 13 |
| OpenCode, `zai-coding-plan/glm-5.3` | 4 | 9, 11, 18, 24 |

## The findings

### Levels

| Provider | Version | Levels of the scale | Modes outside the scale | Default |
| --- | --- | --- | --- | --- |
| Claude | 2.1.284 | `low`, `medium`, `high`, `xhigh`, `max` | `ultracode` | `high` |
| Codex | 0.157.1 | `none`, `low`, `medium`, `high`, `xhigh`, `max` | `ultra` | — |
| Gemini | 0.61.0 | `LOW`, `HIGH` | — | — |
| Goose | 1.52.0 | `off`, `low`, `medium`, `high`, `max` | — | — |
| Kimi Code | 2.0.2, 2.1.1 | `off`, `low`, `medium`, `high`, `xhigh`, `max` | — | `high` |
| OpenCode | 1.18.33 | `none`, `minimal`, `low`, `medium`, `high`, `xhigh`, `max` | `thinking` | — |
| Qwen Code | 0.19.8 | `low`, `medium`, `high`, `xhigh`, `max` | — | — |
| Pi | 0.87.1 | `off`, `minimal`, `low`, `medium`, `high`, `xhigh`, `max` | — | `medium` |
| Kilo Code | 7.3.45 | `instant`, `none`, `minimal`, `low`, `medium`, `high`, `thinking`, `xhigh`, `max` | — | — |
| Antigravity | 1.2.12 | `low`, `medium`, `high`, `max` | `fast` | — |

### How a level is chosen

| Provider | At launch | Other controls |
| --- | --- | --- |
| Claude | `--effort <level>`<br>`--settings {"effortLevel": "<level>"}`<br>`agents --effort <level>` | 2 environment variables, 2 session commands, 6 configuration keys, 1 request field |
| Codex | `-c model_reasoning_effort=<level>` | 5 configuration keys, 2 session commands, 1 request field |
| Gemini | `--model <model>`<br>`-m <model>` | 7 configuration keys, 1 environment variable, 1 session command |
| Goose | `--model <model>-<level>` | 6 environment variables, 2 configuration keys, 1 session command, 4 request fields |
| Kimi Code | none | 1 environment variable, 1 session command, 4 configuration keys, 2 request fields |
| OpenCode | `--variant <level>`<br>`--model openai/gpt-5.2#<level>`<br>`--thinking` | 1 request field, 4 configuration keys, 1 session command |
| Qwen Code | none | 1 session command, 8 configuration keys, 1 request field |
| Pi | `--thinking <level>`<br>`--model <model>:<level>`<br>`--models <model>:<level>` | 3 configuration keys, 1 session command, 1 request field |
| Kilo Code | `--variant <level>`<br>`--thinking` | 4 configuration keys, 1 environment variable, 1 session command, 1 request field |
| Antigravity | `--effort <level>`<br>`--model <model>` | 1 configuration key, 3 session commands |

### Behavior

| Provider | An unaccepted level | Warns | Level reported in | Reasoning text reaches the caller |
| --- | --- | --- | --- | --- |
| Claude | runs at the default | yes | session record | no |
| Codex | fails the request | yes | session record | as a summary |
| Gemini | fails the request | yes | nowhere | no |
| Goose | uses the nearest level | no | session record | in full |
| Kimi Code | fails the request | yes | session record | in full |
| OpenCode | unknown | no | nowhere | no |
| Qwen Code | fails the request | yes | a command | in full |
| Pi | uses the nearest level | no | session record | in full |
| Kilo Code | unknown | unknown | session record | unknown |
| Antigravity | refuses to start | yes | a command | no |

### Research

| Provider | Researcher | Evidence entries | From live tests | Gaps |
| --- | --- | --- | --- | --- |
| Claude | opencode, `zai-coding-plan/glm-5.3` | 23 | 10 | 4 |
| Codex | claude, `sonnet` | 15 | 9 | 8 |
| Gemini | codex, `gpt-6-luna` | 13 | 1 | 4 |
| Goose | opencode, `zai-coding-plan/glm-5.3` | 29 | 0 | 5 |
| Kimi Code | claude, `sonnet` | 23 | 9 | 10 |
| OpenCode | codex, `gpt-6-luna` | 15 | 3 | 8 |
| Qwen Code | opencode, `zai-coding-plan/glm-5.3` | 21 | 6 | 4 |
| Pi | claude, `sonnet` | 25 | 7 | 6 |
| Kilo Code | codex, `gpt-6-luna` | 14 | 1 | 7 |
| Antigravity | opencode, `zai-coding-plan/glm-5.3` | 18 | 10 | 5 |

Goose is not installed on the research host, so its findings rest on
documentation and source.

## What this means for Claudine

1. **One effort setting is feasible.** Seven of the ten providers take a
   level at launch. Six take a flag and a level, and differ only in the
   flag's spelling: Claude, Codex, OpenCode, Pi, Kilo Code, and Antigravity.
   Goose takes it as a suffix on the model name.
2. **Three providers need a different route.** Gemini chooses its level
   through the model or a configuration key. Kimi Code and Qwen Code have no
   launch control.
3. **A request can fail silently.** Goose and Pi use the nearest level
   without a warning, so Claudine must confirm the level afterwards. Gemini
   and OpenCode report the level nowhere.
4. **The levels are nearly a common vocabulary.** `low`, `medium`, `high`,
   and `max` are accepted by nine providers under those names. Gemini has
   `LOW` and `HIGH` only.
5. **`docs/providers/facts/claude.yaml` is wrong.** It records the flag as
   `thinking_effort` with three levels. The provider has `--effort` with
   five levels and one mode.

## What the first run showed about the contract

Revision 1 let these through. Each is corrected in revision 2, and none
appears in the second run.

| Observed | Cause | Revision 2 |
| --- | --- | --- |
| Kilo's first attempt was rejected for `kilo export <sessionID>` | The locator pattern forbids spaces, and a command has them | A command is recorded in `command`, one argument per entry |
| Kilo lists 576 models and Pi lists 96 | Nothing limited the list or allowed grouping | At most 40 entries, and a model may be a pattern such as `gpt-6-*` |
| `ultracode`, `thinking`, and `instant` sit among the levels out of order | A mode is not a point on a scale, and the contract could not say so | `outside_scale`, listed after the scale |
| Each researcher invented its own gap subjects | The subject was free text | A gap names its `area` from a closed set, and optionally its `entry` |
| Claude Code's environment variables are recorded as command-line arguments | The prompt said "for a control usable at launch, give the exact arguments" | The prompt and the description say that `arguments` holds command-line arguments only |

## What the runs showed about the process

| Observed | State |
| --- | --- |
| The fleet retried after a usage limit, which cannot succeed | Fixed: a usage limit is reported and not retried |
| A run could launch one agent while the document recorded another | Fixed: a run researches only the providers assigned to the agent it launches, and refuses a model outside the rotation |
| The relations gate was not part of the fleet | Fixed: the fleet prompt runs it after the shape gate |
| A description containing a colon followed by a space makes the contract unreadable | Open: the lint should reject it |
| `yes`, `no`, and `off` are truth values to a YAML 1.1 reader | Open: Q5 of the spec |
| A run failed on a step it was going to skip, because another run was rewriting the document it tried to read | Fixed: a run reads only the documents it writes |
| A researcher at its usage limit left its providers unresearched | Fixed: the others cover in rotation order |
