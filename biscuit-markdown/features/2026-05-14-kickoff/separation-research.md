# Researching the separation of the `Markdown` struct

## External Callers of Markdown struct

1. External callers of darkmatter::markdown::Markdown

    Six non-darkmatter crates pull in darkmatter. Excluding documentation files, snapshot test fixtures,
    and the .orig files left over from refactors:
    
    Crate: sniff/cli
    Total darkmatter call sites: 13
    Imports Markdown?: yes
    Uses darkmatter-specific APIs beyond Markdown?: no
    ────────────────────────────────────────
    Crate: playa/cli
    Total darkmatter call sites: 3
    Imports Markdown?: yes
    Uses darkmatter-specific APIs beyond Markdown?: darkmatter::testing::strip_ansi_codes (test utility)
    ────────────────────────────────────────
    Crate: biscuit-speaks/cli
    Total darkmatter call sites: 3
    Imports Markdown?: yes
    Uses darkmatter-specific APIs beyond Markdown?: no
    ────────────────────────────────────────
    Crate: research/lib
    Total darkmatter call sites: 1
    Imports Markdown?: no (uses darkmatter::render::Link only)
    Uses darkmatter-specific APIs beyond Markdown?: no
    ────────────────────────────────────────
    Crate: claudine/cli
    Total darkmatter call sites: 43
    Imports Markdown?: yes
    Uses darkmatter-specific APIs beyond Markdown?: uses with_source(ComposeSource::*)
    ────────────────────────────────────────
    Crate: claudine/lib
    Total darkmatter call sites: 83
    Imports Markdown?: yes
    Uses darkmatter-specific APIs beyond Markdown?: heavy: compose::expression, compose::shell_expansion,
    
    compose::ComposeOptions, compose::ComposeContext, compose::ComposeSource,
    compose::ComposePerfReport

2. Which callers need plain markdown vs. full darkmatter
    
    Plain markdown only (3 crates) — these never touch anything outside the Markdown struct's rendering
    and basic frontmatter APIs:
    
    - sniff/cli — three sites in output/{topics,commit_blocks,remote}.rs. Pattern is uniform:
    Markdown::from(s) → for_terminal(&md, TerminalOptions::default()). Nothing else.
    - playa/cli — main.rs only. Same pattern.
    - biscuit-speaks/cli — main.rs only. Same pattern, plus md.content() for fallback.
    
    Doesn't really use Markdown at all (1 crate):
    
    - research/lib — imports darkmatter::render::Link exclusively (a hyperlink type for terminal output).
    Could just as easily get that from biscuit-terminal.
    
    Full darkmatter / compose DSL (2 crates):
    
    - claudine/lib — 83 references, of which 57 are .frontmatter() access (template-substitution
    machinery), plus extensive use of the compose DSL: ComposeOptions, ComposeContext, ComposeSource,
    ComposePerfReport, expression::evaluate, expression::is_truthy, expression::parse_condition,
    shell_expansion::discovery, shell_expansion::policy, etc. This is darkmatter's reason for existing —
    composable markdown templates with conditionals, transclusions, and shell expansion. Cannot be served
    by a plain-markdown library.
    - claudine/cli — similar shape, mostly UI wiring around the same compose pipeline.

## Inside the Markdown struct itself

The struct definition (darkmatter/lib/src/markdown/mod.rs:78):

```rust
pub struct Markdown {
    frontmatter: Frontmatter,
    content: String,
    source: Option<ComposeSource>,   // ← only darkmatter-specific field
}
```

Markdown has 39 public methods. Categorising them by what they touch:

Category: Construction & accessors
Count: 9
Methods: new, with_frontmatter, try_from_content, content(), content_mut(), into_parts, as_string,
as_ast, from_url
Darkmatter-specific?: no
────────────────────────────────────────
Category: Frontmatter access
Count: 7
Methods: fm_get<T>, fm_insert<T>, fm_merge_with<T>, fm_set_defaults<T>, frontmatter(),
frontmatter_mut()
Darkmatter-specific?: no (Jekyll/Hugo-style YAML frontmatter is portable)
────────────────────────────────────────
Category: Reference extraction
Count: 5
Methods: links, image_references, has_inline_html, inline_html_links, inline_html_image_references
Darkmatter-specific?: no (pure markdown / inline HTML scanning)
────────────────────────────────────────
Category: Cleanup / surgery
Count: 8
Methods: cleanup, cleanup_with_indent, cleanup_compact, cleanup_loose, cleanup_with_indent_compact,
cleanup_with_indent_loose, remove_section, remove_sections
Darkmatter-specific?: no
────────────────────────────────────────
Category: Rendering
Count: 2
Methods: as_html(HtmlOptions), as_terminal(TerminalOptions)
Darkmatter-specific?: no (the formats and options are markdown-shaped)
────────────────────────────────────────
Category: Structural analysis
Count: 5
Methods: toc, delta, validate_structure, normalize, normalize_mut, relevel
Darkmatter-specific?: no (heading-hierarchy + diff utilities — generally applicable)
────────────────────────────────────────
Category: Darkmatter-specific
Count: 3
Methods: source(), with_source(ComposeSource), source_context_for_errors()
Darkmatter-specific?: yes — ComposeSource is the darkmatter DSL's notion of where a document came
from

~92% of the Markdown API surface is pure markdown. The only darkmatter-specific pieces are:
- the source: Option<ComposeSource> field
- the three methods that touch it
- the link to biscuit_terminal::errors::SourceContext (used only when reporting compose-time errors)

The compose layer that the methods reference (ComposeSource and its friends) is what's actually heavy
with darkmatter-specific behaviour — but Markdown itself just holds a reference to it.

What this means for your library proposal

The split lines up cleanly with the proposal:

## Recommended layering

biscuit-markdown                   (proposed new crate, no compose, no DSL)
├── Markdown { frontmatter, content }
├── Frontmatter
├── 36 of the current 39 methods
├── impl Renderable for Markdown          ← from biscuit-terminal
└── impl BrowserRenderable for Markdown   ← from biscuit-terminal

darkmatter::markdown                (stays, depends on biscuit-markdown)
├── Re-exports biscuit-markdown::{Markdown, Frontmatter, ...}
├── compose::                       (transclusion, expressions, shell expansion)
├── ComposeSource, ComposeContext, ComposeOptions
└── Helpers that bind Markdown to a ComposeSource

The source: Option<ComposeSource> field needs to live somewhere. Two options:

- A) Leave it on Markdown in biscuit-markdown, but make the type generic: Markdown<S = ()> with S
being the source-attachment type. darkmatter instantiates it as Markdown<ComposeSource>. Plain
consumers use Markdown<()>.
- B) Drop source from the base type entirely and let darkmatter wrap it: darkmatter::ComposedMarkdown
{ md: biscuit_markdown::Markdown, source: ComposeSource }. Cleaner separation, but darkmatter would
need to either duplicate the rendering API by delegating, or expose md and accept that callers reach
through.

I'd argue (B) is cleaner — ComposeSource only matters when the compose layer needs to report where a
fragment came from, and a base Markdown instance with no compose involvement (e.g. anything
sniff/playa/biscuit-speaks does) has no source to track anyway.

Migration impact

Crate: sniff/cli, playa/cli, biscuit-speaks/cli
Change needed: Swap darkmatter::markdown::Markdown → biscuit_markdown::Markdown. Same call shape.
Drops darkmatter dep entirely.
────────────────────────────────────────
Crate: research/lib
Change needed: Either inline a small Link type or move it to biscuit-terminal. Drops darkmatter dep
entirely.
────────────────────────────────────────
Crate: claudine/{cli,lib}
Change needed: No change; keeps using darkmatter for the compose DSL. The Markdown it sees is
re-exported from biscuit-markdown.
────────────────────────────────────────
Crate: darkmatter/lib
Change needed: Becomes thinner — markdown plumbing moves out, compose layer stays.

Two wrinkles worth flagging before you commit

1. as_html(HtmlOptions) and as_terminal(TerminalOptions) today live in darkmatter::markdown::output.
Pulling these into biscuit-markdown means moving the HTML and terminal rendering pipelines (themes,
syntax highlighting, mermaid handling). That's a meaningful chunk of code — possibly the biggest item
in the migration. Worth scoping separately. The user-facing impression of darkmatter's CLI
("darkmatter renders nice markdown") is mostly about this code, not the compose DSL.
2. biscuit-markdown would re-export Frontmatter and MergeStrategy. Today they live in
darkmatter::markdown::frontmatter. Moving them is mechanical but touches the public API of darkmatter
— any external callers (none in this monorepo, but you might have downstream consumers) would need
re-import paths updated.

My take

The split is real and the boundaries are obvious — three crates would lose their darkmatter dep
entirely, only claudine would still need it, and darkmatter itself would be more focused (compose DSL
+ rendering pipelines for that DSL). The migration's actual size is mostly in the output/ rendering
tree, not in the trivial Markdown struct.

Worth doing. If you want, I can scope a follow-up implementation plan that goes through it in pieces
(the struct first, then frontmatter, then the renderers).
