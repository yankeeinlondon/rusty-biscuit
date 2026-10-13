# Cleaning

Cleaning is the compose step that tidies the composed Markdown body after
transclusion and before link normalization. It uses the same engine as
[`md clean`](../cli/clean.md), so it follows the same guarantees: it never
changes what the document means, never touches a directive line, and never
adds trailing whitespace. If it cannot format the body safely, it leaves the
body exactly as it was.

## What compose cleans

Compose cleans the composed body only. It never repairs frontmatter, and it
never sees the separate documents that an expression or transclusion loads
on their own (see [Loading Documents](../darkmatter-compose-pipeline.md#loading-documents)).

Compose does **not** read the document's `style.markdown`. Its settings come
from `ComposeOptions` (and from the matching `md compose` flags), so the same
document composes the same way wherever it is used:

| Rule | Compose default | Set with |
| --- | --- | --- |
| `unwrap` | on: single newlines inside prose are joined | `with_incidental_newline_mode(IncidentalNewlineMode::Preserve)` turns it off |
| `fixed-width` | off | `with_fixed_width(width)`; implies `unwrap`, even under `Preserve` |
| `indentation` | `4` | `with_indentation(Indentation::Two)`, or `md compose --indent 2` |
| `list-item-padding` | `preserve` | `with_list_spacing(ListItemPadding::None)` / `One`, or `md compose --compact` / `--loose` |
| everything else | the built-in default | not configurable here |

Two of these differ from `md clean`: compose joins wrapped prose by default
(`unwrap` on), because composed output is usually read rather than edited.
`md clean` keeps your line breaks unless you ask for `unwrap`.

```rust
use darkmatter::markdown::cleanup::rules::{Indentation, ListItemPadding};
use darkmatter::markdown::compose::ComposeOptions;

let options = ComposeOptions::default()
    .with_indentation(Indentation::Two)
    .with_list_spacing(ListItemPadding::None)
    .with_fixed_width(80);
```

To skip cleaning entirely, disable the operation:

```rust
use darkmatter::markdown::compose::{ComposeOperation, ComposeOptions};

let options = ComposeOptions::default().disable(ComposeOperation::Cleanup);
```

## Output shape

Cleaning does not strip trailing whitespace or force a final newline. If an
interpolation evaluates to an empty string, the spaces around it stay where
they were. `md compose` prints the body followed by one newline.

The full rule reference, including examples for every value, is in
[Markdown Formatting Rules](../topics/markdown-formatting.md).
