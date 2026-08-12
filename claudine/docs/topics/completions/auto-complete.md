---
sections:
    - "": prompt
        
---
# Auto Completion in Claudine

## Completing Prompt Files

Claudine provides the ability to "auto complete" _incomplete_ CLI submissions for all of the [composition](../composition.md) subcommands (e.g., `compose`, `inline-compose`, `sequence`). Unlike shell completions there is no required installation/configuration step needed other than having the Claudine CLI installed on the host.

### `compose` Prompts

Claudine expects, by convention, that your _prompts_ used in a `claudine compose <prompt>` command to live in a certain set of directories:

- these rules are defined in [**Compose** Prompt Rules](./compose-prompt-rules.md)
- these allow for partially specified prompt file to be auto-completed for you

For example let's imagine that:

#### Confirmation on Solo Match

- you have a prompt file `{repo-root}/prompts/review-spec.md` which you use to have a draft specification file be reviewed for inconsistencies, completeness checks, etc.
- the only other prompt file you have is `{repo-root}/prompts/review-implementation.md`
- the specification file you want to have reviewed is found at `{repo-root}/features/do-it.md`
- you type `claudine compose review-spec spec="features/do-it.md"` and press ENTER

In this use case, Claudine is able to determine that only **one** prompt file matches your reference to the prompt file and it responds to popping up an interactive prompt just to be 100% sure that that was indeed what you intended to use.

Claudine wants to reward your laziness but not recklessness.

#### Addressing Multiple Matches

- using the same baseline as the previous example
- imagine you type `claudine compose review spec="features/do-it.md"` and press ENTER

Now Claudine recognizes that _more_ than one match exists for "review" in the possible prompts in the repo. In this case a TUI is brought up to allow you to specify which of these you intended to use.

The selection of the prompt resolves the ambiguity on which prompt the user wanted to use but it also considered confirmation and will run the prompt without any additional confirmation.

### `inline-compose` Prompts

The **compose** operation expresses it's prompt in the body of the document and doesn't _explicitly_ have any expectation on what files the prompt will mutate.

In contrast, the **inline-compose** operation self-contains one or more agent prompts in the `prompt` or `sections` Frontmatter properties but more importantly it has a very explicit goal of mutating the body of this file with the Agent's actions.

This means that the way in which a Markdown document becomes identified as a
candidate for _completion_ in the **inline-compose** operation is completely different:

- for details on the _rules_ for inline-compose see: [`inline-compose` prompt rules](./inline-compose-rules.md)
- in summary though, all Markdown files under the _current working directory_ which define either a `prompt` or a `sections` Frontmatter property are candidates
- outside of which documents are considered

## Completing Setter Values

The same "reward laziness, not recklessness" principle extends to the **setter values** you pass after the prompt (`spec=...`, `cover=...`), not just the prompt file itself.

When a prompt's `$schema` declares a property as a `file` (or `file[]`) with a `match(...)` glob — for example `spec: 'file(required; match(**/*spec*.md))'` — a value you supply that does not resolve to an existing file is treated as a **partial** rather than an error:

- imagine your only spec lives at `{launch-area}/features/2026-06-30-style-everywhere/spec.md`
- you type `claudine compose plan spec=everywhere` and press ENTER
- Claudine walks the `match(**/*spec*.md)` glob from the **launch area** (the same anchor completion suggestions use, so what is _offered_ is always what is _accepted_), then keeps only candidates whose path contains `everywhere` (case-insensitive)

The resolution then follows the familiar pattern:

- **one match** → a confirmation dialog ("Use this file?") before proceeding
- **more than one match** → a chooser TUI to disambiguate
- **zero matches**, a declined confirmation, or a non-interactive session → the original `no existing file matched reference` error is preserved unchanged

The glob is only consulted _after_ a literal path fails, so valid explicit paths behave exactly as before.
