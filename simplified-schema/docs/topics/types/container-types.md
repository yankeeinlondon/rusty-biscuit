# Container Types in Simplified Schema

In **Simplified Schema** we three container types:

1. **Objects** - _a key/value dictionary object_
2. **Arrays** - _an array of a specific type_
3. **Tuples** - _a discrete set of elements of potentially differing type_

Let's hit each of these as it's own section.

## Objects

Objects in **Simplified Schema** are a key/value type and can be defined through four distinct syntaxes:

1. The Wide Object

    ```yaml
    $schema:
        metadata: object -> a key value for storing metadata; all keys and values are allowed
    ```

2. Explicit Object (Single Line)

    ```yaml
    $schema:
        metadata: { size: enum(S,M,L), color: enum(red,blue,green) }
    ```

3. Explicit Object (Key per Line)

    ```yaml
    $schema:
        metadata: 
            size: enum(S,M,L)
            color: enum(red,blue,green)
    ```    

4. Explicit Object (Serialized String)

    ```yaml
    $schema:
        metadata: |-
            {
                size: enum(S,M,L)
                color: enum(red,blue,green)
            }
    ```

### Why So Many Variants?

Why would we have so many variants of an object? Well let's start with the most obvious distinction:

#### Wide versus Narrow

There are certainly times where all you want to express is that a variable is an object but don't want to (or can't) express the types of the various keys and values. Whether that's because you don't know the shape of the object, you don't want to think through that shape at the moment, you're an innately lazy person, you think people who always talk about types as some sort of miracle as assholes and want to be as little like them as you can be. You do you.

Outside of the `object` grammar, the rest of the grammars all define the shape and types at each part of the shape.

#### Valid YAML

Grammar #2 and #3 are both valid ways in YAML to express a key/value object and choosing one over the other really just comes down to which one you prefer in the moment. As a general rule, however, if you only have a few props or there are benefits to having the object property consume a single line then use compact grammar of #2 but in a vast majority of cases we think #3 will likely be the best choice.

Grammar #4 is the odd duckling:

- first off please known that `|-` symbol in YAML indicates that the property is a string and will take it's value as being all of the lines below which are indented.
