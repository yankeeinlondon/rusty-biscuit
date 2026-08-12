---
prompt: "Platform's like Obisidan use the [wiki style links]() because of their powerful two-way linking capabilities. This style of linking ultimately harkens back to the princicple and goals of [hypertext](https://en.wikipedia.org/wiki/Hypertext) which in turn gave rise the World Wide Web as we know it today.\n\n## Task\n\nYour task is to research in depth the idea of \"wiki style links\" as well as major apps/websites who's content are representations of this style of linking.\n\nFollow these steps exactly:\n\n0. Start by adding an H1 Heading/title to the body of `# Wiki Style Links and Hypertext`\n1. Research \"wiki style links\", \"hypertext\", \"interwiki linking\" and then add a H2 Section `## What is Wiki Style Linking` where you will describe all of the relevant terms\n    - describe their intent, major use cases, constraints, etc.\n2. Add a new H2 heading of `## High Profile Use Cases` and then act as an orchestrator and have each subagent tackle a different high profile app or website which uses this style of linking. The examples we will use are:\n    - Wikipedia(MediaWiki)\n    - Obsidian\n    - Notion\n    - Roam Research\n    - GitHub Wikis\n    - Atlassian Confluence\n    - [Foam](https://sourceforge.net/projects/foam.mirror/) \n    - [Dendron](https://marketplace.visualstudio.com/items?itemName=dendron.dendron#:~:text=Most%20PKM%20tools%20help%20you,diligent%20about%20organizing%20their%20knowledge.)\n\n    For each subagent you will provide them a summary of what 'Wiki Style Links' are and ask them to research their specific app/site and provide back the following:\n\n    - How does the given app/site use this style of linking?\n    - How is this app/site faithful to the standards? How does it deviate?\n    - How do these apps/sites provide LSP-like features for their users? Auto-complete? Hover effects with context? etc.\n\n    These subagents can be run in parallel and when one returns it's findings you will be responsible for adding it into the document under an H3 heading named after the site/app\n\n3. Add a H2 heading `## Summary` and just summarize the overall topic of wiki-style linking and how app/sites use it today."
last_updated: 2026-07-04
hash: d41271f6ae92f5c0-33b0ba4650d36491
---
# Wiki Style Links and Hypertext

## What is Wiki Style Linking

Wiki-style linking is a lightweight authoring convention for connecting one page, note, heading, block, file, or concept to another from inside ordinary text. The most recognizable form is the double-square-bracket link:

```markdown
[[Page title]]
[[Page title|display text]]
[[Page title#Heading]]
```

The convention is strongly associated with [MediaWiki links](https://www.mediawiki.org/wiki/Help:Links), where `[[Page title]]` creates an internal link, `[[Page title|label]]` changes the visible label, missing pages become red links, and prefixed forms can link across wiki namespaces or other registered sites. Modern note-taking and knowledge-base tools use the same shape because it is quick to type, readable before rendering, and easy for an editor to autocomplete.

The intent is not just shorter syntax. Wiki-style links make linking a normal part of writing. Instead of stopping to copy URLs or organize files into a strict hierarchy, the writer can name a concept directly and let the system resolve it. This supports:

- Fast internal linking while writing.
- Creation of missing pages from links.
- Backlinks that show where a page is referenced.
- Graph views and relationship discovery.
- Autocomplete over known pages, headings, blocks, tags, and files.
- Refactoring features such as rename-with-link-updates.
- Hover previews and contextual navigation.

Wiki-style links are part of the broader history of [hypertext](https://en.wikipedia.org/wiki/Hypertext): non-linear documents connected by links. Hypertext predates the Web as an idea and became the organizing principle of the World Wide Web. A normal HTML link is usually one-way: page A links to page B. Wiki systems build richer editorial behavior on top of that idea by making the link graph visible, editable, searchable, and often bidirectional.

Interwiki linking extends the idea across wiki boundaries. In MediaWiki and Wikimedia projects, registered prefixes let authors use internal-link-like syntax to point to another project, language edition, or site, such as `[[:en:Apple]]` for English Wikipedia from another Wikimedia wiki. Wikimedia describes this as [interwiki linking](https://meta.wikimedia.org/wiki/Help:Interwiki_linking_on_Wikimedia_wikis). The exact prefixes are configured per wiki, so interwiki linking is more of a managed namespace system than a universal Web standard.

There is no single universal standard for wiki-style links. Common conventions exist, especially `[[target]]` and pipe labels, but each platform defines its own resolution rules, escaping rules, namespace behavior, and editor features. The main constraints are therefore portability and ambiguity. `[[Page]]` is not standard CommonMark Markdown, page titles may conflict or change, filename rules differ across operating systems, and advanced targets such as blocks or headings are often tool-specific.

## High Profile Use Cases

### Wikipedia (MediaWiki)

Wikipedia is the canonical high-profile example of wiki-style linking. Its MediaWiki engine uses double brackets for internal links, pipe syntax for labels, section links for anchors, single brackets for external links, and configured prefixes for interwiki and interlanguage links. MediaWiki’s own help describes internal wikilinks, external links, interwiki links, and interlanguage links as distinct kinds of hypertext links.

Wikipedia is faithful to the classic wiki model: page titles are first-class link targets, missing pages can appear as red links, and pages expose reverse-link navigation through “What links here.” It also supports MediaWiki-specific extensions such as namespaces, files, categories, templates, redirects, and parser behavior that go far beyond a simple `[[target]]` convention.

Its LSP-like features are mature but wiki-specific rather than language-server-based. VisualEditor provides link search and insertion, Page Previews show contextual summaries on hover, Reference Previews show citation context, CodeMirror can highlight wikitext structure, and “What links here” provides a backlink index. Sources: [MediaWiki Help:Links](https://www.mediawiki.org/wiki/Help:Links), [Wikimedia interwiki help](https://meta.wikimedia.org/wiki/Help:Interwiki_linking_on_Wikimedia_wikis), [MediaWiki Page Previews](https://www.mediawiki.org/wiki/Page_Previews), [MediaWiki What links here](https://www.mediawiki.org/wiki/Help:What_links_here).

### Obsidian

Obsidian uses wiki-style links as a core local-first note-taking primitive. Its help documents `[[Three laws of motion]]` as a wikilink and also supports equivalent Markdown links. By default Obsidian generates wikilinks because they are compact, though users can disable wikilinks for more interoperable Markdown output.

Obsidian is faithful to the common wiki convention in its use of `[[note]]`, `[[note|alias]]`, missing-note creation, and backlinks. It extends the model with note headings, blocks, embeds, attachments, aliases from note properties, and vault-wide graph analysis. Its main deviation is portability: wikilinks and block references are not standard Markdown, and its internal-link model is vault-local rather than interwiki by default.

Its editor features are close to what users expect from an IDE. Typing `[[` opens note suggestions, `[[#` suggests headings, `[[##` searches headings across the vault, and block links can search block targets. The Page Preview plugin shows hover previews; Backlinks show linked and unlinked mentions; Outgoing Links helps discover possible links; Graph View visualizes note relationships. Source: [Obsidian Internal links](https://obsidian.md/help/links).

### Notion

Notion uses wiki-style linking as an editor command rather than as durable Markdown or wikitext source. Typing `[[` lets a user search for and link to an existing page, create a subpage, or create a new page. Notion also uses `@` mentions for pages, people, dates, and reminders, and backlinks are created automatically when pages are mentioned.

Notion is faithful to the user experience of wiki linking: fast page search, inline page references, automatic backlinks, and navigation back to the exact place where a page was mentioned. It deviates from classic wiki syntax because the underlying document model is a rich block database, not plain text wikitext. It does not expose MediaWiki-style interwiki prefixes as the central model, and labels are managed through Notion’s rich-page references rather than raw `[[target|label]]` syntax.

Its LSP-like features include page autocomplete, slash commands, page mentions, hover previews for link mentions, synchronized page title/icon display, backlink visibility controls, block links, linked databases, and relation properties for explicit structured relationships. Sources: [Notion links and backlinks](https://www.notion.com/help/create-links-and-backlinks), [Notion creating links and backlinks guide](https://www.notion.com/help/guides/creating-links-and-backlinks).

### Roam Research

Roam Research popularized wiki-style links in modern personal knowledge management. It uses `[[Page title]]` page references to create and connect pages inline, then automatically surfaces linked references as backlinks. Roam also works at the block level: `((block reference))` can point to a specific outline block, not just a page.

Roam is faithful to wiki linking in its double-bracket page references and missing-page creation flow. It deviates by making the graph database and bidirectional reference model the center of the product. Pages are not just documents; they are nodes in a graph of pages and blocks. Roam’s “unlinked references” feature also goes beyond ordinary hypertext by finding plain-text mentions that could become links.

Its LSP-like features include page autocomplete from `[[`, block search from `((`, linked references, unlinked references, filters over references, graph-oriented navigation, and quick creation of pages from references. Public documentation for hover previews is less consistent than for Obsidian or Dendron, so hover behavior should be treated as implementation- or ecosystem-dependent rather than the core contract. Sources: [The Sweet Setup Roam guide](https://thesweetsetup.com/a-thorough-beginners-guide-to-roam-research/), [Ness Labs on Roam](https://nesslabs.com/evernote-to-roam).

### GitHub Wikis

GitHub Wikis are repository-backed documentation spaces. GitHub documents normal Markdown links for Markdown-rendered wiki pages and MediaWiki-style links such as `[[Nameofwikipage|Link Text]]` for supported wiki syntax. In practice, many GitHub wikis also use simple `[[Page Title]]` links.

GitHub Wikis are faithful to the basic wiki idea: a repository gets a documentation area with pages, history, local editing, sidebars, footers, and page-to-page linking. They deviate from richer wiki systems in two ways. First, GitHub’s documentation emphasizes full Markdown URLs for Markdown wiki pages, so `[[...]]` is not treated as core GitHub Flavored Markdown. Second, GitHub does not document configurable interwiki prefixes, backlinks, graph navigation, or rename-aware wiki refactoring as native wiki features.

The native LSP-like feature set is comparatively limited. GitHub provides web editing, preview, page history, revision comparison, wiki search, local Git editing, and some generic Markdown editing affordances. Rich autocomplete, backlinks, hover previews, graph views, and diagnostics usually come from external editors when the wiki repository is cloned locally. Sources: [GitHub editing wiki content](https://docs.github.com/en/communities/documenting-your-project-with-wikis/editing-wiki-content), [GitHub adding or editing wiki pages](https://docs.github.com/en/communities/documenting-your-project-with-wikis/adding-or-editing-wiki-pages).

### Atlassian Confluence

Confluence is a corporate wiki and documentation platform with a long history of wiki-style behavior. Older Confluence wiki markup uses single-bracket forms such as `[pagetitle]`, `[spacekey:pagetitle]`, and `[link alias|pagetitle#anchor]`. Current Confluence Cloud emphasizes a rich editor, link dialogs, Smart Links, and keyboard shortcuts rather than raw double-bracket syntax.

Confluence is faithful to wiki principles: internal pages can be linked by title, spaces behave like namespace containers, links can target anchors and headings, undefined placeholder pages can be created, and incoming/outgoing links are available through page information. It deviates from modern `[[Page]]` conventions because its historical syntax uses single brackets and its current Cloud editor stores rich document content rather than editable wiki source.

Its LSP-like features are strong in the editor. Typing link triggers or pressing `Cmd+K` / `Ctrl+K` searches pages, attachments, user profiles, and other targets. It also autocompletes mentions, files, macros, and emojis. Smart Links can unfurl URLs into inline titles, cards, embeds, or previews; incoming links and undefined-page reports support relationship discovery. Sources: [Confluence links and anchors](https://support.atlassian.com/confluence-cloud/docs/insert-links-and-anchors/), [Confluence autocomplete](https://confluence.atlassian.com/doc/autocomplete-for-links-files-macros-mentions-and-emojis-249858190.html), [Smart Link view options](https://support.atlassian.com/platform-experiences/docs/smart-link-view-options/).

### Foam

Foam is an open-source, VS Code-based personal knowledge management system. The SourceForge page linked in the prompt mirrors the `foambubble/foam` project. Foam encourages Markdown notes connected by `[[wikilinks]]`, with support for section links, block links, aliases, backlinks, graph visualization, and local file ownership.

Foam is faithful to modern wiki-style linking in its double-bracket syntax, aliases, missing-note placeholders, and backlink-centered navigation. Its primary deviation is Markdown portability: `[[wikilinks]]` are not CommonMark. Foam addresses this by optionally generating Markdown link reference definitions so the same notes can work better with standard Markdown processors and publishing tools.

Because Foam is built on VS Code, its LSP-like behavior is especially direct. It provides wikilink autocomplete, filtered suggestions, go to definition, peek references, link preview/navigation, diagnostics for ambiguous links, rename-aware link updates, syntax highlighting for links and placeholders, backlinks, graph visualization, tag exploration, and orphan/placeholder panels. Sources: [Foam GitHub](https://github.com/foambubble/foam), [Foam navigation docs](https://docs.foamnotes.com/getting-started/navigation/), [Foam VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=foam.foam-vscode).

### Dendron

Dendron is an open-source, local-first Markdown knowledge-base extension for VS Code and VSCodium. It uses double-bracket wiki links such as `[[hello]]`, supports aliases, header links, backlinks, graph navigation, and missing-note creation. It also adds a hierarchical naming model and a vault abstraction for organizing large knowledge bases.

Dendron is faithful to wiki-style linking in its `[[...]]` syntax, missing-note affordances, backlinks, and heading links. It deviates in several product-specific ways. Its alias order is documented as `[[label|target]]`, while MediaWiki commonly uses `[[target|label]]`. Dendron also supports URI-like cross-vault links such as `[[dendron://vault/foo]]`, which are closer to a product namespace than classic interwiki prefixes.

Its LSP-like features include IntelliSense-style link autocomplete, hover previews for linked notes, syntax highlighting for existing versus missing notes, backlinks panels, graph view, lookup/create-note flows, broken-link repair through Doctor commands, and refactoring commands for renaming notes, renaming headers, moving notes, merging notes, and updating references. Sources: [Dendron Wiki Link](https://wiki.dendron.so/notes/90mrtp10ucyyvt60qekuj4y/), [Dendron Marketplace](https://marketplace.visualstudio.com/items?itemName=dendron.dendron), [Dendron GitHub](https://github.com/dendronhq/dendron).

## Summary

Wiki-style linking is a practical authoring pattern for building living networks of documents. Its most familiar syntax, `[[Page]]`, descends from wiki systems like MediaWiki, but its deeper purpose belongs to hypertext: let people move through knowledge by association rather than by a single hierarchy.

Today’s apps use the idea in different ways. Wikipedia and MediaWiki preserve the public, collaborative wiki model with red links, interwiki prefixes, and backlinks. Obsidian, Roam, Foam, and Dendron adapt the same pattern for personal knowledge graphs, local Markdown files, block references, graph views, and editor-style navigation. Notion and Confluence translate wiki linking into rich collaborative workspaces with autocomplete, mentions, Smart Links, backlinks, and permission-aware page references. GitHub Wikis keep the simpler repository-documentation version, relying more on Git and Markdown than on graph-native authoring.

The common thread is that a wiki-style link is more than a URL. It is a typed gesture inside the authoring environment: “this idea deserves to be connected.” The best systems then make that connection operational through autocomplete, previews, backlinks, search, graph views, missing-page creation, and refactoring. That is why wiki-style links remain powerful: they turn writing into a navigable, maintainable, two-way knowledge system.
