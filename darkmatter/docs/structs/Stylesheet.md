# `Stylesheet` struct

The `Stylesheet` struct provides a type safe way to define and validate CSS key/value pair definitions.

- `Stylesheet` depends on `CssProp` enum to enumerate all the common CSS properties
    - it includes a `Other(String)` variant to allow other more exotic properties to be added
- it exposes a new() function which initializes an empty Stylesheet
- it exposes a `.add<T,U>(prop: T, value: U) -> Self` method which allows adding properties in a type strong manner
- it implements `TryFrom` from `&str`, `&String`, and `String` which effectively acts as a validation function and an import mechanism.


