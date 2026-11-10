# Using Prose in Schemas

Schemas -- _by their very nature_ -- define **properties** and **types** that constrain data to a shape and that's important to ensure the "sameness" of data. This structure also _implies_ a semantic contract -- _at least when designed well_ -- but this semantic contract is an indirect artifact of the defined properties and types and often leaves a certain semantic ambiguity. To give a richer characterization of your schema's intent you need _prose_ to describe in human terms what certain properties are _for_.

## How to use Prose in `SimplifiedSchema`

Simplified schema uses a syntax of `-> {description}` that can be added to the end of any property in a schema. For example:

```yaml
$schema:
    name: string(required) -> The employee's name
    started-on: date(required) -> The date the employee started with the company
    department: enum(mail room, accounting, sales, hr) -> The department the employee works in; if this property is not set this should be seen as an action item for someone in HR to address this metadata gap.
```

In this example it provides "color" through it's prose instead of the cut and dry of just properties and types. Arguably the prose doesn't expose tons of extra information about `name` and `started-on` (for sure) and maybe even `department` is fairly self-evident based on the properties name and yet the prose gives assurances on meaning and even if that is just re-affirmation of what the property and type implied that is helpful.

### Addressing Ergonomics at the Edge

In most cases when defining a schema, using the default grammar of `-> {description}` should be ergonomic and convenient. In fact it's design was _largely_ designed to **be** as ergonomic as possible so that schema authors are encouraged to provide the color that only prose can provide. 

But behind every great idea, is a dark storm cloud wanting to rain on the designers parade and one of those storm clouds can show up when you want to write prose about a key/value dictionary in your schema.

#### Key/Value Dictionaries

Now let's discuss three ways you can define the same key/value dictionary:

1. Inline Key/Value

    ```yaml
    $schema:
        job: { employer: string(string;not-empty;required), website: url }
    ```

2. Nested Key/Value

    ```yaml
    $schema:
        job:
            employer: string(string;not-empty;required)
            website: url
    ```

3. JSON/JSON5 Key/Value

    ```yaml
    $schema:
        job: "{ employer: string(string;not-empty;required), website: url }"
    ```

Now of these three ... the first two will feel _nicer_ to many. Why? Well probably because you secretly hate string types. Yes you heard it ... you're a string hater and you want to express yourself in real shapes!

Does that make you a bad person? No it does not. You're a fucking artist and you want a full canvas to paint on. Its understandable even if many of your friends sometimes thing you can be a bit demanding. The problem you're now going to face, however, is how can you add a prose description to the `job` property? Option #1 clearly gives you no where to put a `-> {description}` and then it dawns on you that neither does Option #2. Why must art be so hard to create?

For those of you from the accounting department -- who undoubtedly said "string are ok with me" -- you're probably laughing at your creative colleagues because option #3 can use the `-> {description}` that you have grown to know and love:

```yaml
$schema:
    job: "{ employer: string(string;not-empty;required), website: url } -> metadata about a person's job"
```

Good on you. Now get back to crunching numbers because we need an answer for the artists in the audience. And there's good news for the artists ... there is a way you can use real shapes _and_ wax philosophical about your property's deep meaning and importance. How? It comes down to the `$description` property; have a look:

```yaml
$schema:
    job:
        $description: "metadata about a person's job"
        employer: string(string;not-empty;required)
        website: url
```

The `$description` property gives you a place to describe the key/value object you're defining.


## Related Documents

- [Types and their Constraints in Simplified Schema](./types.md)
- [The `SimplifiedSchema` struct](./simplified-schema-struct.md)
