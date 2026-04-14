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
    - in the **headed** section we mentioned that _template_ key/values typically would leverage _interpolation_ with other state properties
    - you can, however, define templates without _interpolation_ and then these variables will act as a constant throughout the sequence

### Parameters

We are all familiar with the idea/concept of **parameters** in programming and Claudine embraces a formalism around parameter definition that offers a simple format to define a schema for what you expect your callers to provide to you.

This topic of parameters is covered in more depth in the [Parameters in Claudine](../validations/parameters.md) document but we'll illustrate here how **headless sequences** can define and call into parameterized schemas.

In the following example, our headless sequence definition will define:
    
- a _required_ parameter `dir` which must be a valid filepath reference, and
- an _optional_ parameter `spec_file` a caller can provide if they want the spec file to have a non-standard name

```yaml
parameters:
    dir: filepath
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
    dir: filepath
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


### Prompt References

As we've already seen in the examples above, the `prompt` property has special meaning for a step in a _headless_ sequence:

- the value of `prompt` property must be a filepath reference

    > **Note:** the `FileReference` struct from **biscuit-file** is used to resolve all filepath references. This is consistent with all other file references in Claudine.

- when a **step** is executed by Claudine, it will resolve the file path in `prompt` and _compose_ this document into an Agent prompt
    - however, before we _compose_ it we will set the `state` in the prompt file's Frontmatter
    - this allows the prompt document to use and work off of this state 
    - when you control the prompt file you can make the `state` shape work for any prompt file
    - this is fine, but often a prompt file will have many primitives which call into it and rather then trying to adapt to each callers internal state the prompt will instead define it's own parameters
- any Markdown document which wants to be called as a prompt but needs some initial state
- we always pass in the `state` object and in prompts which we control, it's not that hard to have `state` and it's key/values 

---


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
      until:
        
sequence:
    - name: Design
      prompt: "@prompt/design.md"
    - name: "group::ICR"
```
