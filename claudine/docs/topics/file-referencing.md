# File Referencing

> **Design-Intent Document.** This topic describes the *target* design for
> file referencing. It intentionally may diverge from the current
> implementation while the design is being finalized. Do **not** "drift-correct"
> this document against the code — where the two disagree, treat the
> disagreement as an open design decision and surface it to Ken rather than
> editing this file. Changes to this document require Ken's explicit approval.

In **Claudine** (_and the underlying **Darkmatter** library_) we rely on a consistent way of allowing for file referencing that is:

1. Deterministic
2. Ergonomic
3. _and_ Performant

A file reference may -- _at design time_ -- be somewhat ambiguous to the full file path it will expand to at _run time_ but it should never be **surprising**. In order to achieve this aim we must have a well grounded set of rules that we follow consistently. This document will focus on **first principles** of what an author and or user of Claudine should expect in terms of _how_ and _where_ a file path will be resolved from.

## Sigils

> Dictionary Definition: **sigil** 
>
> _a symbol or sign that is believed to have magical power or to represent a specific intention._

When _referencing_ files we rely on an enumerated set of **sigils** that provide a useful abstraction to a base for our file path reference. The most common examples include:

- `.` / `./` _and_ `.\` - refers to the operating system's "current working directory"
- `..`, `../` _and_ `..\` - refers to the _parent_ of the "current working directory"
- `~` - refers to the current user's home directory

In addition to these sigils, file references may use native absolute-path and URI forms. These forms are roots or namespaces, rather than additional base-directory sigils:

- `/` - used in all POSIX operating systems to represent the root
- `{drive}:\` _or_ `{drive}:` - represents the root of a Windows drive like `C:` or any other mapped drive
    - `C:\` would be an example
    - as would `C:`
- `file:///` or `file:/` - are local-file URI forms defined by RFC 8089; their root semantics depend on the path that follows
- `file:///C:/` - is the URI form of a Windows drive root
- `file://server/share/` - is the URI form of a Windows UNC share root
- `\\?\` and `\\.\` - more obscure Windows referencing

Windows also has a drive-relative form that must not be confused with a drive-absolute path:

- `C:path\to\file` - is relative to the current directory associated with the `C:` drive; it does **not** mean `C:\path\to\file`

**Claudine** (by way of **biscuit-file**) provides the following _additional_ sigils:

- `@` 
    - a "magic path" sigil which is multi-homed
    - being multi-homed means that it will try _multiple_ base directories at runtime but resolve to just one at runtime
    - many Agentic CLI's use some form of this sigil to help users to autocomplete or reference a file path
    - magic paths _prefer_ the most specific/localized path that is valid
    - the base paths -- _in order of precedence_ -- which will be used are:
        - repo's package root (if **CWD** is inside a monorepo and inside a package of that monorepo)
        - repo's package-area root (if CWD is inside a monorepo and inside a package area of that monorepo)
        - repo's root directory (if CWD is inside a repo)
        - the user's home directory
    - a pattern many developers will be familiar with is referencing an Agent Skill such as: `@.claude/skills/do-me-like-that/SKILL.md`
        - this will TRY to use the repo's skill definition first, but if it doesn't exist then
        - it falls back to `~/.claude/skills/do-me-like-that/SKILL.md`
- `&` **Repo References**
    - provides local repo references
    - it's cheap and cheerful, the `&` sigil is always replaced one for one with repo's root directory
    - this sigil does not consider packages or package areas that might exist in a monorepo
    - `&file/to/path.md` will look for `./file/to/path.md` from the root of the repo's directory
    - it is INVALID to use the `&` sigil in any file that is _not_ in a repo
- `^` **Monorepo References**
    - provides a more discerning variant of the `&` sigil useful in monorepos
    - just like the `&` sigil, it only allows for references to files within the repo and any file using the `^` sigil MUST be in a repo
    - however, the `^` actually behaves very similarly to to the magic sigil in that it is multi-homed and it prefers the most local/specific version of a file
    - the base paths -- _in order of precedence_ -- which will be used are:
      - repo's package root (if CWD is inside a monorepo and inside a package of that monorepo)
      - repo's package-area root (if CWD is inside a monorepo and inside a package area of that monorepo)
      - repo's root directory
    - The key difference from the magic path is that it does **not** reach into the user's home directory as a fallback. This is by design of course but it can also help to avoid a common pitfall when accidentally running into a security violation or an unwanted human-in-the-loop intervention because the current agent is happy to operate on repo file but not on user scoped files.
- `vault:` **Obsidian Vault(s)**
    - this is made reference to here as this will soon be added but currently this is "FUTURE SCOPE"


> **Note:** the `@`, `&`, and `^` sigils are _defensively_ coded so that a following `/` character has no impact
> on the file paths which are evaluated:
>
> - `@path/to/file` is the same as `@/path/to/file`
> - `&path/to/file` is the same as `&/path/to/file`
> - etc.

## The Implicit Relative Path

All file references can and should be thought of as a **base** path joined to a **relative** path:

- what an OS would call an **absolute path** is just something like `/path/to/file.md`
    - the **base** is just `/` and the rest represents a relative path from that base
- an OS would see something like `./path/to/file.md` _or_ `../path/to/file.md` as a **relative path**
    - a relative path doesn't have an explicit **base** path _yet_ but as we've already established:
        - `.` will be converted to the current working directory at run time
        - `..` will be converted to the parent of the working directory at run time
- the `~` sigil acts in the same way ENV-based paths do, such as the `${HOME}` path commonly used in shell scripts
    - a path of `~/path/to/file.md` or `${HOME}/path/to/file.md` has an _abstracted_ base path that is resolved at run time
    - on Windows, `%USERPROFILE%\path\to\file.md` is the corresponding native environment-variable form
    - environment-variable references are substitutions, not additional file-reference sigils; `~`, `${HOME}`, and `%USERPROFILE%` are the **base** paths and `path/to/file.md` is a relative path off of that base

So what then is an **implicit relative path**? It's a path that starts immediately with a relative path segment but without adding in a clear marker for what this relative file path's base should be. An example would be:

- `path/to/file.md`

Without a sigil to guide us, Claudine and Darkmatter must determine what  _base directory_ will be joined with the relative path segment provided. In this case, the solution is to take a dual-pathed approach:

1. if `path/to/file.md` is a valid path off of the _current working directory_ then that is matched first: `${CWD}/path/to/file.md`
2. if the **CWD** based file path doesn't match then we will match on the repo's root: `{repo-root}/path/to/file.md`

Of course if the _current working directory_ is not in a repo then we can ONLY consider the CWD path.

## What is the Current Working Directory?

> formerly titled `What's the frequency Kenneth?` and before that `Yeah but who's on First?`

### Why it's not Obvious

When you run `claudine compose prompt.md` the _current working directory_ (**CWD**) that `prompt.md` will use in its file references can not be ambiguous. While the goal is for the **CWD** to be intuitive and ergonomic for both the prompt author _and_ the operator who is executing the prompt; these two actors sometimes might not see **CWD** the same way:

- the operator _executing_ a prompt is intentionally in a certain directory and would expect that _where they execute_ a prompt should be considered the current working directory
- the prompt author has no idea which directory a future operator will execute from and it will obviously change dynamically on a per call basis
    - there may be some _fancy_ cases where an author might want to play off of where the prompt is being executed out of; however
    - in most cases the most _intuitive_ answer for the prompt author is that the _expected_ current working directory for an author is the directory in which the current file resides

### Further Divergence in Actor Goals

Now let's consider a quite common situation in which an operator is calling `claudine compose` and passing in a path based parameter:

```sh
claudine compose prompts/doit.md spec=features/the-big-one/spec.md
```

- from the operators perspective it makes perfect sense that the relative file path being passed in as `spec` will be relative to the current directory
- in this case the prompt author's role was was likely just to provide the schema type for the `spec` property (although possibly with a "default" path)
- the author doesn't have any direct skin in the game on how the spec file should be resolved into a fully qualified file path 

### And Now For Something Completely Different

While not as directly involved in how to resolve CWD it must be mentioned that:

- when Claudine starts up the CLI it immediately _changes the directory_ to the repo root before doing anything else
- it DOES keep track of the directory in which the CLI was started in:
    - `ctx.cwd` directly reports on this
    - the ENV variable `${AGENT_CWD}` reports on this
    - the `ctx.area`, `ctx.package`, and  `ctx.package_area` context variables are derived from the real starting directory 
- Why do we change to the repo root directory first (at least when the CLI is started in a repo)? Well because it's almost always the right thing to do from a permissions standpoint as well as helping the Agent to find skills, prompts, and other things.

### Addressing the Great Divide

In all cases a prompt author or the operator _could_ opt to use the `&` or `^` sigil's to explicitly express their intentions. Sadly there is a great divide between _could_ and _should_ / _would_. A well designed solution can't offer good defaults for this important variable.

It is an obvious solution to each actor:

- the prompt author believes **CWD** _obviously_ should be the file which they are authoring
- the operator calling the prompt believes **CWD** _obviously_ should be the directory they are calling the prompt from

The solution may surprise you but give it a second thought and I think you'll agree with the approach:

1. **Ruling:** 

    When _composing_ a Markdown file in Darkmatter or Claudine the **CWD** should be the directory of the file being composed; not the caller's directory!

    If you are a "caller" we appreciate that you might be angry. We get it. Yes we know that convention would suggest that where you were calling from should set **CWD**. Unfortunately like all real rulings, this one has been ruled, so let's discuss some tools that do line up in your favor:

    - `ctx.cwd` can be used in a prompt anytime you want to reference the callers **CWD**
    - `ctx.area`, `ctx.package-area`, and `ctx.package` all DO use the directory from which you called the CLI

    Ok so hopefully you've stopped your sobbing now. Now for the really good news ... we've created an "exception clause" that addresses the edge between passing in parameters and the composition flow.

2. **Exception Clause:** 

    - if the caller passes in a file reference as a Frontmatter property the file path will be resolved as it's passed in
    - if the Frontmatter property is set with **eager** evaluation than it will be resolved and validated; if not then it will just be resolved
    - during this resolution process the caller's **CWD** (the original CWD not the repo root) will be used as the CWD during this process.

    This clause allows callers to ergonomically pass in file references with a CWD directory that makes sense to them and then the core composition process across a recursive set of files uses the file's directory as **CWD**.

> **Note:** all references to `claudine compose` apply equally as well to `claudine inline-compose` and `claudine sequence` ... basically what we're referring to is the act of composition.

## Design Time vs Run Time

When a file reference is made _inside a file_ this is deemed to be "design time" reference because a Markdown file is not immediately executable and with _composable_ content we value abstraction and reuse over absolutism. In contrast, when Claudine (or Darkmatter) _composes_ a document it is NOW "run time". Why does this matter?

- at design time you are afforded abstractions which allow for reuse
    - things like ENV variables and sigils like `.` are a _placeholder_ for some future value
    
- this means that at design time you CAN NOT **validate** a file reference
    - abstractions which _will_ be resolved at run time are not yet resolved, and 
    - files and paths which doesn't exist now may exist at the time this file is executed
    
    >  **Note:** you can -- _at design time_ -- invalidate some file paths if the static parts of the path include invalid characters; this type of invalidation falls to the responsibility of a language server like **
    
- at runtime all the _abstractions_ we valued at design time are traded in for a single and valid file path (or error if not possible); to **resolve** a file reference we must:
    - replace all ENV variables with their current values
    - replace sigils like `.`, `..`, and `~` with their file system representation
    - if we find an advanced sigil like `@` which supports multiple file path bases, we return the first valid file path that resolves

In Claudine, we _refer to files_ inside of files in many situations such as:

- shell expansion: `::shell foo/bar/doit.sh`
- conditional blocks: `::block when="file_exists(foo/bar/doit.sh)"`
- etc.


## Eager or Lazy Evaluation of Frontmatter

An edge case we must consider with Claudine is that we can pass in file references as Frontmatter. When we do this the schema type of the property matters and specifically whether the **eager** modifier is used to change the file reference into an eager evaluated reference or not.

If we have the follow prompt file `doit.md`:

```md
---
$schema:
    spec: file(eager;required)
    design: file(eager)
    plan: file
---
```

This prompt file will _eagerly_ evaluate any file reference passed in for the `spec` property but will default to lazy evaluation for the `plan` property. This means that if we executed:

```sh
claudine compose doit.md spec=features/something/spec.md plan=featurees/something/plan.md
```

In this example:

- the file path to `spec` file path must be a valid file reference or else an error will be raised
- the file path to `plan` will be treated as a file path but the file path being passed in -- _while needing to be a valid file path generically (e.g., no invalid characters in path)_ -- will not be checked at runtime to ensure that this file does exist in the file system
