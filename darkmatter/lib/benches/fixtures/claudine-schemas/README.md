# Frozen Claudine Schema Bundle

These nine YAML files are the Claudine-shaped schema bundle the
`schema_validation` benchmark (`just bench-schema`) validates against in its
cold-cache cases. They are byte-for-byte copies of `claudine/schemas/*.yaml`,
copied on 2026-10-23 from the tree at commit `bf89eeef0` (the files were last
changed in `a765bb117`).

They are frozen. Never refresh them to follow Claudine: the benchmark compares
runs over time, and a moving schema would change what is measured. The
benchmark reads only this folder, never `claudine/schemas/`.

The files resolve through Darkmatter alone. Their `@darkmatter:lifecycle`
references name schemas compiled into Darkmatter, and every other reference is
relative to this folder. The benchmark copies the folder into a temporary
repository's `schemas/` root, where trigger discovery finds the three trigger
files (`claudine.yaml`, `inline-compose.yaml`, `sequence.yaml`) as it does in
production.
