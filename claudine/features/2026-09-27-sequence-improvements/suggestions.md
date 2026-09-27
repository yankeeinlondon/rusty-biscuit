# Sequence improvements

## Context

A review/fix cycle for one feature took nine review iterations, each started
by hand. Automating it as a Claudine sequence (`prompts/review-loop.md`)
worked, but the draft needed changes to two shipped prompts, a rename of a
prompt's own frontmatter key, three dry-run cycles, and an unrolled 21-step
list to express a five-cycle loop. Each item below names the gap that caused
one of those costs, shows what had to be written, and shows what would have
been written instead.

Items 1 and 2 are structural and would have removed most of the sequence
file. Items 3 and 4 are what turned one dry-run into three. Items 5 and 6 are
quality of life.

## 1. A step-level `when:` guard

**Problem.** A sequence step has no condition of its own. The only way to make
one conditional is to put `skip` in the *target* document's `initialize`. The
loop's stop rule therefore lives inside two shipped prompts that are also run
standalone, guarded by an `in_loop` flag whose only job is to say "you are
inside a sequence". The condition belongs to the sequence, not to the prompts.

**What had to be written**, in a prompt that is also composed directly:

```yaml
# feature-review.md
initialize:
    stack:
        - when: "in_loop && previous_review && frontmatter(previous_review, 'ready') == true"
          action: [skip]
```

**Proposed:**

```yaml
sequence:
    - name: review-2
      prompt: "@prompts/_reviews/feature-review.md"
      when: "frontmatter(latest_review, 'ready') != true"
      params: { spec: "{{ spec }}" }
```

A false `when:` records the step as skipped, appends nothing to `outputs`, and
leaves the target prompt unchanged. `setup:` cannot serve: a failing setup
marks the task failed, which under `fail_fast` ends the run.

## 2. Bounded repetition of a group

**Problem.** `loop:` repeats one document. `proxy` cannot return to a document
it has visited. A sequence is a static list. A cycle of two or more documents
therefore has to be unrolled by hand. The draft is 21 steps for a cap of five,
and the cap is a sentence in the description because nothing can read it. The
`just flow` bash loop exists for the same reason.

**What had to be written:**

```yaml
sequence:
    - { name: review-1, prompt: "@prompts/_reviews/feature-review.md", params: {…} }
    - { name: repair-1, prompt: "@prompts/_implement/implement-suggestions.md", params: {…} }
    - { name: stage-1,  shell: git add . }
    - { name: commit-1, prompt: "@prompts/commit.md", params: {…} }
    # the same four lines four more times, then review-6
```

**Proposed:**

```yaml
$schema:
    max: number -> repair cycles before giving up
max: 5
sequence:
    - name: review-repair
      repeat:
          until: "frontmatter(latest_review, 'ready') == true"
          max: "{{ max }}"
      group:
          execution: serial
          tasks:
              - prompt: "@prompts/_reviews/feature-review.md"
              - prompt: "@prompts/_implement/implement-suggestions.md"
              - shell: git add .
              - prompt: "@prompts/commit.md"
```

The sequences guide lists "group `loop`" as deferred from v1; this is that
gap. The condition is checked after each repetition, as `loop:` does today, and
`max` bounds it. The `outputs` entry for the step is the concatenation of its
repetitions, in order.

## 3. Reserved overlay keys that do not shadow silently

**Problem.** Inside a sequence, `state`, `previous`, `next`, `outputs`, and
`sequence_id` are injected at the frontmatter root and outrank every other
layer. `previous` and `next` are natural names for a prompt's own keys. The
review prompt had `previous:` meaning the previous review file. Inside the
sequence it became the previous step's state object, with no warning, and
failed only when `basename()` received an object on step 5. The `loop`
parameter collided with the lifecycle key too, but that one failed loudly at
step 1.

**What happened:**

```yaml
# feature-review.md
previous: "{{ iteration > 1 ? decrement_file_index(review) : null }}"
# inside a sequence, `previous` is {id: "commit-1", name: "commit-1", index: 4, …}
```

**Proposed**, either of:

- A typed preflight error. Preflight already loads every referenced prompt,
  so it can see an authored root key that the overlay will shadow:

  ```text
  error: prompts/_reviews/feature-review.md authors `previous`, a reserved
         sequence overlay key; rename it or read the step through `seq.previous`
  ```

- A namespaced overlay, so the collision cannot happen:

  ```md
  {{ seq.previous.name }}   <!-- the step -->
  {{ previous }}            <!-- still the document's own key -->
  ```

The first is additive and could ship alone. The second is a breaking change to
every sequence that reads `{{ state }}` and would need the name-coercion rule
carried over to `seq.state`.

## 4. A distinct way for a step to end the sequence

**Problem.** A skipped step is a success, and `fail_fast` is the only early
exit. The sequence cannot distinguish "the loop reached ready" from "the loop
stopped because a review reported a recurring finding class and a human should
look". Both exit 0 with every remaining step skipped. A bash loop can exit 2
for the second case.

**Proposed:**

```yaml
# in the review prompt's success stack, or a step's teardown
- when: "frontmatter(review, 'recurrence') == true"
  action:
      - warn: "a finding class recurred; stopping for a human"
      - stop_sequence: { code: 2, reason: "recurring finding class" }
```

`stop_sequence` ends the run cleanly, records the reason in the summary, and
sets the process exit code. `fail_fast` stays reserved for real failures.
Outside a sequence the action is a typed error at parse time, like `skip`
outside `initialize`.

## 5. A dry-run that does not run `shell:` tasks

**Problem.** `--dry-run` executes `$(…)` expansions and `shell:` tasks for
real, because their output may feed composition. That is true of `$(…)`. A
`shell:` *task's* stdout only lands in `outputs`, which dry-run already leaves
empty. Rehearsing this sequence staged the working tree five times, and each
run had to be followed by `git reset`.

**Proposed:**

```text
$ claudine sequence --dry-run @prompts/review-loop.md …
 [3/21] stage-1   rendered (dry-run): would run `git add .`
```

Render the task, print the approved bytes, and push an empty entry onto
`outputs`. A `--dry-run-shell` opt-in keeps today's behavior for a sequence
that genuinely needs the side effect during rehearsal.

## 6. Staging folded into the commit step

**Problem.** Every commit in this repository goes through `prompts/commit.md`,
which commits only what is staged. Every repair therefore needs a separate
`shell: git add .` step before it, and the flow recipe has the same shape in
bash. Three steps where one was meant, and the extra one is the step that makes
dry-run unsafe.

**Proposed**, either of:

```yaml
- name: commit-1
  prompt: "@prompts/commit.md"
  params: { agent: "{{ agent }}", stage: all }
```

where `commit.md` runs `git add .` from its own `start` stack when `stage` is
set. This is a prompt change and needs no engine work.

```yaml
- name: commit-1
  prompt: "@prompts/commit.md"
  setup:
      - shell: git add .
```

with `setup:` shells exempt from dry-run under item 5.

## Evidence

The draft that motivated these items is in the working tree as
`prompts/review-loop.md`, with the `in_loop` gates in
`prompts/_reviews/feature-review.md` and
`prompts/_implement/implement-suggestions.md`, and the `previous_review`
rename in the former. `claudine sequence --dry-run` over a completed feature
passes all 21 steps, and the claudine prompt contract, provenance, acceptance,
and route-drift tests pass.
