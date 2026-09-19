## Context

::block when="ctx.name"

- The user's name is {{ctx.name}}.

::end-block

- Today's date is {{ctx.today}}

::block when="ctx.timezone"

- You are in the {{ctx.timezone}} timezone with a {{ctx.timezone_offset}} from UTC
- This timezone IANA name is {{ctx.timezone_iana}}

::end-block

::block when="ctx.location"

- You are located in {{ctx.location}}

::end-block

::block when="ctx.repo"
### Repo Info

- You are in the "{{ctx.repo}}"" repo

::end-block
::block when="ctx.is_monorepo"

- this repo is a monorepo, with the following package areas:

    {{ as_unordered_list(ctx.package_areas) }}

- the programming language(s) found in this monorepo are: {{ as_csv(ctx.programming_languages_in_repo) }}

::end-block

### Host

- The host is running **{{ctx.os}}** on version **{{ctx.os_version}}** (_{{ctx.cpu_arch}}_)
- There is **{{ctx.memory_total}}** memory on this system and **{{ctx.cpu_cores}}** cores

::block when="ctx.gpu"

- The host also has the following GPU resources: **{{ctx.gpu}}**

::end-block

