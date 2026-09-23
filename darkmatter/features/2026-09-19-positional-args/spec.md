# Positional Parameter Passing in Darkmatter

## Intro

Today we have a well established method for passing in **named** parameters into a document we are going to _compose_ but no way to accept _positional_ parameters. This specification will introduce that capability.

The `positional` Frontmatter property will take on the role of allowing authors to specify how to map positional arguments into named Frontmatter properties. 

> Note: if `positional` is not set on a document then that document does NOT support positional arguments.

## How `positional` Works

The `positional` property takes an array of arguments where the arguments are the frontmatter properties that will be mapped to:

```yaml
positional:
    - foo
    - bar
    - baz
```

In this example the `foo` property is mapped to the first positional argument, and then `bar` to the 2nd, `baz` to the third.

### Schema Awareness

When we map positional arguments into frontmatter properties the schemas activated for the document will be used in the mapping:

- all CLI variables are strings so we clearly must rely on coercion where we can
- the basic evaluation should be:
    - if the properties schema type is a string then map into the property; no coercion needed
    - if the properties schema type is not a string then look for _reasonable_ coercion paths to convert the type to the type:
        - "reasonable" includes any cognitively obvious mapping like:
            - "42" -> number
            - "true" / "false" -> boolean
            - "null" -> null
    - if there are no "reasonable" coercions to the type in the schema then we simply map it through "as is" and this will become an error during schema validation


### Optional Types and Variadic Params




## Handling Variadic Parameters



## Avoiding CLI Switches


## Documentation
