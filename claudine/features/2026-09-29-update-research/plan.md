---
total_phases: 6
start_phase: 2
packages:
  - claudine-catalog-types
  - claudine-gen
  - claudine
  - claudine-cli
completed_phases:
  - 0
  - 1
blocked_phases:
  - phase: 4
    on: the sequence and lifecycle improvements of 2026-09-27-sequence-improvements
  - phase: 6
    on: 2026-09-28-content-policy
---

# Plan: research refresh

This plan carries out [spec.md](spec.md). It is written to be run by a
session that has not seen the work so far.

## Before anything else

Read these, in this order:

1. [spec.md](spec.md): what was decided and why.
2. `.claude/skills/claudine/research-contracts.md`: how a topic is built and
   run, the limits of the schema grammar, and the traps. **Every trap in that
   file cost a failed run.**
3. [reasoning-level-findings.md](reasoning-level-findings.md): what a
   finished topic looks like.
4. `claudine/docs/research/reasoning-level/`: the reference topic. Its five
   files are the pattern every other topic follows.

## Execution contract

- Decisions in the spec marked "Decided" are binding.
- The spec's open questions, Q1 to Q5, each carry a recommendation. Where a
  phase depends on one, the phase says so and names the default. Do not
  carry out a default that deletes or renames something until the author has
  answered; do the rest of the phase and report.
- Never commit. The author commits.
- Never run `cargo fmt`.
- Do not build while a fleet is running. A build replaces the binary the
  fleet is using.
- A fleet run spends the allowance of three researchers. Pilot on one
  provider before every full run, and do not repeat a full run to fix a
  contract flaw that a pilot would have shown.
- Tell the author before starting a full run, with the number of documents
  and the expected time. At 10 to 25 minutes a document and three
  researchers, ten documents take about an hour.
- A phase ends at "implementation complete, ready for review".

## Starting baseline

Verified on 2026-09-29.

| Item | State |
| --- | --- |
| Rosters use `sequence:` | Done |
| `just research <topic>` | Works; ran reasoning level for ten providers in 62 minutes |
| Every fleet prompt reports `err.msg` | Done |
| Reasoning level | Ten documents, revision 2, confirmed by both gates |
| Generator tests | 194 of 194 |
| Generator drift check | Clean |
| Steering | Live contract is revision 4. A draft of revision 5 is in `2026-09-29-steering-pipeline`, under `research-schema/` |

Confirm the baseline before Phase 2:

```sh
cd claudine
cargo nextest run -p claudine-gen
cargo run -q -p claudine-gen -- check
cd docs/research/reasoning-level && python3 _relations.py [a-z]*.md
```

## Researchers

| Agent | Model | Effort, topic feeds code generation | Effort, any other topic |
| --- | --- | --- | --- |
| OpenCode | `zai-coding-plan/glm-5.3` | provider default | provider default |
| Claude | `sonnet` | high | low |
| Codex | `gpt-6.1-sol` | medium | low |

The prompt and the recipe hold one effort per researcher today. Steering and
non-interactive sessions feed code generation, so Phases 2 and 3 need no
change. Phase 5 must add the lower effort for topics that do not.

## Phase 2: steering

Steering is the first topic whose research the generator reads. Changing its
contract changes generated code.

### 2.1 Settle the contract

The draft is in `features/2026-09-29-steering-pipeline/research-schema/`:
`_schema.yaml`, `_types.yaml`, `_fleet.md`, and worked examples for Codex and
Pi whose values were transcribed from the shipped adapters.

1. Replace the draft's own copies of `identifier`, `version`, `pointer`,
   `os`, `launch_mode`, and `evidence` with references to
   `docs/research/_types.yaml`, and take `agent`, `model`, and
   `reasoning_effort` from the shared researcher types.
2. Depends on Q5. Default: replace `yes`, `no`, and `off` in every closed set
   of the draft, and teach the lint to reject them. Six sets use `yes` and
   `no`.
3. Apply what reasoning level taught. Give a gap a closed `area`. Bound every
   list a researcher might fill from a catalog. Record a command as a list of
   arguments, never as one string.
4. Run the lint. Validate both worked examples.

### 2.2 Make the relations gate understand revision 5

`claudine providers steering check` is `gen/src/steering_check.rs`. It checks
revision 4 today.

Add, for revision 5:

- a message template uses only the placeholders the contract lists;
- every `channel_id`, `profile_id`, `mechanism_id`, and evidence reference
  names an entry that exists;
- a condition whose test is `equals` has a value;
- `cancel` is present exactly when the operation is `interrupt_then_submit`,
  and its `stop_proof` rule is then not `not_applicable`;
- an HTTP request has `target` and `expect_status`; a stream request has
  neither;
- every `unknown` has a matching entry in `gaps` or `discovery_gaps`.

Keep every revision 4 check that still applies: the 24 baseline
combinations, the case products, and the operation and support consistency.

Add a `--json` output, which the draft prompt expects.

### 2.3 Make the generator read revision 5

| File | Change |
| --- | --- |
| `catalog-types/src/steering.rs` | Types for a channel, a request, a condition, acceptance, and stop proof. `SteeringTransport` and `request_format` are replaced |
| `gen/src/steering_catalog.rs` | Read the new properties; emit them; reject a grant whose mechanism or channel holds an `unknown` |
| `lib/src/steering/generated.rs` | Regenerated, never edited |
| `lib/src/steering/eligibility/tests.rs`, `lib/src/steering/controller/tests.rs` | They construct mechanisms with the replaced fields |
| `gen/tests/l1/steering_activation.rs`, `steering_check.rs` | Fixtures in revision 5 |

Two things must survive unchanged:

- **The activation policy.** `docs/providers/steering-activation.yaml` names
  mechanism identifiers and verification identifiers. The Codex grants must
  still resolve after the refresh.
- **The verification records.** Codex has seven and Pi has seven, each a live
  test. A refresh keeps them. The prompt says so; check that it happened.

The pinned hashes depend on Q1. `gen/tests/l1/drift.rs` compares fifteen
generated files with `gen/tests/fixtures/generated-artifact-baseline.json`
and fails when generated code changes. Default: update the hashes by hand,
say so in the report, and leave retiring the test to the author.

### 2.4 Pilot, then run

1. Move the contract and prompt into `docs/research/steering/`. Change
   `schema_revision` to 5 in the contract, the prompt, and the check.
2. Pilot on Codex: `just research steering only=codex`. Compare the
   researched `channels` and `mechanisms` with the worked example. They
   describe the same protocol, so a difference is a finding about the
   contract or the prompt.
3. Correct the contract and the prompt from the pilot. Pilot again when the
   change is more than wording.
4. Tell the author, then run the fleet: `just research steering`.
5. Regenerate and verify:

   ```sh
   cargo run -q -p claudine-cli --bin claudine -- providers generate --yes
   cargo nextest run -p claudine-gen
   just test
   ```

### 2.5 Record

- A findings document for steering, with its tables produced by a script as
  reasoning level's are.
- `docs/topics/steering-activation.md` and
  `docs/topics/provider-metadata.md`, where they describe the contract.
- The spec's phase table.

### Done when

- Ten steering documents carry revision 5 and the fleet's stamp, and pass
  both gates.
- The generator reads them, its tests pass, and its drift check is clean.
- The Codex grants still resolve, and Codex steering behaves as before:
  `just test-real real_codex_app_server::` passes.
- No production code reads a sentence from steering research.

What this phase does not do: build the delivery pipeline. That is
`2026-09-29-steering-pipeline`, which starts from this phase's output.

## Phase 3: non-interactive sessions

This topic has the most free text of any contract, and it holds what a
managed launch needs: the arguments that start each interface, what shows
readiness, and how a request from the provider is answered when no person is
present.

Follow Phase 2's steps: contract, relations, generator, pilot, run, record.

Points particular to this topic:

- The generator reads `execution_interfaces` and `execution_selection`. Find
  every reader before renaming a property.
- The steering contract's `launch.execution_interface` names an interface
  recorded here. The two must agree, and the relations gate checks it.
- Codex translates the options of `codex exec` into parameters of its
  `thread/start` request. The rules are in
  `cli/src/commands/wrap/exec/codex_app_server/launch.rs`. The contract needs
  a way to record that translation: each option, whether it passes through,
  becomes a parameter, is handled by Claudine, or has no equivalent.

## Phase 4: the driver, as prompts

Blocked until the sequence and lifecycle improvements land.

1. Examine those improvements against the spec's table, "What a prompt-driven
   refresh needs from Claudine". Report which needs are met, and which are
   still missing.
2. For a need still missing, propose the smallest addition to Claudine. Two
   matter most: firing lifecycle events against a stand-in agent, so a prompt
   can be tested without a model, and a plan that reports what would run
   without acting.
3. Build `docs/research/topics.yaml` and the prompts that read it. The
   rotation moves there from the three places it lives today.
4. Replace `just research` with `just refresh-research`, which enters the
   prompts.

## Phase 5: the remaining topics

In this order: signals and agent errors; agent models, model config, agent
CLI, and agent logging; then the rest, with ACP last.

- A topic that does not feed code generation runs Claude and Codex at low
  effort.
- A topic whose summary exists gets its comparison tables from a script.
  Depends on Q3. Default: generate the tables, and leave the prose as it is.
- The agent-errors check, `claudine providers agent-errors check`, already
  exists. Keep its rules.

## Phase 6: freshness by declaration

Blocked on `2026-09-28-content-policy`. Each research document declares when
it goes stale, including a fingerprint of every file of its contract. Until
then a contract change is marked by changing `schema_revision`.

## Defects to raise as their own fixes

The spec lists them under "Defects found outside this fix". None blocks this
plan. Each belongs to the package area named beside it.

## Progress

### Phase 0: repairs (done 2026-09-29)

Rosters renamed to `sequence:` in Claudine and three other package areas.
Three recipes name the `claudine` binary. Twenty fleet prompts report
`err.msg`. Three pre-roster files under `acp/` deleted.

### Phase 1: reasoning level (done 2026-09-29)

| Run | Contract | Confirmed | Notes |
| --- | --- | --- | --- |
| Pilot | Revision 1 | Claude Code | Proved the lifecycle events; found five defects in the prompt |
| First | Revision 1 | 8 of 10 | OpenCode reached its usage limit; five weaknesses found in the contract |
| Second | Revision 2 | 10 of 10 | First attempt for every provider; 62 minutes |

Added along the way: the relations gate, cover for a researcher at its
limit, and a guard so that parallel runs read only their own documents.

After the second run, Codex's model changed from `gpt-6-luna` at high effort
to `gpt-6.1-sol` at medium. No document was researched again for this.
