---
state: foo
---

# Page Blocks

The main idea behind **Page Blocks** blocks is that many times you have different
content for different people, conditions, etc. and you want a way to render only
the appropriate sections.

## An Important Notice

And now for an important message from our fearless leader:

::block when="state == 'foo'"
Just between me and you, **bar** is a piece of shit, I'd go with what **foo** thinks!
::end-block
::block when="state == 'bar'"
Can you believe **foo**? What an asshole. I'll be voting for **bar** on this one.
::end-block
