# Transclusion

The term **transclusion** was coined by [Ted Nelson](https://en.wikipedia.org/wiki/Ted_Nelson) in the 1960's. Ted was a pioneer of information technology, a philosopher of computer science, and a sociologist. Ted also coined the terms **hypertext** and **hypermedia**. Basically ... Ted was a smart cookie.

The _motivation_ for **transclusion** comes from the same ideas or goals that DRY (_**D**on't **R**epeat **Y**ourself_) comes from. Where DRY posits the benefits of capturing some business logic that is known to take place two or more times in a code base and centralizing it to an abstracted function that each area of the code that needs that logic an call, _transclusion_ takes up a similar position for **prose** content.

Today, most of the professional note taking applications include at least some form of transclusion. Why? It is a powerful way of _composing_ content in a consistent way which allows for greater investment in the quality for these shared passages of text than could otherwise be afforded for a one-and-done text.

Darkmatter provides a large variety of ways of performing transclusion but the most fundamental is the `::file` directive:

```md
## Best Practices

::file best_practices_for_abc.md

::file best_practices_for_def.md
```

> this simple directive will bring the content from the two referenced files _into_ the base document.

To make this transclusion grammar more powerful, **Darkmatter** adds in _conditional_ logic and other parameters the allow you to fine tune the operations effect based on the state of Frontmatter, ENV variables or other context variables.
