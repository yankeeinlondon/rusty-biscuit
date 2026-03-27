# Claudine Composition

Claudine supports the ability to _compose_ content leveraging the Darkmatter library's powerful composition features.

We will discuss two kinds of composition:

- direct composition
- inline composition

Both forms share a lot of the key principles and features but the _inline_ style allows a _all-in-one_ file approach to composition which is particularly valuable to certain use cases. Before we go into the differences, let's start with the similarities.

## Composition Basics

A **majority** of the composition features we'll be leveraging in Claudine are by leveraging **Darkmatter**'s [composition features](../../../darkmatter/docs/topics/what-is-composition.md) the remaining of features which Claudine layer's on top are just some conventions, validations, along with the richer output of non-interactive sessions with Claudine.

> **Note:** the composition features can be used with both interactive and non-interactive sessions but we suspect you'll find that you can go further by leveraging the power of non-interactive sessions.

The way you'll use composition with the Claudine CLI includes two base syntaxes:

1. `claudine compose <file-ref> ...`
2. `claudine <agent> --compose <file-ref> ...`
    - examples: `claudine claude --compose ...`, `claudine codex --compose ...`, `claudine opencode --compose ...`

In the first syntax you are _deferring_ an explicit choice of which Agentic CLI you will use versus the second where the Agent is declared as part of the command.

### Simple Example

```sh
claudine codex --compose @commit.md
```

- Claudine then resolves the location of the `@commit.md` file using the file resolution functionality provided by [`biscuit-file`](../../../biscuit-file/README.md); this functionality includes treating the leading `@` character as a "magic path". Magic paths will attempt to resolve the "commit.md" file in multiple locations (in this order):
    - the base of the current repo root
    - if a monorepo:
        - the base of the current package area
        - if in a specific package then the root of that package
    - the user's home directory
- With the file resolved we now use Darkmatter's "compose pipeline on the file
- The resolved Markdown content is then used as a prompt to the Agentic CLI provider
- By default these prompts are non-interactive but you can switch to interactive by using the `--interactive` / `-i` switch
- In the non-interactive mode, we will:
    - pickup the Session ID from the Agent
    - stream the Agent's response to STDOUT but through a filter so that Markdown output (which most Agents report with) will be converted to Terminal enhanced Markdown and look a lot more presentable while streaming
    - the final output will be copied to the user's clipboard on completion


## Inline Composition

- will use the `prompt` property of the **commit.md**'s frontmatter
    - if there is not a `prompt` file then we will return an error
    - `<red><b>ERROR:</b></red> the file <blue>some-file.md</blue> does not have a <b>prompt</b> property in it's frontmatter!`
- pass this prompt through Darkmatter's **compose** pipeline
- then execute a non-interactive prompt -- using claudine -- to perform the work: 
    - get content: `claudine {agent} -n "{prompt}" --silent`
    - save to the Markdown files body
    - update the `last_updated` frontmatter (YYYY-MM-DD)


The _inline_ style of composition is a powerful way to keep Markdown documents in your repo up-to-date. It allows a caller to reference a markdown file and have the `prompt` property of it's frontmatter be used as a non-interactive prompt to build the content for the body of the of document.

The _chained_ style of composition will use Darkmatter's composition transform on the referenced file, then pass that composed content as a prompt to an Agent. No files are mutated in this process but because we transform the prompt with Darkmatter's `compose` pipeline, the prompts we are passing in can act in a dynamic manner and provider far greater reuse then a static prompt.

Another dimension to how Claudine provides composition features is whether you want the _caller_ or the _reference file_ to be the main determinant of which Agent to use.

## Using the `--frontmatter-prompt` for Inline Composition

Using the `--frontmatter-prompt` is an example of an _inline_ composition where the agent being used is explicitly specified by the caller.

> **Note:** `--frontmatter-prompt <file>` or `--fp <file>` is the CLI switch

Running the following command:

```sh
claudine codex --frontmatter-prompt some-file.md
```

- claudine then resolves the location of the `some-file.md` file using [file reference resolution](./file-reference-solution.md)
- will use the `prompt` property of the **some-file.md**'s frontmatter
    - if there is not a `prompt` file then we will return an error
    - `<red><b>ERROR:</b></red> the file <blue>some-file.md</blue> does not have a <b>prompt</b> property in it's frontmatter!`
- pass this prompt through Darkmatter's **compose** pipeline
- then execute a non-interactive prompt -- using claudine -- to perform the work: 
    - get content: `claudine {agent} -n "{prompt}" --silent`
    - save to the Markdown files body
    - update the `last_updated` frontmatter (YYYY-MM-DD)

## Using the `--compose` Switch for Chained Composition

Like the `--frontmatter-prompt` CLI switch, the `--compose` switch is available on all claudine wrapper functions (claude, codex, gemini, etc.) but unlike the `--frontmatter-prompt` using `--compose` performs a chained composition.

For example:

```sh
claudine opencode --compose some-file.md 
```

In this example:

- we resolve the location of the `some-file.md` file using `biscuit-file`'s `FileReference` struct
    - if the file can not be resolved we return an error
- we transform the Markdown using Darkmatter's compose pipeline
- we then pass the chosen Agent (in this example it's `opencode`)

## Using the `compose` command to Abstract Agent Choice

In our first example we were explicit about _which_ agentic CLI we intended to use by using an agent subcommand (codex, claude, etc.) and modifying it with the `--frontmatter-prompt <file>` switch. Sometimes, however, it can be useful to defer the choice of which agent to use 
to the file we're referencing. To do this we have a top level subcommand called `compose` which allows the file we're referring to suggest which agent to use:

The compose command has two major variants:

1. Inline composition of the `prompt` frontmatter
2. Chained composition

### Inline Composition with `compose`

The first variant is VERY similar to the `--frontmatter-prompt` based approach already discussed:

- takes a file reference like `claudine compose inline <file>`
- looks for help on choosing the appropriate agentic CLI based on the referenced `agent` (see Agent Selection process)
- once the agent has been determined, this command performs exactly the same transformation of the file reference as the prior approach did.

#### Agent Selection

> **Note:** the `compose` command provides a CLI switch `--exclude <agent>` which allows the calling user to avoid the use of certain agent providers. The rules below will only select agent's which have not been excluded via the `--exclude` flag. The CLI's shell completions will help a user to choose a valid agent name when using this switch.
>
> **Note:** the `compose` command provides a CLI switch `--interactive`/`-i` which will ensure the agent used is asked interactively of the caller (the "agent selection" below will be used to set the default value of the select input)

- `agent` property exists in reference file:
    - Singular Match:
        - if the reference file that this command was passed has a frontmatter property called "agent" then we will use it to help us determine the agent to use
        - if the lowercased version of the `agent` property is a string subset of one and only one of our "supported CLI's" then we will use that as the preferred agent to use:
            - if the current host system does not have this agent installed we will report an error:
            - `<red><b>ERROR:</b></red> The agentic platform "{agent}" -- <i>which the referenced file specifies as the preferred agent</i> -- is not installed on this computer!\n\nInstall the agent software or if you want to <b>override</b> the agent to be used you can set the environment variable AGENT to the agent you prefer (e.g., <blue>AGENT=claude claudine --frontmatter-prompt {file}</blue>)`
    - Multi Match:
        - if the lowercased version of the `agent` property is a string subset of _more than one_ of our "supported CLI's" then we will present an interactive select dialog to force them to choose explicitly
    - **true** or `interactive`:
        - if the agent property is either `true` or `interactive` then we will always provide an interactive agent choice (only installed agents included, default selection will be the "favorite agent" in repo config or user config if not in repo)
- no `agent` property in referenced file:
    - the favorite agent defined in config (repo config, user config if not in repo) will be tried but if it fails it will automatically retry with the second favorite agent
    - if the second favorite agent fails too then we will return an error

### Chained Composition with `compose`

The second variant of the `compose` function does not mutate any files in the filesystem nor does it expect the referenced file to have a `prompt` frontmatter property. Instead, the default `compose` subcommand will:

- resolve the file reference (to the Markdown file)
- run darkmatter's compose function on the reference file
- send the composed Markdown content to an Agent as a prompt

```sh
claudine compose <file-ref>
```


## Summary Process

1. Leverage the `FileReference` struct from **biscuit-file** to resolve the filepath to the CLI's file reference parameter
2. Transform content using Darkmatter's compose pipeline; where content is:
   - the `prompt` property for _inline_ operations
   - the full document's content for _chained_ operations
3. Wrap the execution -- using **claudine** of a non-interactive agent session to process the prompt:
   - the prompt is the frontmatter property `prompt` (after Darkmatter compose) for inline operations
   - the prompt is the entire document (after Darkmatter compose) for chained operations

The utility of these processes are:

- Inline operations are used to update the _body_ of a document using a composed prompt and a non-interactive agent session
    - this is a common way to compose research or build up content that will be made into a "skill"
    - there are, of course, many other use cases too
- Chained operations are used to compose a reference file and then use it to prompt a non-interactive agent session
    - this provides a way to take a static "template" and inject dynamic content into the content before using it as a prompt to an Agent
