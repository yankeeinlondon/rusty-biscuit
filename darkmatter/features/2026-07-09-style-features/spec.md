# Style Features

When we designed the "style" system for Darkmatter we built a set of CSS-like configuration for certain block items like tables, code-blocks, block quotes, and more. This grammar allows for some
useful configuration of stylistic rendering but one thing that was designed but not yet realized is the idea of a "feature" which a renderable component could declare. When a component expressed it's dependency on a feature, the rendering process would catalog all features on a given page
