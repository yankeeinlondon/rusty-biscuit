---
area: biscuit-file
status: draft-spec
created: 2026-09-30
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
reviewed: true
reviewed_by: "codex/gpt-6.1-sol"
reviewed_on: "2026-09-30"
review_iterations: 0
human_review: false
message_to_agent: |-
  Phase 7 (consumer migration) is complete; read the "Phase 7" section of
  implementation-log.md. Points for Phase 8 (docs, skills, final validation):

  - Darkmatter compose finalization (`compose/link_normalization.rs`) now uses
    `PortablePath::from_path` + `with_ctx(source_link_context(options))` with
    the default strategy. Inputs are ABSOLUTE destinations only; relative and
    sigil destinations are left alone (link_resolve already absolutized
    everything it could). So compose does not preserve authored intent
    (`^/foo.md` comes back as the strategy's spelling of the same file); this
    is recorded as a possible follow-up, not a defect of this feature.
  - An `EnvRootedPath` result is written as `{{{VAR}}}/rest` (interpolation
    literal) so compose -> recompose is a fixed point. The docs page
    `darkmatter/docs/inline/link-normalization.md` was already rewritten in
    Phase 7 (examples, Mermaid, suffixes, warnings); polish, do not revert.
  - `ComposeOptions::with_env_path_whitelist` / `effective_env_path_whitelist`
    / `default_env_path_whitelist` and the PROJECT_ROOT / DOCS_BASE defaults
    are gone; `with_portable_env(names)` + `portable_env()` replace them.
    Phase 8's leftover search should find none (Phase 7 checked
    `darkmatter`, `claudine`, `docs`, `.claude/skills`).
  - Claudine was the skip case: no document-link rewriting (only
    shell-completion insert text). Nothing in Claudine docs to change.
  - Windows traps found and recorded in `.claude/skills/os/windows.md`
    (items 11, 12): a `{{VAR}}` whose value is verbatim `\\?\C:\...` never
    resolves (interpolation concatenates text), and a suffix splitter must
    skip the `?` in a `\\?\` prefix.
  - Pre-existing failures, not from this feature: native Windows still has
    the 7 Darkmatter L1 failures listed in the Phase 4 log; on Ken's Mac
    `claudine-cli completion::composition::tests::compose_magic_does_not_emit_a_nested_file_without_its_scope`
    fails because it reads the real `~/.claudine/prompts/plan.md` (passes with
    an isolated HOME).
  - Still open from earlier phases: `CandidatePlanOrder::AuthoringBaseFirst`
    is not renamed.
---

# Portable Paths

`PortablePath` produces file references that can survive moving a document,
a repository, or a configured file tree to another host. It prefers nearby
relative links, then stable anchors such as the repository root, a configured
environment variable, or the user's home directory. An absolute path is the
last resort and is visible to the caller as a fallback. Its input is either an
absolute path or an authored `FileReference`; for a reference it keeps the
anchors the author chose (`~`, `@`, `^`, `&`, …) and cleans up only links that
merely encode a position, such as `../../../foo.md`.

This feature belongs to the `biscuit-file` library. It builds on
[`FileReference`](../../lib/src/file_reference/mod.rs), which parses and
resolves references, and
[`FileResolutionContext`](../../lib/src/file_reference/context.rs), which
captures the directories and process state used for resolution. The feature
also migrates existing context consumers in Darkmatter and Claudine. Examples
of `md clean` describe a consumer contract; building a new cleanup command or
changing Markdown parsing is outside this feature.

**How this spec is organized.** [The File-Tree Model](#the-file-tree-model-prerequisite)
changes `FileResolutionContext` first: what `base_dir` and `cwd` mean, how the
tree root is chosen, and the boundary relative links must respect.
[What Counts as Portable](#what-counts-as-portable) through
[Diagnostics](#diagnostics) define `PortablePath` itself: its API, strategies,
input forms, verification, environment anchors, and error reporting.
[Implementation scope](#implementation-scope-and-acceptance) lists what must
ship and be tested; the [appendix](#appendix-spike-findings-2026-09-30) records
the dry run that informed the defaults.

## The File-Tree Model (prerequisite)

`PortablePath` and `FileReference` must describe a file tree with the same two
terms:

| Term       | Meaning                                                                                       |
| ---------- | --------------------------------------------------------------------------------------------- |
| `base_dir` | The root of the whole file tree and the boundary for relative links. In a repo it is always the repository root. |
| `cwd`      | The directory relative links (`./`, `../`, bare) start from — usually the directory of the document that holds the link. Normal derivations remain inside `base_dir`; trusted external derivations select a new tree. |

`FileResolutionContext` uses these names differently today, so it changes
before `PortablePath` is built on it.

### Today

| Concept              | Current name                                                                                         |
| -------------------- | ---------------------------------------------------------------------------------------------------- |
| where `./` starts    | `base_dir` on the context (`new(base_dir)`, `base_dir()`, `for_base`, `request_base_dir()`); the resolver's internal `ResolutionContext` already calls it `cwd` |
| root of the tree     | `repository_root`, which exists only when a repository is supplied or discovered                      |

One concept has two names (`base_dir` publicly, `cwd` internally), and outside a
repository the context has **no** tree root at all, so it cannot express the
boundary `PortablePath` needs (`.with_base_dir("/Users/ken/docs")` below).

### The change

The change lands in two steps, in this order. Both values are `PathBuf`, so
swapping meanings in a single step would compile cleanly and silently change
what every existing call site means.

1. **Rename the current `base_dir` to `cwd`.** No behavior change. Renamed
   methods and fields force updates, but positional constructor arguments do
   **not**: `new(base_dir)` and `new(cwd)` have the same Rust signature. Audit
   every constructor call and example explicitly, in addition to compiling
   consumers:

   | Before                              | After                          |
   | ----------------------------------- | ------------------------------ |
   | `FileResolutionContext::new(base_dir)` | `FileResolutionContext::new(cwd)` |
   | `from_snapshot(base_dir, home, env)` | `from_snapshot(cwd, home, env)` |
   | `base_dir()`                        | `cwd()`                        |
   | `for_base(dir)`                     | `for_cwd(dir)`                 |
   | `for_trusted_external_base(dir)`    | `for_trusted_external_cwd(dir)` |
   | `request_base_dir()`                | `request_cwd()`                |

2. **Introduce `base_dir` as the tree root.**
    - **in a repository**, `base_dir` is the repository root, and
      `repository_root()` continues to return it; `repository_root()` is
      `Some` exactly when the tree is a repository.
    - **outside a repository**, `base_dir` comes from an explicit
      `with_base_dir(dir)`, a containing vault, or the `~`/`{{VAR}}` anchor of
      the reference that opened the document, falling back to `cwd`; see
      [how `base_dir` is chosen](#decision-how-base_dir-is-chosen).
    - **`cwd` must be inside `base_dir`** after normal derivation, the containment rule that
      `RepositoryRootNotContainingSource` already enforces for repositories;
      `for_trusted_external_source` / `for_trusted_external_cwd` remain the
      explicit escape hatch.
    - **deriving keeps the tree**: a document reached by a link inside the
      current tree changes `cwd` only; `base_dir` carries over unchanged. A
      document opened through a reference that leads into a different tree
      (`~`, `{{VAR}}`, a vault) takes that tree's root, per
      [how `base_dir` is chosen](#decision-how-base_dir-is-chosen).

```rust
// in a repo: the caller supplies the discovered repo root
let ctx = FileResolutionContext::new("/opt/coding/rusty-biscuit/biscuit-file/docs/topics")
    .with_repository_root("/opt/coding/rusty-biscuit");
ctx.cwd();      // /opt/coding/rusty-biscuit/biscuit-file/docs/topics
ctx.base_dir(); // /opt/coding/rusty-biscuit

// outside a repo: base_dir is supplied
let ctx = FileResolutionContext::new("/Users/ken/docs/notes")
    .with_base_dir("/Users/ken/docs");
ctx.base_dir();        // /Users/ken/docs
ctx.repository_root(); // None
```

### What does not change

- **`FileReference` resolution behaves identically after step 1.** After
  step 2, `./` and bare references still start from `cwd` (today's context
  `base_dir`), and `&` and `^` remain repository-only; the intended resolution change
  is the boundary decision below, including its reader opt-in.
- package scopes (`with_package_root`, `with_package_area`,
  `with_repository_scope_catalog`), home, environment, magic search ordering, and the launch `@` scope
  retain their existing contracts. Vault configuration additionally informs
  tree-root selection as specified below.

### Decision: how `base_dir` is chosen

A document's tree root is the first of these that applies:

1. **the supplied repository root**, when it contains the document, or the
   repository root discovered during ambient preparation; explicit contexts
   never discover repositories themselves;
2. **an explicit `with_base_dir(dir)`**;
3. **the vault root** containing the document, when it is inside a configured
   vault (`add_vault`); choose the deepest containing root, with configuration
   order breaking ties, and include captured `VAULT` roots in their existing
   resolver order;
4. **the anchor of the reference that opened the document**, when that
   reference is `~`- or `{{VAR}}`-anchored — for example a transclusion or link
   inside another document, a configuration entry, or a quoted command-line
   argument (an unquoted `~` is expanded by the shell before the tool sees
   it, so the anchor is already lost):

   ```text
   ::file ~/Downloads/a.md          → a.md gets base_dir = ~       ("../b.md" = ~/b.md, inside the tree)
   ::file {{NOTES}}/inbox/a.md      → a.md gets base_dir = $NOTES
   ```

   Claudine's external prompts, opened as `~/.claudine/prompts/x.md`, get `~`
   as their tree root this way. Only a captured absolute home or environment
   anchor that contains the resolved source qualifies; a relative, unset, or
   foreign-host environment value supplies no tree root. Environment
   portability policy does not affect this reader-side anchor selection;
5. **`cwd`**, as the last resort.

An explicit `with_base_dir` outranks a containing vault: what the caller
states beats what is inferred. Inside a repository it cannot outrank the
repository root: a `with_base_dir` equal to the repository root is accepted,
and any other directory is an `InvalidConfiguration` error, both on
`FileResolutionContext` (with `with_repository_root`) and on `PortablePath`
(with a discovered repository).

**A `base_dir` that fell back to `cwd` is not a boundary.** Nothing told the
resolver where the tree is, so resolution does not reject a relative reference
for leaving it; a link such as `../b.md` resolves as it does today. Writing
stays strict: `PortablePath` treats an upward relative path out of a fallback
`base_dir` as external and does not emit one unless `ExternalRelativePath` is
in the strategy.

- `~` as a tree root is large, so the boundary there only stops links that
  leave the home directory. That is the honest limit of what is known.
- **The opening reference must reach the context.** Today callers derive a
  document's context from an already-resolved path
  (`for_source(path)`, `for_trusted_external_source(path)` in Darkmatter's
  compose, transclusion, and reference code), and a resolved path no longer
  says whether it was reached through `~` or `{{VAR}}`. Deriving from the
  opening `FileReference` is part of this change; see
  [Tree provenance and external documents](#tree-provenance-and-external-documents).

### Decision: `base_dir` is a boundary for relative references

Relative references stay inside the file tree when it has an established boundary. Once step 2 lands, an explicit
or implicit relative reference whose normalized target leaves `base_dir`
(`./../../x.md`, or `a/../../x.md`) returns a typed boundary error instead of reaching
outside the tree. The rule is the same inside and outside a repository: today
an in-repository reference such as `./../../outside.md` may leave the
repository (only `&` and `^` are contained), and after this change it fails
too. This is a deliberate behavior change. The one exception is a `base_dir`
that only fell back to `cwd`, which is not a boundary (see
[how `base_dir` is chosen](#decision-how-base_dir-is-chosen)).

Leaving the tree is opt-in, never a default. `PortablePath` mirrors this with
the `ExternalRelativePath` strategy: it may emit a relative path that leaves
`base_dir` only when the caller includes it, and the default strategy does not.

The boundary constrains the *form* of a reference, not where its target lives.
A target outside `base_dir` is fine when another strategy renders it without a
relative path (`EnvRootedPath` as `{{CONFIG_DIR}}/x.json`, `~/x.json`, `@x.md`,
or `AbsolutePath`). Only an emitted relative path that leaves the tree is
affected, which is exactly what `ExternalRelativePath` produces.

### Decision: the boundary checks where a link really lands

A relative reference must stay inside `base_dir` both **as written** and
**where it actually lands**. Symlinks whose source and target are both inside
the tree keep working; a symlink, junction, or reparse point that leads
outside the tree is rejected with `RelativeTreeEscape`.

```text
tree: ~/code/repo

~/code/repo/docs/current → ~/code/repo/docs/v2      (in-tree symlink)
./docs/current/x.md      → allowed

~/code/repo/shared → /opt/team-docs                 (out-of-tree symlink)
./shared/x.md            → rejected: lands outside the tree
```

- **Reuse the existing `&` / `^` containment check** rather than writing a
  second one: lexical containment after normalization, then canonical
  containment of the existing target, or of its deepest existing ancestor for
  a target not yet created.
- **The reader opt-in (`allow_external_relative()`) also permits a symlink
  escape**, since it permits relative targets outside the tree.
- **A fallback `base_dir` is still not a boundary**, so no check applies there.
- **`PortablePath`** cannot write a relative reference to a target reached
  only through an out-of-tree symlink; that target falls through to later
  strategies (`EnvRootedPath`, `HomeDir`, `AbsolutePath`).
- **This is a reference rule, not a sandbox.** Like the existing repository
  check, it is subject to filesystem changes between the check and a later
  open. The topic docs state this.

### Decision: resolving an external relative path is a reader opt-in

With an established boundary, a relative reference that leaves `base_dir`
resolves only when the resolving context opts in, for example `FileResolutionContext::allow_external_relative()`,
off by default.

- whoever includes the `ExternalRelativePath` strategy turns the opt-in on
  wherever those documents are resolved;
- `PortablePath` turns it on for its own round-trip check when
  `ExternalRelativePath` is the strategy being verified;
- the reference grammar is unchanged. If documents with escaping links come to
  travel between tools, a grammar marker that carries the permission in the
  reference itself is the follow-up; this opt-in does not block it.

### Boundary integration

Apply containment to the resolver's **effective relative kind** after its
existing interpolation pass, including bare repository-fallback candidates.
An absolute environment expansion is not relative and is unaffected.
Candidate planning, detailed resolution, convenience APIs, and completion
must share the same boundary decision. A blocked candidate is a typed
`RelativeTreeEscape { base_dir, candidate, reference }` error, not a missing
file or a reason to silently try another root. Completion must not suggest
escaping relative links when the reader opt-in is off. Recursive relative
searches cannot enumerate an escaping root; their existing no-directory-symlink
traversal contract remains unchanged.

Keep `RepositoryEscape` and the stronger existing `&` / `^` checks intact.
Introduce a non-repository context-containment error instead of reporting
`RepositoryRootNotContainingSource` for an ordinary file tree. The
fallback-root exception applies only to relative target containment; it does
not remove basic absolute-path or context validation.

### Decision: `&` and `^` stay repository-only

Outside a repository, `&` and `^` keep failing with `OutsideRepository`, even
when `base_dir` is supplied. The new `base_dir` gives a non-repository tree a
boundary, not a sigil anchor; `PortablePath`'s `RepoRoot` and `RepoMultiPath`
strategies do not apply there, and a link to the tree root is written relative.

### Tree provenance and external documents

Store the selected tree root together with its origin (repository, explicit,
vault, home, environment, or fallback), and expose whether it enforces a
boundary. A caller must not infer this by comparing `base_dir` with `cwd`:
an explicitly supplied root can equal CWD and still enforce containment.

Ordinary `for_source` / `for_cwd` derivation preserves that origin, the root,
the captured process state, and the launch `@` scope. An anchor in a link does
not replace the tree when its resolved document is already inside that tree.
A trusted external derivation validates the original request independently,
then selects a new source tree from explicitly supplied destination state or
the accepted vault/home/environment anchor, falling back to the new CWD.
An explicit root for the originating tree does not automatically contain an
external tree. If no repository catalog contains the external document, its
source `repository_root`, package, and package-area anchors are absent; the
launch repository remains available only through the unchanged launch `@`
scope. External derivation never discovers another repository.

Add `for_source_reference(&FileReference, resolved_source)` and its named
trusted-external counterpart so the opening anchor survives path resolution.
The resolved source must have been accepted by the caller using this snapshot;
these methods do not re-resolve it or grant permission to open it. Path-only
derivations remain available but cannot infer a lost home/environment anchor.
A caller that knows an external destination's repository supplies its topology
explicitly. Overlapping vaults follow the deepest-containing-root rule above.

The reader opt-in permits relative **targets** outside the selected tree; it
does not exempt an invalid request or document CWD, authorize file access, or
relax repository sigils. Copy it during child derivation. Trusted external
source acceptance and external-relative permission are distinct decisions.

**Cleanup of older links:** `from_reference` can encounter a position link
that the new reader boundary rejects. Evaluation returns `UnresolvableInput`
with a boundary finding, and the caller keeps the link, unless the caller
supplies an opted-in context. A cleanup caller may
opt in for reading its existing links while retaining the strict default
output strategy: it can then replace an escaping `../../x.md` with a valid
home/environment/absolute reference without emitting another escaping link.
Do not silently enable the reader opt-in for arbitrary reference inputs.

### Decision: Darkmatter's `ResolutionContext` adopts the same model

Darkmatter's expression-engine context
(`darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs`) uses
`base_dir` for the document's directory and `repository_root` for the tree
root, the opposite of the vocabulary above. It changes in this feature, not
later:

- **Same two terms, same meaning.** `cwd` is the document's directory;
  `base_dir` is the tree root, chosen by the same rules
  ([how `base_dir` is chosen](#decision-how-base_dir-is-chosen)).
- **Same two-step order.** Rename the current `base_dir` to `cwd` first and audit constructor
  calls that still compile, then introduce
  `base_dir` as the tree root.
- **One behavior in and out of a repository.** The boundary, the defaults,
  and the derivation rules (a link within the tree changes only `cwd`; a
  document opened into another tree takes that tree's root) are the ones
  `FileResolutionContext` follows; Darkmatter does not
  re-derive them. Its values come from the request's `FileResolutionContext`,
  so the two cannot disagree.
- **Coordination.** `2026-09-30-file-refs-use-magic` reshapes the same struct
  around one prepared context. Whichever lands second builds on the other's
  vocabulary; neither reintroduces the old names.

## What Counts as Portable

A reference is portable when it still names the right file after the document,
its tree, or its host changes. Inside a repository every form below applies;
outside one, the repository sigils do not.

| Form                                               | In a repository | Outside a repository |
| -------------------------------------------------- | --------------- | -------------------- |
| `~/…` (home)                                       | yes             | yes                  |
| `{{VAR}}/…` with a [portable](#environment-anchors), absolute `VAR` | yes             | yes                  |
| `@…` (magic search) and `vault:…`                  | yes             | yes                  |
| `&…` (repository root) and `^…` (package scope)    | yes             | no — repository-only |
| `./…`, `../…`, bare — while inside `base_dir`      | yes             | yes, when `base_dir` is known |
| absolute path                                      | no              | no                   |

A bare path is not inherently downward-only: `a/../../x.md` can leave the tree
like any `../` path.

**Awkward paths become `&`.** Because a repository has the `&` sigil, the
default strategy turns an awkward relative path such as `../../foo/bar/baz.md`
into the more readable `&apps/foo/bar/baz.md`; see
[Preference Strategy](#preference-strategy).

## Handling Non-Portable Paths

If a path is deemed non-portable, what should we do with it?

- When the strategy includes `AbsolutePath`, the library returns the absolute
  fallback and its structured diagnostics.
- A consumer warns by default, can suppress that warning, or can omit the
  absolute fallback to require a portable result. The library itself prints
  nothing.

Both are expressed through the strategy rather than a separate setting: including `AbsolutePath` keeps the absolute path, and the caller warns when `portable.strategy()` is `PortabilityPreference::AbsolutePath`; leaving it out turns the same case into `PortablePathError::NoStrategyMatched`. See [Producing the Reference](#producing-the-reference).


## `PortablePath`

This functionality should be provided with a struct called `PortablePath`:

```rust
// FileReference::new("~/.claudine/config.json")
let portable = PortablePath::from_path("/Users/ken/.claudine/config.json");
// FileReference::new("&biscuit-file/docs/topics/file-references.md"), when the
// process runs elsewhere in the repository (e.g. from its root)
let p2 = PortablePath::from_path("/opt/coding/rusty-biscuit/biscuit-file/docs/topics/file-references.md");
```

Now opting in to some environment variables that are portable across hosts (see [Environment Anchors](#environment-anchors)). `.with_portable_env()` takes only the **names** of the portable variables; their values come from the `FileResolutionContext` when one is given, otherwise from the process environment:

```rust
// FileReference::new("{{CONFIG_DIR}}/foobar.json"), where CONFIG_DIR=/opt/config
let with_env = PortablePath::from_path("/opt/config/foobar.json")
    .with_portable_env(["CONFIG_DIR"]);
```

With an existing `FileResolutionContext`, the value of `CONFIG_DIR` is read from the context's captured environment; the context never decides *which* variables are portable, because it holds every variable in the process:

```rust
// FileReference::new("{{CONFIG_DIR}}/foobar.json")
let with_ctx = PortablePath::from_path("/opt/config/foobar.json")
    .with_ctx(&ctx)
    .with_portable_env(["CONFIG_DIR"]);
```

And because `FileResolutionContext` provides not only the file tree's root directory (typically the repo root) but also the current working directory, it can produce better links inside a file tree and/or repo. Imagine that the current working directory is "/opt/coding/rusty-biscuit/biscuit-file/docs/topics"

```rust
// FileReference::new("./file-references.md");
let with_ctx = PortablePath::from_path("/opt/coding/rusty-biscuit/biscuit-file/docs/topics/file-references.md")
    .with_ctx(&ctx);
```

If you wanted to just directly specify the CWD you can also do that without needing a FileResolutionContext:

```rust
// FileReference::new("./file-references.md");
let with_ctx = PortablePath::from_path("/opt/coding/rusty-biscuit/biscuit-file/docs/topics/file-references.md")
    .with_cwd("/opt/coding/rusty-biscuit/biscuit-file/docs/topics");
```

Without an explicit context, `PortablePath` can discover the repository
containing the effective CWD during evaluation. For a non-repository tree,
supply its root through a context or `.with_base_dir()`. Set CWD separately
when the document is not in the process working directory:

```rust
// FileReference::new("./my-doc.md")
let file_tree = PortablePath::from_path("/Users/ken/docs/my-doc.md")
    .with_cwd("/Users/ken/docs")
    .with_base_dir("/Users/ken/docs");
```

### Capturing evaluation state

`from_path` accepts `impl Into<PathBuf>` and requires an absolute host path;
`from_reference` accepts a parsed `FileReference`. Evaluation is fallible, so
builders may collect invalid settings and `file_reference()` validates them.
`with_strategy` replaces the ordered default with the supplied preferences;
an empty list is valid and matches nothing. `with_portable_env` accumulates
names; repeated names are deduplicated.

With `with_ctx(&ctx)`, clone the supplied snapshot. Do not discover missing
repository/package scopes or read the live environment, home, or process CWD.
A context with no repository stays non-repository even when its directories
happen to be in one. To avoid contradictory snapshots, combining `with_ctx`
with `with_cwd` or `with_base_dir` is an `InvalidConfiguration` error in either
builder order; derive the desired context first. Strategy and portable-name
builders remain available with a context.

Without a context, capture CWD, home, and environment once per evaluation;
an explicit `with_cwd` overrides the process CWD, and an explicit
`with_base_dir` supplies the tree root outside a repository (inside one it
must equal the repository root, or evaluation fails with
`InvalidConfiguration`). Discover
repository information once from the effective CWD, using the existing
`biscuit-file` resolver preparation. Package topology remains caller-supplied;
do not invent package roots by walking arbitrary directory names. The target's
repository does not become the document's repository: a target in another
checkout cannot be rendered with the document's `&` anchor. All candidate
checks use the same prepared context. Reuse `with_ctx` for batch processing.

A `base_dir` builder does not set `cwd`. Directories must be absolute host
paths, and a normal CWD must lie inside the selected tree. Invalid contexts
produce an error before strategy evaluation, even if `AbsolutePath` could
otherwise succeed.

## Preference Strategy

`PortablePath` will have a built-in set of preferences for what type of relative link is preferred but a caller should also be able to define it how they want it if the default is not to their liking. This is done by exposing a `PortabilityPreference` enum:

```rust
pub enum PortabilityPreference {
    /// keep a reference input's authored intent form unchanged when it is one
    /// of the given forms (see Input Forms); never applies to a `from_path`
    /// input
    AuthoredIntent(IntentForms),
    /// represents a relative link in the same directory as CWD
    SameDirRelative,
    /// represents a relative link which is a child directory of CWD (any depth)
    ChildDir,
    /// represents a parent directory (one level up); ignored if CWD is already at the root of base directory
    ImmediateParentDir,
    ///a sub directory with the same parent as CWD
    PeerDir,
    /// any relative path that stays inside `base_dir`: any number of levels
    /// up, optionally followed by a path back down (`../../x.md`,
    /// `../../shared/x.md`); the in-tree catch-all, paired with
    /// `ExternalRelativePath` for paths that leave the tree
    ParentDir,

    /// use the `&` sigil to reference from the repo root; does not apply
    /// outside a repo; the optional string is an eligibility filter
    RepoRoot(Option<String>),
    /// use the `^` sigil (searched from the package root, then the package
    /// area, then the repo root); does not apply outside a repo; the optional
    /// string is an eligibility filter
    RepoMultiPath(Option<String>),
    
    /// use the captured launch `@` search roots, including configured roots
    /// outside the repository and home; verify the actual first match; the
    /// optional string is an eligibility filter
    MagicPath(Option<String>),

    /// a relative path that leaves `base_dir`; opt-in only and never part of
    /// a default strategy, because a link that escapes the file tree rarely
    /// survives the tree being moved
    ExternalRelativePath,
    /// a path that is rooted in a portable ENV variable
    EnvRootedPath,
    /// a path under the user's home directory, written as `~/…`; the home
    /// directory comes from the context, or the OS when there is none
    HomeDir,
    /// an absolute path as a file reference (highly non-portable)
    AbsolutePath,
}
```

The default `PortablePath` strategy is:

```rust
let def_strategy = [
    PortabilityPreference::AuthoredIntent(IntentForms::ALL),
    PortabilityPreference::SameDirRelative,
    PortabilityPreference::ChildDir,
    PortabilityPreference::PeerDir,
    PortabilityPreference::ImmediateParentDir,
    PortabilityPreference::RepoRoot(None),
    PortabilityPreference::EnvRootedPath,
    PortabilityPreference::HomeDir,
    PortabilityPreference::AbsolutePath,
];
```

Two orderings in the default are deliberate:

- **`RepoRoot` before `HomeDir`**: a repository usually lives under `~`, so a
  file in `~/code/rusty-biscuit/docs/x.md` is `&docs/x.md`, not
  `~/code/rusty-biscuit/docs/x.md`.
- **`EnvRootedPath` before `HomeDir`**: with `CONFIG_DIR` set to the absolute path of `~/.config/myapp`,
  `{{CONFIG_DIR}}/x.json` survives a host where the config lives elsewhere,
  while `~/.config/myapp/x.json` assumes the same layout everywhere.

While this strategy may be good for a lot of callers, a caller like Claudine might want to make some adjustments like:

```rust
let def_strategy = [
    PortabilityPreference::AuthoredIntent(IntentForms::ALL),
    PortabilityPreference::RepoMultiPath(Some("prompts".into())),
    PortabilityPreference::MagicPath(Some("~/.claudine/prompts".into())),
    PortabilityPreference::SameDirRelative,
    PortabilityPreference::ChildDir,
    PortabilityPreference::PeerDir,
    PortabilityPreference::ImmediateParentDir,
    PortabilityPreference::RepoRoot(None),
    PortabilityPreference::EnvRootedPath,
    PortabilityPreference::HomeDir,
    PortabilityPreference::AbsolutePath,
];
```

`AuthoredIntent` stays first so Claudine's adjustments apply only to position references and absolute paths, never to a sigil a prompt author chose. `MagicPath(Some("~/.claudine/prompts"))` produces an `@…` reference only when `~/.claudine/prompts` is one of the context's `@` search paths; otherwise [Verifying a Candidate](#verifying-a-candidate) rejects it and evaluation moves on.

Preferences are tried in order, stopping at the first verified match. If
none matches, evaluation returns `NoStrategyMatched` with the attempts.

### Exact strategy meanings

Relative strategies compute a component-based path from `cwd` to the target.
All except `ExternalRelativePath` require the target to remain in `base_dir`.
`SameDirRelative` selects a target whose parent is CWD; `ChildDir` selects a
target inside a subdirectory of CWD (any depth); `ImmediateParentDir` selects a target whose
parent is CWD's parent; `PeerDir` selects a target below a sibling directory
(one parent hop followed by a descent). `ParentDir` is the remaining in-tree
relative catch-all. `ExternalRelativePath` selects only targets outside the
tree, and requires a shared filesystem root: different Windows drives or UNC
shares cannot produce a relative path. A target equal to CWD renders as `./`
and belongs to `SameDirRelative`; directories may be emitted but receive a
`TargetNotFile` finding because the current resolver matches regular files.

The optional strings on `RepoRoot`, `RepoMultiPath`, and `MagicPath` are
**eligibility filters**, never additional search roots. `RepoRoot(Some("docs"))`
only considers targets below `<repository>/docs`, but still emits
`&docs/x.md`. `RepoMultiPath(Some("prompts"))` joins the filter to each existing
`^` root and considers targets below those directories, emitting `^prompts/x.md`.
For `MagicPath`, resolve a supplied filter such as `~/.claudine/prompts` with
`FileReference` and the captured context; it must name an existing `@` search
root, and targets must lie below it. Filters cannot escape their roots or
create new roots. Invalid filter syntax is `InvalidConfiguration`; an absent
anchor or nonmatching root is a recorded non-applicability reason.

Without a filter, searched strategies consider roots in resolver order and
verify each possible spelling until one matches the target. Never choose a
shorter spelling by skipping a root that would shadow it. `RepoRoot` only
applies when the current document has a repository anchor and the target
passes that anchor's existing containment checks.

## Input Forms

`PortablePath` accepts either an absolute path or an authored `FileReference`.
An absolute path has lost the author's intent; a reference still carries it,
so a reference input can keep what the author meant and clean up only what
merely encodes a position. Classification uses `FileReference::class()` and
its parsed payload, including the resolver's effective kind after environment
interpolation; it must not reproduce the grammar with string-prefix checks.

```rust
// an absolute path
let a = PortablePath::from_path("/opt/coding/rusty-biscuit/foo.md");
// an authored reference, e.g. a link target found by `md clean`
let r = PortablePath::from_reference(FileReference::new("../../../foo.md")?);
```

Two constructors rather than one `new` taking a string: `docs/x.md` is both a
valid relative path and a valid bare reference, so a string input would be
ambiguous.

### Intent and position

| Class        | Authored forms                                                                                       | Treatment                                     |
| ------------ | ---------------------------------------------------------------------------------------------------- | --------------------------------------------- |
| **Intent**   | `~`, `@`, `^`, `&`, `vault:`, URLs, recursive `%` searches, and a leading `{{VAR}}` **when `VAR` is a portable variable** | kept exactly as authored                      |
| **Position** | `./`, `../`, bare, absolute, and a `{{VAR}}` whose `VAR` is **not** portable                         | resolved to a target, then run through the strategy |

```text
md clean, document at <repo>/apps/web/docs/guide.md

^/foo.md              → ^/foo.md                 (intent: package-first search)
../../../foo.md       → &foo.md                  (position: RepoRoot)
{{CONFIG_DIR}}/x.json → {{CONFIG_DIR}}/x.json    (CONFIG_DIR is portable)
{{TMPDIR}}/x.json     → strategy result          (TMPDIR is not portable)
```

- **A portable variable is intent by the author's choice, not by its value on
  this host.** `{{CONFIG_DIR}}/x.json` is kept even where `CONFIG_DIR` is unset
  or relative; the [Environment Anchors](#environment-anchors) eligibility
  rules govern what `EnvRootedPath` may *emit*, not what an author may write.
  The problem is still reported as a finding.
- **A leading portable variable preserves the whole input**, but each
  remaining placeholder is still checked for unset or non-portable variables.
  `{{CONFIG_DIR}}/{{TMPDIR}}/x.json` is kept when `CONFIG_DIR` is portable;
  `TMPDIR` can still produce a finding.
- **A sigil's intent covers its whole payload.** `&{{PKG}}/README.md` is kept
  as authored even when `PKG` is not portable; rewriting it would require
  expanding `PKG` and dropping the `&` the author chose. The non-portable
  variable is still reported as a finding.
- **A non-leading variable does not make a reference intent.**
  `./{{SUB}}/x.md` is an explicit relative reference and is classified as
  position.
- **A kept reference can still carry findings** — an unset or relative portable
  variable, a non-portable variable inside a sigil, a missing target. They are
  reported through `portable.findings()`; see [Diagnostics](#diagnostics).

### Preserving intent is a strategy

Keeping an intent form is a strategy, `AuthoredIntent(IntentForms)`, not a
separate setting. It is the only strategy that returns the input itself rather
than computing a form from the target. It never rewrites. For local references it performs one detailed lookup to
report missing targets or context failures. URLs are kept without a local
lookup or network request: a remote target is not a missing local file.
Recursive `%` references are also authored intent and remain unchanged by
default, because replacing a search with one match loses search semantics.
If `AuthoredIntent` is absent or comes later, URLs and recursive searches are
not eligible for path-rewriting strategies; if no authored-intent strategy
selects them, evaluation returns `PortablePathError::NormalizationUnsupported`
and the caller keeps the link as authored. Do not fetch a URL or collapse a
recursive search to one target.

```text
from_reference("^/foo.md"), default strategy
1. AuthoredIntent(ALL)  → Matched(^/foo.md)          stops here

from_reference("../../../foo.md")
1. AuthoredIntent(ALL)  → NotApplicable(PositionForm)
2. SameDirRelative      → NotApplicable(...)
   ...
6. RepoRoot(None)       → Matched(&foo.md)

from_path("/repo/foo.md")
1. AuthoredIntent(ALL)  → NotApplicable(NotAReference)
   ...the rest as usual
```

- **`IntentForms` selects which intent forms are kept.** `IntentForms::ALL`
  (`~`, `@`, `^`, `&`, `vault:`, URLs, a leading portable `{{VAR}}`, and recursive `%` searches) is the
  default and the only set defined now; narrower sets (for example only the
  searched forms `^` and `@`, whose rewriting would change which file is
  found) are added when a caller needs one.
- **Position in the strategy is meaningful.** First means "never touch intent
  forms"; after `SameDirRelative` means "keep sigils unless a plain `./x`
  reaches the same file" (so an authored `&docs/x.md` next to the document
  becomes `./x.md`); absent means "normalize everything". The doc comment
  says this.
- **`portable.strategy()` always names the matched preference**, including
  `AuthoredIntent`. An input no strategy can decide is an error (see
  [Resolving a reference input](#resolving-a-reference-input)), never a
  success without a strategy.

### What "semantics untouched" means

A rewritten reference resolves to **the same file in the current tree**. Its
behavior under relocation may change, deliberately: `../../../foo.md` moves
with the document while `&foo.md` stays pinned to the repository root. That
change is the point of the cleanup.

### Minimal churn and idempotence

- A position reference that already has the form the strategy would choose is
  returned exactly as authored: bare `foo.md` and `./foo.md` can both be
  `SameDirRelative`, so neither is respelled **when lookup confirms the bare
  spelling already reaches that same-directory target**. A bare link that
  relies on repository-root fallback is not a same-directory link.
- Evaluation is idempotent: passing a result back in returns it unchanged, so
  running `md clean` twice changes nothing the second time. This is a required
  test property.

### Suffixes stay with the caller

`PortablePath` takes a path or a path reference only. A link layer such as
`md clean` splits off a `#fragment`, a `?query`, and a `:line` / `:line-line`
location suffix before calling it, and reattaches them to the result. Use that
layer's parsed link destination: do not split Windows drive colons, URL ports,
or legal filename characters by blindly scanning punctuation. URL intent is
preserved as a complete URL. `FileReference` itself retains its current suffix
grammar; this feature does not add line-position parsing to it:

```text
../../src/x.rs:42        → &src/x.rs:42
../../docs/x.md#install  → &docs/x.md#install
```

### Resolving a reference input

- A relative reference needs a `cwd`; for `md clean` it is the document's
  directory (`with_ctx(&ctx.for_source(doc))`).
- A target that does not exist is still well-defined for a single-candidate
  form (`../../../foo.md` has one candidate). A multi-candidate form (bare, `@`, `^`, or `vault:` with multiple
  roots) with no existing match has no single target; evaluation returns
  `PortablePathError::UnresolvableInput` (with a `TargetMissing` finding)
  rather than inventing a target.
  A bare form whose deduplicated plan has one candidate can use that candidate.
  Use `FileReference::candidate_plan` and `resolve_detailed`, not a parallel
  resolver. Boundary errors, missing anchors, and I/O failures are not proof
  of a missing target; they also produce `UnresolvableInput`, with a typed
  resolution finding saying which. The error carries the attempts (only
  strategies actually tried) and findings. A cleanup caller such as `md clean`
  treats it as "keep the original link and report it". For a path input,
  equivalent preparation failures return their own typed `PortablePathError`.
- **Output spelling after a sigil has no `/`**: a reference `PortablePath`
  writes itself is `&foo.md`, `^foo.md`, `@foo.md`, matching established
  usage across the repository. A reference kept by `AuthoredIntent` is never
  respelled: an authored `^/foo.md` stays `^/foo.md`.

## Producing the Reference

`file_reference()` evaluates the strategy and returns the chosen reference
together with how it was chosen:

```rust
let portable = PortablePath::from_path("/opt/config/foobar.json")
    .with_ctx(&ctx)
    .with_portable_env(["CONFIG_DIR"])
    .file_reference()?; // Result<PortableReference, PortablePathError>

portable.reference()      // &FileReference → {{CONFIG_DIR}}/foobar.json
portable.strategy()       // &PortabilityPreference that matched
portable.attempts()       // &[Attempt], every strategy tried, in order
portable.findings()       // &[Finding], problems with the returned reference
portable.into_reference() // FileReference, when that is all you need
```

- `PortableReference` implements `AsRef<FileReference>`, so it can be passed
  wherever a `&FileReference` is expected.
- `file_reference()` does real work despite its getter-like name: it may
  discover the repository root, probe the filesystem, and verify each
  candidate (see [Verifying a Candidate](#verifying-a-candidate)). Its doc
  comment says so.
- A fallback is visible as `portable.strategy() ==
  &PortabilityPreference::AbsolutePath`; that is the hook for the "warn by
  default" behavior in [Handling Non-Portable Paths](#handling-non-portable-paths).
  Leaving `AbsolutePath` out of the strategy turns the same case into
  `PortablePathError::NoStrategyMatched`.

## Verifying a Candidate

Before a strategy's reference counts as a match, `PortablePath` checks that it
leads back to the target:

- **Single-location forms** (`./`, `../`, `&`, `~`, absolute, and an
  absolute portable environment anchor) may name a not-yet-created file.
  Build and validate their candidate, including context and boundary checks,
  and compare it with the target. A single candidate is not automatically a
  valid candidate: a missing anchor or containment failure rejects it.
- **Search forms** (`@`, `^`) are used only when the target exists **and**
  looking the reference up finds that file.

| Target                                     | `&docs/x.md` | `@x.md`                         |
| ------------------------------------------ | ------------ | ------------------------------- |
| exists, found by the lookup                | valid        | valid                           |
| exists, but another file is found first    | valid        | not used — `Shadowed`           |
| does not exist yet                         | valid        | not used — `TargetMissing`      |

- Newly generated downward relative references start with `./`; upward
  references start with `../`. Bare authored references can be preserved by
  minimal churn only after verifying their actual search result.
- A target that does not exist still gets a portable answer: a config file
  about to be created comes out as `~/.claudine/config.json` or `&…`; only `@`
  and `^` are unavailable for it.
- The `base_dir` boundary and the external-relative opt-in apply during the
  check, as decided above.
- **Known limit:** an `@` or `^` result can be shadowed later, when someone
  creates a file earlier in the search order. That is inherent to searching,
  and one reason the default strategy prefers single-location forms.

## Environment Anchors

`EnvRootedPath` renders a target as `{{NAME}}/rest` when a portable
environment variable's value is a prefix of the target.

**Which variables are portable.** A variable is portable when it is both

1. likely to be **present** on other hosts, and
2. likely to carry the **same meaning** on every host.

`CONFIG_DIR` or `OBSIDIAN_VAULT`, set deliberately by a user or tool, qualify.
`PWD`, `OLDPWD`, and `TMPDIR` do not: they exist everywhere but mean something
different on every host and at every moment. `HOME` qualifies but is
unnecessary, because `~` already expresses it. Declaring it is allowed (no
special case); because `EnvRootedPath` precedes `HomeDir` in the default
strategy, a caller who declares `HOME` gets `{{HOME}}/…` rather than `~/…`.

**Declaring them.** The portable set is the union of two sources:

- **`PORTABLE_ENV_VARIABLES`**, honored by `PortablePath` itself so every tool
  behaves the same for a user who sets it. It is read from the same
  environment `PortablePath` uses for values (the context's captured
  environment, or the process environment without one), so evaluation stays
  deterministic. Format: comma-separated names, whitespace trimmed, e.g.
  `PORTABLE_ENV_VARIABLES="CONFIG_DIR, OBSIDIAN_VAULT"`. Each name must match
  `[A-Z0-9_]+`, the `{{VAR}}` name grammar; an invalid entry is skipped and
  recorded, not fatal.
- **`.with_portable_env(names)`**, which a caller uses to add names for its own
  needs (Claudine from its user and repository configuration, for example).
  It takes names only; there is no name/value form.

Without either source, no variable is portable. Darkmatter has no built-in
portable variables; its current `PROJECT_ROOT` / `DOCS_BASE` defaults are
removed, along with `ComposeOptions::with_env_path_whitelist` and the
defaults listed in `darkmatter/docs/inline/link-normalization.md`. The set
belongs to `PortablePath`, not `FileResolutionContext`: portability is a
policy, and `FileReference` resolution expands any `{{VAR}}`, portable or not.

**Eligibility.** Even a portable variable is ignored unless its value is an
**absolute path on the current host**:

| Value of `CONFIG_DIR` (target `/opt/config/foobar.json`) | Result                         |
| -------------------------------------------------------- | ------------------------------ |
| `/opt/config`                                            | `{{CONFIG_DIR}}/foobar.json`   |
| `../shared` (relative)                                   | not eligible — `NotAbsolute`   |
| `C:\config` on macOS (absolute only on another OS)       | not eligible — `ForeignAbsolute` |
| unset                                                    | not eligible — `Unset`         |
| `/srv/data` (absolute, but not a prefix of the target)   | not eligible — `NotAPrefix`    |

- **Why absolute only:** a relative value would make `{{NAME}}/rest` expand to
  a relative reference, which the `base_dir` boundary governs, so the emitted
  reference could fail to resolve back to its target.
- **Comparison uses normalized, lossless path components**, not text prefix
  matching; `/opt/config` does not contain `/opt/config-old`. See
  [Path identity and text](#path-identity-and-text) for Windows namespace rules.
- **When several variables qualify, the deepest component prefix wins**;
  equal-depth ties use environment-variable name order. Deduplicate names
  before evaluation. Empty comma-separated entries are ignored; invalid names
  from either source are recorded as findings, and eligibility failures are
  retained inside the one `EnvRootedPath` attempt.
- **An ineligible variable is skipped and recorded, never fatal.** Values
  differ by host, so a variable that cannot anchor here is an ordinary
  "strategy does not apply", and evaluation continues with the next strategy.
  The reason is kept in the attempt record (see [Diagnostics](#diagnostics)),
  so it is never lost silently.
- **This rule belongs to `PortablePath` only.** `FileReference`'s general
  `{{VAR}}` interpolation keeps accepting relative values (`{{SUBDIR}}/x.md` is
  legitimate hand-written syntax); the `base_dir` boundary decides whether
  such a reference resolves.

## Path identity and text

Use one shared internal implementation in `biscuit-file` for prefix comparison
and relative-path computation; migrate the relevant Darkmatter normalization
logic rather than introducing a second definition. Darkmatter's current
[`ComparisonKey`](../../../darkmatter/lib/src/markdown/compose/link_normalization.rs)
compares roots and lossless OS-string components. It is private to Darkmatter
and is not an existing public `biscuit-file` type. Audit its behavior before
moving it: it preserves `..` in comparison keys and preserves literal dot
segments in Windows verbatim namespaces. Do not assume it already performs
all normalization described here.

For ordinary host paths, collapse `.` and `..` without walking above a root,
then compare whole components. Preserve authored/worktree path identity;
do not canonicalize every target, case-fold every filename, or equate symlink
aliases as part of portability preference selection. Normalize a Windows
verbatim prefix only when the existing safe simplification permits it; never
reinterpret literal verbatim dot segments. Different drives/shares are
separate roots. Conservative failure to recognize an alias is acceptable;
rewriting to another file is not. Boundary validation of repository sigils
and relative references canonicalizes as required by
[the boundary checks where a link really lands](#decision-the-boundary-checks-where-a-link-really-lands); that check governs what is allowed, not which spelling is
preferred.

Render generated text through `biscuit-file`'s
[`try_portable_string` / `to_portable_string`](../../lib/src/path_text.rs),
then parse it through `FileReference` and verify the resulting candidate.
These text helpers are presentation tools: they use lossy Unicode conversion
and turn Unix backslashes into slashes. Reject a generated spelling if it
would change native path components or introduce reference grammar, including
`{{VAR}}` in a literal filename. Relative `./` protects a leading sigil in a
filename, but does not protect interpolation within a segment. Non-Unicode
paths that cannot be represented faithfully produce `UnrenderableTarget`,
even when `AbsolutePath` is present. Keep a faithful native Windows UNC
absolute spelling where the parser accepts it rather than forcing slashes.

Resolve and compare under the same prepared snapshot before accepting a
candidate. Missing targets use validated candidate plans rather than
existence-dependent `resolve` alone. Existing search results use the detailed
resolver and its ordered roots. Preserve the resolver's established lexical
path semantics; this feature is not a secure-open API and does not promise
protection from filesystem changes between verification and later access.

## Diagnostics

A caller that gets an error, or a fallback it did not expect, must be able to
see what really happened as structured data, without parsing message text.
Every evaluation therefore records one **attempt** per strategy tried, and both
the success value and the error expose that record.

The success value is a `PortableReference` (see
[Producing the Reference](#producing-the-reference)); its `attempts()` and the
error's `attempts()` accessor carry the same record.

```rust
/// One strategy that was tried, and what happened.
pub struct Attempt {
    pub strategy: PortabilityPreference,
    pub outcome: AttemptOutcome,
}

pub enum AttemptOutcome {
    Matched(FileReference),
    /// The strategy could not produce a reference for this target.
    NotApplicable(NotApplicable),
    /// It produced one, but that reference resolves somewhere else
    /// (e.g. `@x.md` shadowed by an earlier search root).
    Shadowed { reference: FileReference, resolves_to: PathBuf },
}

pub enum NotApplicable {
    NoRepository,
    OutsideBaseDir,
    OutsideFilter { filter: PathBuf },
    TooManyParentHops { needed: usize },
    NotUnderHome,
    NotUnderMagicRoot,
    /// A search form (`@`, `^`) for a target that does not exist yet.
    TargetMissing,
    EnvAnchor { name: String, problem: EnvAnchorProblem },
    /// `AuthoredIntent(..)` on a `from_path` input.
    NotAReference,
    /// `AuthoredIntent(..)` on a position form, or an intent form outside its
    /// `IntentForms`.
    PositionForm,
}

pub enum EnvAnchorProblem {
    Unset,
    NotAbsolute { value: String },     // "../shared"
    ForeignAbsolute { value: String }, // "C:\config" on macOS
    NotAPrefix { value: PathBuf },     // valid, but the target is not under it
}

/// Something worth reporting about the returned reference itself, most often
/// one kept as authored by `AuthoredIntent`. Read with `portable.findings()`;
/// empty when there is nothing to report.
pub enum Finding {
    /// A portable variable the reference uses is unset or unusable here.
    PortableVariableUnusable { name: String, problem: EnvAnchorProblem },
    /// A variable inside a kept sigil reference (`&{{PKG}}/…`) is not portable.
    NonPortableVariable { name: String },
    /// The reference points to a file that does not exist (a broken link).
    TargetMissing,
    /// The reference names a directory; the resolver matches regular files.
    TargetNotFile,
    /// A name in `PORTABLE_ENV_VARIABLES` or `with_portable_env` is not a
    /// valid `{{VAR}}` name; it was skipped.
    InvalidPortableVariableName { name: String },
}

pub enum PortablePathError {
    /// No strategy matched and `AbsolutePath` was not in the strategy.
    NoStrategyMatched { target: PathBuf, attempts: Vec<Attempt> },
    /// The input cannot be a target at all (relative, or absolute only on
    /// another OS).
    InvalidTarget { target: PathBuf, reason: InvalidTarget },
    /// A reference input that cannot be resolved to one target (a broken
    /// multi-candidate link, a missing anchor, a boundary denial, a probe
    /// failure); the caller keeps it as authored.
    UnresolvableInput { reference: FileReference, attempts: Vec<Attempt>, findings: Vec<Finding> },
    /// A URL or recursive `%` input that no authored-intent strategy kept;
    /// no strategy rewrites these.
    NormalizationUnsupported { reference: FileReference, attempts: Vec<Attempt> },
    /// Contradictory or invalid builder settings, such as `with_ctx` combined
    /// with `with_cwd`, a `with_base_dir` that disagrees with the repository
    /// root, or an invalid filter.
    InvalidConfiguration(ConfigurationProblem),
    /// The target cannot be written faithfully as reference text (for
    /// example a non-Unicode path), even as an absolute path.
    UnrenderableTarget { target: PathBuf },
}
```

Example: with the default strategy, `CONFIG_DIR=../shared`, and a target
outside the tree, evaluation falls back to `AbsolutePath`. `portable.attempts()`
includes `EnvRootedPath → NotApplicable(EnvAnchor { name: "CONFIG_DIR",
problem: NotAbsolute { value: "../shared" } })`, so a warning can say why the
variable was not used.

Design rules:

- **One API, not a simple and a detailed variant.** The attempts are always
  available, on success and on error (empty if preparation failed before any
  strategy ran). `FileReference` splits `resolve` from
  `resolve_detailed`; `PortablePath` does not repeat that.
- **Every reason is typed data.** Callers match on variants; Darkmatter and
  Claudine map them into their own vocabularies. `Display` is a one-line
  headline followed by the attempts.
- **Errors implement `Clone`**, so they hold no `std::io::Error`; retain a
  filesystem error's path, `ErrorKind`, and available OS error code as typed
  data. An optional owned message may add detail but is never parsed by callers.
  Permission errors must not become `TargetMissing` or a silent fallback. (`FileReferenceError`
  is not `Clone`, which forces Darkmatter to carry it in an `Arc`.)
- **The variant lists above are illustrative, not a complete Rust API.** The
  implementation must cover these additional outcomes explicitly: invalid
  configuration, unavailable home/CWD, boundary denial, non-file target,
  unsupported normalization, unrenderable text, and resolution/probe errors
  carrying path and `std::io::ErrorKind`. Keep one attempt per preference;
  an attempt can contain several candidate rejections or environment-anchor
  evaluations. An input no strategy decides is an error, never a success
  without a matched preference. Both errors shown above, and any added errors, expose attempts
  through an accessor; an `attempts` field on only one variant is insufficient.

## Implementation scope and acceptance

The terminal state of this feature is implementation complete, ready for
review; the author closes the review cycle and moves the feature afterward.

- Land the vocabulary migration before adding the tree-root meaning. Audit
  constructor arguments and rename context-derived result accessors such as
  `DetailedResolution::base_dir()` to `cwd()` when they still describe the
  authoring directory. Keep `LaunchMagicScope::request_dir` as a request
  directory, not a tree root. Compile all direct consumers identified by a
  source search, including Darkmatter, Claudine, and examples.
- Add the portable-path module and re-export its public types under the
  existing `file-reference` feature. No new dependency or feature is required
  by this design. Preserve the no-default-features build of the existing
  unfeatured path-text helpers.
- Migrate Darkmatter's existing link-normalization consumer to `PortablePath`
  using its captured request context. Removing `with_env_path_whitelist` and
  `PROJECT_ROOT` / `DOCS_BASE` defaults is intentional: previously generated
  environment links may now become relative, repository-rooted, home-rooted,
  or absolute. Provide `with_portable_env` on the consumer options as the
  replacement for explicitly selected names, forward those names to
  `PortablePath`, and update consumers/tests together. Links Darkmatter
  generated earlier use `${VAR}/…`, which is not `FileReference` syntax and
  never resolved as a reference; authored `{{VAR}}` links still resolve, but
  are rewritten unless declared portable or protected by another authored
  intent form.
- Update the `biscuit-file` README, its file-reference topic page and skill
  references, plus affected Darkmatter/Claudine docs and examples. Document
  the rename, boundary origin, fallback exception, reader opt-in, precedence,
  and diagnostics. New topic content must stand on its own and must not point
  readers to this feature snapshot. Update dependency docs only if the final
  implementation actually changes dependencies.
- Use nextest through the package-area `just test` recipes for meaningful
  contract tests, and `just lint` for changed code. Verify same-directory,
  child, peer, immediate-parent, deep-parent and external relative paths;
  repository, vault, home and environment roots; default and reordered
  strategies; filters; shadowing; missing targets and non-file targets;
  relative/environment boundary behavior; external derivation with unchanged
  launch `@` scope; captured-state isolation; and typed probe failures.
- Require idempotence for returned references (an input that fails with
  `UnresolvableInput` fails the same way when retried), and candidate equality for generated single-location references.
  Exercise Windows drives, UNC/verbatim spellings and distinct shares,
  non-Unicode paths, literal backslashes/interpolation syntax, environment
  ties, and in-tree versus out-of-tree symlinks for relative references. Use portable fixtures on macOS,
  Linux, native Windows, and WSL2 under existing test infrastructure; these
  correctness requirements do not introduce new CI gates or matrix cells.
  L2/L3 checks, if needed by consumer changes, must not focus a window.
- Treat the recorded corpus dry run as feasibility evidence, not a timing
  benchmark or proof of the full implementation. It already answers whether
  the simplified default strategy can reduce link churn. No additional
  performance spike is required by this spec; capture state once and reuse
  prepared contexts in batch consumers rather than discovering per link.

## Appendix: Spike Findings (2026-09-30)

A throwaway dry run applied a simplified default strategy (`AuthoredIntent`,
the four relative strategies, `RepoRoot`, `AbsolutePath`; no environment
anchors) to every Markdown link in three trees, using today's `FileReference`
for resolution and verification.

| Tree                               | Files | Local links | Notes                                                |
| ---------------------------------- | ----- | ----------- | ---------------------------------------------------- |
| this repository                    | 5,281 | 7,834       | the useful corpus                                    |
| `~/.claudine` prompts (not a repo) | 25    | 0           | no local links; no signal                            |
| an Obsidian vault (not a repo)     | 3,339 | 19          | 6,846 `[[wiki links]]`, which are not Markdown links |

In this repository:

- **The default strategy behaves as intended.** 121 sigil references kept
  (`@` 96, `^` 25); 3,365 relative links kept by minimal churn; 739 rewritten,
  728 of them `../../…` → `&…` (for example
  `../../../docs/cicd/test-inputs.md` → `&docs/cicd/test-inputs.md`). Zero
  verification failures, zero idempotence failures.
- **No lexical boundary escapes were found in this sample**: no resolved
  relative link leaves the repository root. This does not establish symlink
  containment or prove that other consumers have no external relative links.
- **Relative links leave their own document's directory often**: 1,228 of
  4,104 resolved relative links (30%). This is why a `base_dir` that only fell
  back to `cwd` is not treated as a boundary.
- **`&` would have prevented a large class of broken links.** 1,147 links in
  `_completed` specs resolve from the spec's pre-move location but not from
  the current one: moving a spec one directory deeper broke every `../` link
  in it.
- **`path:line` links are not understood by `FileReference`.** 1,295 links use
  the `file.rs:42` convention: 1,195 parse as a file literally named
  `file.rs:42` (when the path contains `/`) and resolve to nothing; 100 are
  rejected as an unsupported scheme (when it does not, e.g. `spec.md:21`).
- **Ten implicit (bare) links rely on the repository-root fallback** (e.g.
  `claudine/docs/research/mcp/kilo.md` written from inside
  `claudine/docs/research/subagents/`); the default strategy rewrites them to
  `PeerDir` (`../mcp/kilo.md`) rather than `&…`.
- The remaining 1,063 broken links were classified as missing targets by
  the simplified dry run. Implementation must distinguish missing files from
  missing context, boundary denial, and probe errors before reporting
  `TargetMissing`.
