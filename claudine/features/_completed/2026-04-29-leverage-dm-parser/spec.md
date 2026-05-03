- The Darkmatter library has a robust parser/lexer for boolean expressions which it uses for handling expression based interpolation (e.g., "{{ foo || bar }}") as well all of it's directives which use the conditional `when` clause. 
- The Claudine library and CLI are beneficiaries of this but we have just recently exposed this boolean parser/lexer to callers who want to use it themselves. 
- Claudine is a likely candidate for this being a benefit and an opportunity to make it's code more DRY across the Darkmatter & Claudine libraries. Evaluate the spec @darkmatter/ The Darkmatter library has a robust parser/lexer for boolean expressions which it uses for handling expression based interpolation

## Opportunities

### 1. Dispatch Template Interpolation (`dispatch/template.rs`)

**Current state**: `interpolate()` uses a regex to replace `{{variable}}` tokens. It supports simple variable lookup against `TemplateVariable` and `env.VAR | "fallback"` with a hand-quoted default literal. No comparisons, ternaries, or functions.

**Opportunity**: Replace the regex-based resolver with `darkmatter::markdown::compose::expression` in `ParseMode::Interpolation`. This would immediately enable:
- Ternary conditions: `{{git.is_dirty ? "dirty" : "clean"}}`
- Comparisons: `{{hardware.cores > 8 ? "fast" : "slow"}}`
- Fallbacks: `{{env.CI || "local"}}` (already partially supported but with inconsistent syntax)
- Functions: `{{length(git.branch) > 30 ? "long-branch" : git.branch}}`

**Implementation path**: Implement `EvaluationLookup` for `&EventMeta` (or a thin wrapper) and call `darkmatter::markdown::compose::expression::evaluate()` with the parsed `Expr`. The `ExpressionFinder` can locate `{{...}}` blocks, the `Parser` can parse each inner expression, and `evaluate()` resolves against the event metadata.

**DRY benefit**: Eliminates the custom `resolve_expression()`, `parse_default_literal()`, and `rewrite_legacy_single_brace_placeholders()` logic. The `env.*` fallback handling collapses into darkmatter's built-in `env.*` resolution.

---

### 2. Hook Action `when` Conditions (`events/config.rs`)

**Current state**: `HookAction` entries in an `EventBinding` fire unconditionally. The binding has an `enabled` boolean and a top-level `matcher` regex, but no per-action conditional execution.

**Opportunity**: Add an optional `when: String` field to `HookAction` (or action variants that need it) and evaluate it with `darkmatter::markdown::compose::conditions::evaluate_condition_against()` at dispatch time. The `data` payload would be the `EventMeta` serialized as `serde_json::Value`.

**Examples**:

```json
{
  "event": "before_tool",
  "actions": [
    { "type": "speak", "message": "Running Bash", "when": "tool_name == 'Bash'" },
    { "type": "notify", "message": "Deploying", "when": "git.branch == 'main' && !git.is_dirty" }
  ]
}
```

**DRY benefit**: Darkmatter already evaluates `when="..."` on `::block` directives inside its compose pipeline. Reusing the same `evaluate_condition_against` shortcut means Claudine gets lazy `ctx.*` capture, `env.*` resolution, and rich `ConditionError` rendering (via `biscuit_terminal::errors::BlockError`) for free.

---

### 3. Event Binding Matcher (`dispatch/matcher.rs`)

**Current state**: `matches_with_regex()` compiles a regex and tests it against exactly one field (`tool_name` or `notification_type`). It cannot express cross-field conditions like `"tool_name == 'Bash' && git.branch == 'main'"`.

**Opportunity**: Extend the matcher to support an expression mode alongside the existing regex mode. When the matcher string parses as a valid darkmatter condition expression, evaluate it against `EventMeta`; otherwise fall back to regex for backward compatibility.

**DRY benefit**: Avoids inventing a second expression language for event filtering. The same `EvaluationLookup` impl used for template interpolation (Opportunity 1) can be reused here.

---

### 4. Harness Validation Message Templates (`harness/validate.rs`)

**Current state**: `render_template()` does literal `String::replace` for `{{key}}` placeholders against a `HashMap<&str, String>`. No expressions, no defaults, no conditionals.

**Opportunity**: Use darkmatter expression parsing for validation check messages. Pre-check and post-check messages could reference frontmatter values, context variables, and computed expressions.

**Example**:

```yaml
validate:
  pre:
    - file_exists: "Cargo.toml"
      message: "{{source_file}} requires a Cargo.toml in {{cwd}}"
  post:
    - response_includes: "## Summary"
      message: "{{response_includes ? 'Summary found' : 'Missing summary header'}}"
```

**DRY benefit**: Collapses the ad-hoc `render_template()` helper into the shared expression evaluator.

---

### 5. Reporting Query Filters (`reporting/queries.rs`)

**Current state**: `ReportingFilters` exposes four exact-match optional fields (`provider`, `repo`, `package_area`, `package`). `WhereBuilder` emits hardcoded SQL `AND` clauses.

**Opportunity**: Accept an optional expression string that is evaluated in-memory against each `LogEntry` (or compiled to SQL where feasible). This would allow compound filters without adding new struct fields for every permutation.

**Example**:

```bash
claudine logs today --filter "provider == 'claude' && git.branch == 'main'"
```

**Note**: This is lower priority than dispatch opportunities because the SQL-layer filtering is already efficient; expressions would primarily benefit the in-memory/reporting CLI surface.

---

### 6. Resource Linking Filters (`linking/filter.rs`)

**Current state**: `ResourceFilter` supports prefix/suffix matching with `!` negation and `-` exclusion. Boolean composition is hardcoded (`retain()` checks negations first, then positives).

**Opportunity**: Allow filter expressions that combine multiple conditions with `&&` and `||`. A filter like `rust && !test || py!` is currently impossible to express as a single filter string.

**Note**: This is the lowest priority because the current syntax is simple and effective; expression support would be a power-user feature rather than a DRY win.

---

## Recommended Priority

1. **Template interpolation** (Opportunity 1) — lowest effort, highest user value, pure additive change.
2. **Hook action `when`** (Opportunity 2) — unlocks entirely new dispatch behavior; high impact.
3. **Event matcher expressions** (Opportunity 3) — natural extension once `EvaluationLookup` for `EventMeta` exists.
4. **Harness validation templates** (Opportunity 4) — nice-to-have, blocked on harness refactor appetite.
5. **Reporting filters** (Opportunity 5) and **linking filters** (Opportunity 6) — future enhancements.

