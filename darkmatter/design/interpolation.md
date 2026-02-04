# Frontmatter Interpolation

## Functional Goal

- being able to inject Frontmatter variables into the page is a useful feature and DarkMatter documents support this with a syntax that in it's simplest form would look like this: `{{variable}}` being placed in the body of the page somewhere. This would then be replaced with the `variable` property in Frontmatter at render time. If that variable is not set then it will be treated as an empty string.
    - the same variable can be placed on the page as many times as you like

### Variants

From this base there are a few variants that round out this feature:

- **Default Values**
    - It is often important that _something_ be returned in an interpolation even if the variable itself was not set
    - This is achieved with the `||` modifier
    - The string `{{variable||42}}` will _try_ to replace this text with the value of the Frontmatter property `variable` but if it's not set then it will instead use `42`.
- **Conditionals**
    - We may want to say something _conditionally_ based on the state of the Frontmatter
    - Darkmatter provides provides the following conditional operators:
        - **if(prop):**
            - the text `{{if(variable):was set}}` will interpolate to `was set` _if_ the Frontmatter has a property `variable` which has a _truthy_ value.
            - If the variable is any of the following then it will interpolate to an empty string:
                - `variable` is NOT set
                - `variable` is set to an empty string
                - `variable` is set to 'false'
                - `variable` is set to '0'
            - if we change the example text slightly to `{{if(variable):was set||was NOT set}}` we can see that we can assign a different interpolation for the case that the variable is _falsy_ too.
        - **has(prop, substr):**
            - this operator is meant to test whether a Frontmatter property _has_ some substring contained inside of it
            - for example `{{has(variable, monkey):there WAS a monkey||no monkey found!}}`
        - **startsWith(prop, string):**
            - this operator is meant to test whether a Frontmatter property _starts with_ the string passed in
            - for example `{{startsWith(variable,Mr.):a gentleman||a lady!}}`
        - **endsWith(prop, string):**
            - this operator is meant to test whether a Frontmatter property _ends with_ the string passed in
            - for example `{{endsWith(variable,!):exclaimed||said with no emphasis}}`
        - **gt(prop, number):**
            - checks whether the Frontmatter property is both _numeric_ and _greater than_ the number specified
            - example: `{{gt(variable, 1):plural||singular}}`
            - in this case the operator is actually testing two things -- is it a number _and_ is it greater than some amount -- we can optionally use another `||` modifier:
                - `{{gt(variable, 1):plural||singular||NOT NUMBER!}}`
            - If the second `||` is not used then `singular` will be returned in situations where `variable` is a number but not greater than 1 AND if `variable` is NOT a number
        - **lt(prop, number):**
            - checks whether the Frontmatter property is both _numeric_ and _less than_ the number specified
            - example: `{{lt(variable, 2):singular||plural}}`
            - like the `gt` operator, this operator provides a second `||` modifier to be used.
        - **lte(prop, number):** _and_&nbsp; **lte(prop, number):**
            - just like their `gt` and `lt` siblings but the conditional is greater than _or equal_ and less than _or equal_ respectively
        - **between(prop, number, number):**
            - checks whether the Frontmatter property is both _numeric_ and _between_ the two numbers specified (inclusively)
            - example: `{{between(variable, 0, 10):valid-ranking||number is out of bounds||NOT A NUMBER}}`
            - like the other numeric operators, this operator provides a second `||` modifier to be used.
        - **betweenExclusively(prop, number, number):**
            - checks whether the Frontmatter property is both _numeric_ and _between_ the two numbers specified (exclusively)
            - example: `{{between(variable, 0, 11):valid-ranking||number is out of bounds||NOT A NUMBER}}`
            - like the other numeric operators, this operator provides a second `||` modifier to be used.

    > **NOTE:** for conditional operators with more than one parameter, if there is a space following the `,` delimiter it will be stripped out in the comparison operation.

### Utility Frontmatter Values

In addition to the variables explicitly set on the page, some additional properties will be made available to pages for convenience: see [utility frontmatter](../reference/utility-frontmatter.md) for the full list.

### Timing

The timing of _when_ interpolation is executed is important because some of the other directives in the DarkMatter DSL will want to use the "finalized" text value not the intermediary one.

As an example, if the page had a column directive of `::columns {{col||2}}`:

- we would need to resolve the `{{col||2}}` first,
- and then apply the `::columns` directive

**NOTE:** this ordering of operations _could_ have an impact on performance if we need to parse the document more than once. All attempts should be made to not require a multi-pass process. The goal is to have this be as performant as possible.

## Technical Design

### Architecture Overview

Frontmatter interpolation is implemented as a **pre-parse transformation** that occurs before any DSL directive parsing. This ensures all directive parameters are concrete values, not interpolation expressions.

**Critical Timing Requirement**: Interpolation (and text replacement) must complete **before** any DSL directive is parsed, because:

- Directives may have dynamic parameters: `::columns {{col||2}}` must become `::columns 2`
- Transclusion paths may be dynamic: `::file {{path||./default.md}}` must resolve to `::file ./some-path.md`
- Table sources may be dynamic: `::table {{data_source||./data.csv}}`

**Recursive Processing**: When a transclusion directive is encountered (e.g., `::file external.md`):

1. Load external document
2. Extract its frontmatter
3. Merge parent's frontmatter with external document's frontmatter (parent wins conflicts)
4. **Recursively apply interpolation → parse → transclude** to the external document
5. Return fully resolved markdown (no DSL directives remaining)

This means external documents can reference frontmatter properties from their parent document

### Interpolation Syntax Parser

#### Current Implementation

The existing implementation (`lib/src/render/interpolation.rs`) uses a simple regex pattern:

```rust
static INTERPOLATION_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\{\{([a-zA-Z_][a-zA-Z0-9_]*)\}\}").expect("Invalid regex pattern")
});
```

This handles basic variable substitution (`{{variable}}`) but cannot support:

- Default values: `{{variable||default}}`
- Conditional operators: `{{if(prop):truthy||falsy}}`
- Numeric comparisons: `{{gt(count, 1):plural||singular}}`

#### Proposed Parser Design

Replace the regex-based approach with a **state machine parser** that provides:

1. **Better error messages** with line/column information
2. **Extensibility** for future operators without regex complexity
3. **Nested delimiter handling** (balanced `||` separators vs. `||` in text)
4. **Parameter validation** (e.g., numeric checks for `gt`, `between`)

**Parser Structure**:

```rust
pub struct InterpolationParser<'a> {
    input: &'a str,
    position: usize,
    frontmatter: &'a HashMap<String, serde_json::Value>,
}

pub enum InterpolationExpr {
    /// Simple variable: {{variable}}
    Variable {
        name: String,
        default: Option<String>,
    },

    /// Conditional operator: {{op(param, ...):branch1||branch2||...}}
    Conditional {
        operator: ConditionalOp,
        params: Vec<String>,
        branches: Vec<String>,
    },
}

pub enum ConditionalOp {
    If,                                    // if(prop)
    Has,                                   // has(prop, substr)
    StartsWith,                            // startsWith(prop, str)
    EndsWith,                              // endsWith(prop, str)
    Gt,                                    // gt(prop, num)
    Lt,                                    // lt(prop, num)
    Gte,                                   // gte(prop, num)
    Lte,                                   // lte(prop, num)
    Between,                               // between(prop, num, num)
    BetweenExclusively,                    // betweenExclusively(prop, num, num)
}
```

#### Parsing Algorithm

The parser uses a **recursive descent** approach:

```rust
impl<'a> InterpolationParser<'a> {
    /// Main entry point: find and replace all {{...}} patterns
    pub fn process(content: &str, frontmatter: &HashMap<String, serde_json::Value>) -> Result<String, InterpolationError> {
        let mut result = String::with_capacity(content.len());
        let mut parser = Self { input: content, position: 0, frontmatter };

        while let Some(start) = parser.find_next_marker("{{") {
            // Copy literal text before marker
            result.push_str(&content[parser.position..start]);
            parser.position = start;

            // Parse and evaluate expression
            let expr = parser.parse_expression()?;
            let value = parser.evaluate(expr)?;
            result.push_str(&value);
        }

        // Append remaining content
        result.push_str(&content[parser.position..]);
        Ok(result)
    }

    /// Parse a single {{...}} expression
    fn parse_expression(&mut self) -> Result<InterpolationExpr, InterpolationError> {
        self.expect("{{")?;

        // Check if this is a function call (contains '(')
        if self.peek_until_any(&['(', ':' ,'|', '}']) == Some('(') {
            self.parse_conditional()
        } else {
            self.parse_variable()
        }
    }

    /// Parse simple variable with optional default: variable||default
    fn parse_variable(&mut self) -> Result<InterpolationExpr, InterpolationError> {
        let name = self.read_identifier()?;

        let default = if self.peek_str("||") {
            self.advance(2);
            Some(self.read_until("}}")?)
        } else {
            None
        };

        self.expect("}}")?;

        Ok(InterpolationExpr::Variable { name, default })
    }

    /// Parse conditional: operator(params):branch1||branch2||branch3
    fn parse_conditional(&mut self) -> Result<InterpolationExpr, InterpolationError> {
        // Parse operator name
        let op_name = self.read_identifier()?;
        let operator = ConditionalOp::from_str(&op_name)?;

        // Parse parameters: (param1, param2, ...)
        self.expect("(")?;
        let params = self.parse_params()?;
        self.expect(")")?;

        // Validate param count for operator
        operator.validate_param_count(params.len())?;

        // Parse branches: :branch1||branch2||branch3
        self.expect(":")?;
        let branches = self.parse_branches()?;

        // Validate branch count for operator
        operator.validate_branch_count(branches.len())?;

        self.expect("}}")?;

        Ok(InterpolationExpr::Conditional { operator, params, branches })
    }

    /// Parse comma-separated parameters (strips space after comma)
    fn parse_params(&mut self) -> Result<Vec<String>, InterpolationError> {
        let mut params = Vec::new();
        loop {
            let param = self.read_until_any(&[',', ')'])?;
            params.push(param.trim().to_string());

            if self.peek() == Some(',') {
                self.advance(1);
                // Strip optional space after comma
                if self.peek() == Some(' ') {
                    self.advance(1);
                }
            } else {
                break;
            }
        }
        Ok(params)
    }

    /// Parse pipe-separated branches
    fn parse_branches(&mut self) -> Result<Vec<String>, InterpolationError> {
        let mut branches = Vec::new();
        let mut current_branch = String::new();

        // Read until closing }}
        while !self.peek_str("}}") {
            // Check for || separator (but not at start)
            if !current_branch.is_empty() && self.peek_str("||") {
                self.advance(2);
                branches.push(current_branch.clone());
                current_branch.clear();
            } else {
                current_branch.push(self.read_char()?);
            }
        }

        // Push final branch
        if !current_branch.is_empty() {
            branches.push(current_branch);
        }

        Ok(branches)
    }
}
```

### Expression Evaluation

Once parsed, expressions are evaluated against frontmatter values:

```rust
impl<'a> InterpolationParser<'a> {
    fn evaluate(&self, expr: InterpolationExpr) -> Result<String, InterpolationError> {
        match expr {
            InterpolationExpr::Variable { name, default } => {
                self.evaluate_variable(&name, default.as_deref())
            }
            InterpolationExpr::Conditional { operator, params, branches } => {
                self.evaluate_conditional(operator, &params, &branches)
            }
        }
    }

    fn evaluate_variable(&self, name: &str, default: Option<&str>) -> Result<String, InterpolationError> {
        if let Some(value) = self.frontmatter.get(name) {
            self.json_to_string(value)
        } else if let Some(default_val) = default {
            Ok(default_val.to_string())
        } else {
            // Variable not found, no default - return empty string
            Ok(String::new())
        }
    }

    fn evaluate_conditional(&self, op: ConditionalOp, params: &[String], branches: &[String]) -> Result<String, InterpolationError> {
        match op {
            ConditionalOp::If => self.eval_if(params, branches),
            ConditionalOp::Has => self.eval_has(params, branches),
            ConditionalOp::StartsWith => self.eval_starts_with(params, branches),
            ConditionalOp::EndsWith => self.eval_ends_with(params, branches),
            ConditionalOp::Gt => self.eval_gt(params, branches),
            ConditionalOp::Lt => self.eval_lt(params, branches),
            ConditionalOp::Gte => self.eval_gte(params, branches),
            ConditionalOp::Lte => self.eval_lte(params, branches),
            ConditionalOp::Between => self.eval_between(params, branches, true),
            ConditionalOp::BetweenExclusively => self.eval_between(params, branches, false),
        }
    }

    /// Evaluate if(prop): returns branch[0] if truthy, branch[1] if falsy
    fn eval_if(&self, params: &[String], branches: &[String]) -> Result<String, InterpolationError> {
        let prop_name = &params[0];
        let is_truthy = self.is_truthy(prop_name);

        let branch_idx = if is_truthy { 0 } else { 1 };
        Ok(branches.get(branch_idx).cloned().unwrap_or_default())
    }

    /// Check if a property is truthy (exists, non-empty, not false, not 0)
    fn is_truthy(&self, prop_name: &str) -> bool {
        match self.frontmatter.get(prop_name) {
            None => false,
            Some(serde_json::Value::Null) => false,
            Some(serde_json::Value::Bool(b)) => *b,
            Some(serde_json::Value::String(s)) => !s.is_empty() && s != "false" && s != "0",
            Some(serde_json::Value::Number(n)) => {
                // 0 is falsy
                n.as_f64().map(|v| v != 0.0).unwrap_or(true)
            }
            Some(_) => true, // Arrays, objects are truthy
        }
    }

    /// Evaluate has(prop, substr): checks if prop contains substr
    fn eval_has(&self, params: &[String], branches: &[String]) -> Result<String, InterpolationError> {
        let prop_name = &params[0];
        let substr = &params[1];

        let has_match = self.frontmatter
            .get(prop_name)
            .and_then(|v| v.as_str())
            .map(|s| s.contains(substr))
            .unwrap_or(false);

        let branch_idx = if has_match { 0 } else { 1 };
        Ok(branches.get(branch_idx).cloned().unwrap_or_default())
    }

    /// Evaluate startsWith(prop, prefix)
    fn eval_starts_with(&self, params: &[String], branches: &[String]) -> Result<String, InterpolationError> {
        let prop_name = &params[0];
        let prefix = &params[1];

        let matches = self.frontmatter
            .get(prop_name)
            .and_then(|v| v.as_str())
            .map(|s| s.starts_with(prefix))
            .unwrap_or(false);

        let branch_idx = if matches { 0 } else { 1 };
        Ok(branches.get(branch_idx).cloned().unwrap_or_default())
    }

    /// Evaluate endsWith(prop, suffix)
    fn eval_ends_with(&self, params: &[String], branches: &[String]) -> Result<String, InterpolationError> {
        let prop_name = &params[0];
        let suffix = &params[1];

        let matches = self.frontmatter
            .get(prop_name)
            .and_then(|v| v.as_str())
            .map(|s| s.ends_with(suffix))
            .unwrap_or(false);

        let branch_idx = if matches { 0 } else { 1 };
        Ok(branches.get(branch_idx).cloned().unwrap_or_default())
    }

    /// Evaluate gt(prop, num): check if prop > num
    fn eval_gt(&self, params: &[String], branches: &[String]) -> Result<String, InterpolationError> {
        let prop_name = &params[0];
        let threshold: f64 = params[1].parse()
            .map_err(|_| InterpolationError::InvalidNumber(params[1].clone()))?;

        let value = self.frontmatter.get(prop_name);

        match value {
            Some(serde_json::Value::Number(n)) => {
                let num = n.as_f64().ok_or_else(|| InterpolationError::InvalidNumber(prop_name.to_string()))?;
                let branch_idx = if num > threshold { 0 } else { 1 };
                Ok(branches.get(branch_idx).cloned().unwrap_or_default())
            }
            Some(_) => {
                // Not a number - use third branch if provided
                Ok(branches.get(2).cloned().unwrap_or_else(|| branches.get(1).cloned().unwrap_or_default()))
            }
            None => {
                // Property doesn't exist - use third branch if provided
                Ok(branches.get(2).cloned().unwrap_or_else(|| branches.get(1).cloned().unwrap_or_default()))
            }
        }
    }

    /// Similar implementations for lt, gte, lte...

    /// Evaluate between(prop, min, max) or betweenExclusively
    fn eval_between(&self, params: &[String], branches: &[String], inclusive: bool) -> Result<String, InterpolationError> {
        let prop_name = &params[0];
        let min: f64 = params[1].parse()
            .map_err(|_| InterpolationError::InvalidNumber(params[1].clone()))?;
        let max: f64 = params[2].parse()
            .map_err(|_| InterpolationError::InvalidNumber(params[2].clone()))?;

        let value = self.frontmatter.get(prop_name);

        match value {
            Some(serde_json::Value::Number(n)) => {
                let num = n.as_f64().ok_or_else(|| InterpolationError::InvalidNumber(prop_name.to_string()))?;

                let in_range = if inclusive {
                    num >= min && num <= max
                } else {
                    num > min && num < max
                };

                let branch_idx = if in_range { 0 } else { 1 };
                Ok(branches.get(branch_idx).cloned().unwrap_or_default())
            }
            Some(_) | None => {
                // Not a number or doesn't exist - use third branch
                Ok(branches.get(2).cloned().unwrap_or_else(|| branches.get(1).cloned().unwrap_or_default()))
            }
        }
    }
}
```

### Document Processing Pipeline

The correct processing order for each document (applied recursively for transclusions):

#### Main Processing Flow

**Location**: `lib/src/parse/mod.rs` - `parse_document()` function

```rust
pub fn parse_document(
    content: &str,
    source: Resource,
    parent_frontmatter: Option<&Frontmatter>
) -> Result<Document, ParseError> {
    // 1. Extract frontmatter
    let (mut frontmatter, body) = extract_frontmatter(content)?;

    // 2. Merge with parent frontmatter if this is a transclusion
    if let Some(parent) = parent_frontmatter {
        frontmatter.merge(parent.clone());
    }

    // 3. Generate utility variables (dates, times, etc.)
    let utilities = generate_utility_variables();
    let all_vars = merge_utilities_and_custom(utilities, &frontmatter);

    // 4. Apply frontmatter interpolation to ENTIRE body
    let interpolated_body = InterpolationParser::process(&body, &all_vars)?;

    // 5. Apply text replacement (if frontmatter.replace is set)
    let replaced_body = apply_text_replacements(&interpolated_body, &frontmatter)?;

    // 6. Parse markdown and DarkMatter DSL (all parameters now concrete)
    let nodes = parse_markdown(&replaced_body)?;

    // 7. Collect dependencies from nodes
    let dependencies = collect_dependencies(&nodes);

    Ok(Document {
        resource: source,
        frontmatter,
        content: nodes,
        dependencies,
        parsed_at: Utc::now(),
    })
}
```

#### Transclusion Resolution (Recursive)

**Location**: `lib/src/render/transclusion.rs` - modified to pass frontmatter

```rust
/// Resolve a transclusion directive recursively
pub async fn resolve_transclusion(
    node: &DarkMatterNode,
    parent_frontmatter: &Frontmatter,
    cache: &CacheOperations,
    base_path: Option<&PathBuf>,
) -> Result<Vec<DarkMatterNode>, RenderError> {
    match node {
        DarkMatterNode::File { resource, range } => {
            // Load external document
            let content = load_resource_content(resource, cache).await?;

            // Parse with parent frontmatter (triggers recursive interpolation)
            let doc = parse_document(&content, resource.clone(), Some(parent_frontmatter))?;

            // Recursively resolve any transclusions in the external document
            let mut resolved = Vec::new();
            for child_node in &doc.content {
                let resolved_child = resolve_transclusion(
                    child_node,
                    &doc.frontmatter,  // Pass merged frontmatter down
                    cache,
                    base_path
                ).await?;
                resolved.extend(resolved_child);
            }

            // Apply range selection if specified
            if let Some(range) = range {
                filter_by_range(&resolved, range)
            } else {
                Ok(resolved)
            }
        }

        // Similar for ::summarize, ::consolidate, etc.
        DarkMatterNode::Summarize { resource } => {
            let content = load_resource_content(resource, cache).await?;
            let doc = parse_document(&content, resource.clone(), Some(parent_frontmatter))?;

            // Call LLM to summarize the fully resolved document
            let summary = summarize_with_llm(&doc).await?;
            Ok(vec![DarkMatterNode::Text(summary)])
        }

        // Non-transclusion nodes pass through
        _ => Ok(vec![node.clone()]),
    }
}
```

#### Key Changes from Current Implementation

**Current (Incorrect) Order**:

```
parse_document() → resolve_transclusion() → process_interpolation()
```

**Correct Order**:

```
parse_document() {
    extract_frontmatter()
    merge_with_parent()
    interpolate_entire_body()      ← Happens BEFORE parsing
    parse_markdown()
}
resolve_transclusion() {
    for each ::file, ::summarize, etc. {
        parse_document(external, parent_fm)  ← Recursive with frontmatter
    }
}
```

### Performance Considerations

#### Single-Pass Per Document

Each document is interpolated **exactly once** during parsing:

- Interpolation happens on the raw body text (after frontmatter extraction)
- All `{{...}}` patterns are resolved in a single pass
- Result is passed to the markdown/DSL parser
- No re-interpolation occurs during rendering

**Cost Analysis**:

- Parsing overhead: O(n) where n = body length
- State machine avoids regex backtracking
- Memory: Parser maintains minimal state (position counter only)
- No string rebuilding during scan (uses pre-allocated buffer)

#### Recursive Transclusion Cost

When a document includes `::file external.md`:

- External document is loaded once
- Interpolation applied once with merged frontmatter
- If external document has its own transclusions, process continues recursively
- Each unique document is parsed once (cached by content hash + frontmatter hash)

**Worst Case**: Deeply nested transclusions (A includes B includes C includes D...)

- Linear depth → linear cost (not exponential)
- Each document interpolated once at its depth level
- Total cost: sum of all document sizes

#### Caching Strategy

**Document-Level Caching**:

- Cache key: `hash(content + frontmatter)`
- Cached: Fully parsed document (after interpolation)
- Cache hit → skip interpolation + parsing entirely
- Cache miss → perform full parse (including interpolation)

**No Separate Interpolation Cache**: Interpolation results are part of the parsed document cache. This avoids:

- Double caching overhead
- Cache coherence issues
- Additional memory usage

**Frontmatter Changes Invalidate Cache**: If parent frontmatter changes, all transcluded documents must be re-parsed with new merged frontmatter, which correctly triggers cache misses

### Example: Directive Parameter Interpolation

To illustrate the timing requirement, consider this document:

```markdown
---
col: 3
data_source: ./sales-data.csv
---

# Sales Report

::columns {{col||2}}

This is column 1 content.

::break

This is column 2 content.

::break

This is column 3 content.

::end

## Data Table

::table {{data_source||./default.csv}}
```

**Processing Steps**:

1. **Extract frontmatter**: `col: 3`, `data_source: ./sales-data.csv`
2. **Interpolate body**:

   ```markdown
   # Sales Report

   ::columns 3      ← Resolved from {{col||2}}

   This is column 1 content.

   ::break

   This is column 2 content.

   ::break

   This is column 3 content.

   ::end

   ## Data Table

   ::table ./sales-data.csv      ← Resolved from {{data_source||./default.csv}}
   ```

3. **Parse DSL directives**:
   - `::columns 3` parser sees concrete parameter `3`
   - `::table ./sales-data.csv` parser sees concrete path

4. **Render**: Generate 3-column layout and table from CSV

**If Interpolation Happened After Parsing** (incorrect):

- `::columns {{col||2}}` parser would receive `{{col||2}}` as a string
- Parser would fail (expects integer, gets template string)
- Or parser would need interpolation logic (violates separation of concerns)

### Example: Recursive Transclusion with Frontmatter Merging

To illustrate how frontmatter is passed through transclusions:

**Parent Document** (`parent.md`):

```markdown
---
theme: dark
author: Alice
site_name: "My Blog"
---

# Welcome to {{site_name}}

::file ./header.md

Main content here.

::file ./footer.md
```

**Header Document** (`header.md`):

```markdown
---
header_size: large
---

<header class="{{theme}}-theme {{header_size}}">
  <h1>{{site_name}}</h1>
  <p>By {{author}}</p>
</header>
```

**Footer Document** (`footer.md`):

```markdown
<footer class="{{theme}}-theme">
  &copy; {{year}} {{author}}
</footer>
```

**Processing Flow**:

1. **Parse `parent.md`**:
   - Frontmatter: `{theme: "dark", author: "Alice", site_name: "My Blog"}`
   - Interpolate: `# Welcome to My Blog`
   - Parse: Find `::file ./header.md` and `::file ./footer.md` directives

2. **Transclude `header.md`**:
   - Load content
   - Extract frontmatter: `{header_size: "large"}`
   - **Merge**: `{theme: "dark", author: "Alice", site_name: "My Blog", header_size: "large"}`
   - **Interpolate with merged frontmatter**:

     ```html
     <header class="dark-theme large">
       <h1>My Blog</h1>
       <p>By Alice</p>
     </header>
     ```

   - Return resolved HTML (no DSL remaining)

3. **Transclude `footer.md`**:
   - Load content
   - No frontmatter in footer
   - **Merge**: Parent frontmatter + utilities
   - **Interpolate**:

     ```html
     <footer class="dark-theme">
       &copy; 2025 Alice
     </footer>
     ```

   - Return resolved HTML

4. **Final Result**:

   ```markdown
   # Welcome to My Blog

   <header class="dark-theme large">
     <h1>My Blog</h1>
     <p>By Alice</p>
   </header>

   Main content here.

   <footer class="dark-theme">
     &copy; 2025 Alice
   </footer>
   ```

**Key Points**:

- `header.md` references `{{theme}}`, `{{site_name}}`, and `{{author}}` from parent
- `header.md` also has its own `header_size` property (merged into namespace)
- `footer.md` references `{{year}}` (utility variable) and `{{author}}` (from parent)
- All interpolation happens **before** the transcluded content is inserted into parent
- Parent receives fully resolved HTML/Markdown (no `{{...}}` patterns remain)

### Error Handling

The parser provides detailed error messages:

```rust
pub enum InterpolationError {
    UnexpectedEnd { expected: String, position: usize },
    InvalidOperator { name: String, position: usize },
    InvalidParamCount { operator: String, expected: usize, got: usize },
    InvalidBranchCount { operator: String, min: usize, max: usize, got: usize },
    InvalidNumber { value: String },
    UnclosedExpression { position: usize },
}

impl Display for InterpolationError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::UnexpectedEnd { expected, position } => {
                write!(f, "Unexpected end of input at position {}. Expected: {}", position, expected)
            }
            Self::InvalidOperator { name, position } => {
                write!(f, "Unknown operator '{}' at position {}. Valid operators: if, has, startsWith, endsWith, gt, lt, gte, lte, between, betweenExclusively", name, position)
            }
            // ... other error messages
        }
    }
}
```

### Utility Variable Integration

Utility variables (dates, times, etc.) are already generated in `generate_utility_variables()` and merged with custom frontmatter. No changes needed - they work with both simple and conditional interpolations:

```markdown
Last updated: {{today}}
Status: {{gt(days_since_update, 7):stale||fresh}}
```

### Testing Strategy

Comprehensive tests in `lib/src/render/interpolation.rs`:

1. **Simple variables**: `{{var}}`, `{{var||default}}`
2. **Conditionals**: All operators with valid/invalid inputs
3. **Numeric edge cases**: Non-numbers, missing properties, third branch
4. **String matching**: Empty strings, special characters, unicode
5. **Nested usage**: Interpolation in popover content, columns, disclosure blocks
6. **Error cases**: Unclosed braces, invalid operators, wrong param counts
7. **Performance**: Large documents with many interpolations

### Migration Path

The new parser is **backward compatible**:

- Existing `{{variable}}` syntax works unchanged
- Simple regex is replaced but behavior is identical
- No breaking changes to public API

### Summary: Key Architectural Decisions

1. **Interpolation Timing**: BEFORE DSL parsing, not after
   - Ensures all directive parameters are concrete values
   - Enables dynamic directive parameters: `::columns {{count||2}}`
   - Prevents parser complexity (parsers don't need interpolation logic)

2. **Recursive Processing**: Each document fully resolves before insertion
   - External documents interpolate with merged frontmatter
   - Transclusion returns pure Markdown/HTML (no DSL)
   - Deep nesting handled gracefully (linear cost, not exponential)

3. **Frontmatter Inheritance**: Parent frontmatter flows to children
   - Enables templating: reusable components that adapt to context
   - Child can override parent properties
   - Utility variables available at all levels

4. **Single-Pass Efficiency**: Each document interpolated once
   - No redundant processing
   - Results cached by content + frontmatter hash
   - Performance scales linearly with document size

5. **Parser Design**: State machine over regex
   - Supports complex syntax (conditionals, operators)
   - Better error messages with position info
   - Extensible for future operators

### Implementation Impact

**Current Implementation Requires Refactoring**:

The existing code in `lib/src/render/orchestrator.rs` performs interpolation **after** transclusion:

```rust
// CURRENT (INCORRECT)
let doc = parse_document(&content, resource.clone())?;  // Parses with {{...}} in text
let resolved_nodes = resolve_transclusion(...)?;        // Transcludes
let interpolated = process_nodes_interpolation(...)?;   // Interpolates AFTER
```

**Required Changes**:

1. Move interpolation into `parse_document()` (in `lib/src/parse/mod.rs`)
2. Add `parent_frontmatter: Option<&Frontmatter>` parameter to `parse_document()`
3. Interpolate entire body before calling `parse_markdown()`
4. Update `resolve_transclusion()` to call `parse_document()` with parent frontmatter
5. Remove `process_nodes_interpolation()` from orchestrator (no longer needed)

**Benefits of Refactor**:

- Clearer separation of concerns
- Enables dynamic directive parameters
- Simplifies rendering pipeline
- Matches spec exactly


