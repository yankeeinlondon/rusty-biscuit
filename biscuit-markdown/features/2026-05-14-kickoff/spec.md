# Kickoff

## Context

- up until now the **Rusty Biscuit** monorepo has had all of it's Markdown _and_ Darkmatter functionality in the **Darkmatter** package area.
- in this feature rollout we are creating a separate library which will be responsible for pure Markdown rendering (and nothing else)
- Darkmatter will benefit too as it can leverage the newly formed `biscuit-markdown` library as it sees fit but it's main responsibility becomes more about the composition pipeline

## Analysis

The document [separation research](./separation-research.md) was created to analyize the idea of this separation and reading it's findings is a HARD REQUIREMENT before planning and/implementing this solution.
