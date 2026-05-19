# Lessons Learned

> This page is a place to write down novel or surprising things we've encountered (as well as how we were able to address them) as they relate to designing and implementing components through an intermediate IR

## BlockQuote: semantic tree vs. compatibility prefix

`BlockQuote::with_border()` exposes a terminal-specific compatibility API that accepts an arbitrary prefix string, while the render tree represents block quotes semantically and styles borders through typed `Style::Border`. That arbitrary prefix should not be promoted into `NodeKind::BlockQuote`; otherwise the canonical tree would gain a component-specific terminal presentation detail.

The migration pattern for this kind of API is to route the normal/default component through the tree renderer and keep a small bespoke fallback for compatibility-only knobs that are not target-agnostic.

## Compose: document roots are not concatenation containers

`Compose` looks like it can project to a `NodeKind::Root` with one child per
part, but current Terminal and Markdown root rendering treats children as
document blocks and joins them with blank lines. That breaks Compose's core
contract: adjacent parts concatenate with no automatic separator.

The migration pattern is to add an explicit sequence/fragment join contract to
the render tree instead of overloading normal document-root behavior. Normal
Markdown document spacing should remain unchanged; Compose needs a deliberate
no-separator sequence marker or node.
