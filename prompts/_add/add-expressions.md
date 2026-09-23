---
description: |-
    Interactive session that turns a caller's rough description of one or more new Darkmatter
    expression functions (callable from `{{{ … }}}`, `when="…"`, frontmatter, and Claudine
    lifecycle strings) into clarified requirements, a feature spec under `darkmatter/features/`,
    an implementation with its descriptor entry and runtime binding, updated documentation,
    and verified parity with the `claudine context --expressions` report.

    Pass the requirements on the command line:

        claudine compose prompts/_add/add-expressions.md requirements='add shout(text) that upper-cases text and appends "!"'

    Omit `requirements` in a terminal and Claudine collects it interactively before launch.
$schema:
    requirements: string(required; not-empty) -> the caller's description of the expression function(s) to add; rough prose is fine, the session clarifies it
    name: string -> short kebab-case name for the feature directory; derived from the requirements when omitted
kind: "expression function"
kinds: "expression functions"
catalog_command: "claudine context --expressions"
spec_dir: "{{ ctx.repo_root + '/darkmatter/features/' + ctx.today + '-' + (name || 'add-expressions') }}"
interactive: true
initialize:
    stack:
        - when: "!file_exists(ctx.repo_root + '/darkmatter/docs/schemas/expression-functions.yaml')"
          action:
              - error: "The **add-expressions** prompt must run inside the rusty-biscuit checkout: it edits `darkmatter/docs/schemas/expression-functions.yaml`, which was not found under `{{ctx.repo_root}}`."
start:
    stderr: "Starting an interactive session to add expression functions; the agent will clarify the requirements, write a spec to `{{spec_dir}}`, then implement it."
    say: "Starting the add expressions session."
success:
    stderr: "The expression-function session finished; spec and implementation are under `{{spec_dir}}`."
    say: "The expression functions have been added and are ready for review."
    message: "✅  expression functions added in **{{ctx.repo}}** (spec: `{{spec_dir}}`, {{ctx.agent}}/{{ctx.model}})"
failure:
    say: "Adding the expression functions failed."
    message: "❌  the add-expressions session failed in **{{ctx.repo}}** ({{ctx.agent}}/{{ctx.model}}: {{err.msg}})"
---

# Add Expression Functions to Darkmatter

## Context

You are a senior Rust engineer who owns Darkmatter's expression engine: the read-only
language behind `{{{ … }}}` interpolation and `when="…"` conditions, its authored function
catalog, the runtime bindings that execute it, and the `claudine context --expressions`
report that documents it. Load the `darkmatter` skill (its `compose.md` topic in particular),
the `claudine` skill (`composition.md` and `lifecycle.md`), and `sniff` when a function needs
host discovery; load `rust` and `rust-testing` when you write code and tests.

::file ../_repo-context.md

## The Caller's Requirements

{{requirements}}

## Background: How the Expression Engine Is Organized

::file {{ctx.repo_root}}/claudine/docs/topics/state-management/expression-engine.md exclude="## How the*"

The Darkmatter expressions topic is the authoritative language reference. Its "Namespaces"
and "Authoring a New Expression Function" sections are transcluded here; read its
"Read-Side Functions" and "Function Contracts" sections directly when your function is
context-aware, and run `claudine context --expressions` for the current catalog instead of
reading the generated table.

::file {{ctx.repo_root}}/darkmatter/docs/topics/darkmatter-expressions.md exclude="!prelude" exclude="## Core Expression Engine" exclude="## How Parsing Works" exclude="## Where Expressions Read Values From" exclude="## Operator*" exclude="## Truthiness" exclude="## Literals*" exclude="## Interpolation*" exclude="## Variable Access" exclude="## Comparison*" exclude="## Arithmetic*" exclude="## Unary*" exclude="## Functions" exclude="## Provider Query*" exclude="## Null Propagation*" exclude="## Timezone*" exclude="## Token Resolution*" exclude="## Programmatic*" exclude="## Common Patterns" exclude="## Errors and Unsupported*" exclude="## See Also"

## Definition Checklist

Every candidate function needs an answer to each of these before it is specified. This is
also the clarification agenda in Phase 2 below.

1. **Name and aliases** — canonical `snake_case`; the established alias convention is the
   underscore-free spelling (`predict_conflicts` / `predictconflicts`). Check
   `claudine context --expressions` for collisions with names and aliases, and check the
   `ctx.*` catalog: a function that shares a name with a context variable is a **pair** and
   must share its descriptor entry.
2. **Signature** — parameters with types from the catalog's vocabulary (`any`, `string`,
   `number`, `number(integer)`, `boolean`, `file | string`, `IpAddress`, enums), `optional`
   and `variadic` flags, and overloads when arity changes meaning. Path arguments are
   `file | string` and resolve like `absolute()` (document base dir, repo root, magic roots)
   through `FileReference`, never against the process CWD.
3. **Return shape** — type, `array`, `nullable`, literal unions (`boolean | "unstable"`),
   and `fallible`. Say what the function returns for a **valid miss** (the lookup found
   nothing) versus a **contract violation** (bad argument); a miss is a documented value
   such as `""`, `[]`, `false`, or `null`, and a violation is a compose error.
4. **Null and type contract** — the default rule is: a `null` argument propagates `null`,
   a wrong-type argument is an evaluation error, a wrong arity is an evaluation error.
   Inspecting predicates (`is_*`) never error; probes such as `file_exists`/`has_command`
   return `false` instead of erroring on any argument. Choose one family and say which.
5. **Evaluation mode** — `Pure` (arguments only), `Context` (needs the request's
   `ResolutionContext`: paths, repository root, captured observations, shell probe, ICMP
   authority, nested-compose slot), or `Lazy` (short-circuiting, like `and`/`or`).
6. **Captured dependency** — whether the function reads a captured context group (`package`
   reads `Repo`, `ipv4` reads `Network`). Such a function is listed in `FUNCTION_GROUPS` so
   a call demands the group eagerly; a call whose group was never captured is a fatal
   `FunctionContextNotCaptured`, never ambient discovery.
7. **Laziness and memoization** — evaluated at call time with fresh I/O every call
   (`recent_commits`), or cacheable? State it; Darkmatter memoizes nothing across calls by
   default.
8. **Effects and consent** — does the call run a process, launch a shell profile, send
   packets, fetch a URL, or compose nested content? Then it is an **effect**: it must appear
   in compose preflight as a typed record, be gated by the existing consent model
   (`--allow-host`, shell approval, `IcmpAuthority`), answer a neutral value in discovery
   mode, and never run during passive validation, DMLS hover, or preflight.
9. **Surfaces** — the function resolves on every expression surface (frontmatter passes,
   body, `when=`, `$()` ternary branches, Claudine loop and lifecycle strings). Frontmatter
   surfaces are local-only: a remote URL argument there fails loudly.
10. **Examples** — one per overload with a declared result. `verification: executable` runs
    through the real evaluator in tests; use `display-only` with a `reason` only when the
    result depends on the host, repository, clock, or network.
11. **Platform behavior** — anything that differs on macOS, Linux, native Windows, or WSL2
    (shell dialects, path spelling, `PATHEXT`, process containment), and how tests avoid
    depending on the executing host.

## Wiring Recipe (verified against the code on {{ctx.today}}; the code wins on conflict)

Work in this order. The parity tests are red between steps; the notes say which red is
expected.

1. **Author the descriptor.** Add the entry to
   `darkmatter/docs/schemas/expression-functions.yaml`: `name`, `category`, `order`,
   `description`, and `overloads[]` with `parameters`, `returns`, and `example`. The file's
   own `$schema` block at the top is the authoritative shape. For a pair, set `pair: true`
   and omit `description` (it is authored once, on the `ctx` side of
   `darkmatter.yaml`). Until step 2 lands, `catalog_and_runtime_bindings_have_bidirectional_canonical_parity` in `expression/functions/mod.rs` is red; that is expected.
2. **Bind the handler.** In the owning domain module under
   `darkmatter/lib/src/markdown/compose/expression/functions/` (`strings.rs`, `paths.rs`,
   `git.rs`, `network.rs`, `shell.rs`, …; add a module and list it in `mod.rs`'s
   `bindings()` when no domain fits) add one `FunctionBinding { canonical, aliases,
   evaluation, handler }` to that module's `BINDINGS` and implement the handler:
   `fn(&[Value]) -> Result<Value, String>` for `Pure`,
   `fn(&[Value], &ResolutionContext) -> Result<Value, ExpressionError>` for `Context`.
   Use `args::require_args` for arity. Return `ExpressionError::ContractViolation` for
   inputs the spec makes a compose error (it is never demoted to a body warning) and
   `ExpressionError::Other` only for failures a lenient body may tolerate.
3. **Register a captured dependency.** When the function reads a context group, add
   `("name", ContextGroup::X)` to `FUNCTION_GROUPS` in
   `darkmatter/lib/src/markdown/compose/context/capture/groups.rs` and read the retained
   `CapturedObservations` through `ResolutionContext::observations`; never rediscover.
4. **Model any effect.** Effects go through the existing seams: shell probes through the
   bounded launcher in `shell_expansion/launcher.rs` (respecting
   `ComposeOptions::suppress_shell_probes`), ICMP through `compose/icmp.rs`'s
   `IcmpAuthority` with a `discovering()` mode, nested composition through
   `compose/nested.rs`'s `NestedComposeSlot`. Extend `preflight/collect.rs` so the effect is
   recorded from the authored source (untaken branches included) without evaluating, and so
   a shape that depends on the effect's result is rejected up front as
   `UnevaluatedDependencyShape`.
5. **Regenerate the narrative table.** Run `just darkmatter regen-expr-doc` from the repo
   root (or `just regen-expr-doc` in `darkmatter/`) so the generated table in
   `darkmatter/docs/topics/darkmatter-expressions.md` matches;
   `narrative_doc_function_table_matches_catalog` guards it. Then write the prose: a
   paragraph in the matching subsection of that doc (Read-Side Functions for context-aware
   functions, the relevant helper family otherwise) covering the contract, the miss value,
   and any documented gaps.
6. **Keep passive tooling passive.** DMLS completion and hover
   (`darkmatter/dmls/src/overlay/expressions.rs`) read `expression_function_descriptors()`;
   they need no code change, but their descriptor-corpus fixtures may need the new name and
   signature. Nothing there may evaluate.
7. **Claudine needs no listing edit.** `claudine context --expressions` projects the catalog.
   Only touch Claudine when the function reads late-binding or lifecycle state, or when its
   effect needs an invocation-owned capability (a fresh Git read, host evidence); then the
   `claudine` skill's composition and lifecycle topics apply.

The tests that must be green when you finish, all in
`darkmatter/lib/src/markdown/compose/expression/`: the registration invariants in
`functions/mod.rs`, `every_descriptor_overload_is_dispatchable_at_its_declared_arity`,
`every_example_evaluates_to_its_declared_result`, and
`narrative_doc_function_table_matches_catalog` in `catalog/mod.rs`, plus your own behavior
tests in the domain module and one composition through `md compose` on a fixture.

## Rules Learned the Hard Way

These came out of `has_command`, the path helpers, and the `darkmatter/features/2026-09-09-more-context`
feature (`as_markdown`, `package`, `recent_commits`, the network and shell probes).

- **The catalog is metadata; the binding is behavior.** Neither repeats the other. Add one
  side only and the build fails, by design.
- **Never execute what you probe.** `has_command` delegates to `which`; shell probes pass
  the name as data (an environment variable), never interpolated into shell syntax.
- **Passive surfaces observe nothing.** Preflight, schema validation, DMLS, and
  `claudine context` never launch a profile, send a packet, read a file the document did
  not name, or compose nested content. Discovery mode answers `false`/`null`/`""`.
- **Read-side functions resolve or fail loudly on every surface.** They never leak an
  unresolved `{{{ … }}}` literal; a remote URL on a local-only surface is an error.
- **One miss spelling per family.** `package`/`package_area` and the three scope variables
  all say `""` for "no area"; do not introduce a second spelling for the same idea.
- **Numbers are checked, not truncated.** Counts and budgets reject non-finite, negative,
  fractional, and overflowing values before any allocation or I/O.
- **Examples are tests.** An `executable` example that depends on the host makes the
  generated doc nondeterministic; use `display-only` with a reason.
- **Aliases are cheap, renames are not.** GitNexus `rename` understands the call graph;
  find-and-replace does not.
- **Consent never widens implicitly.** An address, host, or command grant is checked at
  the moment of use against the exact target; evaluation can never acquire permission.
- **Impact analysis before edits.** The catalog loader and projection functions are HIGH in
  GitNexus because every reader sits above them; stop and tell the caller before editing
  them.

::file ./_workflow.md

::file ../_no_formatting.md

::file ../_os.md
