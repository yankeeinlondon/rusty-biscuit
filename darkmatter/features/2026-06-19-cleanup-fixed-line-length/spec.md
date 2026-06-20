In Markdown we have a **clean** operation -- which is reachable from the terminal via `md clean <file>` and it does a number of things to
help a markdown file be as standard based as possible.

One thing it doesn't do currently is address a Markdown file who's lines have been fixed to a certain length.
LLM's commonly do this at 80 or 100 characters. I guess the idea is that this is a "feature not a bug" because
strictly from an spec standard there is nothing wrong with putting a single `\n` to truncate.

> Note: in Markdown two or more `\n` characters represents a paragraph boundary but a single `\n` has no semantic
> meaning.

The problem though is that Markdown is a format for _writing_ first (reading second) and if you are adding in extra `\n`
because you think it might look better in some editors (aka, those without word wrap turned on) you are at the same time going against
the principle of Markdown's "notational velocity" which aims to provide an author a clean surface to write content
and stay in the flow. Ironically you might also be making it look far worse if it's being displayed in a terminal
with less columns than your cutoff point.

So in summary, in theory there is nothing with a bunch of single `\n` characters added to a Markdown file but
in practice it most typically not the best choice.

## Feature

- by default Darkmatter's library should include removing all single `\n` characters.
    - we must apply Markdown nuance
    - for instance, if the line with the `\n` we are about to remove has a whitespace preceding the `\n` then we simply remove the `\n` character but if it doesn't then we replace the `\n` with a space.
- there should be an option in the library and the CLI to also:
    - be **neutral** to the input and ignore whatever single `\n` pattern is being used in document
    - allow a fixed length to specified (`ch` as unit):
        - the caller specifies the fixed length they prefer
        - again we must avoid the naive implementation which ignores the inputs single `\n` and just adds new `\n` at the requested fixed length
        - we do two passes (remove all single `\n`, then add new fixed length) which is likely the most obvious solution
        - or maybe there is a single pass solution which can arrive at the same correct outcome
        - the key thing to keep in mind is that everything in the "compose" pipeline (which includes the "clean" operation) must be performant

### CLI

- `md clean <file>` - will by default remove all single `\n` found in the content
- `md clean <file> --fixed-width <#>` - allows the caller to specify a new width to use as the fixed with
- `md clean <file> --ignore-incidental-carraige-returns` - will ignore the single `\n` characters and make no mutations to the input in this regard
    - Note: I'm completely open to a better name than `--ignore-incidental-carraige-returns` if you have a good replacement then mention during the clarification process
