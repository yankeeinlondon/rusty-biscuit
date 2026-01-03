# Production Stacks

Battle-tested monorepo combinations that teams actually run in production, organized by use case and scale.

## Decision Heuristics

### By Team Size

**1-5 developers:**
- Keep it simple
- Minimize tooling overhead
- Focus on workspace standards only
- Add orchestration when builds slow down

**5-20 developers:**
- Add task orchestration for caching
- Remote caching becomes valuable
- Consider CI optimization
- Still manageable with minimal tooling

**20-100 developers:**
- Opinionated tooling reduces friction
- Need enforced boundaries
- Remote caching essential
- Consider code generation

**100+ developers:**
- Enterprise-grade governance required
- Multiple teams, different release cadences
- Hermetic builds may be needed
- Significant CI investment

### By Repository Size

**< 10 packages:**
- Native workspaces sufficient
- Optional: lightweight orchestration (Turborepo)

**10-50 packages:**
- Task orchestration becomes essential
- Caching provides significant value
- Consider Nx or Turborepo

**50-200 packages:**
- Opinionated structure helps
- Affected commands critical
- Nx, Rush, or moon

**200+ packages:**
- Enterprise tooling required
- Consider Rush, Bazel, or Pants
- Remote execution may be needed

### By Language Mix

**Single language (JS/TS):**
- Use language-native workspaces
- Language-specific orchestration (Nx, Turbo)

**Single language (Rust):**
- Cargo workspaces + cargo-nextest
- Optional: moon for task orchestration

**2-3 languages:**
- Language-agnostic orchestrator (moon)
- Or: Nx with plugins

**Polyglot (4+ languages):**
- Bazel or Pants for hermetic builds
- Or: moon with language-specific tooling

## Canonical Stacks

### Minimal / Low-Ceremony JS

**Stack:**
```
pnpm workspaces + Turborepo + Changesets
```

**When to use:**
- Small to medium teams (1-20 devs)
- Libraries or straightforward apps
- Want fast setup and low maintenance
- Prefer simplicity over features

**Setup:**

```yaml
# pnpm-workspace.yaml
packages:
  - 'packages/*'
  - 'apps/*'
```

```json
// turbo.json
{
  "$schema": "https://turbo.build/schema.json",
  "pipeline": {
    "build": {
      "dependsOn": ["^build"],
      "outputs": ["dist/**"]
    },
    "test": {
      "cache": false
    }
  }
}
```

**Characteristics:**
- Minimal configuration
- Fast builds with caching
- Easy to understand
- No code generation
- Simple versioning with Changesets

**Trade-offs:**
- Less opinionated (more decisions to make)
- No built-in code generation
- No architectural enforcement
- Manual dependency management

---

### Opinionated / Product-Scale JS

**Stack:**
```
Nx (end-to-end) + Changesets or Nx Release
```

**When to use:**
- Product companies building apps
- Teams of 5-100 developers
- Want strong defaults and best practices
- Need code generation and scaffolding
- Building complex applications

**Setup:**

```bash
npx create-nx-workspace@latest myorg \
  --preset=ts \
  --packageManager=pnpm
```

**Characteristics:**
- Opinionated project structure
- Generators for consistency
- Enforced module boundaries
- Built-in affected commands
- Strong IDE integration
- Distributed task execution

**Trade-offs:**
- More configuration
- Steeper learning curve
- Opinionated (less flexibility)
- Larger ecosystem to learn

**Best practices:**
- Use tags for module boundaries
- Leverage generators for consistency
- Enable distributed caching
- Use affected commands in CI
- Define clear architectural layers

---

### Enterprise JS / TS

**Stack:**
```
pnpm or npm workspaces + Rush + Changesets
```

**When to use:**
- Large enterprises (100+ developers)
- Multiple teams with different release schedules
- Need strict governance and policies
- Require repeatable builds
- High compliance requirements

**Characteristics:**
- Phantom dependency detection
- Strict version policies
- Phased builds for CI optimization
- Custom commands per project
- Lockfile management across teams

**Trade-offs:**
- Complex initial setup
- Requires dedicated build engineering
- Steeper learning curve
- More ceremony

**Best practices:**
- Use common versions file
- Enable incremental builds
- Implement phased builds for CI
- Use shrinkwrap for reproducibility
- Document policies clearly

---

### Rust-First

**Stack:**
```
Cargo Workspaces + cargo-nextest + custom scripts
```

**When to use:**
- Rust-native projects
- Want to use standard Rust tooling
- Building libraries or binaries
- Need fast, reliable builds

**Setup:**

```toml
# Cargo.toml
[workspace]
members = [
    "crates/*",
]
resolver = "2"

[workspace.dependencies]
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.0", features = ["full"] }
```

**Characteristics:**
- Native Cargo support
- Shared dependencies via workspace.dependencies
- Unified lockfile
- Fast incremental compilation
- cargo-nextest for parallel testing

**Best practices:**
- Use `workspace.dependencies` for version consistency
- Enable `resolver = "2"` for better dependency resolution
- Use cargo-nextest for faster test runs
- Consider cargo-deny for license/security checks
- Use cargo-machete to find unused dependencies

**Optional enhancements:**
- Add moon for cross-language orchestration
- Use cargo-make for complex task orchestration
- Add cargo-release for publishing workflow

---

### Polyglot / Massive Repo

**Stack:**
```
Bazel or Pants + language-specific tooling
```

**When to use:**
- Very large codebases (Google/Meta scale)
- Multiple languages (JS, Rust, Go, Python, etc.)
- Need hermetic, reproducible builds
- Remote execution required
- CI costs are significant

**Bazel characteristics:**
- Hermetic builds (complete isolation)
- Fine-grained caching
- Remote execution support
- Deterministic builds
- Scales to billions of lines

**Pants characteristics:**
- Better developer experience than Bazel
- Automatic dependency inference
- Strong Python support
- Fine-grained invalidation
- Remote execution support

**Trade-offs:**
- Very steep learning curve
- Significant configuration overhead
- Requires build engineering team
- Complex tooling

**Best practices:**
- Invest in build engineering expertise
- Use remote caching from day one
- Implement remote execution early
- Create BUILD file generators
- Document extensively

---

### Modern Language-Agnostic

**Stack:**
```
Workspaces (pnpm / Cargo / etc.) + moon
```

**When to use:**
- Polyglot repos (2-4 languages)
- Want modern, performant tooling
- Need explicit project graph
- Want language-agnostic task orchestration
- Building for scale but not Bazel-level complexity

**Setup:**

```yaml
# .moon/workspace.yml
projects:
  - 'apps/*'
  - 'packages/*'
  - 'crates/*'

# .moon/tasks.yml
tasks:
  build:
    command: 'build'
    inputs:
      - 'src/**/*'
    outputs:
      - 'dist/**/*'
```

**Characteristics:**
- Rust-based (fast)
- Explicit project configuration
- Language-agnostic
- Task inheritance
- Code generation support
- AI integration via MCP

**Trade-offs:**
- Newer tool (smaller ecosystem)
- Explicit configuration (more setup)
- Less mature than Nx/Turbo

**Best practices:**
- Define clear project boundaries
- Use task inheritance for DRY
- Leverage code generation
- Configure project constraints
- Use tags for categorization

---

## Hybrid Approaches

### Nx + Cargo

For mixed TypeScript + Rust monorepos:

```json
// nx.json
{
  "targetDefaults": {
    "build": {
      "dependsOn": ["^build"]
    }
  }
}
```

```json
// packages/rust-lib/project.json
{
  "name": "rust-lib",
  "targets": {
    "build": {
      "executor": "nx:run-commands",
      "options": {
        "command": "cargo build --release",
        "cwd": "packages/rust-lib"
      }
    }
  }
}
```

### Turborepo + Cargo

```json
// turbo.json
{
  "pipeline": {
    "build": {
      "dependsOn": ["^build"]
    },
    "rust-lib#build": {
      "dependsOn": [],
      "outputs": ["target/release/**"],
      "cache": true
    }
  }
}
```

## Migration Paths

### Starting Simple → Adding Scale

1. **Phase 1: Workspaces only**
   - pnpm/npm/Cargo workspaces
   - Manual build orchestration
   - Good for < 10 packages

2. **Phase 2: Add caching**
   - Add Turborepo or Nx
   - Keep existing structure
   - Immediate build speed gains

3. **Phase 3: Add structure**
   - Migrate to Nx generators
   - Enforce module boundaries
   - Add affected commands

4. **Phase 4: Enterprise (if needed)**
   - Migrate to Rush or Bazel
   - Add governance policies
   - Implement remote execution

### From Monolith → Monorepo

1. **Extract shared code first**
   - Identify shared utilities/components
   - Move to packages/
   - Keep monolith as single "app"

2. **Add workspace tooling**
   - Set up workspaces (pnpm/npm/Yarn)
   - Configure orchestration (Turbo/Nx)

3. **Split incrementally**
   - Extract domain-specific packages
   - One feature/module at a time
   - Verify builds after each extraction

4. **Optimize boundaries**
   - Enforce module boundaries
   - Add architectural constraints
   - Document package purposes

## Anti-Patterns

### Over-Engineering

**Symptoms:**
- Bazel for a 5-package repo
- Nx with full generators for simple libraries
- Rush for 3-person teams

**Fix:** Start simple, add complexity when needed

### Under-Engineering

**Symptoms:**
- No caching with 50+ packages
- Manual build orchestration
- No dependency tracking

**Fix:** Add orchestration (Turborepo at minimum)

### Wrong Tool for Scale

**Symptoms:**
- Turborepo struggling with 200+ packages
- Nx without using its features
- Bazel overhead with no build scale

**Fix:** Match tool to actual needs and team size

### Mixing Orchestrators

**Symptoms:**
- Both Nx and Turbo in same repo
- Lerna + Rush together
- Conflicting cache strategies

**Fix:** Choose one orchestrator and commit

## Versioning Strategies

### Independent Versioning (Changesets)

Each package versions independently:

```bash
# Developer creates changeset
npx changeset

# CI creates version PR
npx changeset version

# Publish
npx changeset publish
```

**When to use:**
- Libraries with independent lifecycles
- Open source packages
- Semantic versioning requirements

### Fixed Versioning

All packages share same version:

```json
// lerna.json
{
  "version": "1.0.0"
}
```

**When to use:**
- Application suites (all deploy together)
- Tightly coupled packages
- Single release cadence

## CI Optimization

### Affected-Only Builds

```yaml
# GitHub Actions
- name: Build affected
  run: |
    # Nx
    nx affected:build --base=origin/main

    # Or Turborepo
    turbo build --filter=...[origin/main]
```

### Remote Caching

**Nx:**
```bash
# Link to Nx Cloud
npx nx connect
```

**Turborepo:**
```bash
# Link to Vercel
npx turbo login
npx turbo link
```

### Distributed Task Execution

**Nx Cloud:**
- Automatically distributes tasks across agents
- Scales horizontally
- Pay for what you use

**Bazel Remote Execution:**
- Self-hosted or cloud (BuildBuddy, etc.)
- Full hermetic builds
- Requires infrastructure investment

## Cost Considerations

### Small Teams (1-5 devs)

**Budget: $0-100/month**
- Use free tiers (Nx Cloud free, Vercel caching)
- Self-host if needed
- Focus on local caching

### Medium Teams (5-20 devs)

**Budget: $100-1000/month**
- Remote caching essential
- Nx Cloud or Vercel paid tier
- CI optimization pays off

### Large Teams (20-100 devs)

**Budget: $1000-10000/month**
- Distributed task execution
- Dedicated CI infrastructure
- Remote execution for large builds

### Enterprise (100+ devs)

**Budget: $10000+/month**
- Self-hosted infrastructure
- Dedicated build engineering team
- Custom tooling and optimization
