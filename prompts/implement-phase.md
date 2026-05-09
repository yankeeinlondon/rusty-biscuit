---
phase: 1
total_phases: ""
plan: ""
success: 
    say: "Phase {{phase}} of the plan was implemented"
    message: "✅ phase **{{phase}}** (_of {{total_phases}}_) of the plan `{{plan}}` successfully completed  in the **{{ctx.current_package_area}}** _package area_."
failure:
    say: "Ran into problems implementing phase {{phase}} of the plan!"
    message: "❌️ phase {{phase}} (_of {{total_phases}}_) failed in the plan `{{plan}}`"
loop:
    until: "phase > total_phases"
    actions: increment(phase)
---
::block when="total_phases"
# Implement Phase {{phase}} of {{total_phases}}
::end-block
::block when="!total_phases"
# Implement Phase {{phase}}
::end-block

Your task is to implement phase {{phase}} of the plan found in {{plan}}.

::block when="memory"
> **NOTE:** for context you should read the lessons learned discovered in earlier stages of this plan. You will find these lessons learned in memory/{{memory}}.md. 
::end-block

- use the '{{ctx.current_package_area}}' skill during this implementation

You are done when:

- all functionality defined in phase {{phase}} has been implemented
- all tests are passing (using `just test` in the {{ctx.current_package_area}} package area)
- all lints are passing (using `just lint` in the {{ctx.current_package_area}} package area)
- You must set the following Frontmatter properties:
    - `source_files_during_phase_{{phase}}` should be set to all source code files which were created or updated during this phase of the implementation; put an empty list (e.g., `[]`) if none
    - `docs_updated_during_phase_{{phase}}` should be set to all documentation files which were updated during this phase of the implementation; put an empty list (e.g., `[]`) if none
    - `docs_created_during_phase_{{phase}}` should be set to all documentation files which were created during this phase of the implementation; put an empty list (e.g., `[]`) if none
    - `skills_files_updated_during_phase_{{phase}}` should be set to all agent skill files which were updated during this phase of the implementation; put an empty list (e.g., `[]`) if none
    - if this is a monorepo, then include `packages` as a list of packages in the monorepo which were touched by the implementation in phase {{phase}}
::block when="memory"
- Once all Frontmatter has been set to the plan file ({{plan}}), consider if there was anything surprising or novel that you discovered during this phase that would be valuable to know in future stages. If there is, then add a H2 heading `## Phase {{phase}}` to the end of the file `memory/{{memory}}.md`
::end-block

**IMPORTANT:** 

- use the '{{ctx.current_package_area}}' skill during the implementation
- Do NOT commit or stage files to git, this will be done as a separate process.
- Report a summary of what you did including all the source files you changed.
- You do not need to run tests across the entire monorepo as this will take far too long. Only 
- once the implementation is complete update the '{{ctx.current_package_area}}' if there were any notable changes needed in this skill
- you are running as part of a non-interactive session! Do not ask the user for feedback or permissions as they can not answer!
