Below are the _type_ primitives which [JSON Schema](https://json-schema.org/) allows a type to be composed from:

| JSON Schema Type | JSON Value Kind | Notes |
| ---------------- | --------------- | -------------------   |
| null             | null            | Only the literal null |
| boolean          | true/false      | No additional constraints |
| integer          | JSON number without fractional part | Mathematically integral, not necessarily bounded to JS safe integer |
| number           | Any JSON number | Includes integers |
| string           | JSON string     | Unicode string    |
| array            | JSON array      | Ordered sequence  |
| object           | JSON object     | Unordered key/value mapping |

