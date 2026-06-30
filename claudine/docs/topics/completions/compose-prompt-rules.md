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

By default, the paths you provide are interpreted as _relative_ file paths from any the document roots listed above but that means that multiple matches with the same filename are possible. To handle this we will always resolve to the most "local" variant. By _most local_ we mean the document root 

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
