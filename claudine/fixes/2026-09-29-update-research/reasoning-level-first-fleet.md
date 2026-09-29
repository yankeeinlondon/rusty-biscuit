# Reasoning level: the first fleet

Run on 2026-09-29 under revision 1 of the contract. Eight of ten providers
were researched. Every table below is produced from the documents'
frontmatter by a script, with no reading or interpretation. That is what
narrow types make possible.

## Result

| Outcome | Providers |
| --- | --- |
| Researched, validated, and stamped | Claude Code, Codex, Gemini, Goose, Kimi Code, OpenCode, Pi, Kilo |
| Rejected on the first attempt, passed on the retry | Kilo |
| Not researched | Qwen and Antigravity |

Qwen and Antigravity are assigned to OpenCode. OpenCode's plan reached its
usage limit for `glm-5.3` after Goose. The provider reports that the limit
resets at 14:47:17 on 2026-09-29, without naming a time zone.

## What the research found

### Levels

| Provider | Version | Levels, as the provider spells them | Default |
| --- | --- | --- | --- |
| Claude Code | 2.1.284 | `low`, `medium`, `high`, `xhigh`, `max`, `ultracode` | `high` |
| Codex | 0.157.1 | `none`, `minimal`, `low`, `medium`, `high`, `xhigh`, `max`, `ultra` | — |
| Gemini | 0.61.0 | `LOW`, `HIGH` | `HIGH` |
| Goose | 1.52.0 | `off`, `low`, `medium`, `high`, `max` | — |
| Kimi Code | 2.0.2, 2.1.1 | `off`, `low`, `medium`, `high`, `xhigh`, `max` | `high` |
| OpenCode | 1.18.33 | `none`, `minimal`, `low`, `medium`, `high`, `xhigh`, `max`, `thinking` | — |
| Pi | 0.87.1 | `off`, `minimal`, `low`, `medium`, `high`, `xhigh`, `max` | `medium` |
| Kilo | 7.3.45 | `instant`, `none`, `minimal`, `low`, `medium`, `high`, `thinking`, `xhigh`, `max` | — |

### How a level is chosen

| Provider | At launch | Other controls |
| --- | --- | --- |
| Claude Code | `--effort <level>`<br>`--settings {"effortLevel": "<level>"}`<br>`agents --effort <level>` | 2 environment variables, 2 session commands, 6 configuration keys, 1 request field |
| Codex | `-c model_reasoning_effort=<level>` | 3 configuration keys, 2 session commands, 1 request field |
| Gemini | `--model <model>` | 5 configuration keys |
| Goose | `--model <model>-<level>` | 6 environment variables, 2 configuration keys, 1 session command, 4 request fields |
| Kimi Code | none | 1 environment variable, 4 configuration keys, 1 session command, 2 request fields |
| OpenCode | `--variant <level>`<br>`--thinking` | 1 request field, 4 configuration keys, 1 session command |
| Pi | `--thinking <level>`<br>`--model <model>:<level>`<br>`--models <model>:<level>` | 2 configuration keys, 1 session command, 1 request field |
| Kilo | `--variant <level>`<br>`--thinking` | 3 configuration keys, 1 environment variable, 2 session commands |

### Behavior

| Provider | An unaccepted level | Warns | Level reported in | Reasoning text reaches the caller |
| --- | --- | --- | --- | --- |
| Claude Code | runs at the default | yes | session record | no |
| Codex | fails the request | yes | session record | as a summary |
| Gemini | fails the request | yes | nowhere | no |
| Goose | uses the nearest level | no | session record | in full |
| Kimi Code | fails the request | yes | session record | in full |
| OpenCode | unknown | unknown | nowhere | unknown |
| Pi | uses the nearest level | no | session record | in full |
| Kilo | unknown | unknown | session record | unknown |

### The runs

| Provider | Researcher | Time | Evidence entries | From live tests | Gaps |
| --- | --- | --- | --- | --- | --- |
| Claude Code | opencode, `zai-coding-plan/glm-5.3` | 14 min | 22 | 9 | 3 |
| Codex | claude, `sonnet` | 6 min | 16 | 6 | 9 |
| Gemini | codex, `gpt-6-luna` | 8 min | 12 | 1 | 4 |
| Goose | opencode, `zai-coding-plan/glm-5.3` | 33 min | 28 | 0 | 5 |
| Kimi Code | claude, `sonnet` | 18 min | 18 | 6 | 7 |
| OpenCode | codex, `gpt-6-luna` | 14 min | 13 | 0 | 6 |
| Pi | claude, `sonnet` | 16 min | 23 | 6 | 6 |
| Kilo | codex, `gpt-6-luna` | 10 min, then 7 min | 11 | 1 | 8 |

Goose is not installed on the research host, so its findings rest on
documentation and source.

## What this means for Claudine

1. **One effort setting is feasible.** Six of the eight providers take a
   level at launch. Five take it as a flag and a level, and differ only in
   the flag's spelling: Claude Code, Codex, OpenCode, Pi, and Kilo. Goose
   takes it as a suffix on the model name.
2. **Gemini and Kimi Code need a different route.** Gemini chooses its level
   through the model or a configuration key. Kimi Code has no launch control.
3. **A request can fail silently.** Goose and Pi use the nearest level
   without a warning, so Claudine must confirm the level from the session
   record.
4. **`docs/providers/facts/claude.yaml` is wrong.** It records the flag as
   `thinking_effort` with three levels. The provider has `--effort` with
   five levels and one mode.

## What the fleet showed about the contract

Revision 1 let these through. Each is corrected in revision 2, which is
staged as `reasoning-level-revision-2.patch` and not applied.

| Observed | Cause | Revision 2 |
| --- | --- | --- |
| Kilo's first attempt was rejected for `kilo export <sessionID>` | The locator pattern forbids spaces, and a command has them | A command is recorded in `command`, one argument per entry |
| Kilo lists 576 models and Pi lists 96 | Nothing limited the list or allowed grouping | At most 40 entries, and a model may be a pattern such as `gpt-6-*` |
| `ultracode`, `thinking`, and `instant` sit among the levels out of order | A mode is not a point on a scale, and the contract could not say so | `outside_scale`, listed after the scale |
| Each researcher invented its own gap subjects | The subject was free text | A gap names its `area` from a closed set, and optionally its `entry` |
| Claude Code's environment variables are recorded as command-line arguments | The prompt said "for a control usable at launch, give the exact arguments" | The prompt and the description say that `arguments` holds command-line arguments only |

## What the fleet showed about the process

| Observed | State |
| --- | --- |
| The fleet retried after a usage limit, which cannot succeed | Fixed: a usage limit is reported and not retried |
| A run could launch one agent while the document recorded another | Fixed: a run researches only the providers assigned to the agent it launches, and refuses a model outside the rotation |
| The relations gate is not part of the fleet | Open: it exists as a prototype, run by hand |
| A description containing a colon followed by a space makes the contract unreadable | Open: the lint should reject it |
| `yes`, `no`, and `off` are truth values to a YAML 1.1 reader | Open: Darkmatter and the generator read them as text, but other tools may not |

## What remains

| Item | Needs |
| --- | --- |
| Qwen and Antigravity | OpenCode's limit to reset, or a decision to let another researcher take them |
| Revision 2 | Applying the patch, then a second run of all ten providers |
| The relation findings below | The second run |

## Relation findings

From `prototypes/reasoning_level_relations.py`. The prototype is written for
revision 2, so its check that every `unknown` has a gap cannot judge these
revision 1 documents and is left out.

| Document | Finding |
| --- | --- |
| `claude.md` | `effort-env-var` and `max-thinking-tokens` are environment variables and carry command-line arguments |
| `claude.md` | `ultracode` follows `max` and is mapped to `very_high`, so the levels are not weakest first |
| `opencode.md` | `thinking` follows `max` and is mapped to `high`, so the levels are not weakest first |
| `kimi.md` | A model's default is `on`, which is not a listed level |
| `kilo.md`, `pi.md` | 576 and 96 model entries, against a contract that asks only for exceptions |
| All but `pi.md` | Between one and five evidence entries support no finding |
