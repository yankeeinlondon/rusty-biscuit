In conditional expressions typically bound to the `when` variable, we allow for logical operations like AND and OR.

- currently we support this with a `And(op, op)` styled syntax
- refer to the [Boolean Conditional Logic](claudine/docs/topics/boolean-conditional-logic.md) document for full context
- however, it would be more natural in many cases if we supported inline operands:
    - `&&` for logical AND
    - `||` for logical OR

The feature adds the `&&` and `||` inline operands to all boolean logic operations.

