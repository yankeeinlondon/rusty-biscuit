# Status component

- In this feature we're going to add a `Status` _renderable_ component to `biscuit-terminal` which will have many similarities to the `Todo` component which already exists. 
- The `Status` component's primary use is also quite similar to a TODO: _for reporting the status of some validation or state for an action item_ but unlike `Todo` which tries to play nicely with the [GFM](https://github.github.com/gfm/) the `Status` component makes no attempts to put it's status inside of square brackets.
- the `Todo` status offers multiple themes which change the style of icons used 
- the `Todo` component also offers conditional colorization of icons
    - by default colorization is turned on but a builder method `.no_color_icons()` can turn this off
    - colors which have a `/` in their definition indicate a variant color based on light/dark color mode detected by `Terminal` struct
    - all colors use Tailwind based naming convention -- which is supported by `biscuit-terminal` and the `Prose` component amongst others.


## Programmatic Use

```rust
use biscuit_terminal::prelude::*;
use biscuit_terminal::components::status::{Status, StatusState};

// Create a new status message
let status = Status::new("Review pull request #42");

// Render
let term = Terminal::new();
let output = status.display(&term);
```

### State Rendering

| Theme    | State   | Nerdfont | Non Nerd | Color     |
| -----    | -----   | -------- | -------- | -----     |
| Circular | Not Started | "f4aa"   |    ◻     | gray-500  |
| Circular | Active  | "f0ec2"  |    ◽️    | gray-600/400  |
| Circular | Success | "f05e0"  |    ✓     | green-500 |
| Circular | Failure | "f057"   |    ⤫     |  red-500  |
| Circular | Warning | "f0028"  |    ⚠️     | orange-500 |
| Circular | Info    |  "f449" |    ℹ️     | blue-500 |
| Rounded | Not Started |  "ea72"  |    ◻     | gray-500  |
| Rounded | Active  | "f1500" |    ◽️    | gray-600/400  |
| Rounded | Success | "f14a"  |    ✓     | green-500 |
| Rounded | Failure | "f136e" |    ⤫     |  red-500  |
| Rounded | Warning |  "f0af" |    ⚠️     | orange-500 |
| Rounded | Info    |  "f0bd4" |    ℹ️     | orange-500 |
| Timeline | Not Started | "f0bd2"  |    ◻     | gray-500  |
| Timeline | Active  | "f0bd1"  |    ◽️    | gray-600/400  |
| Timeline | Success | "f1532"  |    ✓     | green-500 |
| Timeline | Failure |  "f1537" |    ⤫     |  red-500   |
| Timeline | Warning |  "f0f95" |    ⚠️     | orange-500 |
| Timeline | Info    |  "f0bd4" |    ℹ️     | orange-500 |

- the **circular** and **rounded** themes are icons which are _circular_ or a _rounded square_ in shape respectively
- the **timeline** theme provides a consistent vertical bar to the left of the icon so that when stacked vertically it will look like a timeline.


### Status Enum

```rust
use biscuit_terminal::components::status::Status;

let states = vec![
    Status::NotStarted,
    Status::Active,
    Status::Success,
    Status::Failure,
    Status::Warning,
    Status::Info,
];
