# Queue CLI additions

## Overview

This project will focus on the `queue` package of this monorepo.

- currently the implementation is just a simple CLI which queues tasks up to be executed at some later date/time.
- the CLI runs the queue as a background task allowing the user to continue using the terminal they're currently in
- there is a concept of foreground/background tasks which the CLI associates with the queued job
- read the @queue/README.md for more context

## Phase 0

Before we do anything else we will need to split this project from one CLI package into two submodules:

1. Library Module [ `./queue/lib` ]
2. CLI Module [ `./queue/cil` ]

To start in this initial Phase 0, we can simply move the current implementation into the new CLI module.

## Functional Change

Following our restructuring work we will
