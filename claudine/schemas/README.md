# Claudine schemas

This folder holds one standalone schema, `review.yaml`, the schema of a
specification review document (`$schema`).

The schemas that type Claudine's own documents are not here. They are the
registration inventory, `claudine/registration/`: the trigger files, the type
libraries they import, the `kind: sequence` YAML definition, the namespace
files of the host globals `seq`, `loop`, and `group`, and the configuration
schema. The `claudine` binary compiles that folder in (its build refuses a
folder that is not a valid inventory) and publishes it to Darkmatter as its
host registration. The generated lifecycle artifact is
`claudine/lib/generated/globals.lifecycle.json`, beside the shipped artifact
`shipped.json`; after editing the inventory, run `just shipped-schemas` from
the repository root and commit both.

[Schemas](../docs/topics/schemas.md) explains what the schemas check and how a
document reaches them.
