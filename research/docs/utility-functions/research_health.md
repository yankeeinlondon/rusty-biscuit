# the `research_health(type, topic)` function

```rust
struct ResearchHealth {
    type: &ResearchType,
    topic: &str,
    /// all underlying data and final deliverables for the research topic are complete and valid
    ok: boolean,
    missing_underlying: string[],
    missing_deliverables: ResearchOutput[],
    skill_structure_valid: boolean
}

function research_health(kind: &ResearchType, topic: &str): Result<ResearchHealth, ResearchMissing> {
    //
}
```

This function -- located in the **validation** module of the Research Library can be passed a research _type_ and _topic_ and it will report on the current health of the topic.
