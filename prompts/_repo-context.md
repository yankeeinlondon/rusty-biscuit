---
description: provides context about the repo/monorepo that the agent is working in
packages: |-
    {{
        ctx.is_monorepo
            ? ', this monorepo has ' + length(ctx.packages) + ' packages in it:\n' + as_unordered_list(ctx.packages)
            : ''
    }}
scoping: |-
    {{
        ctx.worktree
            ? '- you are working in the `' + ctx.worktree + '` worktree of this repo and the `' + ctx.branch + '` git branch'
            : '- you are working in the `' + ctx.branch + '` git branch'
    }}
lang: |-
    {{
        ctx.is_monorepo
            ? '- the programming languages in this monorepo are: \n' + as_unordered_list(ctx.programming_languages_in_repo)
            : '- the predominate programming language in this repo is: **' + ctx.programming_language + '**'
    }}
---
You are running in the **{{ctx.repo}}** {{ ctx.is_monorepo ? "monorepo" : "repo." }}{{ packages }}

{{scoping}}
