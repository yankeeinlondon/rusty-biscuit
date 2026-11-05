---
title: Root
multi: "one\n\ntwo"
draft: false
replace: {ZZ: "a much longer value here!"}
---
# Intro {{ title }}

See [missing one](./missing-one.md)

{{ multi }}

## Second

ZZ then [missing two](./missing-two.md)

::block when="draft"
Hidden [hidden](./hidden.md)
## Removed heading
::end-block

Also see it [missing three](./missing-three.md)

Café — [missing five](./missing-five.md)

::file ./child.md when="nosuch_fn(1)"
