# Workspace Standards by Language

Workspace standards define how multiple packages/modules live in one repository and share dependencies. These handle repo layout, dependency resolution, and install semantics.

## JavaScript / TypeScript

### npm Workspaces

Native npm support for multi-package repos with minimal abstraction and broad compatibility.

**When to use:**
- Simple monorepos without complex orchestration needs
- Teams already using npm
- Maximum compatibility with existing tooling

**Key features:**
- Built into npm (no additional tools)
- Automatic local package linking
- Dependency hoisting to workspace root
- Run scripts across workspaces with `--workspaces`

**Setup:**

```json
// package.json
{
  "name": "my-workspace",
  "private": true,
  "workspaces": [
    "packages/*",
    "apps/*"
  ]
}
```

**Resources:** [npm Workspaces](https://docs.npmjs.com/cli/using-npm/workspaces)

---

### pnpm Workspaces

Strict, performant workspace model with global content-addressable store and powerful filtering.

**When to use:**
- Need disk efficiency (content-addressable store)
- Want strict dependency isolation
- Need powerful filtering (`pnpm --filter`)
- Building with Turborepo or Nx

**Key features:**
- Content-addressable global store (saves disk space)
- Strict dependency resolution (no phantom dependencies)
- Fast installs
- Advanced filtering syntax
- `workspace:` protocol for local dependencies

**Setup:**

```yaml
# pnpm-workspace.yaml
packages:
  - 'packages/*'
  - 'apps/*'
  - '!**/test/**'
```

**Filtering examples:**

```bash
# Build specific package and its dependencies
pnpm --filter @myorg/api build

# Build all packages in apps/
pnpm --filter './apps/**' build

# Build everything that depends on shared-lib
pnpm --filter ...@myorg/shared-lib build
```

**Resources:** [pnpm Workspaces](https://pnpm.io/workspaces)

---

### Yarn Workspaces

Mature workspace implementation frequently paired with additional tooling.

**When to use:**
- Existing Yarn-based projects
- Need Yarn-specific features (plug'n'play, etc.)
- Commonly paired with Lerna or other orchestration

**Key features:**
- Dependency hoisting
- Local package linking
- Works with Yarn Classic and Yarn Berry
- Protocol-specific features (workspace:, portal:)

**Setup:**

```json
// package.json
{
  "private": true,
  "workspaces": [
    "packages/*"
  ]
}
```

**Resources:** [Yarn Workspaces](https://yarnpkg.com/features/workspaces)

---

### Bun Workspaces

Workspace support integrated into Bun's package manager and runtime.

**When to use:**
- Using Bun as runtime
- Need extremely fast installs
- Building modern JS/TS projects

**Key features:**
- Ultra-fast install times
- Native TypeScript support
- Compatible with npm/pnpm workspace formats

**Resources:** [Bun Workspaces](https://bun.com/docs/pm/workspaces)

---

## Rust

### Cargo Workspaces

Official Rust solution for multi-crate repos with shared lockfile and target directory.

**When to use:**
- Any Rust monorepo
- Need unified build across crates
- Sharing dependencies across crates

**Key features:**
- Shared `Cargo.lock` and `target/` directory
- Unified dependency resolution
- Build/test all crates together
- Automatic local path dependencies

**Setup:**

```toml
# Cargo.toml (workspace root)
[workspace]
members = [
    "crates/core",
    "crates/api",
    "crates/cli",
]
resolver = "2"

[workspace.dependencies]
# Shared dependency versions
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.0", features = ["full"] }
```

```toml
# crates/core/Cargo.toml
[package]
name = "my-core"
version = "0.1.0"

[dependencies]
serde.workspace = true
tokio.workspace = true
```

**Common commands:**

```bash
# Build all workspace crates
cargo build --workspace

# Test all crates
cargo test --workspace

# Build specific crate
cargo build -p my-core

# Run example from specific crate
cargo run -p my-cli --example demo
```

**Resources:** [Cargo Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)

---

## Go

### Go Workspaces (go.work)

Official multi-module development workflow for Go.

**When to use:**
- Developing multiple Go modules together
- Need to work on modules with interdependencies
- Local development across modules

**Key features:**
- Multi-module build context
- Local module replacement
- Unified go.mod resolution
- Seamless transition to/from replace directives

**Setup:**

```go
// go.work
go 1.21

use (
    ./services/api
    ./services/worker
    ./packages/common
)
```

**Common commands:**

```bash
# Initialize workspace
go work init ./services/api ./packages/common

# Add module to workspace
go work use ./services/worker

# Sync workspace
go work sync

# Build from workspace context
go build ./...
```

**Resources:** [Go Workspaces](https://go.dev/doc/tutorial/workspaces)

---

## JVM

### Gradle Multi-Project Builds

Root + subprojects model for large builds.

**When to use:**
- JVM-based monorepos (Kotlin, Java, Scala)
- Need sophisticated build orchestration
- Want plugin ecosystem and flexibility

**Key features:**
- Declarative project structure
- Dependency configuration across projects
- Extensive plugin ecosystem
- Incremental builds and caching

**Setup:**

```kotlin
// settings.gradle.kts
rootProject.name = "my-monorepo"

include(
    "apps:api",
    "apps:web",
    "packages:common",
    "packages:database"
)
```

```kotlin
// build.gradle.kts
plugins {
    kotlin("jvm") version "1.9.0" apply false
}

subprojects {
    apply(plugin = "org.jetbrains.kotlin.jvm")

    dependencies {
        implementation(kotlin("stdlib"))
    }
}
```

**Resources:** [Gradle Multi-Project Builds](https://docs.gradle.org/current/userguide/multi_project_builds.html)

---

### Maven Multi-Module Projects

Aggregator POM ("reactor") driving multiple modules.

**When to use:**
- Existing Maven-based projects
- Enterprise Java ecosystems
- Need Maven-specific tooling integration

**Key features:**
- Parent POM inheritance
- Dependency management across modules
- Reactor builds
- Module-level lifecycle management

**Setup:**

```xml
<!-- pom.xml (root) -->
<project>
    <modelVersion>4.0.0</modelVersion>
    <groupId>com.example</groupId>
    <artifactId>my-monorepo</artifactId>
    <version>1.0.0</version>
    <packaging>pom</packaging>

    <modules>
        <module>packages/common</module>
        <module>apps/api</module>
    </modules>

    <dependencyManagement>
        <dependencies>
            <!-- Shared versions -->
        </dependencies>
    </dependencyManagement>
</project>
```

**Resources:** [Maven Multi-Module Projects](https://maven.apache.org/guides/mini/guide-multiple-modules.html)

---

## Decision Matrix

| Language/Ecosystem | Standard Choice | Alternative |
|-------------------|-----------------|-------------|
| JavaScript/TypeScript (small) | npm workspaces | pnpm workspaces |
| JavaScript/TypeScript (scale) | pnpm workspaces | Yarn workspaces |
| JavaScript/TypeScript (modern) | pnpm workspaces | Bun workspaces |
| Rust | Cargo workspaces | (none) |
| Go | Go workspaces | (none) |
| JVM (Kotlin/Java) | Gradle | Maven |
| Polyglot | (none - use orchestrator) | Bazel/Pants |

## Migration Paths

### npm → pnpm

1. Install pnpm globally
2. Create `pnpm-workspace.yaml`
3. Delete `node_modules` and `package-lock.json`
4. Run `pnpm install`
5. Update CI/CD to use pnpm

### Standalone packages → Cargo Workspace

1. Create workspace root `Cargo.toml`
2. Move packages to `crates/` subdirectories
3. Update path dependencies
4. Run `cargo build --workspace` to verify
5. Commit shared `Cargo.lock`
