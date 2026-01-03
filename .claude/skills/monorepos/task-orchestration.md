# Task Orchestration Tools

Task orchestration tools schedule, cache, and execute tasks across monorepo projects. These are the "monorepo engines" that make builds fast and efficient.

## JavaScript-Centric Tools

### Nx

Opinionated, graph-aware build system with generators, caching, and CI integration.

**When to use:**
- Need opinionated structure and best practices
- Want code generation and scaffolding
- Building product-scale applications
- Need distributed task execution
- Want enforced architectural boundaries

**Key features:**
- Project graph visualization and analysis
- Local and distributed computation caching
- Affected command (only build what changed)
- Generators and executors (scaffolding)
- Plugin ecosystem for frameworks
- AI agent support
- IDE integration

**Setup:**

```bash
# New Nx workspace
npx create-nx-workspace@latest

# Add to existing monorepo
npx nx@latest init
```

```json
// nx.json
{
  "tasksRunnerOptions": {
    "default": {
      "runner": "nx/tasks-runners/default",
      "options": {
        "cacheableOperations": ["build", "test", "lint"]
      }
    }
  }
}
```

**Common commands:**

```bash
# Run target for specific project
nx build my-app

# Run for all affected projects
nx affected:build

# Visualize project graph
nx graph

# Generate new library
nx g @nx/js:lib my-lib
```

**Best for:** Product-scale apps, teams wanting strong opinions, enforced architecture

**Resources:** [Nx](https://nx.dev/)

---

### Turborepo

Fast task runner with hashing + caching; intentionally minimal.

**When to use:**
- Want simple, fast builds without ceremony
- Need caching but not code generation
- Prefer minimal configuration
- Building libraries or simple apps
- Using pnpm or npm workspaces

**Key features:**
- Content-aware hashing
- Local and remote caching (Vercel)
- Parallel task execution
- Simple configuration
- Pipeline-based task dependencies
- Incremental builds

**Setup:**

```bash
# Add to existing monorepo
npm install turbo --save-dev
```

```json
// turbo.json
{
  "$schema": "https://turbo.build/schema.json",
  "pipeline": {
    "build": {
      "dependsOn": ["^build"],
      "outputs": ["dist/**", ".next/**"]
    },
    "test": {
      "dependsOn": ["build"],
      "outputs": ["coverage/**"],
      "cache": false
    },
    "lint": {
      "cache": false
    }
  }
}
```

**Common commands:**

```bash
# Run build pipeline
turbo build

# Run with filtering
turbo build --filter=@myorg/api

# Clear cache
turbo build --force

# Enable remote caching
turbo login
turbo link
```

**Best for:** Minimal setups, teams wanting speed without opinions, simple monorepos

**Resources:** [Turborepo](https://turbo.build/)

---

### Lage

Microsoft's dependency-aware task scheduler focused on speed.

**When to use:**
- Already in Microsoft ecosystem
- Need simple task orchestration
- Want caching without complexity
- Working with Rush Stack

**Key features:**
- Dependency-aware scheduling
- Task caching
- Parallel execution
- Simple configuration

**Best for:** Microsoft-centric teams, Rush Stack users

**Resources:** [Lage](https://microsoft.github.io/lage/)

---

### Rush (Rush Stack)

Enterprise-scale orchestration for very large JS/TS repos with strong governance.

**When to use:**
- Enterprise repositories (hundreds of packages)
- Multiple teams with different release cadences
- Need strict governance and policies
- Require phased builds
- Building for scale and repeatability

**Key features:**
- Phantom dependency detection
- Strict version policies
- Phased builds
- Custom commands
- Install lifecycle management
- Azure/AWS cache integration

**Setup:**

```bash
# Initialize Rush
npm install -g @microsoft/rush
rush init
```

**Best for:** Very large enterprises, strict governance requirements, multiple teams

**Resources:** [Rush](https://rushjs.io/)

---

### Lerna

Originally the monorepo standard, now primarily a publishing/versioning layer atop native workspaces.

**When to use:**
- Publishing multiple npm packages
- Managing package versions together
- Need familiar, stable tooling
- Now maintained by Nx team

**Key features:**
- Independent or fixed versioning modes
- Package publishing workflow
- Version management
- Conventional commits integration
- Now uses Nx for task running

**Setup:**

```bash
npx lerna init
```

```json
// lerna.json
{
  "version": "independent",
  "npmClient": "pnpm",
  "useWorkspaces": true
}
```

**Best for:** Publishing workflows, teams familiar with Lerna, versioning needs

**Resources:** [Lerna](https://lerna.js.org/)

---

### moon (moonrepo)

Rust-based, language-agnostic task runner with explicit project graphs.

**When to use:**
- Need language-agnostic tooling
- Want Rust-level performance
- Building polyglot monorepos
- Need explicit project configuration
- Want AI integration (MCP server)

**Key features:**
- Rust-based for performance
- Language-agnostic (JS, TS, Rust, Go, etc.)
- Explicit project graph
- Task inheritance
- Code generation
- Project constraints enforcement
- MCP server for AI integration

**Setup:**

```bash
# Install moon
npm install -g @moonrepo/cli

# Initialize
moon init
```

```yaml
# .moon/workspace.yml
projects:
  - 'apps/*'
  - 'packages/*'
```

**Best for:** Polyglot repos, teams wanting explicit configuration, modern tooling

**Resources:** [moon](https://moonrepo.dev/moon)

---

## Polyglot / Large-Scale Tools

### Bazel

Hermetic, reproducible, multi-language build system for massive monorepos.

**When to use:**
- Massive scale (Google-sized repos)
- Need hermetic, reproducible builds
- Building across many languages
- Require remote execution
- CI costs are significant

**Key features:**
- Hermetic builds (completely isolated)
- Multi-language support
- Remote execution and caching
- Fine-grained dependency tracking
- Scales to billions of lines
- Deterministic builds

**Setup:**

```python
# WORKSPACE
workspace(name = "my_workspace")

# BUILD.bazel
load("@rules_nodejs//:index.bzl", "npm_package")

npm_package(
    name = "my-package",
    srcs = glob(["src/**/*"]),
    deps = [
        "//packages/common",
    ],
)
```

**Best for:** Massive monorepos, polyglot builds, teams needing hermetic builds

**Gotchas:**
- Steep learning curve
- Significant configuration overhead
- Build files for everything

**Resources:** [Bazel](https://bazel.build/)

---

### Pants

Developer-friendly large-repo build system with strong incremental builds.

**When to use:**
- Large polyglot monorepos
- Need better DX than Bazel
- Want automatic dependency inference
- Python or JVM heavy
- Need remote execution

**Key features:**
- Automatic dependency inference
- Multi-language support
- Remote execution and caching
- Fine-grained invalidation
- Better DX than Bazel
- Strong Python support

**Best for:** Large repos prioritizing developer experience, Python-heavy codebases

**Resources:** [Pants](https://www.pantsbuild.org/)

---

### Buck2

Meta's high-performance build system for very large codebases.

**When to use:**
- Meta-scale requirements
- Need extreme performance
- Building with Rust (Buck2 written in Rust)
- Want modern Bazel alternative

**Key features:**
- Written in Rust (fast)
- Modern architecture
- Remote execution
- Large-scale focus
- Meta battle-tested

**Best for:** Extremely large monorepos, Meta-like scale

**Resources:** [Buck2](https://buck2.build/)

---

## Comparison Matrix

| Tool | Languages | Scale | Complexity | Caching | Generators | Best For |
|------|-----------|-------|------------|---------|------------|----------|
| **Nx** | JS/TS/Multi | Medium-Large | Medium | Yes (local/remote) | Yes | Product apps, strong opinions |
| **Turborepo** | JS/TS | Small-Medium | Low | Yes (local/remote) | No | Simple, fast, minimal |
| **Lage** | JS/TS | Small-Medium | Low | Yes | No | Microsoft ecosystem |
| **Rush** | JS/TS | Large-Enterprise | High | Yes | No | Enterprise governance |
| **Lerna** | JS/TS | Small-Medium | Low | Via Nx | No | Publishing/versioning |
| **moon** | Multi | Medium | Medium | Yes | Yes | Polyglot, modern |
| **Bazel** | Multi | Massive | Very High | Yes (remote) | Limited | Google-scale repos |
| **Pants** | Multi | Large-Massive | High | Yes (remote) | Limited | Large polyglot, Python |
| **Buck2** | Multi | Massive | Very High | Yes (remote) | Limited | Meta-scale |

## Decision Flow

```
Do you have a polyglot repo with 100+ projects?
├─ Yes → Bazel (if need hermetic) or Pants (if need DX)
└─ No ↓

Is this primarily JavaScript/TypeScript?
├─ Yes ↓
│   Do you want opinionated structure and generators?
│   ├─ Yes → Nx
│   └─ No ↓
│       Do you need enterprise governance?
│       ├─ Yes → Rush
│       └─ No → Turborepo (simple) or moon (explicit)
└─ No ↓

Is this primarily Rust?
├─ Yes → Cargo workspaces + cargo-nextest
└─ No → moon (language-agnostic) or Bazel (hermetic)
```

## Migration Strategies

### From no orchestration → Turborepo

1. Install turbo: `npm install turbo --save-dev`
2. Create `turbo.json` with basic pipeline
3. Add `turbo` prefix to package.json scripts
4. Test: `turbo build`
5. Incrementally add caching config

### From Turborepo → Nx

1. Install Nx: `npx nx@latest init`
2. Nx will detect turbo.json and migrate
3. Add project.json files as needed
4. Leverage Nx generators for new projects
5. Gradually adopt Nx features (affected, graph)

### From Lerna → Nx

1. Run: `npx nx@latest init`
2. Nx preserves lerna.json
3. Lerna now uses Nx for task running
4. Optionally migrate to Nx versioning
5. Keep Lerna for publishing if preferred

## Performance Optimization

### Caching Best Practices

1. **Define outputs explicitly** - Only cache what's generated
2. **Hash inputs correctly** - Include all files that affect output
3. **Use remote caching** - Share cache across team/CI
4. **Exclude volatile files** - Don't hash timestamps, logs
5. **Cache test results** - Speed up CI significantly

### Parallelization Tips

1. **Declare dependencies accurately** - Enables maximum parallelism
2. **Avoid sequential task chains** - Flatten where possible
3. **Use task pipelines** - Let orchestrator handle scheduling
4. **Profile builds** - Find bottlenecks (nx graph, turbo --graph)

## Common Patterns

### Nx: Enforce Module Boundaries

```typescript
// .eslintrc.json
{
  "overrides": [
    {
      "files": ["*.ts"],
      "rules": {
        "@nx/enforce-module-boundaries": [
          "error",
          {
            "allow": [],
            "depConstraints": [
              {
                "sourceTag": "scope:shared",
                "onlyDependOnLibsWithTags": ["scope:shared"]
              }
            ]
          }
        ]
      }
    }
  ]
}
```

### Turborepo: Remote Caching

```bash
# Link to Vercel remote cache
npx turbo login
npx turbo link
```

```json
// turbo.json
{
  "remoteCache": {
    "enabled": true
  }
}
```

### Bazel: Remote Execution

```python
# .bazelrc
build --remote_executor=grpc://remote-execution-server:8980
build --remote_cache=grpc://cache-server:8980
```
