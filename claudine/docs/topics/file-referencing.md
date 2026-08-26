# File Referencing

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

- `.` - refers to the operating system's "current working directory" 
- `..` - refers to the _parent_ of the "current working directory"
- `~` - refers to the current user's home directory

In addition to the most common sigils above, we're also all familiar with different for the ROOT of the host's file system:

- `/` - used in all POSIX operating systems to represent the root
- `\` - used by Windows (and apparently cavemen) to represent the root
- `file:///` or `file:/` - used by IETF as RFC 8089 (e.g., the `file` URI Scheme) to represent the root

**Claudine** (by way of **biscuit-file**) provides the following _additional_ sigils:

- `@` 
    - a "magic path" sigil which is multi-homed
    - being multi-homed means that it will try _multiple_ base directories at runtime but resolve to just one at runtime
    - many Agentic CLI's use some form of this sigil to help users to autocomplete or reference a file path
    - magic paths _prefer_ the most specific/localized path that is valid
    - the base paths -- _in order of precedence_ -- which will be used are:
        - repo's package root (if CWD is inside a monorepo and inside a package of that monorepo)
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
    - The `^` sigil is automatically used when a file reference falls into the pattern we refer to as an "ambiguous relative path".


> **Note:** the `@`, `&`, and `^` sigils are _defensively_ coded so that a following `/` character has no impact
> on the file paths which are evaluated:
>
> - `@path/to/file` is the same as `@/path/to/file`
> - `&path/to/file` is the same as `&/path/to/file`
> - `^path/to/file` is the same as `^/path/to/file`

## The Ambiguous Relative Path

All file references can and should be thought of as as a **base** path joined to a **relative** path:

- what an OS would call an **absolute path** is just something like `/path/to/file.md`
    - the **base** is just `/` and the rest represents a path from that base
- what an OS would see clearly as a relative path is something like `./path/to/file.mdto/path.md`
    - a relative path _doesn't_ have 


## Concept: Design Time vs Run Time

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

### Eager or Lazy Evaluation of Frontmatter

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


### Absolute, Relative, and Abstracted File Paths

In operating systems we're used to two basic file references: _relative_ and _absolute_ file paths: 

- An **absolute** file path starts from the recognized root of the operating system's root file system and explicitly defines the entire path needed to get to the file
    - this form of file reference can only point to a single location on a given host's file system 
    - it leaves **zero ambiguity** on the where the file should be found
    - while this form of file referencing has no **ambiguity** it trades that for less **portability** 
        - across hosts absolute paths tend to be bad idea
        - absolute paths should NEVER be used in repos that by their very nature pivot of a _root_ in the file system that is highly dynamic
    - examples include:
        - `/etc/resolv` uses the `/` sigil to specify it is an absolute path
        - `file://etc/resolv` use the more formal `file://` sigil which indicates both that it's an absolute path but also that it should be treated as a file
        - `\Users\Bob\.env` uses the `\` sigil which Windows users will be familiar with
- A **relative** path describes a file path _relative_ to another file path
    - relative paths often start with a `.` or `..` sigil to indicate both that it IS an explicitly _relative_ path and that it is relative to the _current working directory_
    - more opaque relative paths are often used too like `foo/bar/baz.md` 

- relative paths use symbols like `.` to represent the OS's _current working directory_, `..` to represent the parent of the current working directory
    - assuming the "base directory" is clearly established and unambiguous -- and especially when used inside a **repo** -- relative paths are highly useful as they become highly portable across hosts

Though less commonly discussed there are also a category of file references which we will call **abstracted** file references:

- an _abstracted_ file reference provides some form of indirection to a part of the file path; in this document we'll focus primarily on abstractions which define the "base directory" but indeed a file path could also include 
- the most commonly used example is the `~` symbol which most OS's will infer to mean "the current user's home directory"
    - in this example something like `~/foo/bar/baz.md` uses `~` as an abstracted base directory to allow for a relative path to be used as an offset
    - in fact, one could refer to an absolute path as nothing more than a relative path off 
- the next most common example is the use of an environment variable:
    - `${HOME}/foo/bar/baz.md` this path depends on the `HOME` environment variable being set and being a valid file path
    - many OS's will set `${HOME}` for you 

#### Examples

1. `/etc/resolv` is unambiguously an absolute file path based on the leading `/` sigil in the file path
1. `file://etc/resolv` is unambiguously an absolute file path which uses the `file://` protocol specifier to more explicitly state scoping
1. `./children/one.md` is unambiguously a relative file path based on the leading `.` which means "current working directory"
1. `children/one.md` _suggests_ that it is relative path but is not explicit about it's base
1. `../bar.md` is unambiguously a relative file path of the _parent_ of the current working directory
1. `~/.env` is _abstracted_ file reference 

### Repos as Bounding Scope


## In Practice

Claudine provides the following **sigil**'s which can be used to reference the BASE for a relative file reference.

for providing a base or _set of bases_ for a file reference:

- `file://`, `/`, `\` all map to the base of the host OS's file system and are considered "absolute paths"
- `.`
