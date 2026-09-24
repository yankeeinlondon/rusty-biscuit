---
description: |-
    This prompt **implements** a _feature_ or _fix_ specification and _plan_. It can do this two ways:

    1. looping phase-by-phase of the plan
    2. a single agent acting as an _orchestrator_

    If you prefer that a single agent be used (as an orchestrator) then you must set the `orchestrator=true` flag to true.

    Also, this prompt requires that the caller provide a reference to either the `spec` or `plan` and that both artifacts (_plan_ and _specification_) exist in the same directory.

    > **Note:** if there is a `design.md` file in the feature/fix's directory then this will be assumed to
    > to be a technical design file which compliements the spec file.

    > **Note:** if the plan doesn't specify a `start_phase` frontmatter property then we will initially assume **1** but as soon as the implementation's log file has been created it will become the more formal record
$schema:
    - plan: file(required;eager;match(**/*plan*.md))
      orchestrator: boolean -> boolean flag which when set to `true` will use an orchestrator strategy
      spec: file -> the specification providing the functional contract for the plan
      log: file -> the implementation's log file
    - spec: file(required;eager;match(**/*spec*.md))
      orchestrator: boolean -> boolean flag which when set to `true` will use an orchestrator strategy
      plan: file -> the plan for implementing the specification
      log: file -> the implementation's log file
# Spec File (will attempt to auto complete it from the plan file if not passed in directly)
spec: |-
    {{
        !spec && plan && file_exists(plan) && file_exists(parent_dir(plan) + '/spec.md')
            ? parent_dir(plan) + '/spec.md'
            : null
    }}
# Plan File (will attempt to auto complete it from the spec file if not passed in directly)
plan: |-
    {{
        !plan && spec && file_exists(spec) && file_exists(parent_dir(spec) + '/plan.md')
            ? parent_dir(spec) + '/spec.md'
            : null
    }}
# Tech Design File (completely optional)
design: |-
    {{
        !design && spec && file_exists(spec) && file_exists(parent_dir(spec) + '/design.md')
            ? parent_dir(spec) + '/design.md'
            : null
    }}
# The implementation's log file
log: |-
    {{
        parent_dir(plan) + "/implementation-log.md"
    }}
phase: |-
    {{
        frontmatter(log, "phase_completed") + 1 || frontmatter(plan, "starting_phase") || 1
    }}
initialize:
    - stack:
        - when: "!orchestration"
          action:
              - stderr: "looping phase-by-phase chosen over single agent orchestrating"
              - message: "will implement `{{ parent_dir(spec) }}` using a phase-by-phase loop"
              - proxy: "./implement-plan.md"
        - when: "orchestration"
          action:
              - stderr: "implementing feature plan as an orchestrator"
              - message: "will implement plan for `{{ parent_dir(spec) }}` as orchestrator"
start:
    - message: "starting "
---
# Implement Feature as an Orchestrator

**Plan:** {{plan}}
**Spec:** {{spec}}
::block when=design
**Technical Design:** {{design}}
::end-block

You are responsible for acting as an orchestrator to complete the plan listed above:

::file ../_test-tiers.md

::block when="!design"
- this plan was based on implementing everything described in the specification file listed above
- you must implement this plan and you are not done until you've successfully completed all phases but also implemented the requirements found in the specification document
::end-block
::block when=design
- this plan was based on implementing everything described in the specification file (along with the supporting technical design) listed above
- you must implement this plan and you are not done until you've successfully completed all phases but also implemented the requirements found in the two design documents above
::end-block
- if the plan didn't already require it you must also ensure that documentation is up to date:
