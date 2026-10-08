# Dependencies

`unicode-normalization` 0.1 supplies Rust identifier NFC comparison. Its pure
Rust implementation works across supported platforms, and it was already in
the workspace dependency graph. ECMAScript identifiers use a separate escape
decoder because literal composed and decomposed Unicode identifiers are distinct.
