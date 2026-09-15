---
kind: cononical-doc
id: code-blocks
related: 
    - code-highlighting
    - mermaid-rendering
    - dot-rendering
todo: true
---
# Code Blocks in Darkmatter

The behavior of code blocks in Darkmatter is a superset of how it is handled in normal Markdown. 

## CommonMark

So let's first agree on what Markdown (and by this I mean CommonMark) say about code blocks:

There are two types of code blocks: 

- **Indented:** each line is indented by at least four spaces, may not have an "info string".
- **Fenced:** opened by at least three backticks or tildes and closed with the same number of the same character.

Most people gravitate to assuming you're referring to **Fenced** code blocks and that's usually fair considering they are far more common and they have an "info string". Why would you want an info string and what good does it do for you?

~~~ts I'm a lumberjack and I'm ok
const x = 1;
~~~

CommonMark says that everything following the opening markers (and on the first line) of a fenced code block is considered the **info block**. That means that in the example above "ts I'm a lumberjack and I'm ok" is the info block.

CommonMark goes on to say that the first space-delimited word in the info block is _typically_ the language identifier. CommonMark is a bit shy and doesn't want to be too definitive and that's fair but Darkmatter is less bashful as you'll see in the next section.

## Extending CommonMark

Darkmatter is more opinionated than CommonMark _but_ it's #1 opinion is that Darkmatter code blocks must ALWAYS be valid CommonMark code blocks. How Darkmatter starts to separate is by more opinionated use of the **info block**.

Sadly with all great rules, comes understandable exceptions and Darkmatter makes one:

- in order to align with the Rustdoc convention of using a "," as a separator Darkmatter allows for it
- that means the following code block -- who's language is "officially" rust,no_run -- is parsed so that `rust` is the language and `no_run` is separated out as the remaining info block:

    ```rust,no_run
    // ...
    ```

## Darkmatter Superpowers

Darkmatter's use of the "info block" provides it super powers that include:

- `title={title}`
- `footer={footer}`
- `when={condition}`
- `theme={theme}`

    People love their color themes and Darkmatter knows it. When rendering with Darkmatter there are a few concepts that are important to understand:

	- in the scary "outside world" color themes typically are a light or dark colored theme but not both
	- in Darkmatter we have a set of themes to choose from that will _adjust_ based on whether the terminal or browser is in light or dark mode
	- For a list of all the themes you can choose from see: [Darkmatter Themes](./color-themes.md)

    In most cases you'll want to choose a code theme at render time and then have all code blocks use the same theme but people are never happy so Darkmatter has decided to reward your poor color judgment and allow you to change colors on a _per_ block basis if you want.

- `line_numbers={boolean}`
- `use={lang}`
- `highlight={grammar}`

These superpower will be evident not when you _compose_ a document -- because in those situations the info block just remains text -- but when you are _rendering_ a document the semantic meaning of the info block comes into bloom.

### Rendered Languages

When you are using Darkmatter to render to the browser (HTML), the terminal, or any other non-Markdown format we can faithfully render a huge number of common programming languages (Rust, Javascript, Python, Typescript, Golang, etc.) with a wealth of color themes to choose from. The syntax is no different regardless of the output target you choose.

If you choose a language that is outside of our supported list (or you just made up the language), you will by default have your block rendered as `text` but you can modify that with the `use` property discussed earlier. 

For instance, imagine you've just created a language called "suck-a-duck" and even though you feel it should be getting more recognition it still remains a fringe language. You know -- as do your hidden tribe -- that your language is too good to render as plain text so you take advantage of the fact that this language as a very Javascript based grammar to it (some say your language is just a hack copy of Javascript ... but you not that's not true). Don't despair, the following code block will render with lovely Javascript colors and grammar rules:

~~~md
```suck-a-duck use=js
var duck = "suck"
```
~~~

## Special Cases

Code blocks are so good that people have decided to abuse them. Yes you heard me right, that code block you were just were looking at might be an imposter, a hack, maybe something far worse. Don't worry fair ladies and gentleman as these scary hacks are almost all a good thing!

The exemplar "hack" is Mermaid. When you specify the language `mermaid` you will not see the Mermaid DSL color coded in full glory. Oh no, it will instead _render_ the diagram you've described with the DSL. Well that's nice! I mean it will only work if you have a render that knows about the dark secrets Mermaid holds. Fortunately for you Darkmatter carries a lot of secrets including Mermaid's.

To find out more about special renderers that Darkmatter supports choose any of the following links:

- [**Mermaid** rendering](../rendering/mermaid.md)
- [**Graph** rendering using DOT syntax](../rendering/dot-graph.md)
- FUTURE:
    - [**Git Flow** rendering](../rendering/git-flow.md)
    - [**Bar Chart**](../rendering/bar-chart.md)

All of these _special case_ renderers can be used with code blocks but they can also be used with Darkmatter's inline directives that have the benefit of being able to source the data portion of these renderers from an external file (or even remotely via HTTP):

- use `::mermaid <file>` for rendering Mermaid diagrams
- use `::dot <file>` for rendering DOT graphs

## Runtime Checking (_future_)

Rust's Rustdoc spec allows you to embed Rust code snippets and then have that code compiled, validated, etc. This of course has nothing to do with Darkmatter strictly speaking but it's a really powerful feature because it allows you to ensure that your code blocks in documentation are valid and useful to readers. Hopefully all code blocks start out this way but changing API surfaces has a way of silently invaliding code blocks over time and this can be dangerous if not embarrassing.

> Note: we hope to offer some AST or runtime validation features for other languages that can leverage the same syntax established by Rustdoc.
