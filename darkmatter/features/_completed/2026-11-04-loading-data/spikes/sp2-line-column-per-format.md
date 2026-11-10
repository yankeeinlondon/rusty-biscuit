# SP2: Line and Column per Data Format

Spike for 2026-11-04-loading-data, R5 (`data_location {file, pointer, line,
column}`, 1-based, columns in characters, CRLF-safe). Question: for each
format R3 loads, can a JSON Pointer such as `/servers/1/port` be mapped to a
line and character column in the source text cheaply and reliably?

Date: 2026-11-05. The spike code was throwaway (a standalone crate in the
session scratchpad, depending on `simplified-schema`, `biscuit-file`, `csv`,
`csv-core`, `toml`, `json-five`, `serde_json`, and `yaml-rust`, all at the
versions already in `Cargo.lock`). No tracked file was changed.

## Verdict

| Format | Verdict | Locator | Effort | Owner |
|---|---|---|---|---|
| YAML | **works, with limits** | `PositionMap::project` with the mapping-root check removed, then `byte_range` | 0.25 d (relax root) | simplified-schema |
| YAML: aliases, multi-line flow collections | **not feasible today**, so the pointer only | none: projection is refused for the whole file | 0.5 d (aliases) + 1 d (multi-line flow), optional | simplified-schema |
| JSON (pretty, tabs, minified, duplicate keys, escapes) | **works with effort** | `serde_json` `RawValue` path walk (subslice address = byte offset) | 0.25 d | biscuit-file |
| JSON through the YAML locator | **not feasible** | fails on multi-line JSON, tabs, duplicate keys, and surrogate escapes | — | — |
| JSONL / NDJSON | **works with effort** | the JSON locator per record line, plus the line's start offset | 0.1 d on top of JSON | biscuit-file |
| CSV / TSV | **works with effort** | `csv-core` `read_field` input-consumed counts give each field's raw start | 0.5 d | biscuit-file (delimited reader) |
| TOML | **works** | `toml::de::DeTable::parse` (`Spanned` keys and values) | 0.25 d | biscuit-file |
| JSON5 | **works with effort** | recursive descent over `json_five::tokenize_str` byte spans | 0.5 d | biscuit-file |
| Shared: byte offset → (line, char column) and parser-error normalization | **needed** | one helper; every locator returns a byte offset | 0.25 d | biscuit-file |

All of the above total about 2 to 2.5 days, of which about 1.5 days is
optional YAML hardening.

## Method

Each locator produces a byte offset for the value at a pointer. One helper
turns that offset into a position:

```rust
fn line_col(text: &str, offset: usize) -> Option<(usize, usize)> {
    let before = text.get(..offset)?;
    let line_start = before.rfind('\n').map_or(0, |i| i + 1);
    Some((before.matches('\n').count() + 1, before[line_start..].chars().count() + 1))
}
```

This is the rule darkmatter's `one_based_position` and simplified-schema's
`numeric/ingress.rs::line_column` already use. Lines break on `\n`, so the
`\r` of a CRLF ends its line and never moves a column. Columns count `char`s.

Each format was tested against fixtures covering: a list root, a scalar
root, nested mappings and sequences, flow and inline collections, a
multi-byte character before the target on the same line, the CRLF twin of
an LF fixture, and the format's own quoting traps. 28 tests, all green.
Pointers are decoded per RFC 6901 (`~1` → `/`, `~0` → `~`). A duplicate
JSON key resolves to the last occurrence, matching serde_json's last-wins
behavior from R3.

## Details

### YAML

simplified-schema's `PositionMap::project` (`positions.rs`) runs
`locate_frontmatter_value`, a **line-based** block locator, then checks the
located tree against `MappingOrders` collected from the parsed value. The
spike copied `project` with the `SchemaValueKind::Mapping` root check
removed (it uses only public items: `simplified::locate_frontmatter_value`,
`simplified::yaml_scalar::alias_token`, `mapping_orders::MappingOrders`).

| Fixture | Result |
|---|---|
| spec `os.yaml` example, `- macOS` at `/0` | `(1, 3)` ✔ (needs the relaxed root) |
| scalar root `42` | `(1, 1)` ✔ |
| nested block maps/sequences, `/servers/1/port` | ✔ |
| list of maps root, single-line flow maps and sequences | ✔ |
| block scalar, multi-line double-quoted scalar, comments, `---` | ✔ |
| core tag `!!str`, an anchor that is never aliased | ✔ |
| multi-byte (`- {名前: é, n: bad}`) | ✔ after converting the byte offset |
| CRLF twin | same line and column as LF ✔ |
| alias anywhere (`other: *b`, `- *a`) | **refused** for the whole file |
| multi-line flow collection (`servers: [\n {port: 1},\n …]`) | **refused**: the locator gives up |
| 2000-object single-line flow array | ✔, about 25 ms in a debug build |

Findings:

- **Columns from `PositionMap` are UTF-8 bytes** (`ValidationProblem.column`,
  documented by the `columns_count_utf8_bytes` test). `data_location` must
  not copy `problem.column`. Take `PositionMap::byte_range(&problem)` (already
  public) and convert the start offset with the shared helper.
- The relaxed root affects only `project`. A frontmatter block whose root
  is not a mapping is rejected before validation, so frontmatter behavior is
  unchanged. Verify this with the existing `positions.rs` tests plus a
  list-root and a scalar-root case.
- An alias, or a flow collection that spans lines, makes `project` return
  `None`. The fallback `PositionMap::top_level` then reports top-level keys
  at column 1 for a mapping root and nothing for a list root. For
  `load_data` treat that as **pointer only**: a column-1 guess must not be
  presented as an exact position.
- Optional hardening, in shared code:
  - **Aliases.** Let `matches_orders` skip the subtree below an alias token
    instead of refusing the file. A path through an alias already stops at
    the alias scalar's start (`locate_range_in`'s "path through a scalar"
    rule). About 0.5 d.
  - **Multi-line flow collections.** Teach `locate_inline` to take a range
    across lines (`flow_collection_end` already handles quotes and
    comments). About 1 d. This would also give frontmatter positions for
    multi-line flow values.
- An event-based alternative was tried: `yaml-rust` 0.4.5, already in the
  lockfile through `serde_yaml` 0.8. Its marked events locate everything,
  including multi-line flow, and give character columns natively. It is
  unmaintained (RUSTSEC-2024-0320) and is a second YAML grammar beside
  libyaml, so it is **not recommended**.

### JSON

**Not through the YAML locator.** That path failed on:

- pretty-printed or any multi-line JSON (the locator is line-based)
- tab indentation (the locator gives up)
- duplicate keys (`serde_yaml_ng`: `duplicate entry with key "a"`, though R3
  says the last one wins)
- surrogate-pair escapes such as `"\ud83d\ude00"` (`serde_yaml_ng`:
  invalid Unicode escape)

It worked only for single-line minified JSON.

**Through `serde_json::value::RawValue`, it works.** Deserialize
`&RawValue`. At each pointer step, re-read only the container on the path,
as `Vec<(String, &RawValue)>` for an object (duplicates kept in written
order, and the **last** match taken) or as `Vec<&RawValue>` for an array.
The value's byte offset is `raw.get().as_ptr() - source.as_ptr()`.
simplified-schema already uses this exact technique in
`numeric/ingress.rs` (`admit_json_text_numbers`, `JsonEntries`,
`line_column`) to locate rejected numbers, with the same serde_json
features darkmatter uses (`raw_value`, `preserve_order`). The cost is
O(depth × container size) per located pointer, which is fine for at most
20 records.

The spike checked the RawValue locator against the json-five token locator
on 7 fixtures: pretty, CRLF, tabs, duplicate keys, escaped keys with
surrogates, multi-byte, and a whitespace-padded root. Every result was
identical.

### JSONL / NDJSON

Each record is one physical line. Run the JSON locator on that line's slice
and add the line's start offset (or add `record_line - 1` to the line). The
spike skips whitespace-only lines but keeps physical line numbers, so a
record after a blank line reports its real line. CRLF is ✔: the line
slice's trailing `\r` follows the value. A malformed line already carries
its line number under R3.

### CSV / TSV

- `csv::StringRecord::position()` gives each record's **start** byte and
  line correctly, including after a quoted field that contains a newline.
  This covers R4's start-line reporting.
- **Field ranges cannot be used for columns.** `ByteRecord::range(i)`
  indexes the record's *unescaped* buffer. For `a,"x""y,z",q`, field 2 has
  range `6..7` while its raw offset is 11.
- **`csv-core` gives exact raw offsets.** `csv_core::Reader::read_field`
  returns the number of input bytes it consumed, so a running total gives
  every field's raw start (the opening quote for a quoted field). Leading
  `\r`/`\n` bytes of a record's first field are blank lines that csv-core
  skips, so the start is advanced past them. These cases pass:
  - embedded delimiters (`"a, inc"`)
  - doubled quotes (`"say ""hi"""`)
  - quoted newlines (the next row reports the right line)
  - multi-byte cells before the target (`日本,"9"` → column 4)
  - CRLF
  - TSV with a quoted tab
  - a blank line between records
  - header and header-less pointers (`/1/port` and `/0/2`)
- Recommendation: build biscuit-file's delimited reader on `csv-core`, or
  have it run one `csv-core` offset pass beside the `csv` crate, so each
  record carries `start_line` and per-field raw byte offsets. `csv-core`
  0.1.13 is already in the lockfile as a dependency of `csv` 1.4.0; add it
  as a direct dependency. About 0.5 d including R4's strict-shape checks,
  which are unchanged.

### TOML

`toml::de::DeTable::parse(text)` (toml 1.1.2, the `parse` feature, which
biscuit-file's default `toml` dependency enables) returns
`Spanned<DeTable>`. Keys and values are `Spanned`, and `.span()` is a byte
range into the input. Walking `DeValue::Table` / `DeValue::Array` by pointer
segments located:

| Construct | Example pointer | Result |
|---|---|---|
| standard table | `/owner/age` | the value ✔ |
| array of tables | `/servers/1/port` | the value in the second `[[servers]]` ✔ |
| inline table | `/db/conn/port` | ✔ |
| dotted key | `/db/net/ip/v4` | ✔ |
| inline array | `/db/list/1` | ✔ |
| super-table defined after a sub-table (`[a.b]` … `[a]`) | `/a/d` | ✔ |
| multi-byte quoted key and value before the target | — | ✔ |
| CRLF | — | ✔ |

A pointer to a table itself lands on its header line at column 1, which
is good enough. `toml_edit` was not needed. This walk is independent of
`toml_to_json`'s value conversion and the number-admission work R3
requires.

### JSON5

`json-five` 0.3.1 (biscuit-file's JSON5 crate) exposes spans in two ways:
`tokenize_str` returns `(start, TokType, end)` **byte** spans of significant
tokens (whitespace and comments dropped; the tokenizer walks
`char_indices`), and `rt::parser` round-trips whitespace and comments. A
recursive descent over the token spans of about 60 lines builds the pointer
tree. Keys are decoded with `json_five::from_str::<String>` for quoted keys
and taken raw for `Name` tokens; unary `+`/`-` joins its operand. It passes:

- line and block comments
- unquoted and single-quoted keys
- trailing commas
- hex numbers, `-.5`, and `+1`
- a string with a line continuation (the next value's line is correct)
- multi-byte characters
- CRLF

Because JSON is a subset of JSON5 tokens, the same locator also handles
JSON. It agreed with the RawValue walk on every JSON fixture, so either can
serve JSON. RawValue is preferred because it is serde_json's own grammar.
`jsonc-parser` 0.33.2 is also in the lockfile (through `sniff`) and has a
ranged AST, but it does not cover all of JSON5 and would add a third JSON
grammar. Not needed.

### Malformed input: parser-reported positions

Parsers disagree on what a "column" is. For a bad token at character
column 9 / byte column 13:

| Parser | Reports |
|---|---|
| `serde_json` | `line 2, column 13`: **byte** column |
| `serde_yaml_ng` | `Location { line, column, index }`: index is a byte offset |
| `json-five` | `line 2 column 9 (char 10)`: character column |
| `toml` | `span() = 17..18`: byte range |
| `csv` | `Position { byte, line, record }` |

Rule: turn every parser error into a **byte offset** (for serde_json, the
line start plus `column - 1` bytes), then apply the shared helper. Never
pass a parser's own column through.

### BOM, URLs, line breaks

- The BOM is stripped before parsing (R3), so offsets index the stripped
  text and line 1 columns match what an editor shows.
- URL-loaded data uses the same locators on the fetched text. Nothing
  depends on the source being a file.
- A lone `\r`, not followed by `\n`, is **not** a line break under the
  shared helper. YAML treats it as one, and so do `positions.rs`'s own
  `LineIndex` and csv-core. Either state "LF or CRLF" in R5, or have the
  shared helper also break on a lone `\r`. This is minor; such files are
  rare.

## Ownership

`biscuit-file` sits below `simplified-schema`. All format-specific parsers
(`toml`, `json-five`, `serde_json`, and the new `csv`/`csv-core`) are or will
be its dependencies.

- **biscuit-file**: a small locate module (or one function per format
  module) returning `Option<Range<usize>>` for a decoded pointer, with
  `json_pointer_span` (RawValue), `json5_pointer_span` (tokens), and
  `toml_pointer_span` (DeTable). The CSV/TSV reader exposes per-record
  `start_line` and per-field raw offsets. The shared offset → (line, char
  column) helper lives here (feature-free) so darkmatter and
  simplified-schema can stop carrying private copies. That consolidation is
  optional and should not be done in this feature.
- **simplified-schema**: YAML only. Relax `PositionMap::project`'s root
  check. Optionally harden aliases and multi-line flow. No new public API
  is required, because `PositionMap::byte_range` is already public.
- **darkmatter**: selects the locator by format, converts offsets with the
  shared helper, fills `data_location`, and treats a `None` from any
  locator as pointer only.

## Recommended spec changes

**R3**

- Add `csv-core` (lockfile 0.1.13, through `csv`) to the biscuit-file
  dependency note, next to `csv`.
- The delimited reader returns each record's start line **and each field's
  raw byte offset**, since the `csv` crate's field ranges index the
  unescaped buffer.
- Every malformed-input position is normalized to a byte offset and then to
  line and character column. Parsers report columns inconsistently (see the
  table above).

**R5 design notes.** Replace "Multi-line JSON, TOML, and JSON5 depend on
spike SP2" with the per-format locators above.

- YAML: `PositionMap::project` accepts any root; the column comes from
  `PositionMap::byte_range` converted to characters, **not** from
  `ValidationProblem.column`, which counts bytes.
- A YAML file containing an alias or a multi-line flow collection reports
  the pointer only (unless the optional hardening is in scope). The
  column-1 `top_level` fallback is never reported as `data_location`.
- State the line-break rule: LF or CRLF; a lone CR is not a break (or the
  alternative chosen).

**R5 acceptance criteria**

- Keep the YAML `os.yaml` criterion (`/0` → line 1, column 3; confirmed).
- One contract-failure fixture per format: YAML, JSON (pretty), JSONL, CSV,
  TSV, TOML (array of tables), JSON5.
- A YAML fixture with an alias, and one with a multi-line flow sequence,
  each report the pointer without `line` and `column`.
- A JSON fixture with a duplicate key reports the position of the last
  occurrence.
- A CSV fixture with a quoted field containing a delimiter, a doubled quote,
  and a newline before the bad cell reports the cell's raw column.

**Definition of done.** The pointer-only formats named in the docs and the
implementation log are **YAML files containing aliases or multi-line flow
collections**, and no other format.

**Testing.** The Diagnostics bullet's "line and column where SP2 confirms
them" becomes "for every format", with the two YAML pointer-only cases.
