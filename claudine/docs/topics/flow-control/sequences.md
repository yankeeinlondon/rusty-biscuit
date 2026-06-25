# Sequences in Claudine

A **sequence** in Claudine is a group of "steps" which will be executed serially.

![sequence overview](sequence-overview.png)

## Defining a Sequence

The definition of a sequence can be done in the `sequence` property of a Markdown document or as a YAML file. In both cases we use the same YAML schema to define our sequences.

### Simple Named Steps

The simplest way to define a sequence is just to provide a list of string names:

```yaml
sequence:
    - one
    - two
    - three
    - one
```

In the example above we will convert this to the data structure: 

```json5
[
    { id: "one", name: "one" },
    { id: "two", name: "two" },
    { id: "three", name: "three" },
    { id: "one-1", name: "one" },
]
```

This data structure represents a more complete definition of each **step**'s "state". The `id` property is guaranteed to be unique and does this by being the _dasherized_ version of `name` and if that's already been used it will add an index to it (e.g., `-1`, `-2`, etc.).

### Defining Step State as an Object

In our first example we just defined our states with a string but we're able to add more metadata by using a key/value dictionary:

- the only _required_ property is the "name" property
- like with the simple string definitions, the `id` property will be created for you
- if you **want** to define the `id` then you must define it for every step in the sequence and it must be unique
- all other key/values are available to you at your discretion

Here's a simple example of how you might define dictionary based state in a Markdown document:

```md
---
sequence:
    - name: Bob
      age: 32
    - name: Sally
      age: 36
    - name: John
      age: 18
---
Find the customer {{state.name}}, who is {{state.age}} years old in our corporate database 
and **append** all the information we have on him to "reports/dodgy-people.md".
```

#### YAML Templates

If you wanted to replicate the functionality in the last example but define sequence data externally in YAML you can do that:

```yaml
template:
    description: "{{name}} ({{age}} years old)"
sequence:
    - name: Bob
      age: 32
    - name: Sally
      age: 36
    - name: John
      age: 18
```

Externalizing the sequence data is useful for at least two reasons:

1. the data you're wanting to iterate over in a sequence is often highly reusable
1. when you choose the external representation, you can use the "template" section of the YAML to define a property which will be made available in every step's state. It can be static but typically it would reference and format information from the other structured data defined.

This YAML file could now be referenced in the Markdown like so:

```md
---
sequence: "path/to/data.yaml"
---
Find the customer {{state.name}}, who is {{state.age}} years old in our corporate database 
and **append** all the information we have on him to "reports/dodgy-people.md", add the information under an 
H2 heading of `## {{state.description}}`.
```

## Advanced Techniques

So far we've been showing you a very popular style of sequence which consists of changing key/values for each step's state but on each step we're using the body of the same document to act as the prompt.

> **Note:** this approach has a surprisingly large amount of mileage. Whether you're iterating over variants which you want to prompt with similar prompts (this is what we do this repo often for research across the various providers we're supporting) or leveraging Darkmatter's `::block` templates to swap in and out various parts of the template based on the state.

In this section, however, we'll explore some additional techniques which **sequences** are allowed to do:

### Headless Sequences

 We can define a _headless sequence_ by defining the YAML like we've done before but instead of then _referencing_ that YAML definition in a Markdown document we instead point the **claudine** CLI directly at the YAML:

 ```sh
 claudine sequence @defn.yaml
 ```

- the power of the **headed** sequences comes from running a sequence of "states" over the Markdown document which acts as a prompt
- a **headless** sequence has no root document to act as the _prompt_ but instead leverages a combination of:

    - Prompt References (`prompt` prop)
    - Shell Command Blocks (`shell` prop)
    - Named Groups (`group` prop)

Let's start with an example of the first two (prompt references and shell command blocks):

```yaml
template:
    - dir: "path/to/some/location"
sequence:
    - name: Design
      prompt: "@prompt/design.md"
    - name: Implement
      prompt: "@prompt/implement.md"
    - name: Commit
      shell: "just lint"
    - name: Review
      prompt: "@prompt/review.md"
```

- in this example, we will serially run through the different steps/states just like we would with a **headed** sequence
- but where the **headed** sequence would call an Agent at every step, using the document's body as the prompt
- in this **headless** example we:
    - use `prompt` property to make a file reference to a prompt document
    - or we use `shell` to call one or more shell commands
- we also define a `dir` property which will be the same across all steps/states


### Prompt References

- When a sequence defines a `prompt` property  the value must be a valid filepath (at the time of composition)
- When a step which has a Prompt Reference in it becomes active, Claudine will:
    - resolve the filepath reference
    - _compose_ the referenced document
    - provide an Agent the composed content as a prompt

### Shell Command Blocks

A step in a sequence that defines the `shell` property can either:

- take a single string value that represents the shell command
- take a list of strings with _each_ representing a shell command

```sh
sequence:
    - name: JustOne
      # run one command
      shell: "just test"
    - name: Multiple
      # run multiple
      shell:
          - "just test"
          - "just lint"
```

A step defined by a shell command block successfully passes when:

- all of the shell commands provided return a 0 exit code
- each command is allowed 30 seconds to complete execution before they are _timed out_
    - to override this default timeout window you can set a property `timeout` inside the step definition to a numeric value (representing seconds)
    - this will change the timeout window for each of the commands in that step (but not outside it)
- all shell commands in a **sequence** are included in Claudine's [preflight checks](../pre-flight-checks.md) and if any of the commands are not already whitelisted then Claudine will bring up the interactive dialog for approval immediately upon execution (versus later when that step's command may be run).


## Parameters

We are all familiar with the idea/concept of **parameters** in programming and Claudine embraces a formalism around parameter definition that offers a simple format to define a schema for what you expect your callers to provide to you.

This topic of parameters is covered in more depth in the [Parameters in Claudine](../validations/parameters.md) document but we'll illustrate here how **headless sequences** can define and call into parameterized schemas.

In the following example, our headless sequence definition will define:
    
- a _required_ parameter `dir` which must be a valid filepath reference, and
- an _optional_ parameter `spec_file` a caller can provide if they want the spec file to have a non-standard name

```yaml
parameters:
    dir: Filepath
    spec_file: Option<Filepath>
template:
    - spec: "{{dir}}/{{spec_file || "spec.md"}}"
    - log: "{{dir}}/log.md"
sequence:
    - name: Design
      prompt: "@prompt/design.md"
    - name: Implement
      prompt: "@prompt/implement.md"
    - name: Commit
      shell: "just lint"
    - name: Review
      prompt: "@prompt/review.md"
```

Now when someone calls this sequence, they must pass in `dir` or get an error:

```sh
claudine sequence @sequence/example.yaml dir=features/my-feature
```

Every step's `state` in the sequence will have all the properties from `parameters` and `template` made available to it. However, just like a **sequence** can define parameters, so too can a prompt document and if we want to pass our state into a prompt reference we would do it like:

```yaml
parameters:
    dir: Filepath
    spec_file: Option<Filepath>
template:
    - spec: "{{dir}}/{{spec_file || "spec.md"}}"
    - log: "{{dir}}/log.md"
sequence:
    - name: Design
      prompt: "@prompt/design.md"
      params:
          dir: "{{dir}}"
    - name: Implement
      prompt: "@prompt/implement.md"
      params:
          log: "{{log}}"
          spec: "{{spec}}"
    - name: Commit
      shell: "just lint"
    - name: Review
      prompt: "@prompt/review.md"
```


## Groups

Groups allow a set of **Prompt References** and/or **Shell Command Blocks** be grouped together and named.

- having a group allows for additional reuse patterns, _as well as_
- allowing for _cycling_ patterns to be better defined

A group is always defined in a YAML file, either the same YAML file as a sequence is defined or separate from it. The groups definition is defined under the `groups` property as a list of group definitions:

- groups must define both the `name` and `members` properties

Here's a simple example:

```yaml
groups:
    - name: ICR
      members:
        - name: Implement
          prompt: "@prompt/implement.md"
        - name: Commit
          command: "just commit"
        - name: Review
          prompt: "@prompt/review.md"
```

To add this group


A group _can not_ be executed by itself but rather must be executed as a part of a **sequence**:

- a single group, however, can be run more than once in a sequence
- a single group can also be shared across multiple sequences
- sequences have no visibility into a groups individual members but instead interact with the group at the group level
- like sequences, a group can define parameters

```yaml
groups:
    - name: ICR
      variables:
        dir: String
        iteration: [Number, 1]
      template:
        log: "{{dir}}/log.md"
      members:
        - name: Implement
          prompt: "@prompt/implement.md"
        - name: Commit
          command: "just commit"
        - name: Review
          prompt: "@prompt/review.md"
          params:
            review: "{{log}}"
      until:
        fm: ["{{log}}", "done"]
```

### Group Cycling

- `until`
- `while`

Conditions:

- `frontmatter`
- 


## Operations

Claudine has had a CLI switch `--operation <op>` available for a long time. Using the CLI switch sets the OPERATION environment variable and this ENV variable is logged to Claudine's logs making it a dimension which you can report off of.

Any step in a sequence -- headless or headed -- is allowed to define a property `operation` and the value at this key will be assigned to the OPERATION environment variable.
