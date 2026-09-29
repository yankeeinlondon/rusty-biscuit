---
area: claudine
status: draft-spec
created: 2026-09-29
owner: Ken Snyder <ken@ken.net>
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
reviewed: false
review_iterations: 0
implemented: false
related:
    - 2026-09-29-steering-pipeline
    - 2026-09-29-update-research
---
# Narrow Research Contracts

Status: Draft for author review.
Created: 2026-09-29

## Purpose

Provider research drives generated metadata, and generated metadata is meant
to configure shared pipelines. That only works when research records exact
values. Today most of it is prose.

This spec sets one standard for every research topic, the order in which
topics are brought up to it, and the gates that enforce it.

## What Was Found

Measured on 2026-09-29 across the 21 topic contracts and 195 research
documents:

| Finding | Measure |
| --- | --- |
| Free-text properties | Most contracts have more free-text properties than typed ones. Non-interactive sessions has about 109 |
| Property descriptions | None of the 21 contracts describes a property in the contract itself |
| Object notation | Every object is written as one quoted string |
| Validation on success | 4 of 21 fleet prompts validate the contract. 17 check only that the date changed |
| Relation checks | 2 of 21 topics have one |
| Researcher recorded | 107 of 195 documents record the model as `default` |
| Age | 18 topics were last researched between 2026-07-01 and 2026-07-14 |
| Fleets runnable | None before this work. See [The roster blocked every fleet](#the-roster-blocked-every-fleet) |

## The Standard

A research contract meets the standard when all of these hold.

### 1. Types are as narrow as the fact allows

- A value from a known set is an `enum`.
- An identifier, a version, a field path, a flag, or a token has a `pattern`.
- A number has bounds where bounds exist.
- A plain `string` is used only for text a person reads, and is `not-empty`.
- `unknown` is a member of a set wherever research may fail to establish the
  fact. A document never omits a required property to avoid answering.

### 2. Objects are named types in nested notation

Every object shape is a named type in a `_types.yaml` file, written as a YAML
mapping with one property per line. Shapes shared between topics live in
`docs/research/_types.yaml`.

A named type is closed, so a misspelled key is an error. A mapping nested
directly inside another is open and accepts any key, so it is not used.

```yaml
condition:
    field: pointer(required)@this -> Where to look in the value being tested
    test: enum(equals,present,absent; required) -> Whether the field must hold a value, merely exist, or not exist
```

### 3. Every property has a description that carries meaning

A description tells the researcher what to record and tells the reader what
the value is for. It states something the name and type do not: the purpose,
the consequence, or the rule for choosing a value.

| Weak | Meaningful |
| --- | --- |
| Sources for this mechanism | Evidence for the message shape, the delivery boundary, and the provider's answer |
| Result of the test | Result of the test; only passed can support an activation grant |

Descriptions appear in validation errors, so a rejected document tells the
researcher what each failing property means.

### 4. Lifecycle hooks communicate and validate

Every fleet prompt uses these events:

| Event | Communicates | Validates |
| --- | --- | --- |
| `initialize` | Which provider is being researched, by which agent and model, or why it was skipped | Nothing; it may not run commands |
| `start` | Whether the provider is installed on the host | Nothing |
| `success` | That the document satisfies its contract, with a link | The document was updated today; it records the assigned agent and model; it satisfies the contract's shape; it satisfies the topic's relations |
| `failure` | What failed and why, on the terminal and the messaging route | Nothing |
| `finalize` | Nothing | Retries once after a failure |

A failed validation in `success` raises an error, which turns the run into a
failure. The retry then shows the researcher the validator's output, so the
second attempt corrects the named properties.

[docs/research/reasoning-level/_fleet.md](../../docs/research/reasoning-level/_fleet.md)
is the first prompt written this way.

## Two Gates

| Gate | Checks | State |
| --- | --- | --- |
| Shape | Types, closed sets, patterns, unknown keys | `md schema validate`; works today |
| Relations | What a shape cannot express: an identifier that must name an existing entry, a property required only in some cases, an `unknown` that needs a gap entry | Hand-written for steering and agent errors only |

The relations gate is one checker configured per topic, in keeping with the
rest of this work. Each topic declares its relations in a file beside its
contract, and `claudine providers research check <topic> <slug>` enforces
them. The steering and agent-errors checkers become configurations of it.

## Researcher Rotation

Research rotates over three agents by position in the roster:

| Agent | Model | Effort |
| --- | --- | --- |
| Codex | `gpt-6-luna` | high |
| Claude | `sonnet` | high |
| OpenCode | `zai-coding-plan/glm-5.3` | provider default |

The order is chosen so that no agent researches its own provider. The
contract accepts only these agents and models, so a document that records
`default` is rejected.

**Effort cannot yet be set from a prompt.** It is passed on the command line
with a different spelling per agent, which one run over three agents cannot
do. Until Claudine has an effort setting, a fleet that must control effort
runs once per agent. The reasoning-level topic produces the facts that
setting needs.

## New Topic: Reasoning Level

`docs/research/reasoning-level/` records, per provider, the levels that exist,
every way to choose one, which models accept which levels, what happens when
an unaccepted level is requested, and how to confirm afterwards which level a
run used. It is written to this standard from the start.

## Order

Each topic follows the same steps: narrow the contract, test it against
values already known from code, pilot it on one provider, then run the fleet.

1. Reasoning level (new)
2. Steering
3. Non-interactive sessions
4. Signals and agent errors
5. Agent models, model config, agent CLI, and agent logging
6. The remaining topics, with ACP last

A topic is refreshed as soon as its contract is narrowed. Topics are not
refreshed under their current contracts.

## The Roster Blocked Every Fleet

`docs/providers.yaml` and `docs/local-runners.yaml` held their entries under
`list:`. The sequence loader rejects that key and asks for `sequence:`, while
the generator read `list:` in three places. Every fleet prompt that names the
roster failed before it started.

Both files and the generator now use `sequence:`, so one spelling serves both
readers. The generator's tests pass and its drift check is clean.

## Defects Found in Darkmatter

Each is a difference between the documentation and the parser, observed with
`md` 0.1.0:

- A `pattern(...)` that contains parentheses or a space does not parse.
- A multi-line inline object does not parse in a contract file.
- A contract file with `kind: schema` rejects a `$schema` key, so a contract
  and its types cannot share one file.
- A mapping nested directly in a contract is accepted; the documentation says
  it is an error.

## Acceptance Criteria

- Every contract passes a lint that fails on a property without a
  description, on a description that repeats another or restates the
  property's name, and on an object written as a string.
- Every fleet prompt validates shape and relations in `success` and reports
  through the events in the table above.
- No research document records an agent or model outside the rotation.
- A fleet run on a host with the roster fix starts, and a document that
  violates its contract is rejected and retried once.

## What the Pilot Showed

The reasoning-level prompt was run for Claude Code on 2026-09-29, with
OpenCode and `zai-coding-plan/glm-5.3` as the researcher.

| Hook behavior | Result |
| --- | --- |
| `initialize` reports the provider, agent, and model | Confirmed |
| `success` validates the document and reports the link | Confirmed; the document passed on the first attempt |
| `success` rejects a document that violates the contract | Confirmed with a deliberately invalid document |
| `failure` reports the reason | Confirmed after a correction, below |
| `finalize` retries once | Confirmed |
| The retry shows the researcher the validator's output | Confirmed |
| A second failure ends the run with a non-zero exit | Confirmed |

The pilot found these defects, none of which a dry run shows:

| Defect | Where | State |
| --- | --- | --- |
| A per-step `agent` expression fails the opening gate, because one agent is planned before any step exists | Claudine sequences | Open; a fleet names its agent on the command line and runs once per agent |
| A hook may not run a command whose executable is interpolated | The prompt | Fixed; the prompt uses `has_binary` |
| A document dated today was treated as current even when invalid | The prompt | Fixed; current means recent and valid |
| `err.message` is not a field; the reason printed empty | The prompt | Fixed; the prompt uses `err.msg`. 20 of the 22 fleet prompts still use `err.message` |
| `cargo run -p claudine-cli` fails because the package has four binaries | `just run-fleet-research` | Open |
| The contract could not say that a provider warns while substituting a level | The contract | Fixed; `warns` is separate from `behavior` |
| `reporting.locator` accepted a sentence | The contract | Fixed; it has a pattern, and `notes` holds the prose |

The pilot also found that `docs/providers/facts/claude.yaml` records
Claude Code's effort flag as `thinking_effort` with three levels. The
research found `--effort` with five.

## Relation to 2026-09-29-update-research

That fix builds the machinery that runs a refresh: a topic manifest, one
freshness oracle, gates, generation, and a refresh record. This spec decides
what a refresh must produce. They meet at four points.

| Point | That fix | This spec | Resolution |
| --- | --- | --- | --- |
| Researcher | Codex with `gpt-5.6-sol` at low effort for every topic | A rotation of three agents, two at high effort | The manifest's `agent` and `model` defaults carry the rotation; its wall-clock estimates are re-measured |
| Contract changes | None, to avoid a fleet-wide edit | Every contract is narrowed | Narrowing replaces that decision; typed values are what make its frontmatter diff readable |
| First full refresh | Runs under the current contracts | No topic is refreshed until its contract is narrowed | The first refresh runs topic by topic, in this spec's order |
| Freshness | The driver decides, from age and then from fingerprints of the prompt and the contract | The fleet stamps `contract_checked` after validation | The stamp is an input to the driver. A fingerprint must cover `_types.yaml` and `docs/research/_types.yaml` as well as `_schema.yaml` |

A contract changed during the pilot without a revision change, and the
already-written document became invalid while still looking current. A
fingerprint of the contract files, as that fix proposes, removes the need to
remember a revision change.

## Open Decisions

1. **Description lint.** Whether the lint is a Darkmatter feature, available
   to every contract in the monorepo, or a Claudine check.
2. **Other rosters.** Three roster files outside Claudine still use `list:`:
   in Darkmatter, biscuit-terminal, and biscuit-tui.
