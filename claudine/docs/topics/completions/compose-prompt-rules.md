# Compose Prompt Rules

## Rules

Claudine expects, by convention, that your _prompts_ used in a `claudine compose <prompt>` command to live in a certain set of directories:

- `{repo-root}/prompts`,
- `.claudine/prompts`,
- `.claude/commands` (for backwards compatibility; note we only use locally scoped commands not user scoped),
- `~/.claudine/prompts`

> **Note:** when working inside a monorepo, this will be extended to include (note: that these two document root are considered the most "local"):
> 
> - `{package}/prompts`
> - `{package-area}/prompts`

In addition to focusing our auto complete candidates to the aforementioned directories we also:

- exclude any Markdown documents that include 

all Markdown documents contained in a directory starting with the `_` character are automatically _excluded_ from the auto-completion candidates. 

### Local Wins

By default, the paths you provide are interpreted as _relative_ file paths from any of the document roots listed above but that means that multiple matches with the same filename are possible. To handle this we will always resolve to the most "local" variant. By _most local_ we mean the document root closest to where you launched Claudine, and the rule behind that is simple:

> **The local file tree resolves first.** Its root is the repository root when you launch inside a repository, and the launch directory itself when you don't — whether or not that directory lives under `$HOME`. Only when the local tree can't resolve a reference do we fall back to your home directory.

So every local root — `{package}/prompts`, `{package-area}/prompts`, `{local}/prompts`, `{local}/.claudine/prompts`, `{local}/docs`, the local agent-skill directories, and the package, package-area, and local roots themselves — is searched before `~/.claudine/prompts`, `~`, or `~/.claudine`. A `commit.md` sitting in your `~/config/sh` repository (or in a plain `~/scratch` directory) beats the `~/.claudine/prompts/commit.md` you keep for everywhere else. Your user prompts are still reachable from any directory; they just never shadow a local file with the same name. The full order is listed under [Scopes](./shell-completions.md#scopes).

The same holds when you launch from `$HOME` itself: `~/.claudine/prompts` and `~/.claudine` are always treated as _user_ locations, so they stay behind your local files even when the local tree and your home directory are the same place.

#### Prompts that live somewhere else

A prompt doesn't have to come from the local tree — `@commit.md` may well resolve to `~/.claudine/prompts/commit.md` or to a prompt in another repository. When _that_ prompt contains references of its own, they are resolved like this:

- **`@` references** keep the launch tree: a nested `@style.md` is searched for in the tree you launched Claudine from first, then in your home locations — not in the directory or repository the prompt was loaded from. `@` means "the usual places for this invocation", so a shared user prompt picks up the local project's files.
- **`./`, bare, `&`, and `^` references** keep their source-relative meaning: `./style.md` and `style.md` resolve from the prompt's own directory (a bare path then falls back to the prompt's repository root), and `&` / `^` anchor on the prompt's own repository and packages.

In other words, pick `@` when a prompt should adapt to wherever it's run, and a relative path when it must always use its own companion file.

## Magic Paths

In addition to "relative" file paths, which are often the most 

## Goals for Rules

This ruleset allows:

- a compact set of possible Markdown documents to autocomplete from
- allows prompt authors to provide helper documents for which they **don't** want to be used for completion purpose to be nested inside one of the prompt directories under a subdirectory starting with `_`.
- 

## Living Outside the Law

> "I fought the law, and the law won"
> 
> - [The Crickets, 1960](https://tvtropes.org/pmwiki/pmwiki.php/Main/IFoughtTheLawAndTheLawWon)

Are you a rebel? No respect for the law? Do you want to use the **compose** command against prompt files that _don't_ live in the document directories provided to you by Claudine?

Well you're allowed to break the rule but don't expect Claudine to help you! Claudine is a law abiding citizen and simply looks the other way at your reckless view on life.

To run a directory in a non-blessed directory just supply the full relative (from where you started Claudine) or absolute file path. No auto-complete for you! 

## Back Links

- back to [auto-complete](./auto-complete.md)
- back to [shell-completions](./shell-completions.md)
