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

## Example

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
claudine compose "foobar.md"
```

- Claudine would detect the `sequence` property and then run the prompt three times (once for each state)
- Because the prompt uses the `{{state}}` variable to direct the output the end results is that three files with a joke in each one are created: `one.md`, `two.md`, and `three.md`.


## Concurrent Actions in a Sequence


## The `sequence` subcommand
