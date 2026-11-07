---
kind: documentation-index
---

# Map and Filter Directives

The `::map` and `::filter` directives provided by Darkmatter allow for all _iterables_ types to be addressed at the element level.


## Map Directive

As most people who are familiar with **map** functions will know, the map function allows you work with elements in an _iterable_ one by one.

```md
---
people:
    - { name: Bob, is: "dumb" }
    - { name: John, is: "tall" }
    - { name: Mary, is: "smart" }
---
Dear Executive,

We are glad you joined Acme Corp, as head of the Shit Stirrers division here is a summary of your team and how to think of them:

::map people as i
- {{i.name}} is {{i.is}}. No really! {{i.name}} is _really_ {{i.is}}.
::end-map

Good luck and we hope you have a long career here at Acme.
```

In the example above the rendered output would be:

```md
Dear Executive,

We are glad you joined Acme Corp, as head of the Shit Stirrers division here is a summary of your team and how to think of them:

- Bob is dumb. No really! Bob is _really_ dumb.
- John is tall. No really! John is _really_ tall.
- Mary is smart. No really! Mary is _really_ smart.

Good luck and we hope you have a long career here at Acme.
```

For more information on all the features of the `::map` directive please read:

- [Map Directive in Darkmatter](./map.md)

## Filter Directive
