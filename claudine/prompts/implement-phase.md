---
phase: 1
total_phases: ""
plan: ""
memory: ""
success:
    stderr: "Phase {{phase}} of {{plan}} has been implemented"
    message: "Phase {{phase}} of {{plan}} has been implemented"
failure:
    message: "Phase {{phase}} of {{plan}} failed to complete"
---
::block when="total_phases"
# Implement Phase {{phase}} of {{total_phases}}
::end-block
::block when="!total_phases"
# Implement Phase {{phase}}
::end-block

Your task is to implement phase `{{phase}}` of the plan found in `{{plan}}`.

::block when="memory"
Read `memory/{{memory}}.md` before implementation and preserve any lessons that affect this phase.
::end-block

## Requirements

- Use the `{{ctx.current_package_area}}` skill during implementation.
- Implement all functionality defined for phase `{{phase}}`.
- Keep changes scoped to the phase.
- Do not commit or stage files.
- Run `just test` in the `{{ctx.current_package_area}}` package area.
- Run `just lint` in the `{{ctx.current_package_area}}` package area.
- Update the plan frontmatter after implementation:
    - `source_files_during_phase_{{phase}}`: all source code files created or updated in this phase, or `[]`
    - `docs_updated_during_phase_{{phase}}`: all documentation files updated in this phase, or `[]`
    - `docs_created_during_phase_{{phase}}`: all documentation files created in this phase, or `[]`
    - `skills_files_updated_during_phase{{phase}}`: all skill files updated in this phase, or `[]`
    - `packages`: monorepo packages touched by this phase

::block when="memory"
If this phase surfaces a durable lesson that would help later phases, append it under `## Phase {{phase}}` in `memory/{{memory}}.md`.
::end-block

## Closure

- Report what changed, including every source file created or updated.
- Report test and lint results.
- If tests or lints cannot be run, explain the blocker.
- Do not ask the user for feedback or permission; this is a non-interactive session.
