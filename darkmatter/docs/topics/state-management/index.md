---
kind: canonical-documentation
---

# State in a Document

A composed Markdown document can read its own frontmatter, facts about its
runtime, and values supplied by its host. These value sources are called
**state**. Choose a namespace to make clear which source you mean.

```md
---
title: Release notes
env: staging
---
# {{ title }}
Project: {{ ctx.repo }}
Stage: {{ fm.env }}
Home: {{ env.HOME }}
```

Here `title` is a document property, `ctx.repo` is captured runtime context,
`fm.env` is the document's own `env` property, and `env.HOME` is an environment
variable. The `env` namespace takes precedence over a property with that name.

## Frontmatter: `fm`

Bare property names read the document's effective frontmatter: `{{ title }}`
and `{{ fm.title }}` agree. The explicit `fm.` form also reaches properties
whose names collide with namespaces or host globals. Use brackets for a key
containing a period: `{{ fm['release.channel'] }}` reads one key, while
`{{ fm.release.channel }}` traverses two nested properties.

Effective frontmatter includes caller overrides and successful lifecycle
`set` writes. A later expression sees those writes. Bare `fm` returns a copy
of the effective object at that evaluation; an earlier returned object does
not change afterward. Reading a string never executes expressions or shell
syntax stored inside it.

During frontmatter interpolation, a templated property becomes available
after its dependencies resolve. Both `fm.name` and `fm['name']` wait for the
property `name`; bare `fm` returns only what is resolved so far. See
[Frontmatter Interpolation](../../inline/fm-interpolation.md).

## Captured context and environment: `ctx` and `env`

`ctx` supplies facts such as the repository name, date, operating system, and
package. Context is captured for the composition request, and only the
groups the document needs are gathered. It does not fall back to arbitrary
frontmatter properties. See [Context Variables](context-variables.md).

`env` supplies the document's captured environment. All expressions and
environment-expanded file references in that document use the same map.
Transclusions share it; a host running another document supplies another map.
Loop passes, retries, and resumes reuse their document's map. A child shell
cannot change its parent's environment by exporting a variable. The
[request guide](../compose-requests.md#one-environment-per-document) explains
capture and the explicit host hook for choosing body provider identity.

## Fresh observations: `current`

`current` has the same keys as `ctx`, but refreshable facts are observed when
an expression reads them. For example, `ctx.branch` is captured context and
`current.branch` asks for the branch at evaluation time. Repeated reads in
one evaluation agree; a later evaluation may observe a change. Repository
identity, directory anchors, and document identity stay fixed even through
`current`. See [The lazy `current` namespace](../darkmatter-expressions.md#the-lazy-current-namespace).

## Lifecycle and host globals

Lifecycle events declare `err` for failure information and `timing` for
observed durations. Availability depends on the event: declaring a global
does not mean its value can be read before it exists. Claudine also supplies
`seq`, `loop`, and `group` in the scopes where those values apply. These
globals shadow same-named document properties; `fm.err`, for example, still
reads the property. See [Lifecycle](../lifecycle/index.md).

The root lookup order is a built-in namespace (`fm`, `ctx`, `env`, or
`current`), then a global declared for the scope, then a document property.
A missing property is `null`. `doc` and `current_env` are ordinary property
names, with no compatibility alias to their former namespace meanings.
Claudine hook dispatch has its own `hook` payload root and no document
frontmatter; see [Configuring Actions](../../../../claudine/docs/topics/configuring-actions.md#the-hook-root).

For operators, functions, and namespace migration examples, use the
[expression reference](../darkmatter-expressions.md). To change state or
perform effects, use [Safe Side Effects](../safe-side-effects.md).
