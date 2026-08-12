---
reviewed: false
---

# Lifecycle Ergonomics

When we first created lifecycle events in Claudine we just provided a dictionary of methods for _communicating_ the event to callers. Later we added the stack based approach which allowed for far greater range in terms of what could be done at these events as well introduce the ability to make the actions we take _conditional_.

In this feature we will focus on making the API surface that has developed over time more ergonomic. This will be a breaking change and it's important to know that we do not currently have production users so this change can and will be made without the need to support both API variants for an interim period.

## Ergonomic Shift

One of the first things we will do to improve the ergonomics is remove the dual shapes of "messages dictionary" _and_ "stack" and instead ONLY have a stack but because we only have one shape for lifecycle events. For example, what would have been defined as:

```yaml
start:
    message: "starting something"
    stack: 
        - action:
            - shell: "git status"
```

Would not be defined as:

```yaml
start:
    - message: starting something"
    - shell: "git status"
```

It should be obvious from this simple example that configuration has gotten _easier_. It is a single flat array surface
instead of a nested structure. There is no need to explicitly state `stack` anywhere.

- although the example didn't illustrate it, every _step_ in the stack can add a conditional `when` clause just like we can today.
- similarly, the available _actions_ are unchanged from what is available in the stack now it's just that there is no need to express `action` explicitly.

## Stack Structure

Today the "stack" is an explicit element but because the stack is all we have in the future state, there is no need to have it be explicit. Most of the general rules today still apply for this feature update:

- (no change from current) actions consist of:
    - communication (say/speak, effect, message, stderr, stdout, info, warn)
    - the provided side-effects Darkmatter provides
    - shell commands (_bespoke_)
    - flow control operations (retry, resume, proxy, ...)
- (no change from current) the late binding variables like `err`, `current`, and `timing` are still available in exactly the same way
- (no change from current) when a flow control action is encountered that becomes the last item in the stack that is executed (remaining items are ignored)
    - if a flow control action is wrapped in a conditional block then it's completely normal for other actions to follow because the flow control action may or may not happen
    - if, however, a flow control action is listed in a stack _unconditionally_ and other actions follow it; the following actions will never run and this is consider an anti-pattern and results in an immediate error
        - DESIGN CHALLENGE: the DMLS, being a part of Darkmatter, will not out-of-the-box be able to warn about this but it would be nice if we could figure out a way to allow Claudine to configure DMLS somehow. The most natural mechanism would be to create a `schema-trigger` so long as we can describe this pattern in `SimplifiedSchema` (I suspect right now that would be very hard)

In addition to NOT having the "stack" property, in this feature's release we will also get rid of the `action` property. For actions which you don't want to put behind a conditional expression then the actions can be placed directly at the root level of the stack:

```yaml
success:
    - message: starting something"
    - shell: "git status"
```

But if you want to add conditional's we still provide the `when` property and we add the `else` property. The `when` property behaves in exactly the same fashion as it did before
except that it uses `then` instead of `action` as the aggregation point for the actions which are are being wrapped in that conditional block:

```yaml
start:
    - when: "ctx.season == 'summer'"
      then: 
          - info: "it is summer!"
          - message: "we're starting something in the summer"
          - shell: "run-summer-program"
```

This is the normal mode of defining a conditional block but there are two other variants that exist:

1. Dictionary Grammar
2. Long Form

### Dictionary Grammar

The backbone of the stack is an array which preserves a discrete order which is important
for the effective operation of the stack. However, there are _leaf_ nodes of the stack which can opt to use a key/value shape defining their actions. As an example:

```yaml
start:
    - message: "hi"
    - when: "ctx.season == 'summer'"
      message: "the heat is overwhelming"
      say: "do you have any water?"
```

In this example the `when`, `message`, and `say` properties are bundled into group and if `when` is one of the member's it is always going to be evaluated first and if it returns true then all other actions in the dictionary will be executed. 

This grammar is compact but has some limitations:

- the user can not specify the ordering of operations
    - Claudine will simply run them all concurrently
- each action can only be used once (aka,  you can't call `say` twice because there's only one `say` key)
- no long form actions are allowed

This grammar can also be used at the root of the lifecycle event. That means that the following is valid:

```yaml
start:
    message: "hi"
    say: "nice work"
    shell: "doit"
```

In this example all three actions will be executed by Claudine concurrently and when the last action completes the lifecycle event is over.

> **Note:** 
>
> - the one exception to concurrency is in the case that one of the keys is a flow-control event. 
> - any more than one flow-control action in this structure is an immediate error, but a single flow control action is allowed.
> - when a single flow-control event is included as part of the key/values then all the non-flow-control actions are run concurrently and as soon as the final action completes in that group, the flow control event determines where execution goes next

### Long Form

This is not a real change in behavior, the current implementation already supports the idea of a long form action too. The basic idea is that our shorthand syntax of: `{command}: {param}` works very well for most cases because almost all of the actions really only _require_ a single parameter. However, many actions offer optional parameters that give the caller greater control over what the action does.

A good example of this is that all flow control directives provide an optional `use` parameter that allows for the Frontmatter state to be better prepared for the next flow state.

- the default behavior for flow-state transitions is to move the current state exactly to the new flow-state (which might be the same prompt, a different one, or a sequence)
- the default behavior is good for a lot of flow-state transitions but it is very common that a caller will want to mutate state slightly for the next flow state. 
- an example of this is when an agent hit's an error of some sort on a prompt and you want to **retry** the action but you want the prompt to know that it's not the first time this has been tried and what the error was last time it happened:

```yaml
failure:
    - retry:
        use:
            reason: err.msg
        max: 3
```

In the shorthand of a retry we would have done something like:

```yaml
failure:
    - retry: 3
```

The short form is _actionable_ but all it is able to express is how many times the retry logic should be tried before giving up.

By contrast, the long form allows us far greater expression and control:

- in our example we again set the maximum retries to 3
- but then we also set the `reason` frontmatter state to the _reason_ why the prior run failed allowing the conditional blocks and interpolation on the page to respond appropriately when the reason property is populated.


### Else Block

> DESIGN DECISION: should `else` be a property of `when` or be a peer of `when`?

In this feature we will introduce an `else` block that is similar to a `when` conditional but matches on any state that _does not_ match any of the `when` conditions. This allows simple if/else logic that is fairly common for lifecycle events:

```yaml
start:
    - when: "ctx.season == 'summer'"
      then:
        - info: "it is summer!"
        - message: "we're starting something in the summer"
        - shell: "run-summer-program"
    - else:
        - info: "it is not summer"
```

But let's explore a slightly a config with a wrinkle:

```yaml
start:
    - when: "ctx.season == 'summer'"
      then:
        - info: "it is summer!"
        - message: "we're starting something in the summer"
        - shell: "run-summer-program"
    - else:
        - info: "it is not summer"
    - message: "all is well that ends well"
```

In this example either the `then` or `else` block will be executed but in BOTH cases the final `message` will be executed because it is not conditional and not contained by either block of the conditional blocks.


### Conditional Nesting

Currently conditional blocks can not nest but with this feature we will start to allow nesting. We can place a limit on the nesting level:

- I can't imagine ever needing to go beyond 10 nesting levels
- if there were a technical reason that 5 or 3 nesting levels were better those too would be fine
- the most common requirement is to just be able to add a second nesting level

The syntax for nesting is keeping in line with the normal grammar; an example would be:

```yaml
start:
    - when: "ctx.season == 'summer'"
      then:
          - when: "ctx.month == 'July'"
            message: "damn it is hot!"
    - else:
        - message: "damn it is hot; at least it is not July!"
```

## Schemas

The schema support that Darkmatter provides now is fairly comprehensive and the DMLS language server is a great aid in helping authors create _valid_ Darkmatter documents. However, to date, we have not yet brought in the Claudine schemas but with this feature release we will bring in schemas for Claudine (not just the lifecycle properties).

### Darkmatter Provisioning

Darkmatter provides both inline schema definitions via the `$schema` property of a document as well as a "baseline" schema which provides the default schema:

- the `$schema` property can be defined inline to a Markdown document or can be pointed at an external file.
    - when pointed to an external file that file must be both a YAML document _and_ conform to the `schema` _kinded_ format
    - alternatively you can point to a local JSON Schema definition though this is not generally recommended (as the `SimplifiedSchema` is both more ergonomic and able to express more semantically than JSON schema)
- underneath the direct attribution of schema with the `$schema` property is the idea of a base schema which Darkmatter leverages to define the variables which have special semantic meaning

### Claudine Schema Structure

There are some important **schemas** and **schema-triggers** that need to be defined so that the stack is able to be supported in DMLS. 

| file path      | type   | description |
| ---------      | ----   | ----------- |
| action.yaml    | schema | defines the `action` enumeration which enumerates all of the possible actions a user can take and includes both short and long form shapes |
| lifecycle.yaml | schema | uses the `action` definition as a building block to describe the full shape of a lifecycle event |
| inline-compose.yaml | schema-trigger | defines the shape and trigger pattern of an inline-compose document | 
| sequence.yaml | schema-trigger | defines the shape and trigger pattern of a sequence |
| claudine.yaml  | schema | defines all Frontmatter properties that are common to 

> Note: all schema files should be saved to @claudine/docs/schemas where they can serve as both documentation _and_ be incorporated into typed metadata for Claudine during code generation stage.



## Background Audio

Today we have both TTS and sound effects which are played through the hosts audio system.

- because it can be useful to play a sound effect to get a user's attention _and then_ have the TTS speak afterwards we added actions to the API surface that we might not have otherwise like `say_first` and then relied on a built-in ordering to put sound effects _before_ `say` but before both we had `say_first`. The idea was this allowed the user to reverse the order.
- In addition to the awkwardness of this API, was the fact that:
    - TTS expressions almost always have a certain delay before they're ready to be spoken
        - some of the TTS providers have an intermediary step of producing an audio file and then Claudine (via Playa) will use the hosts audio API's to play this file
    - Sound effects and TTS expressions take many seconds in the best of conditions to emit and so things like `warn`, `info`, `stderr`, and `stdout` messages were noticeably delayed until after all audio had completed fully

To move away from this awkwardness, this feature will provide the following things:

- pre-compilation of audio assets
    - when a page first loads it will create a concurrent thread that evaluates all of the TTS (e.g., `say`/`speak` actions) events the page defines
        - the evaluation is to determine if the phrase that the user will speak is based on "late binding" variables
            - if it is then we can **not** pre-compile the audio
        - if the phrase is either just static text or static text interpolated with normal state variables (ctx, doc global variables or Frontmatter variables) then we **can** pre-compile
            - for the TTS solutions which produce a cached file as an intermediary product (in the OS's temp directory) we simply produce that file as soon as we realize that it might be needed so that _if_ it's needed there will be almost no latency in having the TTS played
            - for TTS solutions like macOS's `say`, there is not -- at least transparently -- an audio file produced as an intermediary. If this is the configured TTS we're using then we need to research and decide whether pre-computing the TTS phrase makes sense.
- removal of the `say_first` action
    - to really understand `say_first` the user is required to know too much about how Claudine and Playa are ordering things
    - if you care about order then you should simply order using the array as the regulator
    - what Claudine/Playa will actually do though is to place all audio events onto their own thread which will run concurrently with the main execution thread
        - that means if you first have a `effect` action defined, then a `message` action, and finally a `say` action:
            - the `effect` and `say` are placed onto the audio thread and executed serially
            - meanwhile the non-audio work -- in this example that's just the `message` -- will be executed serially as well (but concurrently to the audio thread)


## Loop Lifecycle

Of all the lifetime events that Claudine exposes, the `loop` event is _slightly_ different because it MUST lead by
