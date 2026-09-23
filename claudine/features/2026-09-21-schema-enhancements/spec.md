---
parent: ./schema.md
peers: ./coercion-design.md 
research:
    - ./current-state-coercion.md 
---
# Schema Enhancements

The current `SimplifiedSchema` is very helpful but it has some constraints that make it less capable and ergonomic than it could be. This specification tries to address that as well as some impacts to how Darkmatter and DMLS build their internal model for function types.

> For Reference, here is the most authoritative document we currently have for our schema support: 
>
> - [schema definition](darkmatter/docs/topics/schema-definition.md)


## Types Added

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

    ```yaml
    doit: function ( parameters(name: string, age: numberlike) )
    ```

    In the above example both `name` and `age` are required parameters to the function. If we wanted to bring in an optional parameter we will borrow a Typescript grammar to represent that:

    ```yaml
    doit: function( parameters(name: string, age?: numberlike) )
    ```

    The `?:` symbol indicates the `age` parameter is an _optional_ parameter.

    > Note: being made an "optional parameter" is distinct from having a union type that includes `null`

    When a function takes no parameters we allow this to be specified with `void`:

    ```yaml
    no_input: function( parameters(void) )
    ```

2. `returns`

    This allows the user to type the return value of a function. Any valid type/constraint combination is allowed.

    ```yaml
    doit: function( parameters(name: string, age?: numberlike); returns(boolean) )
    ```

    Schema _properties_ have a syntax for union types that would be awkward inside of a constraint so instead we are introducing the `|` operator to represent a union inside of properties like `returns`:

    ```yaml
    doit: function( parameters(name: string, age?: numberlike); returns(boolean | enum("banned","underage")) )
    ```

3. `category`

    This is a new _constraint_ but as a part of the spec it is described below because this constraint is available to all types.

4. `fallible`

    Fallibility indicates whether a function -- when passed valid inputs -- can return an error instead of the expected type.

    Being considered "valid inputs" means that the values passed in either:

    - are an exact match for the type requirements that the function has defined
    - there is an "obvious" way to coerce the value passed in to make it align to the parameters type:
        - if a string "4" is passed into a parameter expecting a number this is an obvious coercion situation
        - we do want to coerce where there's no question what the caller's intent was; especially in a medium where many values are inherently force to being a string (aka, shell command outputs, values passed in via the CLI, etc.)

5. `example`

    Functions tend to benefit more from _examples_ than other types do and therefore we'll add an `example()` constraint but only to the `function` type.

    Unlike other constraints, a constraints block can contain 0:M examples:

    ```yaml

    ```

#### Examples

```yaml
min: |-
    function(
        parameters(a: number, b: number);
        returns(number);
        category(Math);
        fallible;
    ) -> Returns the smaller of two numbers.
```

The document @claudine/docs/schemas/partials/functions.yaml has a full list of function examples.

### Using Function Schemas during Generation

We currently use the YAML doc @darkmatter/docs/schemas/expression-functions.yaml during the code generation stage of building Darkmatter. As a part of this specification we want to move away from this definition and start using @claudine/docs/schemas/partials/functions.yaml in it's place.

- the new function catalog doesn't have an "order" property but we shouldn't need it:
    - in documentation generation -- like what we do for `claudine context --expressions` we should group by "category" and sort alphabetically within the group.


### The Constraint based Union Type

As was mentioned above, when you're inside the constraint block of a schema properities definition we need a compact and ergonomic way of expressing a union type. Whenever a type constraint needs this capability it will always be done exactly the same:

- the `|` operator separates two legs of a union type
    - the expression `string | number`
    - a function should be as precise about their type needs as possible; for instance:
        - if a function wants to be accommodating by specifying it's input type as `numberlike` instead of explicitly stating it needs a `number` then the function becomes responsible for coercion (something we're trying to prevent)
        - while the idea of being accommodating is entirely reasonable (even desirable) the function level is not where we should do that
        - see the section below: [explicit coercion rules](#explicit-coercion-rules)

As a part of this specification we have three places where the constraint system can use this new `|` operator:

- a function's parameters
- a function's return type
- a tuple definition

> Note: just because we have more ways to create union types doesn't mean we _want_ more union types! Union types are expensive to represent compared to non-union types so use them where you need them but don't over do it.

### The `category` Constraint

We are introducing a new schema "constraint" (it's really _not_ a constraint but a means to add metadata) that allows a property in a schema to be associated with a category.

While this need was largely intended to meet the needs of representing Darkmatter's function catalog it feels like this sort of metadata has good reuse potential so it's being made available to all types.

## Explicit Coercion Rules

The current coercion rules are not DRY at all, leaving every function to implement their own (even though some of that might call into shared functions). Instead what we need to do is:

1. narrow the parameter types that these functions need to do their work and 
2. build a shared coercion layer that attempts to map the inputs it gets to what the function needs

This means:

- the functions should never do type coercion themselves (unless there's a really good reason but there really shouldn't be)
- the coercion layer attempts to provide the function with the types it needs but returns an error if it can not.
- the Darkmatter library needs to expose a high-performance way to:
    - take an input type and a desired output type
    - and return a boolean value on whether this is possible
    - we might even consider taking a batch of type tests like this if there is performance benefit for doing the checks concurrently
    - DMLS is likely the most important client for this
- when a call to a Darkmatter provided function gets an invalid input the error returned should be an InvalidType like error and should come from the coercion layer; the underlying function is never called
    - the error should, however, be able to express which function was being attempted for context reasons

We have produced a more detailed design document for this section to supplement this specification: [Coercion Design](./coercion-design.md)

## How DMLS Should Treat Functions

The goal for DMLS is to be helpful but not overbearing and the key question we need to address when discussing schema functions and DMLS is:

1. what do we do when a schema-defined function is called with a _wider_ union type than a parameter requires?

    - in essence we're dealing with the "it might be ok, but it might not be"
    - The classic example of this is that Frontmatter's default type is `unknown`/`any` which is defined as a union of all types. That makes it wider than any parameter _other_ than one expecting `unknown`.
    - we have a number of **Type Predicate** functions like `is_number(val: any) => boolean` that do take _any_ type and they in turn "prove" that the passed in variable is of a certain type.
    - currently many functions provided by darkmatter are very type-lenient by declaring their parameters as _any_ but over time we will be tightening that up that we can clearly distinguish between the error condition of a function getting the wrong type (aka, non-coercible) versus the functions internal logic reaching an error state.

2. what do we do when we know the parameter type is wrong?

    This is fairly straight forward ... we highlight as an error indicating that the wrong type is being used.

## Documentation

We need to be sure that all of the topics we've hit in the specification has complete and high quality documentation that is able to be read by a human audience who:

- is technical, but
- has no knowledge of this monorepo's capabilities, packages, etc.

The language must avoid jargon and time should be spent to make sure clear language, good examples, and smart structure is shown in every document
