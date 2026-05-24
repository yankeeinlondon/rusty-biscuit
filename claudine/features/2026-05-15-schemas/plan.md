---
phases: 5
created: 2026-05-15
start_phase: 1
---

# Execution Plan: Schema Support in Claudine

This plan outlines the steps required to incorporate schema support into Claudine, leveraging Darkmatter's `SimpleSchema` definitions.

## Phase 1: Foundation & Infrastructure

In this phase, we prepare the configuration and error handling systems to support schema-related logic.

- [ ] **Task 1: Update Configuration Schema**
    - Modify `claudine/lib/src/config/claudine_config.rs` (or appropriate config file) to add `prompt_for_missing: bool`.
    - Ensure it defaults to `true` if not present in the user's config file.
- [ ] **Task 2: Define `MissingProperties` Error**
    - Add a `MissingProperties` variant to the error enum in `claudine/lib/src/error.rs`.
    - Include fields for:
        - Path to the prompt file.
        - List of missing property names.
        - Map of property descriptions (from schema).
- [ ] **Task 3: Implement Advanced Error Reporting**
    - Update the `Display` or specialized reporting logic for `MissingProperties`.
    - Use `biscuit-terminal` / `Prose` to render the error.
    - Include OSC8 link to the prompt file.
    - Format missing properties and their descriptions using `<i><dim>{description}</dim></i>`.

## Phase 2: Schema Validation Logic

In this phase, we implement the core logic for validating composition properties against the schema defined in the prompt frontmatter.

- [ ] **Task 1: Integrate `SimpleSchema` into Composition**
    - Update the composition parsing logic (e.g., in `claudine/lib/src/composition/prepare.rs` or `resolve.rs`) to detect the `$schema` property.
    - Use Darkmatter's schema utilities to load and parse the referenced schema.
- [ ] **Task 2: Implement Property Validation**
    - Create a validation function that checks provided properties against the schema.
    - Categorize properties as: Fulfilled (Correct Type), Fulfilled (Wrong Type), or Missing.
- [ ] **Task 3: Implement "Wrong-Type" Logic**
    - **Required Properties:** If a required property has the wrong type, implement a hard abort that emits a diagnostic report and a hard error.
    - **Optional Properties:** If an optional property has the wrong type, implement logic to drop it from the execution and log a warning/note.
- [ ] **Task 4: Implement Non-Interactive Missing Property Detection**
    - If `prompt_for_missing` is `false` OR stdin is not a TTY:
        - Detect missing required properties and return the `MissingProperties` error.

## Phase 3: Interactive Mode (TUI)

This phase implements the interactive user experience for fulfilling missing required properties.

- [ ] **Task 1: Implement Diagnostic Status Reporting**
    - Implement the logic to log the contextual status message before entering Interactive Mode.
    - Use the specified symbols (✓, ⛔️, ⍉, ⚠) and colors.
    - Use `biscuit-terminal` for rich text output.
- [ ] **Task 2: Integrate `biscuit-tui` for Prompts**
    - In the CLI layer (`claudine/cli/src`), implement the interactive loop for missing properties.
    - Ensure it only triggers when `prompt_for_missing` is `true` and stdin is a TTY.
- [ ] **Task 3: Map Schema Types to TUI Widgets**
    - Map `enumeration` -> `choose_one`.
    - Map `enumeration array` -> `choose_many`.
    - Map `string` or `number` -> `text_input`.
    - Map `boolean` -> `boolean_switch`.
- [ ] **Task 4: Implement Numeric Validation and Retry**
    - For `number` types in `text_input`, add validation logic to convert the string.
    - If conversion fails, re-prompt the user with an error message instead of aborting.

## Phase 4: Shell Completions

Update the shell completion system to be aware of schemas.

- [ ] **Task 1: Extract Schema Properties for Completions**
    - Update `claudine/cli/src/completion/frontmatter.rs` (or similar) to parse the schema when generating completions.
- [ ] **Task 2: Prioritize Required Properties**
    - Adjust the completion engine to suggest properties defined in the schema.
    - Ensure required properties appear ahead of non-required ones in the suggestion list.

## Phase 5: Documentation & Validation

Finalize the feature with documentation and comprehensive tests.

- [ ] **Task 1: Update Documentation**
    - Update `claudine/README.md` and relevant docs under `claudine/docs/` to describe schema support and the `prompt_for_missing` configuration.
- [ ] **Task 2: Unit Testing**
    - Add unit tests for schema validation logic and "Wrong-Type" handling in `claudine/lib`.
- [ ] **Task 3: Integration Testing**
    - Add integration tests for:
        - Non-interactive mode (verifying `MissingProperties` error).
        - Shell completions (verifying prioritized suggestions).
        - Mocked interactive mode (if possible within the test harness).

## Validation Checkpoints

1. **Infrastructure Check:** Verify `prompt_for_missing` is correctly loaded from config and defaults to `true`.
2. **Error Presentation Check:** Trigger a `MissingProperties` error in a non-TTY environment and verify the OSC8 link and formatting.
3. **Validation Logic Check:** Verify that wrong-type required properties cause a hard abort with a diagnostic report.
4. **Interactive Flow Check:** Verify that missing required properties trigger the status report and subsequent TUI prompts.
5. **Completion Check:** Verify that `claudine <prompt> --<TAB>` suggests schema properties, with required ones first.
