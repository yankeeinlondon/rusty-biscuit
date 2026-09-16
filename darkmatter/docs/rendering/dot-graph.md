# Graphing with DOT Grammar

## Code Blocks

Darkmatter supports [DOT](https://graphviz.org/doc/info/lang.html) grammar to render graph structures which offers a convenient way to quickly show graphs in a document:

~~~md
```dot
Start -> Validate -> Render;
Validate -> Retry
```
~~~

Will render a graph that looks something like:
![Graph Expression](graph-expression.png)

- [DOT grammar](https://graphviz.org/doc/info/lang.html) was defined by [Graphviz](https://graphviz.org) but is now used commonly as a text based grammar to express graph structures
- This offers an alternative to [Mermaid](./mermaid.md) for situations where you prefer the DOT ergonomic and/or standards based grammar
    - Note: it's not that DOT grammar is truly a standard but it's a _soft_ standard because of the popularity of [Graphviz](https://graphviz.org)

## Dot Directive

While for simple graphs the easiest solution for graph rendering is likely using `dot` as a language for code block and that works just fine but when you want to reuse DOT grammar or it's getting complex enough that you'd prefer to have it in a different file the `::dot` directive can be used:

- **Syntax:** `::dot <file> [params]`

This directive can be used as either a _block_ or _inline_ directive:

- Block Directive: 
    - you can start any line with the `::dot` prefix
    - you can also have any amount of whitespace before the prefix `{whitespace}::dot`
- Inline Directive:
    - to support putting these diagrams into tables and other structural containers that Markdown provides, it's very useful to have an _inline_ variant as well
