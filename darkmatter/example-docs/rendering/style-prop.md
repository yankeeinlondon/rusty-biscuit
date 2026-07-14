---
style:
    page:
        left-margin: 2ch
        right-margin: 4ch
        top-margin: 1
        bottom-margin: 0
    table:
        alignment: right
        max-width: 50%
    ol:
        alignment: right
    ul:
        alignment: left
        left-margin: 4ch  
        max-width: 40
---

# Testing the `style` Property

## The Page

This page has margins of:

- left: 2ch
- right: 4ch
- top: 1
- bottom: 0

## Tables

We also changed the alignment of **tables** to be right aligned and with a **max-width** of 50%:

- this has nothing to how things are aligned within the cells of the table
- only that because the table is only allowed to be a maximum of 50% of the available real-estate, that the table is aligned to the _right_ half
- though not via the `style` property:
    - the first column of the table, via Markdown syntax, is left aligned, 
    - the next column (e.g., "status") is _centered_, 
    - and then the final column (e.g., "amount") is right justified.

Example Table (`{ alignment: "right", max-width: 50% }`):

| Name  | Status | Amount |
|:------|:------:| ------:|
| iPhone | `ordered` | 999 |
| iPad   | `delivered` | 1599 |
| Dish Washer | `wish list` | 3299 |

## Lists

Unordered Lists and Ordered Lists are both able to be _styled_ via the `style` property.

The configuration of this page is with regard to lists is:

- Ordered Lists are right justified
- Unordered Lists are left justified (the default) but a left margin of 4ch has been applied
- They are also limited to a max-width of 40 characters (uses a bare number not a unit based one)

In order to see the ordered list (note: _ordered list should be right justified_):

1. Style the page
2. Render the page
3. Soft Applause
