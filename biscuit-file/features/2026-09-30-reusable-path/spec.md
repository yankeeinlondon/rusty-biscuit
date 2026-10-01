# Portable Paths

Absolute paths are great when you're ON a host but when you need portability across hosts they are terrible. 

## Prerequisite: one vocabulary for file trees

`PortablePath` and `FileReference` must describe a file tree with the same two
terms:

| Term       | Meaning                                                                                       |
| ---------- | --------------------------------------------------------------------------------------------- |
| `base_dir` | The root of the whole file tree and the boundary for relative links. In a repo it is always the repository root. |
| `cwd`      | The directory relative links (`./`, `../`, bare) start from — usually the directory of the document that holds the link. Always inside `base_dir`. |

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

1. **Rename the current `base_dir` to `cwd`.** No behavior change. Every call
   site stops compiling and is updated deliberately:

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
    - **`cwd` must be inside `base_dir`**, the containment rule that
      `RepositoryRootNotContainingSource` already enforces for repositories;
      `for_trusted_external_source` / `for_trusted_external_cwd` remain the
      explicit escape hatch.
    - **deriving keeps the tree**: a document reached by a link inside the
      current tree changes `cwd` only; `base_dir` carries over unchanged. A
      document opened through a reference that leads into a different tree
      (`~`, `{{VAR}}`, a vault) takes that tree's root, per
      [how `base_dir` is chosen](#decision-how-base_dir-is-chosen).

```rust
// in a repo: base_dir is discovered as the repo root
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
  `base_dir`), and `&` and `^` remain repository-only; the one behavior change
  is the boundary decision below.
- package scopes (`with_package_root`, `with_package_area`,
  `with_repository_scope_catalog`), home, environment, magic paths, vaults,
  and the launch `@` scope are untouched.

### Documentation

The same change updates `biscuit-file/docs/topics/file-references.md` (its
"Base directory" definition and the capture-state examples), the `biscuit-file`
skill's `references/file-references.md`, and the Claudine and Darkmatter call
sites and docs that name the context's `base_dir`.

### Decision: `base_dir` is a boundary for relative references

Relative references stay inside the file tree. Once step 2 lands, an explicit
or implicit relative reference whose normalized target leaves `base_dir`
(`./../../x.md`, or `a/../../x.md`) fails to resolve instead of reaching
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

### Decision: `&` and `^` stay repository-only

Outside a repository, `&` and `^` keep failing with `OutsideRepository`, even
when `base_dir` is supplied. The new `base_dir` gives a non-repository tree a
boundary, not a sigil anchor; `PortablePath`'s `RepoRoot` and `RepoMultiPath`
strategies do not apply there, and a link to the tree root is written relative.

### Decision: Darkmatter's `ResolutionContext` adopts the same model

Darkmatter's expression-engine context
(`darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs`) uses
`base_dir` for the document's directory and `repository_root` for the tree
root, the opposite of the vocabulary above. It changes in this feature, not
later:

- **Same two terms, same meaning.** `cwd` is the document's directory;
  `base_dir` is the tree root, always the repository root inside a
  repository, supplied or defaulted to `cwd` outside one.
- **Same two-step order.** Rename the current `base_dir` to `cwd` first so
  every call site fails to compile and is updated deliberately, then introduce
  `base_dir` as the tree root.
- **One behavior in and out of a repository.** The boundary, the defaults,
  and the derivation rules (`cwd` changes per document, `base_dir` carries
  over) are the ones `FileResolutionContext` follows; Darkmatter does not
  re-derive them. Its values come from the request's `FileResolutionContext`,
  so the two cannot disagree.
- **Coordination.** `2026-09-30-file-refs-use-magic` reshapes the same struct
  around one prepared context. Whichever lands second builds on the other's
  vocabulary; neither reintroduces the old names.

### Decision: how `base_dir` is chosen

A document's tree root is the first of these that applies:

1. **the repository root**, when the document is in a repository;
2. **an explicit `with_base_dir(dir)`**;
3. **the vault root** containing the document, when it is inside a configured
   vault (`add_vault`);
4. **the anchor of the reference that opened the document**, when that
   reference is `~`- or `{{VAR}}`-anchored:

   ```text
   md compose ~/Downloads/a.md      → base_dir = ~        ("../b.md" = ~/b.md, inside the tree)
   md compose {{NOTES}}/inbox/a.md  → base_dir = $NOTES
   ```

   Claudine's external prompts, opened as `~/.claudine/prompts/x.md`, get `~`
   as their tree root this way;
5. **`cwd`**, as the last resort.

An explicit `with_base_dir` outranks a containing vault: what the caller
states beats what is inferred.

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
  opening `FileReference` (or passing its anchor alongside the path) is part
  of this change.

### Decision: resolving an external relative path is a reader opt-in

A relative reference that leaves `base_dir` resolves only when the resolving
context opts in, for example `FileResolutionContext::allow_external_relative()`,
off by default.

- whoever includes the `ExternalRelativePath` strategy turns the opt-in on
  wherever those documents are resolved;
- `PortablePath` turns it on for its own round-trip check when
  `ExternalRelativePath` is the strategy being verified;
- the reference grammar is unchanged. If documents with escaping links come to
  travel between tools, a grammar marker that carries the permission in the
  reference itself is the follow-up; this opt-in does not block it.

## In a Repo

- any file reference starting with a `~`, `@`, `^`, `&`, `vault:` sigil
- implicit relative paths
- explicit relative paths that remain inside the repo
- a "reusable" ENV rooted variable (e.g., `{{CONFIG_DIR}}/path/to/file.md` when `CONFIG_DIR` has been configured as a "reusable" base)

### Further Optimization

1. Awkward Paths

    Because we have use of the `&` sigil, we opt-in to converting awkward relative paths like `../../foo/bar/baz.md` to something more immediately understandable like `&apps/foo/bar/baz.md`.

## Outside a Repo

- any file with a leading `~` sigil
- implicit relative paths (as they only map to children, never upward)
- explicit relative paths that stay inside `base_dir`


## Handling Non-Portable Paths

If a path is deemed non-portable, what should we do with it?

- default behavior is to keep the absolute path but provide warning
- allow user to suppress warning or have it elevated to an error condition

Both are expressed through the strategy rather than a separate setting: including `AbsolutePath` keeps the absolute path, and the caller warns when `portable.strategy()` is `PortabilityPreference::AbsolutePath`; leaving it out turns the same case into `PortablePathError::NoStrategyMatched`. See [Producing the Reference](#producing-the-reference).


## `PortablePath`

This functionality should be provided with a struct called `PortablePath`:

```rust
// FileReference::new("~/.claudine/config.json")
let portable = PortablePath::from_path("/Users/ken/.claudine/config.json".into());
// FileReference::new("&biscuit-file/docs/topics/file-references.md")
let p2 = PortablePath::from_path("/opt/coding/rusty-biscuit/biscuit-file/docs/topics/file-references.md");
```

Now opting in to some ENV variables that are deemed to be "reusable" across hosts. `.with_portable_env()` takes only the **names** of the portable variables; their values come from the `FileResolutionContext` when one is given, otherwise from the process environment:

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
    .with_cwd("/opt/coding/rusty-biscuit/biscuit-file/docs/topics".into());
```

Up to now we've implicitly been assuming that the file reference exists inside of a repo. If this is the case `PortablePath` can figure out the repo's root all by itself but in cases where a caller is working with a file tree that is NOT in a repo then we must specify the root of the file tree. This can be done by passing in a `FileResolutionContext` or with the `.with_base_dir()` builder:

```rust
// FileReference::new("./my-doc.md")
let file_tree = PortablePath::from_path("/Users/ken/docs/my-doc.md")
    .with_base_dir("/Users/ken/docs".into());
```


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

    /// use the `&` sigil to reference from the repo root; if not in a repo 
    /// then this option is ignored
    RepoRoot(Option<String>),
    /// use the `^` sigil to reference multiple paths; if not in a repo
    /// then this option is ignored
    RepoMultiPath(Option<String>),
    
    /// use the `@` sigil to reference paths; if this is neither in a repo or a child of the user's home directory then this is used
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
- **`EnvRootedPath` before `HomeDir`**: with `CONFIG_DIR=~/.config/myapp`,
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

- Where we allow strategy paths, this allows a caller to make exceptions for certain paths that are detected.

A file path will be evaluated on each of the strategies sequentially until a strategies that can be used for the given file path matches. If none of the strategies provided can render the absolute file path then an error is returned.

## Input Forms

`PortablePath` accepts either an absolute path or an authored `FileReference`.
An absolute path has lost the author's intent; a reference still carries it,
so a reference input can keep what the author meant and clean up only what
merely encodes a position.

```rust
// an absolute path
let a = PortablePath::from_path("/opt/coding/rusty-biscuit/foo.md".into());
// an authored reference, e.g. a link target found by `md clean`
let r = PortablePath::from_reference(FileReference::new("../../../foo.md")?);
```

Two constructors rather than one `new` taking a string: `docs/x.md` is both a
valid relative path and a valid bare reference, so a string input would be
ambiguous.

### Intent and position

| Class        | Authored forms                                                                                       | Treatment                                     |
| ------------ | ---------------------------------------------------------------------------------------------------- | --------------------------------------------- |
| **Intent**   | `~`, `@`, `^`, `&`, `vault:`, URLs, and a leading `{{VAR}}` **when `VAR` is a portable variable** | kept exactly as authored                      |
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
than computing a form from the target. It never rewrites, and does one lookup
of the kept reference so a missing target can be reported as a finding.

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
  (`~`, `@`, `^`, `&`, `vault:`, URLs, a leading portable `{{VAR}}`) is the
  default and the only set defined now; narrower sets (for example only the
  searched forms `^` and `@`, whose rewriting would change which file is
  found) are added when a caller needs one.
- **Position in the strategy is meaningful.** First means "never touch intent
  forms"; after `SameDirRelative` means "keep sigils unless a plain `./x`
  reaches the same file" (so an authored `&docs/x.md` next to the document
  becomes `./x.md`); absent means "normalize everything". The doc comment
  says this.
- **`portable.strategy()` always names what decided the result**, including
  `AuthoredIntent`.
- **A kept reference is not rewritten but can carry findings** (an unset or
  relative portable variable, a non-portable variable inside a sigil, a missing
  target); see `portable.findings()` in [Diagnostics](#diagnostics).

### What "semantics untouched" means

A rewritten reference resolves to **the same file in the current tree**. Its
behavior under relocation may change, deliberately: `../../../foo.md` moves
with the document while `&foo.md` stays pinned to the repository root. That
change is the point of the cleanup.

### Minimal churn and idempotence

- A position reference that already has the form the strategy would choose is
  returned exactly as authored: bare `foo.md` and `./foo.md` are both
  `SameDirRelative`, so neither is respelled.
- Evaluation is idempotent: passing a result back in returns it unchanged, so
  running `md clean` twice changes nothing the second time. This is a required
  test property.

### Suffixes stay with the caller

`PortablePath` takes a path or a path reference only. A link layer such as
`md clean` splits off a `#fragment`, a `?query`, and a `:line` / `:line-line`
location suffix before calling it, and reattaches them to the result:

```text
../../src/x.rs:42        → &src/x.rs:42
../../docs/x.md#install  → &docs/x.md#install
```

### Resolving a reference input

- A relative reference needs a `cwd`; for `md clean` it is the document's
  directory (`with_ctx(&ctx.for_source(doc))`).
- A target that does not exist is still well-defined for a single-candidate
  form (`../../../foo.md` has one candidate). A searched form (bare, `@`, `^`)
  with no existing match has no single target; the reference is returned
  unchanged and the attempts record `UnresolvableInput`. A cleanup tool does
  not fail on a broken link it cannot improve.
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
    .file_reference()?; // Result<PortableReference, PortablePathError>

portable.reference()      // &FileReference → {{CONFIG_DIR}}/foobar.json
portable.strategy()       // the PortabilityPreference that matched
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
  PortabilityPreference::AbsolutePath`; that is the hook for the "warn by
  default" behavior in [Handling Non-Portable Paths](#handling-non-portable-paths).
  Leaving `AbsolutePath` out of the strategy turns the same case into
  `PortablePathError::NoStrategyMatched`.

## Verifying a Candidate

Before a strategy's reference counts as a match, `PortablePath` checks that it
leads back to the target:

- **Single-location forms** (`./`, `../`, `&`, `~`, `{{VAR}}`, absolute) look
  in exactly one place, so they are always valid, whether or not the target
  exists yet.
- **Search forms** (`@`, `^`) are used only when the target exists **and**
  looking the reference up finds that file.

| Target                                     | `&docs/x.md` | `@x.md`                         |
| ------------------------------------------ | ------------ | ------------------------------- |
| exists, found by the lookup                | valid        | valid                           |
| exists, but another file is found first    | valid        | not used — `Shadowed`           |
| does not exist yet                         | valid        | not used — `TargetMissing`      |

- `PortablePath` always writes a relative reference as `./x`, never as a bare
  `x`, so bare paths (which also search) never need this check.
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
- **Comparison uses normalized values** (`ComparisonKey`), so trailing
  separators, `..` segments, and Windows `\\?\` spellings neither create false
  matches nor hide real ones.
- **When several variables qualify, the longest prefix wins**, as Darkmatter's
  link normalization does today.
- **An ineligible variable is skipped and recorded, never fatal.** Values
  differ by host, so a variable that cannot anchor here is an ordinary
  "strategy does not apply", and evaluation continues with the next strategy.
  The reason is kept in the attempt record (see [Diagnostics](#diagnostics)),
  so it is never lost silently.
- **This rule belongs to `PortablePath` only.** `FileReference`'s general
  `{{VAR}}` interpolation keeps accepting relative values (`{{SUBDIR}}/x.md` is
  legitimate hand-written syntax); the `base_dir` boundary decides whether
  such a reference resolves.

## Diagnostics

A caller that gets an error, or a fallback it did not expect, must be able to
see what really happened as structured data, without parsing message text.
Every evaluation therefore records one **attempt** per strategy tried, and both
the success value and the error expose that record.

The success value is a `PortableReference` (see
[Producing the Reference](#producing-the-reference)); its `attempts()` and the
error's `attempts` field carry the same record.

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
    /// A searched reference input with no existing match has no single target.
    UnresolvableInput,
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
}

pub enum PortablePathError {
    /// No strategy matched and `AbsolutePath` was not in the strategy.
    NoStrategyMatched { target: PathBuf, attempts: Vec<Attempt> },
    /// The input cannot be a target at all (relative, or absolute only on
    /// another OS).
    InvalidTarget { target: PathBuf, reason: InvalidTarget },
}
```

Example: with the default strategy, `CONFIG_DIR=../shared`, and a target
outside the tree, evaluation falls back to `AbsolutePath`. `portable.attempts()`
includes `EnvRootedPath → NotApplicable(EnvAnchor { name: "CONFIG_DIR",
problem: NotAbsolute { value: "../shared" } })`, so a warning can say why the
variable was not used.

Design rules:

- **One API, not a simple and a detailed variant.** The attempts are always
  available, on success and on error. `FileReference` splits `resolve` from
  `resolve_detailed`; `PortablePath` does not repeat that.
- **Every reason is typed data.** Callers match on variants; Darkmatter and
  Claudine map them into their own vocabularies. `Display` is a one-line
  headline followed by the attempts.
- **Errors implement `Clone`**, so they hold no `std::io::Error`; a filesystem
  probe's result is recorded as a typed reason instead. (`FileReferenceError`
  is not `Clone`, which forces Darkmatter to carry it in an `Arc`.)
- **The variant lists above are placeholders.** The real `NotApplicable` list
  falls out of writing down each strategy's "does not apply" conditions, which
  is also the precise definition of each strategy.

## Spike Findings (2026-09-30)

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
- **The boundary breaks nothing inside the repository**: no relative link
  leaves the repository root.
- **Relative links leave their own document's directory often**: 1,228 of
  4,104 resolved relative links (30%). Outside a repository this is what a
  `base_dir` defaulted to the document's directory would reject.
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
- The remaining 1,063 broken links are genuinely missing targets; `md clean`
  would report them as `TargetMissing` findings.
