---
$schema:
    phase: number(required)
    total_phases: number(required)
    plan: file(required)
phase: 1
dir: "$(dirname '{{plan}}')"
area: "{{ctx.current_package_area == 'root' ? ctx.current_package || '' : ctx.current_package_area}}"
pass_icon: "{{ _loop_is_last ? '✅' : '🧑‍💻' }}"
start:
    message: "🎬  starting the implementation of phase **#{{phase}}** of `{{ctx.current_package_area}}/{{plan}}`"
success: 
    say: "Phase {{phase}} of the plan in the {{area}} package area, was implemented successfully"
    message: "{{pass_icon}} phase **{{phase}}** (_of {{total_phases}}_) of the plan `{{area}}/{{plan}}` successfully completed"
blocked:
    message: "💥  phase **{{phase}}** (_of {{total_phases}}_) was **blocked** because it has shell commands which were not approved!"
failure:
    say: "Phase {{phase}} of a plan in the {{area}} package area, ran into problems!"
    message: "❌️  phase {{phase}} (_of {{total_phases}}_) failed in the plan `{{area}}/{{plan}}`"
loop:
    until: "phase > total_phases"
    action: "increment(phase)"
---
::block when="total_phases"
# Implement Phase {{phase}} of {{total_phases}}
::end-block
::block when="!total_phases"
# Implement Phase {{phase}}
::end-block

Your task is to implement phase {{phase}} of the plan found in '@{{area}}/{{plan}}'.

::block when="memory"
> **NOTE:** for context you should read the lessons learned discovered in earlier stages of this plan. You will find these lessons learned in memory/{{memory}}.md. 
::end-block

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

## Be Efficient in Testing/Building

- when building or testing, make sure to only build/test the _specific packages_ or package area you are working; not the entire monorepo (this will take too long)
- The session was started in the "{{area}}" package area and so that's very likely an area you'll be focused on however, 
- most plan's will have a `packages` Frontmatter property which will explicitly state which packages are being mutated in this plan
- use to to ensure that you're being efficient while testing and building

**IMPORTANT:** 

::block when="area"
- use the '{{area}}' skill during the implementation
::end-block
- Do NOT commit or stage files to git, this will be done as a separate process.
- Report a summary of what you did including all the source files you changed.
- You do not need to run tests across the entire monorepo as this will take far too long. Only 
- once the implementation is complete update the '{{ctx.current_package_area}}' if there were any notable changes needed in this skill
- you are running as part of a non-interactive session! Do not ask the user for feedback or permissions as they can not answer!
