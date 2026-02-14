# `Stylesheet` struct

The `Stylesheet` struct provides a type safe way to define and validate CSS key/value pair definitions.

- `Stylesheet` depends on `CssProp` enum to enumerate all the common CSS properties
    - it includes a `Other(String)` variant to allow other more exotic properties to be added
    - we create a lookup mapping each CssProp variant to an appropriate type category:
        - CSS Sizing (singular)
        - CSS Sizing (singular, two way, three way, four way)
        - Color
        - Integer
    - A property variant like `ZIndex` would be mapped to `i32`
    - A property variant like `TopMargin` would be mapped to `CssSizing` enum
    - A property variant like `Margin` would be mapped to `CssSizingMulti` enum
- it exposes a new() function which initializes an empty Stylesheet
- it exposes a `.add<T,U>(prop: T, value: U) -> Self` method which allows adding properties in a type strong manner
- it implements `TryFrom` from `&str`, `&String`, and `String` which effectively acts as a validation function and an import mechanism.
- it provides four render functions:
    - `to_css() -> String`
    - `to_terminal(&Terminal) -> void` (just like `to_css()` but we render to the terminal and use some color coding to make the CSS more readable)
    - `to_json() -> String`
    - `to_json5() -> String`

