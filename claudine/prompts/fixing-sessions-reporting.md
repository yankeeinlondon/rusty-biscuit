Let's get 1 & 2 fixed first before tackling 3.

## Formatting Changes

- let's remove the `Model` column from this report
- currently at least the `Turns` information has very low information content (and is confusing for non-interactive sessions), we should remove that too.
- add a column who's header is `Int` and values are based on whether the session was interactive or not:
    - if terminal is using a nerdfont (available from `Terminal` struct) then:
        - interactive: `f0134`
        - non-interactive: `f0130`
    - if not a nerdfont:
        - interactive: "✓"
        - non-interactive: ""

## Zero Turn Sessions (#1)

- I'm going to assume that we can distinguish between:
    - CLOSEOUT: someone closing an interactive session the next day without actually doing any work
    - TERMINATED: a session that WAS started but immediately exited without any turns taking place.
    - NON-INTERACTIVE: a session which was run non-interactively

Let's add another column to report on whether the session is now completed or not. To keep the heading compact (aka, not consuming a lot of horizontal space) we will use a single character to represent it:
    - nerdfont: `f45e`
    - not nerdfont: ◻

In order to make sure viewers of this report understand what the column is about we will need to add a information line item after the table. Let's describe this as being in the "footer" or "gutter" area and items in this area will be rendered as an `UnorderedList` struct. This individual line item will be: `the {icon} column indicates whether the <i>session</i> is now complete or not`.

## Duplicate Records for Same Session (#2)

The main problem we need to solve here is making sure we remove duplicates but another issue is that for our reporting purposes, these ID's tend to be quite long and when we're reporting to the terminal this can be quite problematic as it eats up the available horizontal space.

I think we should do a small refactor of how we treat session id's:

- when we wrap interactive or non-interactive agent sessions we get to set the `CLAUDINE_SESSION_ID` and all events 
