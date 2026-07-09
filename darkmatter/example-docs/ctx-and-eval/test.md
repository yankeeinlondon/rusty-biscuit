# Context Variables and Darkmatter Expression Engine

## Context Variables

Many things are available on context, including:

### Host Computer

- `ctx.os`: {{ctx.os}}
  ::block when="ctx.distro"
- `ctx.distro`: {{ctx.distro}}
  ::end-block
- `ctx.os_version`: {{ctx.os_version}}
- `ctx.cpu_arch`: {{ctx.cpu_arch}}
- `ctx.cpu_cores`: {{ctx.cpu_cores}}
- `ctx.memory_total`: {{ctx.memory_total}}
- `ctx.gpu`: {{ctx.gpu}}

### Documents

- `ctx.docs_readme`: {{ as_csv(ctx.docs_readme) }}
- `ctx.docs_skill`: {{ctx.docs_skill}}
- `ctx.docs_drift`: {{ as_csv(ctx.docs_drift) }}
- `ctx.docs_blast_radius`: {{ as_csv(ctx.docs_blast_radius) }}

### Repo

- `ctx.repo`: {{ctx.repo}}
- `ctx.is_monorepo`: {{ctx.is_monorepo}}
- `ctx.repo_root`: {{ctx.repo_root}}
- `ctx.packages`: {{ as_csv(ctx.packages) }}
- `ctx.package_areas`: {{ as_csv(ctx.package_areas) }}
- `ctx.current_package_area`: {{ctx.current_package_area}}
- `ctx.current_package`: {{ctx.current_package}}

### Changed Files

- `ctx.dirty_files`: {{ as_csv(ctx.dirty_files) }}
- `ctx.dirty_source_code_files`: {{ as_csv(ctx.dirty_source_code_files) }}
