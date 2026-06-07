# Inline Compose Sequence Mismatch

## Purpose

Prevent `claudine inline-compose <file>` from executing a document that declares both an inline prompt and a non-null sequence. Such a document defines an inline sequence and must be run with `claudine sequence <file>` so that each sequence state is applied to the prompt.

For example:

```yaml
prompt: |-
  How do you say "{{state.name}}" in Italian?
sequence:
  - name: "Hello"
  - name: "Goodbye"
```

`claudine sequence <file>` remains valid for this document. `claudine inline-compose <file>` must reject it rather than running the prompt once without sequence state.

## Scope

This specification defines:

- detection of the inline-compose/sequence command mismatch;
- the mismatch's precedence relative to existing document validation;
- the diagnostic shown for the mismatch;
- the command's failure behavior and required lack of side effects; and
- acceptance criteria for this behavior.

## Out of Scope

- Implementing the future `sections` feature.
- Changing `claudine sequence` behavior or sequence validation.
- Changing how a document with only `prompt` is processed by `inline-compose`.
- Changing existing file-reference resolution, YAML parsing, schema validation, provider selection, or provider execution behavior except for the new early rejection defined here.
- Defining exact diagnostic prose, colors, or terminal component internals beyond the content and presentation contract below.

## Normative Behavior

After the source document has been resolved, loaded, and parsed successfully, `inline-compose` must inspect the authored frontmatter before applying command-line property overrides, schema processing, composition, provider selection, or provider execution.

The command must reject the document with the mismatch diagnostic when both conditions are true:

1. The authored frontmatter contains a `prompt` key whose YAML value is not null.
2. The authored frontmatter contains a `sequence` key whose YAML value is not null.

The values' types or usability do not affect mismatch detection. For `prompt`, empty strings, collections, numbers, booleans, mappings, and other wrong-type but non-null values trigger the mismatch. For `sequence`, empty strings, empty collections, scalars, mappings, and otherwise invalid sequence definitions trigger the mismatch. Existing prompt or sequence-shape validation does not run before this check.

```yaml
# Triggers the mismatch.
prompt: Do something
sequence: []
```

```yaml
# Does not trigger the mismatch.
prompt: Do something
sequence: null
```

```yaml
# Does not trigger the mismatch.
prompt: null
sequence:
  - name: Hello
```

When either key is absent or its value is null, ordinary `inline-compose` validation and execution continue unchanged. In particular, `prompt: null` and `sequence: null` do not make an otherwise invalid document valid; existing prompt, schema, and other validations still apply.

### Validation Precedence

The externally observable validation order is:

1. Existing command-line argument and option validation occurs unchanged.
2. The file reference is resolved and the source is loaded using existing behavior. Resolution, read, or frontmatter parse failures retain their existing diagnostics and take precedence because no reliable frontmatter keys are available to inspect.
3. The mismatch check defined by this specification runs against the authored, parsed frontmatter.
4. If no mismatch exists, existing `inline-compose` prompt validation continues. This includes missing, empty, null, and wrong-type prompt behavior.
5. All later schema, preparation, provider-selection, execution, and closure behavior remains unchanged.

Because the mismatch check tests non-null authored values rather than type validity, an empty or wrong-type but non-null `prompt` combined with a non-null `sequence` produces the mismatch diagnostic. A null `prompt` does not produce the mismatch and instead proceeds to existing prompt validation. When both values are non-null, the user must invoke `sequence` before sequence-specific or prompt-type validation determines whether that document can execute successfully.

## Diagnostic Contract

The mismatch must be reported as a user-facing error on the command's existing error output stream. It must use Claudine's normal terminal rendering so styled text and an OSC 8 document link are available when supported, while remaining readable as plain text when styling or hyperlinks are unavailable.

Exact wording and grammar are not normative. The diagnostic's information, paragraph order, and general layout are normative and must include:

1. A concise opening statement that the user attempted `inline-compose` on a document configured as a sequence.
2. A blank-line-separated explanation that:
   - links to or identifies the resolved document;
   - identifies both the `prompt` and `sequence` properties;
   - explains that `sequence` causes each state to invoke an inline-compose operation using `prompt`; and
   - directs the user to run the document with `claudine sequence`.
3. A blank-line-separated note about the future `sections` feature. The note must retain the guidance that `sections` may be a better fit when operations should update particular sections of a document, while making clear that the feature is upcoming rather than currently available.
4. When the existing error output stream is a TTY, a blank-line-separated introduction stating that the document's full YAML definition follows.
5. When that stream is a TTY, a blank line followed by a YAML-rendered block containing the original frontmatter YAML text.
6. When that stream is not a TTY, a blank-line-separated explanation that the YAML definition was withheld to avoid exposing frontmatter. The YAML introduction and block are omitted. No flag or other override is provided to reveal the YAML in non-TTY output.

An acceptable prose model is:

> You tried to run an inline-compose operation on a document configured as a sequence.
>
> The document `<document>` defines both `prompt` and `sequence`. Run it with `claudine sequence` so that each sequence state invokes an inline-compose operation using the prompt.
>
> Note: The upcoming `sections` feature may be a better fit when each operation should update a particular section of the document. It may not suit every sequence workflow and is not available yet.
>
> Below is the full YAML definition of the document:

### YAML Fidelity

"Full YAML definition" means the original YAML lines in the interior of the document's frontmatter delimiters, exactly as authored. Capture begins with the first YAML line after the opening delimiter line and ends with the final YAML line before the closing delimiter line. Neither delimiter line nor the line-ending sequence that terminates a delimiter line is part of the captured YAML. The captured payload likewise excludes any line-ending sequence whose sole purpose is to separate the final YAML line from the closing delimiter.

Within those boundaries, the displayed content must preserve, without parsing and reserialization:

- property order;
- whitespace and indentation;
- comments;
- quoting;
- anchors and aliases;
- block scalar indicators and style; and
- every line-ending sequence between YAML lines, including preserving LF as LF and CRLF as CRLF.

The frontmatter delimiters and Markdown body are not part of the YAML definition and must not appear in the YAML block. Syntax highlighting or terminal styling may decorate the text but must not alter, normalize, redact, or reorder its visible content. A terminal rendering component may append its own final line termination after the captured YAML payload; that renderer-added termination is not considered part of the YAML and need not match the source document's line-ending style.

## Input, Output, and Failure Behavior

### Input

The input is the source document selected by `claudine inline-compose <file>` after existing file-reference resolution. Mismatch detection uses the document's authored frontmatter, not composed frontmatter or command-line property overrides.

### Output

On a mismatch:

- the required diagnostic is emitted through Claudine's existing error-reporting path;
- the document reference identifies the resolved source document and is hyperlinked when terminal capabilities permit;
- when the existing error output stream is a TTY, the complete original frontmatter YAML text is included as specified above;
- when that stream is not a TTY, the YAML block is omitted and the diagnostic explicitly states that it was withheld to avoid exposing frontmatter; and
- the command returns the standard nonzero failure outcome used for Claudine command errors.

This specification does not assign a new numeric exit code.

### Side Effects

Mismatch rejection is fail-fast. Before returning the error, Claudine must not:

- execute frontmatter or template shell commands;
- prompt for schema values or shell approval;
- select, launch, or communicate with a provider;
- modify the source document, including its body or `last_updated` value; or
- create execution artifacts whose creation belongs to later composition or provider phases.

Normal work required to resolve, read, and parse the source document is permitted.

## Acceptance Criteria

1. A document with a valid string `prompt` and a nonempty `sequence` list is rejected by `inline-compose` with a nonzero result and the required mismatch diagnostic.
2. A document with `prompt` and `sequence: []` is rejected; an empty sequence is non-null.
3. A document with `prompt` and a non-null scalar, mapping, or otherwise sequence-invalid value is rejected with the mismatch diagnostic before sequence-shape or schema errors.
4. A document with `prompt` and `sequence: null` does not produce the mismatch diagnostic and proceeds through ordinary `inline-compose` validation.
5. A document with `prompt` and no `sequence` key retains existing `inline-compose` behavior.
6. A document with a non-null `sequence` but no `prompt` key does not produce this mismatch diagnostic; existing missing-prompt behavior applies.
7. A document with an empty or wrong-type but non-null `prompt` and a non-null `sequence` produces the mismatch diagnostic before existing prompt validation.
8. A document with `prompt: null` and a non-null `sequence` does not produce the mismatch diagnostic; existing null-prompt validation applies.
9. Malformed frontmatter retains the existing frontmatter-parse diagnostic rather than producing the mismatch diagnostic.
10. Command-line property overrides do not create or suppress the mismatch. Detection reflects the authored `prompt` and `sequence` values.
11. In a TTY, the diagnostic identifies the resolved document, names `prompt` and `sequence`, directs the user to `claudine sequence`, retains the future `sections` guidance, and includes the full authored frontmatter YAML.
12. In redirected or other non-TTY error output, the diagnostic retains the command-mismatch and future `sections` guidance, omits the YAML introduction and YAML block, and explicitly explains that the YAML was withheld to avoid exposing frontmatter.
13. A TTY fidelity test using comments, noncanonical property order, anchors or aliases, and a block scalar confirms that the diagnostic's YAML block preserves the original frontmatter text without reserialization.
14. Line-boundary tests confirm that opening and closing delimiter lines and their associated separator line endings are excluded. Separate LF and CRLF fixtures confirm that line endings between captured YAML lines are preserved. Any final line termination added by the terminal renderer is ignored when comparing the captured YAML payload.
15. Rejection occurs before shell execution, approval prompts, provider selection or launch, and source-file mutation. Tests must verify that configured shell commands are not run, a provider stub is not invoked, and the source file remains byte-for-byte unchanged.
16. The diagnostic remains understandable when terminal styling and OSC 8 hyperlinks are unavailable.

## Definition of Done

This change is complete when all acceptance criteria pass, existing `inline-compose` and `sequence` tests remain passing, and user-facing composition documentation is updated if it currently describes behavior that conflicts with this specification.

## Settled Decisions

- The mismatch trigger is a present, non-null authored `prompt` value plus a present, non-null authored `sequence` value.
- `prompt: null` or `sequence: null` does not trigger the mismatch; ordinary validation continues.
- Empty or wrong-type but non-null prompt values still trigger the mismatch when sequence is non-null.
- The diagnostic's required information and general layout are normative, but exact wording and grammar may be improved.
- Guidance about the future `sections` feature remains in the diagnostic and must identify it as upcoming.
- The YAML block appears only when the existing error output stream is a TTY. Non-TTY output omits it and explains that it was withheld to avoid exposing frontmatter; there is no reveal flag.
- The YAML block uses original frontmatter source text, not parsed and reserialized YAML, and preserves authored formatting, YAML constructs, and line endings between YAML lines while excluding delimiter lines and delimiter-associated separator line endings.
