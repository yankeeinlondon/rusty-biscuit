# Claudine Sequences

In this feature we will be introducing a new capability for the `compose` subcommands of Claudine. This feature is called **Sequence** and it is represented by a Markdown document which sets the `sequence` property as a YAML list of either string values or a dictionary of key/value pairs (but requiring that the "name" property be defined and be a string).

When a `sequence` is detected, Claudine starts up a serial process of composing each element in the list:

- Each list item (starting with the first), will compose the prompt document with the `sequence` property:
    - The `state` variable will be set to the current sequence item being processed
    - The `previous_state` and `next_state` properties will be set with the _previous_ and _next_ items in the sequence respectively
        - if the current `state` is working on the first item, then `previous_state` is null,
        - if the current `state` is the last item, then `next_state` is null
    - Even though `previous_state` and `next_state` _could_ be used to detect if the sequence is at the beginning or end, Claudine provides `is_first` and `is_last` boolean flags that a user can use to operate conditional logic on the Markdown document.
    - in the same vein, Claudine's Sequence functionality will set `step` and `total_steps` to the current step number and the total number of steps in the sequence. This can be useful in reporting progress or other measures though Claudine does report on the Sequence's progress itself too.
- If any of the _steps_ in the sequence ends in error then we will by default exit the Sequence immediately with a clear error message to the user on what has happened.
- If you want a sequence to report errors but _continue_ with the sequence if when one of the steps fails, you can do this two ways:
    - set the `fail_fast` frontmatter to `false`; this changes the "default behavior" for anyone running compose on this document
    - the CLI should offer an optional `--fail-fast <boolean>` CLI switch that allows a one-time override of the default behavior for errors 
- regardless, the ENV variable `FAIL_FAST` will be set to true or false when a document has the `sequence` property in it

## Simple Example

If there were a document called `test.md` which looked like:

~~~md
---
sequence:
    - one
    - two
    - three
---

Come up with a great joke to tell at a party.

Save the joke to "./{{state}}.md"
~~~

and then the user ran:

```sh
claudine sequence "foobar.md"
```

- Claudine would detect the `sequence` property and then run the prompt three times (once for each state)
- Each time the Markdown file is used as a prompt, it is first "composed" (e.g., the Darkmatter composition pipeline) with each step's state
- Because the prompt uses the `{{state}}` variable to direct the output the end results is that three files with a joke in each one are created: `one.md`, `two.md`, and `three.md`.

## Adding Structured State

In the first example each step in the sequence just consisted of a string for state. If we need to have multiple variables which represent state for each step this too is possible. Here's an example configuration that would be considered valid:

```md
---
sequence:
    - name: "one"
      color: "red"
    - name: "two"
      color: "blue"
    - name: "three"
      color: "green"
---
```

In this example we've extended our definition of state to include two independent variables `name` and `color`. When we move to this dictionary based definition of state, the only **required** property is `name` but you can add any other key/values you like to each state.

## Referencing State in External YAML

While most basic sequences are defined _inline_ of a Markdown page's Frontmatter you also can use a file reference to a YAML file if you prefer. In this configuration style you're Markdown file might look something like:

```md
---
sequence: path/to/file.yaml
---
```

Then the YAML file which is being referenced would be required to have a root node of `sequence` and a definition following exactly the same structure/rules as an _inline_ sequence definition would. Here's an example of what the YAML file might look like:

```yaml
sequence:
    - name: "one"
      color: "red"
    - name: "two"
      color: "blue"
    - name: "three"
      color: "green"
```

While both forms of configuration are perfectly valid, using an _external_ reference can have strong reuse advantages for many use cases. 

### Template Section of Referenced State

The external referencing style not only provides some useful reuse opportunities but also opens up a feature only used in an externally referenced YAML file: the "template".

```yaml
kind: sequence
template:
    desc: "{{name}} (_site: {{site}}, repo: {{ repo || 'n/a' }}_)"
list:
    - name: Claude Code
      file: ./claude.md
      site: https://code.claude.com/docs/en/overview
      repo:
    - name: Codex CLI
      file: ./codex.md
      site: https://developers.openai.com/codex/cli
      best_practices: https://developers.openai.com/codex/learn/best-practices
      repo: https://github.com/openai/codex
    - name: Gemini CLI
      file: gemini.md
    - name: Goose CLI
      file: goose.md
      site: https://block.github.io/goose/
      repo: https://github.com/block/goose
    - name: Kimi Code CLI
      file: ./kimi.md
      site: https://moonshotai.github.io/kimi-cli/
      repo: https://github.com/MoonshotAI/kimi-cli
    - name: OpenCode CLI
      file: ./opencode.md
      site: https://opencode.ai
      repo: https://github.com/anomalyco/opencode
```

In this example configuration, we define distinct values for the `name`, `file`, `site`, and `repo` information but we also provide a Template called `desc`. Having this template means that _every_ state in the sequence will be provided the `desc` property and the content for this property can leverage Darkmatter's interpolation engine to integrate the other state properties into a summary property composed from various other pieces of "state".



