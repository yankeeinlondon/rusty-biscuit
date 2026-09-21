---
parent: ./schema.md
---
# Schema Enhancements

The current `SimplifiedSchema` is very helpful but it has some constraints that make it less capable and ergonomic than it could be. This specification tries to address that.

## Types Added

# Schema Additions
> For Reference: 
>
> - the current set of schema **types**: 
> - the current set of schema **constraints**: 

> Note: 
>
> - one consistency concession I think we need to make for ergonomics is that functions parameters and tuple elements should be considered required unless specified as optional. 
> - for people more involved in language structure this may seem like an obvious thing but for others it may be a surprise
> - however I think that most of the people who would be writing schemas for functions would be a very limited subset and probably leaning toward more advanced in language design
> - in a _way_ this isn't actually a full consistency break both functions and tuples are describing _elements_ of a property in their own constraint block versus property definitions which are at a different level

### Tuples

Today we provide for array types -- and the constraint system _does_ provide the ability to specify a min and max length -- but arrays and tuples are only related cousins, not the same type category.

The **type** will be called `tuple` and through it's constraint system we will support not only fixed length tuples but also variadic types.

- the basic syntax for a fixed length tuple would look like:

    ```yaml
    my_tuple: tuple([string,number,boolean])
    ```

- if we wanted to make one or more of the trailing properties _optional_ we would use the `?` operator:

    ```yaml
    my_tuple: tuple([string,number?,boolean?])
    ```    

- it's important to understand that the property `my_tuple` is _optional_ because all schema types are optional by default. If I wanted the property to be required I'd use precisely the same grammar as you'd use for any other property:
    
    ```yaml
    my_tuple: tuple([string,number?,boolean?]; required)
    ```   

#### Variadic Tuples

- to allow for a "variadic tuple" (aka, a tuple shape that a variadic component to it that makes the _length_ indeterminate) we will again lean into Typescripts grammar a bit. Here's a simple example:


    ```yaml
    my_tuple: tuple([string, ...number[]])
    ```yaml

- as you can see the `...` prefix operator indications that the _type_ it is operating on is _spread_ into the type definition.
- to keep complexity within bounds the grammar will only allow for _one_ spread per property
- Note: the `?` operator should not be applied to a variadic declaration because it's redundant ... all variadic types in a tuple are optional by nature.

### Functions

Although functions will not be a strong requirement for many Markdown document's schemas, we have an immediate need for being able to expression function syntax.

The easy part is the generic **type** which is just called `function`, the complications arise in being able to create a capable set of **constraints** which allows for ergonomic, understandable syntax while complimenting the rest of the grammar.

#### Constraint System

1. `parameters`

    The parameters constraint is a comma separated list of name/type pairs:

    ```ts
    parameters(name: string, age: numberlike)
    ```

    In the above example both `name` and `age` are required parameters to the function. If we wanted to bring in an optional parameter we will borrow a Typescript grammar to represent that:

    ```ts
    parameters(name: string, age?: numberlike)
    ```

    The `?:` symbol indicates the 

2. `returns`
3. `category`
4. `fallible`

### How DMLS Should Treat Functions

The goal for DMLS is to be helpful but not overbearing and the key question we need to address when discussing schema functions and DMLS is:

1. what do we do when a schema-defined function is called with a _wider_ union type than a parameter requires

    - The classic example of this is that Frontmatter's default type is `unknown`/`any` which is defined as a union of all types. That makes it wider than any parameter _other_ than one expecting `unknown`.

2. what do we do when we know the parameter type is wrong?

### Example

```yaml
min: |-
    function(
        parameters(a: number, b: number);
        returns(number);
        category(Math);
        fallible;
    ) -> Returns the smaller of two numbers.
```
