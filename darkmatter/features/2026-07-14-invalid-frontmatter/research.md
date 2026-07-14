---
prompt: "Your task is to research online the following topics:\n\n- what are the most common mistakes people make when creating YAML files?\n- what technics can be used algorthmically to fix common YAML file problems?\n    - when doing this research look at the YAML support that `biscuit-file` provides (as this is used throughout this monorepo for YAML support)\n    - when you talk about solutions for fixing YAML consider that this solution will likely hang off of the YAML support in biscuit-file\n\nOnce you've researched these two topics, write the following sections into the body of this page:\n\n- `## Common Mistakes` - all of the common mistakes you've found, give real examples, explain why these mistakes are common\n- `## Algorthmic Opportunities` \n    - all of the opportunities you've come up with for addressing problems algorithmically\n    - tag each opportunity as one of the following:\n        - `deterministic`:\n            - those algorithms which can identify an error with 100% certainty and auto-correct with 100% certainty should be marked with that tag\n        - `deterministic-find-non-deterministic-solution`\n            - those algorithms which can 100% identify a problem and then try to fix a problem but could accidently introduce a follow on problem \n            - note: in this category we're assuming that the algorithm in question \n        - `non-deterministic-find`\n            - these we detect what \"looks like\" a code smell but we are not 100% able to be sure\n            - these should never change values but might be useful as suggestions provided back on STDERR channel"
last_updated: 2026-07-14
hash: c14d4cea3bfdd4a4-88b0921f8e8909c9
---
## Common Mistakes

YAML’s minimal punctuation makes it readable, but also makes whitespace, scalar style, and small punctuation changes semantically significant. The recurring problems below are reflected in the [YAML 1.2.2 specification](https://yaml.org/spec/1.2.2/) and in the checks provided by [yamllint](https://yamllint.readthedocs.io/en/stable/rules.html).

### Incorrect indentation

Block structure is determined by indentation. Child nodes must be indented farther than their parent, while siblings must have identical indentation.

```yaml
services:
  - name: api
    port: 8080

   - name: worker
    port: 8081
```

The second list item is one space left of the first. Depending on its surroundings, this either fails to parse or creates a structure different from the one the author intended.

This is common because indentation errors are visually subtle, copying blocks can preserve the wrong leading whitespace, and different editors display spaces differently.

### Tabs used for indentation

Tabs are forbidden in indentation because their displayed width varies between tools and environments.

```yaml
server:
<TAB>host: localhost
<TAB>port: 8080
```

Tabs remain easy to introduce because the Tab key is naturally associated with indentation and many editors do not visibly distinguish tabs from spaces. Tabs are permitted in some scalar and separation contexts, so globally replacing every tab is not safe. The YAML specification specifically prohibits tabs only where they act as indentation.

### Missing whitespace after mapping and sequence indicators

A block mapping uses a colon followed by whitespace, and a block sequence item uses a dash followed by whitespace.

```yaml
host:localhost
ports:
  -80
  -443
```

This may parse as plain scalar content instead of the intended mapping and sequence:

```yaml
host: localhost
ports:
  - 80
  - 443
```

This mistake is common because `key:value` is valid in many other configuration and programming languages, while a negative number such as `-80` is itself a valid scalar. Consequently, this error can produce valid YAML with the wrong shape rather than a parser error.

### Misaligned sequence items and mappings

Authors frequently mix up the indentation contributed by a sequence marker with the indentation of the mapping inside that item.

```yaml
users:
  - name: Alice
    roles:

      - admin

  - name: Bob

  roles:

    - reader
```

Here `roles` for Bob is no longer part of Bob’s mapping. This is common in nested lists because the `-` visually resembles indentation even though it is a structural token.

### Unquoted scalars containing YAML indicators

Plain scalars are context-sensitive. They cannot begin with most indicator characters, cannot contain `: ` or ` #`, and have additional restrictions inside flow collections.

```yaml
schedule: @daily
message: deployment: ready
token: abc #123
```

These should be expressed as strings:

```yaml
schedule: "@daily"
message: "deployment: ready"
token: "abc #123"
```

In the original `token`, ` #123` begins a comment, so the loaded value is only `abc`. This is common because URLs, shell fragments, timestamps, selectors, and natural-language text routinely contain punctuation that YAML also uses structurally. The exact restrictions are documented in the specification’s [plain scalar rules](https://yaml.org/spec/1.2.2/#733-plain-style).

### Incorrect quoting and escaping

Double-quoted YAML scalars interpret backslash escapes. Single-quoted scalars do not interpret backslashes, but represent an apostrophe by doubling it.

```yaml
windows_path: "C:\Users\Ken"
owner: 'Ken's account'
```

Correct forms include:

```yaml
windows_path: 'C:\Users\Ken'
owner: 'Ken''s account'
```

This is common because authors transfer escaping rules from JSON, shell syntax, or programming-language string literals. A Windows path is particularly error-prone because a backslash in a double-quoted YAML scalar starts an escape sequence.

### Unbalanced or malformed flow collections

Compact collections require matching brackets or braces and comma-separated entries.

```yaml
ports: [80, 443
labels: {environment: production region: us-west}
```

Correct YAML is:

```yaml
ports: [80, 443]
labels: {environment: production, region: us-west}
```

These errors are common during manual editing because flow-style YAML looks like JSON while still allowing YAML-specific syntax. A missing delimiter can also make the parser report an error later than the actual mistake.

### Implicit scalar typing

An unquoted scalar may resolve to null, a boolean, an integer, or a float rather than a string.

```yaml
enabled: TRUE
release: 1.20
answer: null
limit: .inf
```

Possible loaded values are a boolean, a floating-point number, null, and positive infinity. If text was intended, it should be explicit:

```yaml
enabled: "TRUE"
release: "1.20"
answer: "null"
limit: ".inf"
```

This is common because the scalar’s source spelling does not visually reveal its runtime type. YAML 1.1 and YAML 1.2 also differ in their implicit typing rules, producing portability problems between parsers. The current `biscuit-file` parser, `serde_yaml_ng 0.10`, applies YAML 1.2-style boolean resolution: variants of `true` and `false` become booleans, while `yes`, `no`, `on`, and `off` remain strings. Other tools may use a different profile. The [YAML Language Server](https://github.com/redhat-developer/yaml-language-server) exposes YAML 1.1 versus 1.2 as an explicit setting for this reason.

### Accidental null values

Omitting node content is legal YAML and commonly resolves to null.

```yaml
timeout:
features:
  -
```

This is syntactically valid and represents a null mapping value and a null sequence item. It is common when authors leave placeholders to complete later or assume an empty value means an empty string. The distinction becomes especially important during YAML-to-TOML conversion because TOML has no null type; `biscuit-file` currently drops nulls by default in that conversion.

### Duplicate mapping keys

Mapping keys must be unique.

```yaml
environment: production
replicas: 3
environment: staging
```

Duplicate keys commonly arise from copying configuration blocks or resolving merge conflicts. Parsers have historically varied between rejecting duplicates and keeping either the first or last value, so accepting them would make behavior tool-dependent. `serde_yaml_ng`, and therefore `biscuit-file::Yaml`, rejects duplicate keys rather than silently selecting a value. Key uniqueness is part of the [YAML representation model](https://yaml.org/spec/1.2.2/#3211-nodes).

### Unknown, forward, or misleading aliases

An alias must refer to a previously declared anchor.

```yaml
production: *defaults
defaults: &default
  retries: 3
```

The alias name is also misspelled. A valid version is:

```yaml
defaults: &defaults
  retries: 3
production: *defaults
```

These mistakes are common after renaming or moving anchored blocks. Duplicate or unused anchors are also suspicious even when accepted by a parser. Anchors are relatively uncommon syntax, so authors often treat them like ordinary symbolic references and overlook YAML’s declaration-order rule.

### Misunderstood block scalars

Literal (`|`) and folded (`>`) block scalars have different newline behavior.

```yaml
script: >
  echo first
  echo second
```

The loaded value is effectively:

```text
echo first echo second
```

For a line-oriented script, the likely intent is:

```yaml
script: |
  echo first
  echo second
```

Chomping indicators such as `|-`, `|+`, `>-`, and `>+` also control trailing newlines. These mistakes are common because scalar indicators look like formatting choices even though they change the resulting string. The difference is defined by the specification’s [block scalar rules](https://yaml.org/spec/1.2.2/#81-block-scalar-styles).

### Multiple documents passed to a single-document API

A YAML stream may contain multiple documents:

```yaml
---
name: first
---
name: second
```

This is valid YAML, but `biscuit_file::Yaml::from_str` deserializes into one `serde_yaml_ng::Value` and rejects a stream containing more than one document. This mismatch is common when content is copied from Kubernetes manifests or other tools that use multi-document streams. Frontmatter delimiters can create similar confusion if a caller passes an entire Markdown document to a YAML parser instead of extracting the frontmatter first.

### Syntactically valid but structurally invalid data

Parsing establishes YAML syntax, not application correctness.

```yaml
timeuot: 30
port: "8080"
workers: many
```

A parser cannot know that `timeuot` is misspelled, that `port` should be an integer, or that `workers` must satisfy a numeric range. JSON Schema can identify missing properties, additional properties, invalid types, invalid enum values, and other structural constraints. The [JSON Schema object rules](https://json-schema.org/understanding-json-schema/reference/object) distinguish required properties, additional properties, and null values.

This is common because users interpret “valid YAML” as “valid configuration.” In the current `biscuit-file` implementation, constructing `Yaml` performs syntax validation and `Yaml::validate()` is consequently a no-op. Schema-related feature wiring and error variants exist, but schema validation is not yet exposed by the implemented `Yaml` API.

## Algorthmic Opportunities

The repair system should live beside `biscuit_file::Yaml`, but syntax parsing, diagnostics, repair, and lossy format conversion should remain distinct operations. In particular, YAML-to-JSON or YAML-to-TOML conversion must not be used as a repair mechanism: current conversion policies may stringify non-string keys, replace non-finite floats with null, drop nulls, or stringify heterogeneous arrays.

A suitable API would return source-oriented diagnostics and candidate edits:

```rust
pub struct YamlDiagnostic {
    pub code: YamlDiagnosticCode,
    pub span: SourceSpan,
    pub message: String,
    pub classification: YamlRepairClassification,
    pub repairs: Vec<YamlRepair>,
}

pub struct YamlRepair {
    pub span: SourceSpan,
    pub replacement: String,
    pub explanation: String,
}
```

`serde_yaml_ng::Error` already provides byte index, line, and column information through [`Error::location`](https://docs.rs/serde_yaml_ng/0.10.0/serde_yaml_ng/struct.Error.html). `biscuit-file` should preserve that structured information instead of requiring callers to parse a rendered error string.

### Source normalization — `deterministic`

Normalize source representations that YAML defines as presentation details:

- Remove a single UTF-8 BOM at the beginning of the stream.
- Normalize CRLF and CR line endings to LF.
- Remove trailing whitespace only when a lexical scan proves it is outside scalar content.
- Add or remove a final newline only when reparsing proves the resulting `serde_yaml_ng::Value` is identical.

For parseable input, the hard safety gate should be:

1. Parse the original source.
2. Apply the proposed edit.
3. Parse the candidate.
4. Require exact `serde_yaml_ng::Value` equality.
5. If a schema is associated, require identical schema results as well.

This provides proof that the serialized data did not change. It should patch the original text rather than serialize the parsed `Value`, because `serde_yaml_ng::Value` does not preserve comments, anchors, scalar style, or whitespace.

### Parse-equivalent whitespace cleanup — `deterministic`

Whitespace around comments, commas, braces, brackets, mapping colons, and sequence markers can be normalized when both the original and candidate parse successfully and their values compare equal.

For example:

```yaml
ports: [ 80,443 ]
```

can safely become:

```yaml
ports: [80, 443]
```

The equality gate is essential. It prevents an apparently cosmetic edit such as changing `host:localhost` to `host: localhost` from being auto-applied because those two inputs produce different YAML structures.

### Schema-proven scalar quoting — `deterministic`

Quoting can be automatic when all of the following are true:

- The original YAML parses successfully.
- The associated schema rejects the node solely because it is not a string.
- The source node is a plain scalar.
- Quoting the exact original lexeme produces a string without changing its characters.
- The repaired document passes the complete schema.
- No other source range is changed.

For example, if an authoritative schema requires `release` to be a string:

```yaml
release: 1.20
```

can become:

```yaml
release: "1.20"
```

This must not be generalized to arbitrary type coercion. Converting `"10"` to `10`, `null` to an empty string, or `.inf` to a finite number requires an intent decision.

### Parser-guided syntax repair — `deterministic-find-non-deterministic-solution`

A parser error proves that the document is invalid, but a successful parse after an edit does not prove that the edit captured the author’s intent. Candidate generation can use the parser location plus nearby lexical context to try:

- Inserting whitespace after `:` or `-`.
- Replacing indentation tabs with candidate space widths.
- Aligning a line with a nearby sibling indentation level.
- Closing an unterminated quote, bracket, or brace.
- Inserting a missing comma in a flow collection.
- Quoting a scalar that begins with a reserved indicator.
- Escaping a backslash in a double-quoted scalar.
- Adjusting block-scalar indentation.

Candidates should be minimal, reparsed independently, and ranked by edit distance and locality. Even a single parseable candidate should be presented for confirmation rather than silently applied: indentation repair can move nodes between parents, and quoting can change implicit types.

A tolerant concrete syntax tree can improve candidate generation by retaining valid regions around an error. Tree-sitter, for example, represents unrecognized text with `ERROR` nodes and inferred tokens with `MISSING` nodes during [error recovery](https://tree-sitter.github.io/tree-sitter/using-parsers/queries/1-syntax.html). Such a parser could supplement `serde_yaml_ng`, while `serde_yaml_ng` remains the final acceptance authority for `biscuit-file`.

### Duplicate-key resolution — `deterministic-find-non-deterministic-solution`

Duplicate mapping keys can be identified with certainty, but no algorithm can safely decide whether to:

- Keep the first value.
- Keep the last value.
- Rename one key.
- Merge two mappings or sequences.
- Delete one entry.

The repair engine should report both source spans and offer explicit candidates without selecting one automatically. If both values are structurally identical, deleting the duplicate is still not wholly deterministic because its comments or anchor placement may be meaningful to a human reader.

### Schema-guided key correction — `deterministic-find-non-deterministic-solution`

When a schema rejects an additional property and reports a similarly named allowed property, calculate candidates using edit distance, keyboard adjacency, separator normalization, and case normalization.

```yaml
timeuot: 30
```

may suggest:

```yaml
timeout: 30
```

A candidate becomes stronger when `timeout` is required, `timeuot` is forbidden, the value validates against `timeout`, and no `timeout` property already exists. It is still not certain: the author may have intended a different property or may be using a newer schema.

### Schema-guided shape and type repair — `deterministic-find-non-deterministic-solution`

A schema can definitively identify missing required properties, unexpected properties, wrong node kinds, invalid enums, and range violations. It can then generate possible repairs:

- Insert a required property from an explicit application-owned default.
- Convert a singleton value into a one-item sequence.
- Suggest an allowed enum member close to the supplied value.
- Move an unexpected property to a schema-valid parent.
- Quote or unquote a scalar to test the required type.

JSON Schema’s `default` keyword is an annotation, not a universal instruction to mutate the instance, so defaults should only be inserted when `biscuit-file` or the consuming application explicitly opts into that behavior.

### Multiple-document handling — `deterministic-find-non-deterministic-solution`

When the configured consumer accepts only one document, counting two or more YAML documents identifies the incompatibility with certainty. Possible repairs include:

- Split the stream into separate files.
- Select one document.
- Convert the documents into a sequence.
- Reject the stream with a targeted explanation.

No choice is universally correct. `biscuit-file` should expose explicit single-document and stream APIs rather than making repair logic guess which model the caller intended. `serde_yaml_ng::Deserializer` already supports iterating documents even though deserializing directly into one value rejects multiple documents.

### Anchor and alias repair — `deterministic-find-non-deterministic-solution`

An undeclared alias is a definite error. Candidate anchors can be ranked using spelling distance, preceding declaration order, and structural compatibility.

```yaml
service: *defualts
defaults: &defaults
  retries: 3
```

The spelling candidate is plausible, but the declaration-order problem remains. Moving the anchor, changing the alias, or expanding the value can each alter graph semantics, so these repairs require confirmation.

### Ambiguous scalar linting — `non-deterministic-find`

Warn about unquoted scalars whose runtime type or portability is surprising:

- Boolean and null spellings in any supported YAML version.
- Numeric-looking identifiers, versions, ZIP codes, and values with leading zeros.
- Hexadecimal, octal, binary, scientific, NaN, and infinity forms.
- Date- or timestamp-looking strings.
- Scalars whose type differs between configured parser profiles.
- Non-string mapping keys.

The diagnostic should show the parsed type and value without changing either:

```text
release: 1.20 parses as number 1.2; quote it if the trailing zero is significant
```

Schema knowledge can suppress false positives when the parsed type is exactly the required type.

### Suspicious empty values — `non-deterministic-find`

Report empty mapping values and sequence entries as possible accidental nulls:

```yaml
timeout:
features:
  -
```

Null may be intentional, so the algorithm must not replace it with `""`, `{}`, `[]`, or a default. If a schema rejects null, the schema violation is deterministic, but choosing the replacement remains non-deterministic.

### Suspicious block scalar choice — `non-deterministic-find`

Warn when a folded scalar appears to contain line-oriented content such as shell commands, source code, PEM material, patches, or newline-sensitive templates.

```yaml
script: >
  cargo build
  cargo nextest run
```

The tool can explain that `>` folds the newline and show a preview of the loaded value. It must not change `>` to `|`, because natural-language prose often intentionally uses folding.

### Comment truncation and indicator smells — `non-deterministic-find`

Warn when a plain scalar is followed by a comment whose text resembles value content:

```yaml
color: #fff
token: abc #123
```

The first value is null and the second is `abc`, but both could be intentional comments. Suggestions can show quoted alternatives while leaving the source unchanged.

Likewise, warn about shell fragments, URLs, selectors, and Windows paths in contexts where YAML indicators or double-quoted escapes are likely to surprise the author.

### Inconsistent style and indentation width — `non-deterministic-find`

Detect locally inconsistent indentation widths, mixed block and flow styles, unusual sequence indentation, inconsistent quoting, and inconsistent boolean spelling. These are maintenance smells rather than semantic errors. They should be reported only when inconsistency is strong enough to be useful, and should be suppressible through a project policy.

### Similar or misplaced keys — `non-deterministic-find`

Without a schema, compare sibling keys and repeated mapping shapes to identify possible typos or misplaced nodes:

```yaml
development:
  timeout: 10
production:
  timeuot: 30
```

The repeated structure makes `timeuot` suspicious, but custom environment-specific keys remain possible. This should produce a suggestion on STDERR and never mutate the document.

### Repair pipeline for `biscuit-file`

A strategic implementation would add `diagnose` and `repair_candidates` APIs to `biscuit_file::Yaml` and use the following pipeline:

1. Establish the YAML version, single-document versus stream policy, and optional schema before analysis.
2. Scan the original source into context-aware tokens and source spans.
3. Parse with `serde_yaml_ng`, retaining its structured error location.
4. Run schema validation after successful parsing.
5. Run non-semantic lint checks over the tokens and parsed value.
6. Generate bounded, minimal edit candidates near each diagnostic.
7. Reparse every candidate and, when available, revalidate it against the schema.
8. Auto-apply only `deterministic` candidates whose invariants are proven.
9. Return the other candidates as structured suggestions.
10. Patch source spans rather than reserializing the whole YAML document.

For file-backed `Yaml`, the current `YamlSource::Path` stores the path but not the original text. Format-preserving repair will therefore need to retain or reread the raw source. Inline text and byte sources already retain their input.

CLI diagnostics should be rendered to STDERR using `TerminalRenderable` components, with machine-readable output available separately. Each diagnostic should include a stable code, source range, classification, explanation, and optional diff.

The implementation should be tested against the language-independent [YAML Test Suite](https://github.com/yaml/yaml-test-suite), which provides valid inputs, canonical outputs, event streams, and expected parse failures. Additional mutation tests should inject each supported mistake into real monorepo YAML and frontmatter samples, then verify that:

- Deterministic repairs preserve parsed values and schema results.
- Non-deterministic repairs are never silently applied.
- Comments and untouched source ranges remain byte-for-byte unchanged.
- CRLF, LF, BOM, and path behavior remain portable across macOS, Windows, and Linux.
