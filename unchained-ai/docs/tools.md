# Tools

The [`rig` ecosystem](./rig-integration.md) is leveraged heavily to provide tool features and in particular the `Tool` trait which it defines (and this library reports as `ToolDefinition`). We will dip into creating tools a little later but to start we'll discuss the basic interaction most callers need to be aware of.

In a word, most callers just need to know about the `Tool` enumeration.

- please note that this is **not** the `Tool` trait from rig
- when using this library it is important to distinguish:
    - `Tool` an enumeration of all available tools to other pipelining structs
    - `ToolDefinition` the _trait_ defined in the **rig** crate

## Using the `Tool` Enumeration

This enumeration provides a full catalog of statically available tools as well as a lookup variant which can provide:

### Popular MCP Servers

- `Context7` -- FUTURE (SOON)
- `Exa` -- FUTURE (SOON)

### Rust Built-in Tools

- `WebSearch`
    - Uses either [`BraveSearch`](https://brave.com/search/api/) or [`Firecrawl Search`](https://www.firecrawl.dev/playground?endpoint=search) under the hood
    - [`BraveSearch`](https://brave.com/search/api/) is ready;
    - [`Firecrawl Search`](https://www.firecrawl.dev/playground?endpoint=search) coming soon
    - FUTURE: build a free search tool which can be used for internal searches but would not be able to be used in a

- `ScreenScrape`
    - Currently we have a custom tool for scraping
    - [`Firecrawl Scrape`](https://www.firecrawl.dev/); coming soon


> Plenty more **future** tools are planned: [Future Tools](./future-tools.md)
