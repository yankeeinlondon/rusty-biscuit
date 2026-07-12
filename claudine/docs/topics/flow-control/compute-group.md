# Compute Groups

A set of prompts can be _grouped_ together with the goal of:

- having the group be **looped** over (in a similar fashion to how the [loop lifecycle hook](./looping.md) already works for single prompt documents)
- the group can be used as a **fan-out** mechanism to achieve concurrency
- a group can also be used to demarcate a semantic grouping of operations to aid in user and AI understanding
