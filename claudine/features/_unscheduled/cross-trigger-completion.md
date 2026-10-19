---
status: proposal
created: 2026-10-18
author: Ken Snyder <ken@ken.net>
related:
  - 2026-10-26-task-inputs-and-providers
---

# Completion Across Claudine Triggers

## Problem

Each Claudine frontmatter key is declared by exactly one trigger schema:
`sequence.yaml` owns `sequence`/`template`, and `claudine.yaml` owns the root
lifecycle events and `loop`. DMLS offers completion only from triggers that
already match the document, so a sequence document whose root has only
`sequence:` is never offered `start:`, `loop:`, or any other root event key
until the author types one from memory.

Declaring the event keys in both triggers would fix completion, but it costs
every Claudine launch (all trigger payloads load whenever trigger discovery
runs: about +0.05 s release, +0.34 s debug, measured 2026-10) and creates two
declarations that can drift.

## Direction

Let completion discover keys from sibling triggers in the same schema bundle
without validating against them: when a document matches a trigger, offer the
top-level keys of the other triggers that writing such a key would activate.
Validation stays one declaration per key.

Open questions:

- Is this a Darkmatter/DMLS feature (generic "activating keys" completion for
  any bundle) or Claudine-specific configuration?
- Should hover or a hint explain that typing the key activates another schema?
