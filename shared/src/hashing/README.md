# Hashing

In 2026 the best in class hashing algorithms for their respective areas are:

- **xxHash** - _for non-cryptographic hashes_
- **argon2is** - _for password hashes_
- **blake3** - _for cryptographic hashes_

This library provides all three for calling libraries.

## Semantic Hash Variants

We provide a basic `xx_hash()` function to hash any content but it is often more useful to "prepare" text content a little before hashing to avoid "false positive" in change detection. This is particularly true in whitespace insensitive grammars like Markdown or HTML.

The `HashVariant` enumeration provides the following options:

- `BlockTrimming` - trims the content block's leading and trailing whitespace
- `BlankLine` - all blank lines are removed
- `LeadingWhitespace` - all whitespace at the _end of each line_ is removed
- `TrailingWhitespace` - all whitespace at the _beginning of each line_ is removed
- `InteriorWhitespace` - all _extra_ space (aka, after the first whitespace character) _on each line_ is removed.
- `ReplacementMap(map)`
    - The replacement map variant requires that you provide a `HashMap<Into<String>,Into<String>>` as configuration
    - The hashmap provided represents a text replacement strategy where the _keys_ are the text we'll look for, and the _values_ are the text we'll replace it with
        - we will leverage the `interpolate()` function to do this for us
    -

### Using the `HashVariant` to Hash with xxHash

The `xx_hash(content)` always just hashes exactly what you give it but if you want to pre-process the content with one or more of the variants provided above you will use the `xx_hash_variant(content, [v1,v2,v3])` which takes one or more variants as input.

