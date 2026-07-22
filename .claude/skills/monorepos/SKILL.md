---
name: monorepos
description: Expert knowledge for building and managing monorepos across JavaScript, TypeScript, Rust, Go, and JVM — workspace standards (npm, pnpm, Yarn, Cargo, Go workspaces, Gradle, Maven), task-orchestration tools (Nx, Turborepo, Bazel, Pants, Rush, Lerna, moon), and versioning (Changesets). Use when structuring a monorepo, choosing task-orchestration tooling, or setting up workspace versioning and release flows.
last_updated: 2026-07-15T00:00:00Z
hash: a275077af6cb393c-a1926b83eb6b8e02
---

# Monorepos

Expert guidance for structuring and managing monorepos—single repositories containing multiple packages or projects with shared dependencies and coordinated builds.

## Core Principles

- **Layer separation** - Workspace standards handle package layout; task orchestration handles builds/caching
- **Language-first workspace choice** - Use native workspace solutions (Cargo for Rust, Go workspaces for Go, pnpm/npm for JS/TS)
- **Match tool to scale** - Small teams: Turborepo/pnpm; Product-scale: Nx; Enterprise: Rush/Bazel
- **Cache everything** - All modern orchestrators (Nx, Turbo, Bazel) use content-aware caching
- **Explicit dependencies** - Task runners need dependency graphs to parallelize correctly
- **Conventional commits** - Structured commit messages enable automated releases and changelogs
- **Changesets for versioning** - De facto standard for monorepo version management and publishing
- **Hermetic builds for scale** - Bazel/Pants/Buck2 when reproducibility and massive scale matter
- **Incremental adoption** - Start with workspaces only, add orchestration when needed
- **Choose one orchestrator** - Don't mix Nx + Turbo; pick based on needs and commit

## Rusty Biscuit Verification Override

The generic workspace-wide Cargo examples in monorepo references are not local
verification advice. Rusty Biscuit has 48 workspace members:

- Identify changed packages and upstream/downstream symbol impact first, then
  map it to package areas with `sniff`.
- Run each affected area’s `just build`, `just test`, and `just lint` recipes,
  or exact package selectors when a narrower supported recipe is sufficient.
- Do not use `cargo build --workspace`, `cargo check --workspace`, a bare root
  Cargo lifecycle command, or an unscoped root `just` lifecycle recipe as a
  routine safety net. Workspace-wide aggregation requires an explicit user or
  CI/release requirement and a documented reason.

## Mental Model

Every monorepo has distinct layers:

| Layer | Purpose | Tools |
|-------|---------|-------|
| **Package Layout** | How packages live together | npm/pnpm/Yarn/Cargo/Go workspaces |
| **Task Execution** | How builds/tests run and cache | Nx/Turborepo/Bazel/Pants |
| **Versioning** | How changes are released | Changesets |
| **Workflow** | How humans communicate intent | Conventional Commits |

## Quick Reference

### By Ecosystem

**JavaScript/TypeScript:**
- Workspaces: npm, pnpm, Yarn, Bun
- Orchestration: Nx, Turborepo, Lage, Rush, Lerna, moon
- Versioning: Changesets

**Rust:**
- Workspaces: Cargo Workspaces
- Orchestration: cargo-nextest, custom scripts, or polyglot tools

**Go:**
- Workspaces: Go Workspaces (go.work)
- Orchestration: Native Go tooling or polyglot tools

**JVM:**
- Workspaces: Gradle Multi-Project, Maven Multi-Module
- Orchestration: Built into Gradle/Maven or Bazel/Pants

**Polyglot:**
- Orchestration: Bazel, Pants, Buck2, moon

### Common Production Stacks

See [production-stacks.md](./production-stacks.md) for detailed decision heuristics.

**Minimal/Low-Ceremony JS:**
```bash
pnpm workspaces + Turborepo + Changesets
```
Fast, simple, easy to reason about.

**Opinionated/Product-Scale JS:**
```bash
Nx (end-to-end) + Changesets or Nx Release
```
Strong defaults, code generation, enforced architecture.

**Enterprise JS/TS:**
```bash
pnpm/npm workspaces + Rush + Changesets
```
Governance, scale, repeatability.

**Rust-First:**
```bash
Cargo Workspaces + cargo-nextest
```
Cargo already is the standard.

**Polyglot/Massive Repo:**
```bash
Bazel or Pants + language-specific tooling
```
Hermetic builds, maximal reproducibility, CI scale.

**Modern Language-Agnostic:**
```bash
Workspaces (pnpm/Cargo/etc.) + moon
```
Explicit project graph without Bazel-level complexity.

## Topics

### Workspace Standards

- [Workspace Standards by Language](./workspace-standards.md) - Detailed breakdown of workspace tools for JavaScript/TypeScript, Rust, Go, and JVM ecosystems

### Task Orchestration

- [Task Orchestration Tools](./task-orchestration.md) - In-depth comparison of Nx, Turborepo, Bazel, Pants, Rush, Lerna, Lage, moon, and Buck2

### Production Guidance

- [Production Stacks](./production-stacks.md) - Battle-tested combinations, decision heuristics by team size, repo size, and language requirements

## Tool Selection Matrix

| Need | Recommended Tool |
|------|------------------|
| Simple JS/TS monorepo | pnpm workspaces + Turborepo |
| Opinionated structure + generators | Nx |
| Enterprise governance | Rush |
| Polyglot builds | Bazel or Pants |
| Rust-native | Cargo workspaces |
| Go-native | Go workspaces |
| Modern language-agnostic | moon |
| Version management | Changesets |

## Common Patterns

### Initialize pnpm Monorepo

```yaml
# pnpm-workspace.yaml
packages:
  - 'packages/*'
  - 'apps/*'
```

```json
// package.json
{
  "name": "my-monorepo",
  "private": true,
  "workspaces": [
    "packages/*",
    "apps/*"
  ]
}
```

### Add Turborepo

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
      "dependsOn": ["build"],
      "cache": false
    }
  }
}
```

### Cargo Workspace

```toml
# Cargo.toml (workspace root)
[workspace]
members = [
    "crates/*",
]
resolver = "2"
```

## Resources

- [npm Workspaces](https://docs.npmjs.com/cli/using-npm/workspaces)
- [pnpm Workspaces](https://pnpm.io/workspaces)
- [Cargo Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [Nx](https://nx.dev/)
- [Turborepo](https://turbo.build/)
- [Bazel](https://bazel.build/)
- [Changesets](https://changesets-docs.vercel.app/)
- [Conventional Commits](https://www.conventionalcommits.org/)