---
description: a snippet that be used to produce a good suggestion on what skills an agent should use
$schema:
    spec: file -> if the calling prompt is focused around a _specification_ file then that's a strong hint on a possible skill to look for
    explicit: boolean -> whether the skills 
    current_skill: string -> a suggested skill based on where the caller called Claudine (where appropriate)
current_skill: |-
    {{
        ctx.current_package && has_local_skill(ctx.current_package)
            ? '- this agent was started in the **' + ctx.current_package + '** package and there is an agent skill "' + ctx.current_package + '" that you should consider using:\n\t' + local_skill_description(ctx.current_package)
            : ctx.current_package_area && has_local_skill(ctx.current_package_area)
                ? '- this agent was started in the **' + ctx.current_package_area + '** package area and there is an agent skill "' + ctx.current_package_area + '" that you should consider using:\n\t' + local_skill_description(ctx.current_package_area)
            : has_local_skill(ctx.repo)
                ? '- this agent was started in the **' + ctx.repo + '** repo and there is a local agent skill "' + ctx.repo + '" that you should consider using:\n\t' + local_skill_description(ctx.repo)
            : null
    }}
skill_map: "<< skills-map.yaml"
---

{{current_skill}}
::loop skill_map where="i => find(ctx.self_content, i.trigger || i)"


::end-loop
