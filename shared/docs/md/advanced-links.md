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
| `data-*`       | no effect   | passed through as properties to the HTML link |


The syntax which is used here follows a `key=value` syntax and can be delimited by a comma or whitespace:

- `[my link](https://somewhere.com prompt="click me",class=buttercup style="background:red" )`

The above example is a valid syntax and:

- we can see that property values can be quoted but don't need to be (though "quoting generally considered the safer option")
- key/values can be delimited by a `,` or whitespace

No that we have the basic concept down let's discuss the `Link` struct and then move into the details of each target platform.

## The Link struct

The `Link` struct, defined in the shared library, has a lot of the implementation logic already implemented but there may be some things which are still needed such as:

- support `data-*` properties
- properly parse the `style` properties CSS into a key/value

The intention is that any valid Markdown string can be passed into the `Link` struct's builder methods or alternatively be parsed from the `try_from` implementation of string types.

## Output Targets

### Targeting the Terminal

- The terminal needs to support basic linking first (currently not even this is working currently)
    - this will use the `Link` struct which will detect if the terminal supports OSC8 and if not will add the link target as a separate text element
- The Link struct should be able to be passed a Markdown Link using `try_from` to have it parsed.


### Targeting the Browser


Today all the popular browsers now provide a much more capable Popover system:

- [Modern Popovers in the Browser](./modern-popovers-in-the-browser.md)

> **IMPORTANT:** if you're planning or implementing a popover solution for HTML then you MUST read the link above for full context!


We will use this system to make the Markdown links more capable when we render our Markdown to HTML.
