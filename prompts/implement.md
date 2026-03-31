---
topic: ""
plan: ""
plan_file: "@{{ctx.current_package_area || env.PACKAGE_AREA}}/features/{{topic}}/{{plan || "plan.md" }}"
log_file: "@{{ctx.current_package_area || env.PACKAGE_AREA}}/features/{{topic}}/implement-phase-{{phase || 1}}.md"
pre_checks:
    - file_exists: "@{{ctx.current_package_area || env.PACKAGE_AREA}}/features/{{topic}}/{{plan || "plan.md" }}"
    - dir_exists: "@{{ctx.current_package_area || env.PACKAGE_AREA}}/features/{{topic}}"
---
## Task

You are in the **rusty-biscuit** repo. We are working in the **{{ctx.current_package_area}}** package area.

- your responsibility is to fully implement **phase {{phase}}** of the plan below
- you should append your progress to the implementation log file: {{log_file}}
    - Log entries should include a timestamp (HH:MM:SS); use local time not UTC

## Plan

::file {{plan_file}}
