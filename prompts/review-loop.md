---
$schema:
    spec: file(required;eager;match(**/*spec*.md)) -> the specification whose implementation is being reviewed
    review_agent: string(required) -> the agent that writes each review
    repair_agent: string(required) -> the agent that implements the plan and each review's findings
review_agent: codex
repair_agent: claude
description: |-
    Implements the plan first when the spec is not yet marked `implemented`, then alternates
    a feature review with an implementation of its findings, committing after each repair,
    until a review is marked ready or reports a recurring finding class.
    The list is static, so the cap is the number of repair cycles written below (5);
    once a review is ready every remaining step opts out from its own `initialize`
    and launches nothing. Reviews run under `review_agent` and implementation and
    repairs under `repair_agent`; commits use the commit prompt's own default.
    Do not pass a `--<provider>` flag when running this sequence: a command-line
    provider overrides every step's agent.
fail_fast: true
sequence:
    # Implements the plan when the spec is not yet marked `implemented`. The
    # target commits each phase itself, so no stage/commit step follows it.
    - name: implement
      prompt: "@prompts/_implement/implement-plan.md"
      params:
          spec: "{{ spec }}"
          agent: "{{ repair_agent }}"
          in_loop: true
    - name: review-1
      prompt: "@prompts/_reviews/feature-review.md"
      params:
          spec: "{{ spec }}"
          agent: "{{ review_agent }}"
          in_loop: true
    - name: repair-1
      prompt: "@prompts/_implement/implement-suggestions.md"
      params:
          spec: "{{ spec }}"
          agent: "{{ repair_agent }}"
          in_loop: true
    - name: stage-1
      shell: git add .
    - name: commit-1
      prompt: "@prompts/commit.md"
      params:
          message: "implementation of the findings in review 1 of {{ parent_dir(spec) }}"
    - name: review-2
      prompt: "@prompts/_reviews/feature-review.md"
      params:
          spec: "{{ spec }}"
          agent: "{{ review_agent }}"
          in_loop: true
    - name: repair-2
      prompt: "@prompts/_implement/implement-suggestions.md"
      params:
          spec: "{{ spec }}"
          agent: "{{ repair_agent }}"
          in_loop: true
    - name: stage-2
      shell: git add .
    - name: commit-2
      prompt: "@prompts/commit.md"
      params:
          message: "implementation of the findings in review 2 of {{ parent_dir(spec) }}"
    - name: review-3
      prompt: "@prompts/_reviews/feature-review.md"
      params:
          spec: "{{ spec }}"
          agent: "{{ review_agent }}"
          in_loop: true
    - name: repair-3
      prompt: "@prompts/_implement/implement-suggestions.md"
      params:
          spec: "{{ spec }}"
          agent: "{{ repair_agent }}"
          in_loop: true
    - name: stage-3
      shell: git add .
    - name: commit-3
      prompt: "@prompts/commit.md"
      params:
          message: "implementation of the findings in review 3 of {{ parent_dir(spec) }}"
    - name: review-4
      prompt: "@prompts/_reviews/feature-review.md"
      params:
          spec: "{{ spec }}"
          agent: "{{ review_agent }}"
          in_loop: true
    - name: repair-4
      prompt: "@prompts/_implement/implement-suggestions.md"
      params:
          spec: "{{ spec }}"
          agent: "{{ repair_agent }}"
          in_loop: true
    - name: stage-4
      shell: git add .
    - name: commit-4
      prompt: "@prompts/commit.md"
      params:
          message: "implementation of the findings in review 4 of {{ parent_dir(spec) }}"
    - name: review-5
      prompt: "@prompts/_reviews/feature-review.md"
      params:
          spec: "{{ spec }}"
          agent: "{{ review_agent }}"
          in_loop: true
    - name: repair-5
      prompt: "@prompts/_implement/implement-suggestions.md"
      params:
          spec: "{{ spec }}"
          agent: "{{ repair_agent }}"
          in_loop: true
    - name: stage-5
      shell: git add .
    - name: commit-5
      prompt: "@prompts/commit.md"
      params:
          message: "implementation of the findings in review 5 of {{ parent_dir(spec) }}"
    - name: review-6
      prompt: "@prompts/_reviews/feature-review.md"
      params:
          spec: "{{ spec }}"
          agent: "{{ review_agent }}"
          in_loop: true
---

This document is a sequence. Every step names its own prompt, so an agent that
reaches this body has been launched by mistake: do nothing and report that the
`review-loop` sequence composed its body instead of a step.
