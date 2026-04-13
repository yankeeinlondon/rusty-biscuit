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
and **append** all the information we have on him to "reports/dodgy-people.md", add the information under an H2 heading of `## {{state.description}}`.
```

## Advanced Techniques

So far we've been showing you a very popular style of sequence which consists of 
