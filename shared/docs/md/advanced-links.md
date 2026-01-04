# Advanced Links

## Leveraging the Markdown Standard

In Markdown the overwhelming popular usage for _hyperlinks_ is: `[display text](resource)` where:

- the resource is either a relative link to something (but typically another Markdown page)

Most people do not know that the specification actually allows for a second parameter within the parenthesis:

> [display text](resource title)

The `title` was added to the original specification so that people could add some basic "popover" content since browsers have for a long time provided a rather crude popover effect for links if you added a `title` property.

### How to use the `title` Property

When we render Markdown which use the title property we will do it in one of two modes:

1. **Title Mode**
2. **Structured Mode**

The mode which will be used will be based on whether structured content is found in the `title` property or not. This ensures that if the author of the Markdown was intended to just use the standard then they will get that behavior but the extra capabilities of **Structured Mode** can be leveraged if the author choses to use them.

#### Title Mode

This mode resembles what the Markdown standard expects and Uses all content after the URI/resource link as the "title".

#### Structured Mode

This mode leverages the standard but extends it's capabilities by viewing the `title` text as a bunch of key/value pairs. The key's which have special meaning are:

| key       | terminal     | browser |
| ---       | --------     | ------- |
| `title`   | no effect    | adds the text to `title` property of link |
| `prompt`  | no effect    | adds the text to the `prompt` property of a link (which will trigger modern Popover) |
| `class`   | limited*    | adds the classes specified in the key's value to the HTML link |
| `style`   | limited*    | allows the user to add CSS properties to the HTML link |
| `*`       | no effect   | all other key values are passed through as properties to the HTML link |


The syntax which is used here follows a `key=value` syntax and can be delimited by a comma or whitespace:

- `[my link](https://somewhere.com prompt="click me",class=buttercup style="background:red" )`

The above example is a valid syntax and:

- we can see that property values can be quoted but don't need to be (though "quoting generally considered the safer option")
- key/values can be delimited by a `,` or whitespace

## Output Targets

### Targeting the Terminal




### Targeting the Browser


Today all the popular browsers now provide a much more capable Popover system:

- [Modern Popovers in the Browser](./modern-popovers-in-the-browser.md)

> **IMPORTANT:** if you're planning or implementing a popover solution for HTML then you MUST read the link above for full context!


We will use this system to make the Markdown links more capable when we render our Markdown to HTML.
