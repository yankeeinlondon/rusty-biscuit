---
phase: 1
plan: ""
success: 
    say: "Phase {{phase}} of the plan was implemented"
failure: 
    say: "Ran into problems implementing phase {{phase}} of the plan!"
---
# Implement Phase {{phase}}

Your task is to implement phase {{phase}} of the plan found in {{plan}}.

- use the '{{ctx.current_package_area}}' skill during this implementation

You are done when:

- all functionality defined in phase {{phase}} has been implemented
- all tests are passing (using `just test` in the {{ctx.current_package_area}} package area)
- all lints are passing (using `just lint` in the {{ctx.current_package_area}} package area)

**IMPORTANT:** 

- use the '{{ctx.current_package_area}}' skill during the implementation
- Do NOT commit or stage files to git, this will be done as a separate process.
- Report a summary of what you did including all the source files you changed.
- You do not need to run tests across the entire monorepo as this will take far too long. Only 
- once the implementation is complete update the '{{ctx.current_package_area}}' if there were any notable changes needed in this skill
- you are running as part of a non-interactive session! Do not ask the user for feedback or permissions as they can not answer!
