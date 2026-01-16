# `biscuit-hash` Library

<table>
<tr>
<td><img src="../assets/biscuit-hash.png" style="max-width='25%'" width=200px /></td>
<td>
<p>This library provides a useful trifecta of of hash algorithms:</p>

1. `blake3` for cryptographic hashes
2. `argon2id` for password hashes
3. `xx_hash` for blazingly fast non-cryptographic hashes

<p>
Each of these algorithms is "best in class" for their purpose in 2026+.
</p>
</td>
</tr>
</table>

## Feature Flags

By default only the `xx_hash` algorithm is included but `blake3` and `argon2id` can be enabled where needed:

```sh
cargo add biscuit-hash -F blake3,argon2id
```


## Semantic Hash Variants

We provide a basic `xx_hash()` function to hash any content but it is often more useful to "prepare" text content a little before hashing to avoid "false positive" in change detection. This is particularly true in whitespace insensitive grammars like Markdown or HTML.

The `HashVariant` enumeration provides the following options:

- `BlockTrimming` - trims the content block's leading and trailing whitespace
- `BlankLine` - all blank lines are removed
- `LeadingWhitespace` - all whitespace at the _beginning of each line_ is removed
- `TrailingWhitespace` - all whitespace at the _end of each line_ is removed
- `InteriorWhitespace` - all _extra_ space (aka, after the first whitespace character) _on each line_ is removed.
- `ReplacementMap(map)`
    - The replacement map variant requires that you provide a `HashMap<String,String>` as configuration
    - The hashmap provided represents a text replacement strategy where the _keys_ are the text we'll look for, and the _values_ are the text we'll replace it with
- `DropChars(chars)`
    - Removes all occurrences of the specified characters from the content before hashing
    - Useful for ignoring specific punctuation or symbols

### Using the `HashVariant` to Hash with xxHash

The `xx_hash(content)` always just hashes exactly what you give it but if you want to pre-process the content with one or more of the variants provided above you will use the `xx_hash_variant(content, [v1,v2,v3])` which takes one or more variants as input.

