# Stream Parsing

When **Claudine** is running in non-interactive mode it will ask the Agent to respond in a streaming format. Each Agent has slightly different structure and semantics but we standardize the way we want to interact with this streaming information with the [`StreamParser`](@claudine/lib/src/stream/parser.rs) trait.


