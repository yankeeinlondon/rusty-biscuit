---
spec: /Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-06-dotted-paths/spec.md
plan: darkmatter/features/2026-11-06-dotted-paths/plan.md
implemented_by: claude/opus
started_phase: "1"
decisions_for_review:
    - |-
        Compose setters keep reaching transcluded child documents, and each
        document reads a setter through its own schema. The plan (rule 16) chose
        this; the spec does not cover the case. `md compose p.md v=7` already
        changes a child's `v` today, because options flow down to children, so
        keeping it preserves current behavior. One new consequence: a nested
        setter that meets a scalar parent or an out-of-range index in a child
        fails the whole run and names that child.
        Alternatives: apply setters to the root document only, which makes
        children predictable but silently changes what `v=7` does today; or skip
        a child where the path cannot apply, which never fails on a child but
        hides a setter that did nothing.
        To choose another option, change the setter plan's application step in
        the pipeline and preflight callers, plus the child-document tests. The
        path parser, value parser, and text editor would not change.
    - |-
        The spec's phrase "recursive merge behavior of those older options" is
        wrong about `--set`, and later phases follow the code. `--set` replaces a
        top-level key whole and only `--state` deep-merges, as
        `compose/util.rs` shows. The plan called for this correction. Neither
        option changes, and the spec stays as written because a spec is a
        snapshot; current docs will describe the real behavior.
        The alternative was to make `--set` merge recursively so the spec's
        wording became true, but that changes the behavior of an existing option
        and the spec puts both options out of scope.
        If the reviewer wants `--set` to merge, the change is confined to the
        `--set` block of `prepare_frontmatter_for_compose` and its tests, and the
        documented precedence examples would need rewriting.
    - |-
        Under a declared type of `unknown`, a command-line value is guessed the
        way an undeclared one is, so `x=5` is the number 5. The plan says any
        declared shape reads the text through the conversion engine, which would
        keep "5" as a string, because `unknown` accepts anything unchanged.
        Following the plan literally would change what `x=5` does today for every
        key typed `unknown`, including keys in open objects, while the type says
        nothing about which reading is meant.
        Alternative: follow the plan, so a declared key is always text first; the
        author then writes numbers in a container or `--set` to get a number.
        To choose it, delete `declares_nothing` in `setter_value.rs` and its test.
    - |-
        A YAML flow value with a tag, anchor, or alias is read as plain text, not
        as an error. The plan says the flow reader rejects tags and block
        documents without saying what rejecting means, and does not mention
        anchors or aliases. Text fits the spec's "syntax rejection can fall back
        to text", and nothing in the value runs. A declared object type then
        fails schema validation with the text in view.
        Alternatives: fail with a setter error, which is clearer for someone who
        really meant a tag but breaks a plain string that merely looks like one;
        or allow anchors and aliases, which have no use within one argument.
        To change it, adjust `is_one_plain_flow_mapping` in `setter_value.rs`,
        plus its tests.
    - |-
        A bare number that cannot be held exactly, such as `9007199254740993`,
        becomes text when no type is declared. The plan says this "stays text as
        today", but reading today's `parse_shorthand_value` shows it stays a JSON
        number and only non-finite values become text. I took the plan's outcome,
        since the spec forbids an invalid number becoming a rounded value. The
        change is visible only to code that checks the value's type; it prints
        the same.
        Alternative: keep today's number, which keeps exact digits in the value
        but lets a number outside the numeric model into the pipeline.
        To change it, adjust `guess_scalar` in `setter_value.rs` and the
        `a_bare_inadmissible_number_stays_text` test.
    - |-
        A JSON5-quoted value is not converted when it is bound: `"5"` under
        `number` stays the string "5" at that step. The plan names binding for
        text and containers only. Schema validation then treats it exactly as it
        treats an authored `count: "5"`, so compose still gets 5.
        Alternative: bind quoted values too, so `md set` would write 5 rather than
        "5", overriding quotes the author typed on purpose.
        To change it, route the `Quoted` arm of `SetterValue::bind` through
        `bound_or_original`.
    - |-
        A value such as `{foo}` now reads as the object `{foo: null}`, where today
        it is the string "{foo}". This follows from the spec's YAML fallback, which
        requires `{ a:1, max-width: 2 }` to read `a:1` as a key with no value; the
        two cannot be told apart. `{{ template }}` values stay text, because their
        key would be a mapping, which no JSON object can hold.
        Alternative: refuse keys with no value, which keeps `{foo}` as text but
        contradicts the spec's example.
        To change it, add that check to `json_from_flow_yaml` and update the
        `{ a:1, max-width: 2 }` test.
    - |-
        Additions the plan did not list. `PathEditError::RemovalConflict` reports
        duplicate or ancestor removals, since none of the four planned variants
        describes them. `json5_is_well_formed` in biscuit-file tells a grammar error
        from an integer beyond `u64`: `Json5::from_str` fails on both, and treating
        that as syntax let `{a:99999999999999999999}` reach YAML as a key named
        `a:9999...`. `set_at` and `RemovalPlan` take the frontmatter `IndexMap` only,
        while `resolve` also takes a `Value`, because every planned caller edits a
        frontmatter map. `Resolved` uses the existing `PathStep` as its typed step
        rather than a parallel type.
        Alternatives would be a separate conflict error type owned by `md rm`, or
        value-rooted edits; either is a small change to `property_path.rs`.
    - |-
        I added `edited_document_text` beside the planned `edit_frontmatter_paths`.
        It holds the fallback in the library, so `md set` and `md rm` call one
        function and Phase 4's `edited_text` becomes a thin wrapper. The plan named
        only the editor and described the fallback as a rule. The fallback also
        differs from today's: it puts the serialized YAML between the source's own
        fences and keeps the body's bytes. Today's `Markdown::as_string` drops the
        body's final line break and turns CRLF into LF, which breaks the spec's rule
        that the body stays unchanged.
        Alternative: leave the fallback in the command and keep `as_string`, which
        is less code here but keeps that body damage.
        To change it, edit `serialized_into_source` in `hash/path_edit.rs`.
    - |-
        The editor changes a flow collection when the path names it whole, and
        treats an empty `{}` as a parent it can add keys to. The plan lists flow
        collections among the forms to refuse. Refusing only walks into a non-empty
        one keeps comments in more documents, and matters in practice: `md rm` leaves
        `{}` behind, and the next `md set` under it would otherwise re-serialize the
        whole frontmatter and lose every comment.
        Alternative: refuse every flow collection, which is a simpler rule but costs
        those comments.
        To change it, remove the `empty_mapping` branch in `walk`.
    - |-
        Only a parse failure that says `unknown anchor` counts as an orphaned alias,
        and has no fallback. Any other parse failure of the edited text is an
        unsupported form, which falls back. The plan does not say how to classify a
        result that no longer parses. The parser's wording is the only signal, so an
        upgrade that rewords it would turn this refusal into a fallback; the values
        would still be right, with the alias written out, and
        `removing_an_anchor_an_alias_needs_is_refused_with_no_fallback` would fail.
        Alternative: treat every parse failure as a refusal, which needs no wording
        check but blocks a save the fallback could do.
        To change it, edit `reload_values`.
    - |-
        Any difference outside the named paths is refused with no fallback, even if
        the document has no alias. This is what the plan says, but it means a mistake
        in the editor's own positioning would show as an error message about aliases,
        and the user could not save until the source changed. Nothing is written in
        that case. Testing found one such mistake, a new key cutting into a `|+`
        scalar, which is fixed.
        Alternative: refuse only when the source has an anchor, and otherwise fall
        back, which lets such a save succeed but no longer proves the editor correct
        by failing loudly.
        To change it, add that check in `verify_edited`.
    - |-
        Three comment choices the plan leaves open. A comment on the line above a
        removed node stays, because it is outside the node. A comment inside a
        replaced container goes with the container, except on its key's line, which
        stays. When a value grows to several lines, a comment after it moves to its
        first line, because a block scalar's last line would read it as content.
        Alternative: remove a comment directly above a removed node, on the guess that
        it describes that node; this tidies the common case but sometimes deletes a
        comment about something else.
        To change it, adjust `remove_node` and `in_place`.
    - |-
        `md set` judges a document with the validation that defers pending values,
        not the plain one the plan names. The plan says `EffectiveSchema::validate`,
        which judges a whole `{{ template }}` as written, so `md set a.md count
        '{{ fm.n }}'` under `number` would be refused. The spec says pending values
        keep the existing composition rules, and composition accepts that value until
        it renders. A malformed template is still refused.
        Alternative: follow the plan, which is stricter but blocks a value compose
        would accept.
        To change it, call `validate` in `problems_of` in `commands/frontmatter.rs`,
        and update `a_pending_value_is_written_unevaluated`.
    - |-
        Under `boolish`, an unquoted `true` or `false` is written as a boolean. Read
        strictly, the plan's "goes through the conversion engine as text" keeps it as
        the string "true", because `boolish` keeps what it is sent. The baseline
        declares `draft: boolish`, so `md set doc.md draft true` would write
        `draft: 'true'` and break a call that writes `draft: true` today. `On` and
        `yes` stay text, as the spec requires. `string | boolean` is not changed, so
        `true` stays text there, under the spec's union rules.
        Alternative: keep text everywhere, which is simpler, and accept the quoted
        result, which still reads as truthy.
        Compose setters in Phase 5 inherit this because they share `bind`. To change
        it, remove `keeps_booleans_as_sent` in `setter_value.rs` and its rows in
        `text_goes_through_the_declared_type`, then decide what
        `test_set_boolean_value` should expect.
    - |-
        A declared type comes from Darkmatter's compiled baseline, matching triggers,
        and the document's schema, even when the baseline is replaced or disabled. The
        plan names `def_at_path` without saying how a baseline reaches it, and only
        `EffectiveSchema::property_shape` merges the layers, always with the compiled
        baseline. So a key only a custom `BASELINE_SCHEMA` declares reads as undeclared
        (`1.10` becomes 1.1 before validation judges it), and a key the disabled
        baseline declares still reads through that type. Validation always uses the
        real baseline selection, so a wrong type is never written unnoticed.
        Alternative: derive the conversion from the merged JSON Schema, which covers
        custom baselines but would add a second route from schema to type, for a rare
        setup.
        To change it, edit `declared_shape` in `schemas/completion.rs`, which Phase 5
        also calls.
    - |-
        What `md set` accepts for `$schema`, where the spec only says "a valid schema
        reference". Unquoted text is always a reference, so `$schema 2` is the bare
        name "2". An inline object or array is also accepted if it parses as a schema,
        because `$schema` can be written that way, and an edit inside one such as
        `$schema.count number` judges the whole result. `null`, numbers, and booleans
        are refused; `md rm` removes the key. No schema file is read, so a reference to
        a file that does not exist is accepted.
        Alternative: accept only strings, which would stop inline schemas being set at
        all.
        To change it, edit `set_schema_key` in `commands/frontmatter.rs`.
    - |-
        `md rm` keeps today's message, `Property "x" not found in frontmatter`, for a
        key that does not exist, including listing every missing path, because existing
        tests and scripts may read it. The plan routes all failures through
        `PathEditError`, whose wording is "`x` does not exist". An out-of-range index, a
        scalar parent, and a conflict use `PathEditError`'s own messages.
        Alternative: one wording everywhere, which means less code but changes an
        existing message.
        To change it, delete the `not_found` block in `rm_value` and update `rm.rs`.
    - |-
        Additions the plan did not list, outside the phase's named files. The library
        gains `schemas::completion::declared_shape`, which is rule 7's Darkmatter caller
        and which Phase 5 also needs, and `frontmatter_as_json` becomes public so the
        CLI judges the same value the library does. `CommandRun::add_diagnostics` lets a
        skipped trigger's warning reach a data output even when the edit then fails.
        `schema/validate.rs` exposes `request_schemas`, so `md set` selects its baseline
        as `md schema validate` does. `--help` text for the three commands now describes
        paths. On stdin, `md set` discovers triggers from the launch directory's
        repository, since a pipe has no directory of its own.
        Alternative for the library function: keep it in the CLI, then move it in Phase 5.
    - |-
        A schema problem on a value a setter supplied names the setter in its
        message and has a null line and column, in a `dm.schema.*` record. The
        plan says schema failures keep their codes and says nothing about their
        positions, but left alone they pointed at the old on-disk scalar, which
        the spec forbids. The record's file stays, as for any schema problem.
        Alternative: report these as `dm.setter.invalid`, which loses the
        schema's code that a consumer may branch on.
        To change it, edit `SetterPlan::attribute_problem` and its callers in
        `schema_validation.rs`.
    - |-
        A `dm.setter.invalid` record keeps `source: null` even when `md compose`
        knows its document, so `with_document_fallback` skips that code. The plan
        requires null, and the fallback would otherwise have filled in the file.
        When a path cannot be applied in a document, the message names that
        document, which is how a child is named (plan rule 16).
        Alternative: a file with null line and column, like other failures with
        no position. It reads as though the value were in that file, and the
        spec says it is not.
        To change it, remove the code check in `with_document_fallback`.
    - |-
        A nested setter on a `file` value resolves from the launch directory, as a
        top-level one does, and its absolute path stays in the frontmatter. Two
        things differ from a top-level setter: `{{ author.avatar }}` prints the
        absolute path rather than the portable one, and a `match()` glob on it is
        judged from the document's folder, because presentation values and caller
        origins are keyed by top-level property. A whole object or a `--set` object
        keeps today's limit: a file value inside one is not projected at all.
        Alternative: extend both to paths, which changes `CallerOrigins` in
        simplified-schema and the effective state's presentation map, for a rare
        case.
        To change it, start at `resolve_nested_setter_files` in
        `schema_validation.rs`.
    - |-
        Added a public `ValidationProblem::offending_properties`, listing every
        undeclared key of a failure. The plan compares problems without one: an
        object's undeclared keys are one problem, and `offending_property` holds
        only the first. That let `md set doc.md author.mistake 1` write when
        `author` already had another undeclared key, a Phase 4 defect now fixed in
        `blocking_problems` and tested, and it hid the setter in the same case.
        Alternative: one problem per key, which is simpler to read but changes
        how many diagnostics every consumer reports.
        To change it, expand the problems in `validate.rs` and drop the field.
    - |-
        `apply_setter_plan` returns what it wrote, and the pipeline stores that on
        the request, rather than taking a mutable request as the plan's signature
        suggests. The pre-flight walks hold a shared request and need only the
        document and provenance. A nested setter into a key `--set` supplied
        records the whole key, so the `--set` record cannot restore the old value.
        A repeated path keeps its first position with its last value, as the old
        map did. Nothing else depends on these choices; each is local to
        `setters.rs`.
    - |-
        Tab candidates read the schema through every layer, not through the function
        the plan names. The plan says to use `for_path` below the root, but that reads
        only the document's own schema, so `style.<Tab>` would offer nothing.
        `path_candidates` calls `for_path_in` over `EffectiveSchema::property_shape`,
        the same merged view `declared_shape` uses.
        Alternative: extend `for_path` to merge the layers, which would also change
        what the language server offers. No caller here needs that change.
        To change it, edit `SchemaView` in `schemas/completion.rs`.
    - |-
        Additions to the candidate API the plan did not list. The plan's query has one
        typed prefix; `PathPosition` also lets `md set` ask about its separate value
        argument. The plan's four candidate kinds have no way to say "offer files", and
        the function does no I/O, so `path_value_takes_files` tells the caller to add
        them.
        Alternatives: a fifth candidate kind for files, which would be a candidate with
        no text to insert; or a second query type for values, which duplicates fields.
        Either is a small change in `schemas/completion.rs` plus the Phase 7 adapter.
    - |-
        A candidate's `insert` is the whole word under the cursor, so a setter's value
        reads `status=draft`. The spec shows `left center right` and says bash needs the
        value alone. Most shells replace the whole word, so bash's own completer strips
        the `key=` in Phase 7, rather than every other shell adding it.
        Alternative: return the value alone and have the adapter add `key=` for other
        shells. To change it, edit `lead` in `value_candidates`.
    - |-
        A path that could only fail when run offers nothing, including a declared object
        whose present value is a scalar or `null`. So `author: text` offers `author=` but
        not `author.`, and `items.9.` and `missing.0.` offer nothing. The spec's table
        speaks only of types, and says validation is the judge; but `md set` and `md
        compose` refuse these paths outright, so offering them leads straight to an error.
        Alternative: offer by declared type alone. To change it, edit `node_spellings`
        and `walk_path`.
    - |-
        An object offers both `author.` and `author=` at every level in a setter command,
        and a container item offers both `items.0.` and `items.0=`. The spec's table says
        this for objects, though its opening example lists only `author.` and `style.`,
        and its array row can be read as `items.0=` being for scalars only. I followed the
        table, since replacing a whole object or item is valid.
        Alternative: offer only descent where children exist, which shortens the list but
        hides a valid edit. To change it, edit `node_spellings`.
    - |-
        The value being replaced does not narrow its own choices. `md set doc.md block.kind
        <Tab>` on a `kind: rust` block offers `rust` and `python`, though the spec says a
        tag narrows the keys and values of its arm. Narrowing there would only ever offer
        the value already set. Other keys of the block still narrow by that tag.
        Alternative: narrow by every authored value, which makes a tag impossible to change
        through Tab. To change it, drop `without_value` from `value_candidates`.
    - |-
        A value keeps its plain spelling unless the value parser would read it as something
        else, and is then JSON-quoted: an enum member `null` is offered as `"null"`, `{a}`
        as `"{a}"`. The check runs the real parser and binding on each candidate. With no
        effective schema there is no declared type, so a numeric-looking member would be
        quoted too. The spec requires the value to decode as intended without saying how.
        Alternative: always quote, which is simple but puts quotes around every ordinary
        word. To change it, edit `spell_value`.
    - |-
        The bash and zsh registration scripts are produced by asking `clap_complete`
        for its built-in script and applying small text edits to it. The plan said the
        scripts would be "adapted from" the built-ins, which reads as keeping a copy.
        Editing keeps about ten lines instead of about sixty and picks up upstream
        quoting of the program path. The trade-off is that an edit stops applying if an
        upgrade rewords the script; `every_registration_edit_applies` fails when that
        happens. A kept copy would never fail that way, but would drift from upstream
        unseen. Choosing a copy would change only `completion_shells.rs`.
    - |-
        Bash's word shape after `=` differs from what the spec recorded. On bash 5,
        `x=<Tab>` makes the current word `=` itself with an empty `$2`, and the built-in
        script overwrites that word, which loses the `=`. The script now keeps it, and
        `md` accepts both `x =` and the spec's `x = ""` as `x=`. Bash also splits at `:`
        and at each further `=`, so the rejoin handles those too, which is more than the
        plan's `key`, `=`, `value`. One ambiguity remains: from the words alone,
        `x= y<Tab>` looks like `x=y`, so a value deliberately left empty joins the next
        word. That only changes what an earlier setter holds during completion. Passing
        `COMP_LINE` to `md` would remove the ambiguity, at the cost of a second source of
        truth for the words; that change would stay inside `completion_shells.rs`.
    - |-
        `md` now shell-unquotes the words bash and zsh hand over, and escapes bash
        replies with backslashes (`two\ words`). The plan said only that shell quoting
        comes after path escaping. Without this, an enum member with a space or `$` would
        be inserted as source text and arrive as several words, and a typed `a\\.b` would
        be read with its shell escape still on it. Inside an open quote a reply is not
        escaped, and readline closes the quote. Fish, elvish, and PowerShell keep their
        built-ins, and fish offers nothing after an open quote because it passes the raw
        token. Handling that would mean replacing fish's adapter too, which the plan asks
        to avoid unless needed.
    - |-
        On bash 3.2, which macOS ships, a finished candidate carries its own trailing
        space. `compopt` does not exist there, so `nospace` cannot be switched off per
        reply, and without this every accepted value would need a typed space. The built-in
        script has the signal (`_CLAP_COMPLETE_SPACE`) but ignores it. The space is
        omitted inside an open quote. The alternative is to leave bash 3.2 with no spaces,
        which only needs deleting that branch of `bash_reply`.
    - |-
        Completion honors `--state`, `--set`, and `--magic-root` from the words before
        the cursor, as the spec lists, but not `--baseline-schema`, `--disable
        baseline-schema`, or `--disable schema-triggers`. So after one of those flags, Tab
        still reads the normal baseline and triggers. The candidates are advisory and
        validation stays the judge, so the cost is a suggestion that may not apply. Adding
        them means reading those flags in `CommandLine::read` and passing them to the
        schema assembly, which is contained in `path_completion.rs` and
        `commands/schema/validate.rs`.
    - |-
        `RequestSnapshot::from_process` now has a second production call site, in
        `completion_request` in `cli/src/request.rs`, with a stated allow-list entry in
        `context_construction_guard.rs`. The guard's rule was one read, in `main`.
        `CompleteEnv` answers a Tab press and exits before `main` reaches that read, and
        the plan asked for the snapshot to be built through `request.rs`. The alternative
        is capturing the snapshot in `main` before `CompleteEnv` runs, which charges every
        ordinary `md` run for a read only completion uses at that point, or passing it in
        a global.
    - |-
        `md compose x=<Tab>` for a word whose key is not setter grammar, such as
        `./foo=ba`, now completes Markdown files. It offered nothing before, because any
        `=` suppressed files. This follows the plan's one-classification rule. `md schema
        validate` keeps the older `complete_compose_args`, which suppresses files after any
        `=`, because this phase does not own that command and it has its own grammar.
    - |-
        The L1 subprocess tests landed in Phase 7 in `cli/tests/l1/path_completion.rs`,
        a file the plan gives to Phase 8. Phase 7 needed end-to-end proof of its wiring.
        Phase 8 keeps ownership and extends the same file rather than starting another.
    - |-
        The read limit is proven by what can be observed, not by counting file
        opens. The plan says to assert it through the work counters, and the spec
        says each schema file is read once. The counters and the schema resolver's
        own list of read files show one schema preparation, one trigger scan, one
        parse of a file imported twice, and no read of an unrelated schema or of a
        child document's schema. They cannot count a second read of a file whose
        parsed table was already held. That case is settled by reading the code:
        the resolver reuses a namespace it has already loaded for that file.
        Alternatives: add a counter at the file-read call, which is a small
        production change to simplified-schema; or trace system calls, which needs
        elevated rights on macOS and is not portable. A named pipe was tried as a
        read-once file, and a schema path that is not a regular file is refused
        before it is read, so it proves nothing.
        To choose a read counter, add a `Work` kind at `buffers::read` and tighten
        `one_tab_prepares_once_and_reads_only_the_documents_schema_files`.
    - |-
        A shell that a host does not have reports itself unproven and the test
        passes, rather than failing. The spec says an L1 prerequisite uses
        `expect_level!` and cannot silently skip, and also says to select
        platform-specific tests deliberately and record an absent shell as
        unproven. So bash (Unix), bash 3.2 and zsh (macOS), and PowerShell
        (Windows) are required where they are guaranteed, while fish is optional
        everywhere and prints `unproven`. CI will mostly not run fish. Elvish has
        no script test at all, because none could be checked here and an unrun test
        would claim more than is known. PowerShell's script load is proven on native
        Windows; its real Tab behavior is not.
        Alternatives: install every shell on the CI images, which adds a cost CI has
        no other need for; or fail when a shell is absent, which turns a missing
        optional shell into a red build.
        To change it, provision the shell and turn its early return into
        `expect_level!`.
    - |-
        The real Tab test types the dot itself: `style.pa<Tab>` stops at `style.page`,
        and the test then types `.`. The spec's sequence, `style.pa<Tab>` then
        `ge.<Tab>`, reads as if Tab might not advance at all. Tab stops there because
        both `style.page.` and `style.page=` are valid continuations, as the spec's
        own spelling table requires, so no shell can pick one. The finished line is
        still exactly `style.page.alignment=center`, with no stray space in any shell
        tested, and the command then runs.
        Alternative: offer only `style.page.` for an object key, so one Tab reaches the
        dot, which removes the way to replace a whole object by Tab.
        To choose it, change the object-key row in `path_candidates` and its tests.
    - |-
        Two dotted grammars remain on `md`, and the docs say so. `md schema validate
        post.md user.email=x` keeps its own older assignment rules (a YAML value
        and its own key splitting), not `PropertyPath` and the shared value parser.
        The spec's scope lists `get`, `set`, `rm`, and `compose` only, so nothing
        changed; `docs/cli/index.md` names the exception.
        Alternative: move those assignments onto `PropertyPath` and `SetterValue`,
        so one spelling works everywhere, at the cost of changing what a value such
        as `tags=[a, b]` means there.
        To choose it, change `commands/schema/assignment.rs` and its docs in
        `topics/schemas/definition.md`.
    - |-
        Migration tables sit at the end of each command's own page, rather than on
        one migration page. The plan asks for migration examples without saying
        where. A reader of `md set` finds what changed for `md set`, and `get`, `rm`,
        and `compose` each list only their own differences. The grammar and value
        rules are one section each in `docs/cli/index.md`, as the spec asks, and every
        page links there.
        Alternative: one migration page, which is shorter overall but sits away from
        the command being read.
        To choose it, move the "Migrating older calls" sections; no other text
        depends on them.
    - |-
        The per-shell table in `docs/cli/index.md` says "tested" or "not tested" for
        each of three levels, and does not claim support for elvish or for
        PowerShell's real Tab. It says elvish is "likely" to add a space after `style.`,
        because that comes from reading its built-in script, with no run behind it.
        The spec allows either a pass or a documented limit for each shell.
        Alternative: run both shells to replace those words with facts, which needs
        elvish installed and a native-Windows real-terminal run.
        To change it, update that table and its list when evidence exists.
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - biscuit-file/lib/src/json5/mod.rs
    - darkmatter/cli/tests/l1/output_commands.rs
    - biscuit-file/lib/src/json5/scan.rs
    - biscuit-file/lib/src/lib.rs
    - simplified-schema/lib/src/lib.rs
    - simplified-schema/lib/src/numeric.rs
    - simplified-schema/lib/src/numeric/ingress.rs
    - simplified-schema/lib/src/property_path.rs
    - simplified-schema/lib/src/setter_value.rs
    - simplified-schema/lib/tests/l1/main.rs
    - simplified-schema/lib/tests/l1/property_paths.rs
    - simplified-schema/lib/tests/l1/setter_values.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
packages:
    - biscuit-file
    - darkmatter
    - darkmatter-cli
    - simplified-schema
source_files_during_phase_3:
    - darkmatter/lib/src/markdown/hash/path_edit.rs
    - darkmatter/lib/src/markdown/hash/mod.rs
    - darkmatter/lib/src/markdown/hash/write.rs
    - darkmatter/lib/tests/l1/frontmatter_path_edit.rs
    - darkmatter/lib/tests/l1/main.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/frontmatter.md
source_files_during_phase_4:
    - darkmatter/cli/src/args/command.rs
    - darkmatter/cli/src/commands/frontmatter.rs
    - darkmatter/cli/src/commands/schema/validate.rs
    - darkmatter/cli/src/output/command.rs
    - darkmatter/cli/tests/l1/get_set_rm.rs
    - darkmatter/cli/tests/l1/output_commands.rs
    - darkmatter/lib/src/markdown/schemas/completion.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - simplified-schema/lib/src/setter_value.rs
    - simplified-schema/lib/tests/l1/setter_values.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/darkmatter/frontmatter.md
source_files_during_phase_5:
    - darkmatter/cli/src/commands/compose.rs
    - darkmatter/cli/src/commands/frontmatter.rs
    - darkmatter/cli/src/commands/mod.rs
    - darkmatter/cli/tests/l1/compose_setters.rs
    - darkmatter/cli/tests/l1/get_set_rm.rs
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
    - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
    - darkmatter/lib/src/markdown/compose/preflight/collect.rs
    - darkmatter/lib/src/markdown/compose/report_diagnostics.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/compose/setters.rs
    - darkmatter/lib/src/markdown/compose/util.rs
    - darkmatter/lib/src/markdown/compose/value_origin.rs
    - darkmatter/lib/src/markdown/errors/blocks.rs
    - darkmatter/lib/src/markdown/schemas/bound_operands.rs
    - darkmatter/lib/src/markdown/schemas/yaml_documents/load.rs
    - darkmatter/lib/src/markdown/types.rs
    - darkmatter/lib/tests/l1/compose_setters.rs
    - darkmatter/lib/tests/l1/error_snapshots/markdown_error.rs
    - darkmatter/lib/tests/l1/main.rs
    - simplified-schema/lib/src/validate.rs
    - simplified-schema/lib/src/validation.rs
docs_updated_during_phase_5:
    - darkmatter/docs/cli/data-contracts.md
    - darkmatter/docs/errors/composition-problems.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/darkmatter/compose.md
source_files_during_phase_6:
    - darkmatter/lib/src/markdown/schemas/completion.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/path_candidates.rs
docs_updated_during_phase_6: []
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
    - .claude/skills/darkmatter/schema.md
source_files_during_phase_7:
- darkmatter/cli/src/args/command.rs
- darkmatter/cli/src/args/completion.rs
- darkmatter/cli/src/args/mod.rs
- darkmatter/cli/src/args/path_completion.rs
- darkmatter/cli/src/commands/frontmatter.rs
- darkmatter/cli/src/commands/schema/validate.rs
- darkmatter/cli/src/completion_shells.rs
- darkmatter/cli/src/lib.rs
- darkmatter/cli/src/main.rs
- darkmatter/cli/src/request.rs
- darkmatter/cli/tests/l1/context_construction_guard.rs
- darkmatter/cli/tests/l1/main.rs
- darkmatter/cli/tests/l1/path_completion.rs
- darkmatter/lib/src/markdown/compose/setters.rs
docs_updated_during_phase_7: []
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
- .claude/skills/darkmatter/cli.md
source_files_during_phase_8:
- darkmatter/cli/Cargo.toml
- darkmatter/cli/src/args/path_completion.rs
- darkmatter/cli/tests/l1/main.rs
- darkmatter/cli/tests/l1/path_completion.rs
- darkmatter/cli/tests/l1/property_paths.rs
- darkmatter/cli/tests/level2/level2_path_completion.rs
- darkmatter/cli/tests/level2/main.rs
- darkmatter/lib/tests/l1/compose_setters.rs
docs_updated_during_phase_8: []
docs_created_during_phase_8: []
skills_files_updated_during_phase_8:
- .claude/skills/darkmatter/testing.md
- .claude/skills/os/windows.md
source_files_during_phase_9: []
docs_updated_during_phase_9:
    - darkmatter/docs/cli/compose.md
    - darkmatter/docs/cli/data-contracts.md
    - darkmatter/docs/cli/get.md
    - darkmatter/docs/cli/index.md
    - darkmatter/docs/cli/rm.md
    - darkmatter/docs/cli/set.md
    - darkmatter/docs/errors/composition-problems.md
    - darkmatter/docs/topics/frontmatter.md
    - darkmatter/docs/topics/numbers.md
    - simplified-schema/README.md
    - simplified-schema/docs/topics/foundations.md
docs_created_during_phase_9:
    - simplified-schema/docs/topics/property-paths.md
    - simplified-schema/docs/topics/setter-values.md
skills_files_updated_during_phase_9:
    - .claude/skills/biscuit-file/references/api.md
    - .claude/skills/darkmatter/cli.md
    - .claude/skills/os/macos.md
    - .claude/skills/simplified-schema/SKILL.md
source_files_during_phase_10:
    - darkmatter/cli/tests/l1/compose_setters.rs
docs_updated_during_phase_10: []
docs_created_during_phase_10: []
skills_files_updated_during_phase_10: []
source_code:
    - biscuit-file/lib/src/json5/mod.rs
    - biscuit-file/lib/src/json5/scan.rs
    - biscuit-file/lib/src/lib.rs
    - darkmatter/cli/Cargo.toml
    - darkmatter/cli/src/args/command.rs
    - darkmatter/cli/src/args/completion.rs
    - darkmatter/cli/src/args/mod.rs
    - darkmatter/cli/src/args/path_completion.rs
    - darkmatter/cli/src/commands/compose.rs
    - darkmatter/cli/src/commands/frontmatter.rs
    - darkmatter/cli/src/commands/mod.rs
    - darkmatter/cli/src/commands/schema/validate.rs
    - darkmatter/cli/src/completion_shells.rs
    - darkmatter/cli/src/lib.rs
    - darkmatter/cli/src/main.rs
    - darkmatter/cli/src/output/command.rs
    - darkmatter/cli/src/request.rs
    - darkmatter/cli/tests/l1/compose_setters.rs
    - darkmatter/cli/tests/l1/context_construction_guard.rs
    - darkmatter/cli/tests/l1/get_set_rm.rs
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/cli/tests/l1/output_commands.rs
    - darkmatter/cli/tests/l1/path_completion.rs
    - darkmatter/cli/tests/l1/property_paths.rs
    - darkmatter/cli/tests/level2/level2_path_completion.rs
    - darkmatter/cli/tests/level2/main.rs
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
    - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
    - darkmatter/lib/src/markdown/compose/preflight/collect.rs
    - darkmatter/lib/src/markdown/compose/report_diagnostics.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/compose/setters.rs
    - darkmatter/lib/src/markdown/compose/util.rs
    - darkmatter/lib/src/markdown/compose/value_origin.rs
    - darkmatter/lib/src/markdown/errors/blocks.rs
    - darkmatter/lib/src/markdown/hash/mod.rs
    - darkmatter/lib/src/markdown/hash/path_edit.rs
    - darkmatter/lib/src/markdown/hash/write.rs
    - darkmatter/lib/src/markdown/schemas/bound_operands.rs
    - darkmatter/lib/src/markdown/schemas/completion.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/yaml_documents/load.rs
    - darkmatter/lib/src/markdown/types.rs
    - darkmatter/lib/tests/l1/compose_setters.rs
    - darkmatter/lib/tests/l1/error_snapshots/markdown_error.rs
    - darkmatter/lib/tests/l1/frontmatter_path_edit.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/path_candidates.rs
    - simplified-schema/lib/src/lib.rs
    - simplified-schema/lib/src/numeric.rs
    - simplified-schema/lib/src/numeric/ingress.rs
    - simplified-schema/lib/src/property_path.rs
    - simplified-schema/lib/src/setter_value.rs
    - simplified-schema/lib/src/validate.rs
    - simplified-schema/lib/src/validation.rs
    - simplified-schema/lib/tests/l1/main.rs
    - simplified-schema/lib/tests/l1/property_paths.rs
    - simplified-schema/lib/tests/l1/setter_values.rs
documentation:
    - .claude/skills/biscuit-file/references/api.md
    - .claude/skills/darkmatter/cli.md
    - .claude/skills/darkmatter/compose.md
    - .claude/skills/darkmatter/frontmatter.md
    - .claude/skills/darkmatter/schema.md
    - .claude/skills/darkmatter/testing.md
    - .claude/skills/os/macos.md
    - .claude/skills/os/windows.md
    - .claude/skills/simplified-schema/SKILL.md
    - darkmatter/docs/cli/compose.md
    - darkmatter/docs/cli/data-contracts.md
    - darkmatter/docs/cli/get.md
    - darkmatter/docs/cli/index.md
    - darkmatter/docs/cli/rm.md
    - darkmatter/docs/cli/set.md
    - darkmatter/docs/errors/composition-problems.md
    - darkmatter/docs/topics/frontmatter.md
    - darkmatter/docs/topics/numbers.md
    - simplified-schema/README.md
    - simplified-schema/docs/topics/foundations.md
    - simplified-schema/docs/topics/property-paths.md
    - simplified-schema/docs/topics/setter-values.md
completed_phase: 10
implemented: true
---

# Implementation Log for 2026-11-06-dotted-paths (10 phases)

## Phase 1

Phase 1 changes no code. It records the contract the later phases implement and
establishes the test baseline they are measured against.

### Contract recorded from the plan

`plan.md` is the authority for the detail of each rule; this digest is for
orientation.

| Rule | Contract | Phase |
|---|---|---|
| 1 | `PropertyPath` in `simplified-schema`: `\.` and `\\` are the two escapes; the whole argument `''` is one empty segment; other empty segments are an error; printing round-trips | 2 |
| 2 | `PropertyPath::resolve` walks a value; a canonical decimal is an index only on an array; overflow is out of range; a `null` or scalar cannot be traversed; conversion to `PathStep` and `JsonPointer` needs the walk | 2 |
| 3 | Pure `set_at` and `RemovalPlan`/`remove_at` with `PathEditError`; mapping parents are created; a digit under a missing parent is an error; emptied parents stay | 2 |
| 4 | A path syntax failure is a usage error, exit 2; an edit failure or rejected value is a run failure, exit 1 | 4 |
| 5 | `SetterValue::parse`: JSON5, then YAML flow for a value starting with `{`, then text | 2 |
| 6 | Parsed containers are admitted at the token level through `json5_scan` and `numeric::admit_json5_text_numbers`; a bad container fails and never becomes text | 2 |
| 7 | `SetterValue::bind` converts through the declared `Shape`; with no declaration it keeps today's JSON5 guess | 2 |
| 8 | `hash/path_edit.rs` edits nested nodes and refuses forms it cannot edit safely | 3 |
| 9 | Every edit is reparsed and compared with the expected values before any write; `WouldChangeUnnamed` has no fallback; `UnsupportedForm` falls back to verified serialization | 3, 4 |
| 10 | `md get` stays schema-free and flat; paths are parsed before the document loads | 4 |
| 11 | `md set` blocks on a problem at or under the changed path, or on any new problem; `$schema` only checks its reference | 4 |
| 12 | `md rm` plans against the original; conflicts and missing targets cancel the command; array removals use original indexes | 4 |
| 13 | `compose::setters::classify_token` is the one setter/path classifier, shared by execution and completion | 5 |
| 14 | An ordered `SetterPlan` replaces the setter map; overlaps resolve by specificity, before any document is read | 5 |
| 15 | `apply_setter_plan` runs per document right after schemas are prepared; an empty plan costs nothing; no retry | 5 |
| 16 | Setters keep reaching transcluded children, bound per document (see below) | 5 |
| 17 | Provenance and caller origins cover only a setter's own path | 5 |
| 18 | `MarkdownError::Setter` is reported as `dm.setter.invalid` with `source: null` | 5 |
| 19 | One neutral `path_candidates` function serves completion; no baseline union over a valid effective schema | 6 |
| 20 | The CLI adapter alone reads process arguments; every failure means fewer candidates and no output | 7 |
| 21 | Custom `EnvCompleter`s only where the built-in shell scripts fail | 7 |
| 22 | Tests are tiered and hermetic; an absent shell is recorded as unproven, never as passing | 8 |

### Planning decision: setters and transcluded children (rule 16)

Setters keep reaching transcluded children, and each document binds a setter
through its own schema. The spec does not cover this case. A nested setter that
hits a scalar parent or an out-of-range index in a child fails the run and names
that child. This is listed in `decisions_for_review`.

### Correction to the spec

The spec's "recursive merge behavior of those older options" describes `--state`
only. In `prepare_frontmatter_for_compose` (`darkmatter/lib/src/markdown/compose/util.rs`),
`--state` goes through `deep_merge` (line 177), while `--set` overwrites each
top-level key with `fm.insert` (line 191). I confirmed this by reading the code
on 2026-11-06 and did not rerun the commands; the plan records the runtime
observation. Neither option changes. The spec is left as written.

### Baseline results (macOS, 2026-11-06)

`just test` in each area, before any code change:

| Area | Result | Exit |
|---|---|---|
| `biscuit-file` | 1187 passed, 0 skipped; then `test-minimal`: 6 passed, 67 filtered out by that recipe | 0 |
| `simplified-schema` | 1401 passed, 0 skipped; then a second run: 1213 passed, 0 skipped | 0 |
| `darkmatter` | 13979 passed, 10 skipped | 0 |

The 10 darkmatter skips are excluded by the recipe's selection, in both runs. I did
not enumerate them.

### Gates

The scope check does not allow skipping the gates. Its output for the phase's plan
and log edits was `packages: [darkmatter, darkmatter-cli]`, `change_class: package`:

```sh
git status --porcelain --untracked-files=all | awk '{print $NF}' \
  | xargs python3 scripts/ci/affected_scope.py
```

`just lint` and `just test` in `darkmatter/` were therefore run after the plan and
log edits. Lint exited 0. Tests exited 0 with 13979 passed and 10 skipped.

### Load-sensitive test

`darkmatter-cli::l1 compose_shell_template::every_suggested_rule_approves_the_command`
passed in all three runs but went over the 5 s L1 budget in one of them:

| Run | Duration |
|---|---|
| first full suite | 4.341 s |
| second full suite, host load about 37 | 5.522 s |
| alone, host load about 32 | 2.819 s |

No code changed in this phase, so this phase did not cause it. Other work on the
host, including Blender, accounted for part of the load. I left the test as it is.

### Test design and other checks

- **Requirement-to-test mapping:** none. This phase changes no behavior and adds no
  tests; later phases carry the mapping.
- **Input Robustness Matrix:** not applicable to this phase, since it adds no reader.
- **Cross-OS check:** not run; a Markdown-only change carries no OS risk.
- **Spec:** unchanged. This phase names no spec frontmatter to set.
- **Skill:** the `darkmatter` skill is unchanged; nothing it records has changed.

## Phase 2

Added the path type and its pure edits (rules 1 to 3) and the shared value parser
(rules 5 to 7). No command uses them yet, so `md` behaves as it did.

### What landed

| Item | Where |
|---|---|
| `PropertyPath`, `Resolved`, `ResolveStop`, `set_at`, `remove_at`, `RemovalPlan`, `PathEditError` | `simplified-schema/lib/src/property_path.rs` |
| `SetterValue`, `SetterValueError` | `simplified-schema/lib/src/setter_value.rs` |
| `numeric::admit_json5_text_numbers` | `simplified-schema/lib/src/numeric/ingress.rs` |
| `json5_scan`, `Json5Scan`, `json5_is_well_formed` | `biscuit-file/lib/src/json5/scan.rs` |

### Defects the tests found in my first version

- **Oversized integer bypassed admission.** `json_five` accepts the grammar of
  `{a:99999999999999999999}` but cannot build a value, so `Json5::from_str` fails.
  Treating that as a syntax error sent it to YAML, which read a key named
  `a:99999999999999999999`. `json5_is_well_formed` now separates the two cases.
- **`removal_order` was not a total order** (found on review, before running). Rust's
  sort may panic on an inconsistent comparator. It now orders every pair, and
  `removal_order_is_consistent_across_unrelated_targets` covers mixed targets.

### Behavior worth knowing

- In YAML flow, `{ a: NaN, max-width: 1 }` holds the string "NaN", because YAML spells
  it `.nan`. `{ a: 1e400, max-width: 1 }` holds the string "1e400", which is how
  `load_admitted_yaml` already reads frontmatter. Neither is rounded, and a declared
  `number` still rejects both. The spec's failure rule is therefore enforced for
  YAML's own non-finite spellings, not for these strings.
- `{ name:, b: 1 }` (no space after the colon) is a YAML syntax error in this parser,
  so it stays text. `{ name: , b: 1 }` and `{ b: 1, name: }` are null.

### Requirement-to-test mapping

All tests are L1, in process through each crate's public API (fidelity F1). No file,
terminal, or CLI boundary is crossed yet; the command-level wiring and shipped-artifact
tests belong to Phases 4, 5, and 8.

| Requirement | Tests |
|---|---|
| Rule 1: escapes, left-to-right pairs, literal backslash | `property_paths::dots_separate_and_the_two_escapes_decode`, `escape_pairs_match_left_to_right`, `a_backslash_before_anything_else_is_itself` |
| Rule 1: empty key, `a.`, `.a`, `a..b` | `the_whole_empty_argument_is_one_empty_key`, `an_empty_segment_in_a_longer_path_is_a_syntax_error`, `an_escaped_dot_does_not_empty_a_segment` |
| Rule 1: Unicode, whitespace, `$schema` | `unicode_and_whitespace_are_kept_unnormalized`, `a_special_first_character_is_an_ordinary_key` |
| Rule 1: round trip, canonical printing, decoded identity | `round_trip_holds_for_the_hard_spellings`, `parse_of_print_is_the_segments` (proptest), `the_argument_is_kept_as_written_and_printing_is_canonical`, `equality_and_hashing_compare_decoded_segments`, `from_segments_refuses_what_no_path_can_spell` |
| Rule 2: index versus key, `items.01`, 40-digit index | `a_number_is_an_index_on_an_array_and_a_key_on_a_mapping`, `only_a_canonical_decimal_is_an_index`, `a_huge_index_is_out_of_range_rather_than_wrapping` |
| Rule 2: scalar and null traversal, stop point, conversions | `a_present_null_or_scalar_cannot_be_traversed_but_can_be_found`, `a_walk_reports_how_far_it_got`, `the_pointer_comes_from_the_typed_walk_and_escapes_its_keys`, `a_value_root_and_a_map_root_walk_alike` |
| Rule 3: parents, `NumberUnderMissingParent`, no partial write | `set_creates_missing_mapping_parents`, `a_number_under_a_missing_parent_is_refused_even_at_the_end`, `a_refusal_deep_in_an_existing_parent_creates_nothing`, `a_digit_key_on_an_existing_mapping_is_a_key`, `a_parent_that_cannot_hold_the_key_names_its_type` |
| Rule 3: replacing, key order, arrays never append | `naming_a_null_or_scalar_replaces_it`, `replacing_keeps_the_key_position_and_new_keys_append`, `set_edits_one_existing_array_element_and_never_appends`, `a_literal_dotted_key_differs_from_nested_segments`, `the_empty_key_is_a_top_level_key` |
| Rule 3: removal, emptied parents, original indexes, conflicts | `removing_the_last_child_leaves_the_parent`, `surviving_mapping_keys_keep_their_order`, `array_removals_use_original_indexes_in_either_order`, `a_removal_inside_an_element_survives_removing_an_earlier_element`, `removal_order_is_consistent_across_unrelated_targets`, `a_target_that_does_not_exist_cancels_the_whole_removal`, `a_duplicate_or_an_ancestor_with_its_descendant_is_a_conflict`, `siblings_sharing_a_key_prefix_do_not_conflict`, `a_plan_exposes_typed_targets_in_argument_order` |
| Rule 5: forms, YAML fallback, `{ a:1, max-width: 2 }`, tags, block documents, neither | `setter_values::json5_and_yaml_flow_both_read_as_objects`, `json5_goes_first_so_a_mixed_value_reads_a_colon_key_through_yaml`, `a_quoted_string_decodes_once`, `null_is_null_and_its_lookalikes_are_not`, `a_scalar_keeps_its_spelling_until_bound`, `empty_input_is_the_empty_string`, `text_that_parses_as_neither_is_text`, `a_template_is_never_read_as_a_mapping`, `a_tagged_yaml_value_is_text_and_runs_nothing`, `an_exclamation_mark_inside_a_scalar_is_no_tag`, `an_anchor_or_alias_is_text`, `a_block_or_second_document_is_text`, `what_yaml_flow_syntax_allows_is_accepted`, `both_loaders_preserve_mapping_order` |
| Rule 6: duplicates, non-finite, inexact, underflow in both loaders | `a_duplicate_key_fails_in_both_loaders`, `an_inadmissible_number_in_a_container_fails_in_both_loaders`, `a_shadowed_number_is_still_judged`, `spellings_yaml_reads_as_strings_are_never_rounded`, `a_bare_inadmissible_number_stays_text`, `json5_number_admission_reports_where`, `json5_number_admission_accepts_every_admissible_spelling`; token walk in `biscuit_file json5::scan::tests` (11 tests) |
| Rule 7: `string`, `string \| number`, `numberlike`, `boolish`, failure keeps raw, no declaration | `text_goes_through_the_declared_type`, `a_value_the_type_rejects_stays_as_written_for_validation_to_report`, `with_no_declaration_text_is_guessed_as_today`, `a_type_declaring_no_structure_guesses_too`, `a_quoted_string_is_not_converted_by_binding`, `null_binds_to_null_whatever_the_type`, `a_number_in_a_container_has_already_lost_its_spelling` |

### Input Robustness Matrix

Default outcome: an inadmissible parsed container is a typed `SetterValueError`.
Exceptions, each tested: rejected syntax is text; empty input is `""`; `null` is kept
for the schema to judge; scalar spelling is kept until bound; a value that fails binding
is returned unconverted for validation to report.

| Shape, inside a container (both loaders) | Outcome | Test |
|---|---|---|
| control: valid text-to-number conversion | converts | `control_a_conversion_inside_a_container_converts` |
| absent field | stays absent | `an_absent_field_stays_absent` |
| explicit null, including YAML `k: ` | null, not absent | `an_explicit_null_field_stays_null_and_is_not_absent` |
| wrong type, whole field | kept for validation | `a_wrong_typed_field_keeps_the_whole_value_for_validation` |
| wrong type, one element | kept, not filtered | `one_invalid_element_is_kept_not_filtered` |
| wrong type, every element | kept, not emptied | `every_invalid_element_is_kept_not_emptied` |
| empty `""`, `[]`, `{}` | empty, not absent | `an_empty_field_is_empty_not_absent` |
| duplicate key | error | `a_duplicate_key_fails_in_both_loaders` |
| trailing or invalid content | text | `a_block_or_second_document_is_text`, `text_that_parses_as_neither_is_text` |
| format syntax: JSON5 comments and trailing commas; YAML comment, `k: `, second document | as the format defines | `json5_comments_and_trailing_commas_are_accepted`, `what_yaml_flow_syntax_allows_is_accepted`, `a_block_or_second_document_is_text` |

Code-smell grep over `setter_value.rs`, `property_path.rs`, and `json5/scan.rs`: no
`unwrap_or_default`, no `filter_map`, and three `.ok()`, all intentional:

- `setter_value.rs`, `serde_json::to_value` of an already admitted YAML number; a
  failure leaves the value as text.
- `property_path.rs`, `segment.parse().ok()`; an index too large for `usize` is out of
  range by contract.
- `json5/scan.rs`, the second parse of a key with an escape; on failure the key compares
  by its written text.

### Gates (macOS, 2026-11-06)

The scope check does not allow skipping the gates. It reported
`packages: [biscuit-file, simplified-schema, test-toolkit, darkmatter, darkmatter-cli]`,
`change_class: package`.

| Area | `just test` | `just lint` |
|---|---|---|
| `simplified-schema` | exit 0: 1479 passed, 0 skipped; second run 1291 passed, 0 skipped | exit 0 |
| `biscuit-file` | exit 0: 1198 passed, 0 skipped; `test-minimal` 6 passed, 67 filtered out by that recipe | exit 0 |
| `darkmatter` | exit 0: 13979 passed, 10 skipped by the recipe's selection, as at baseline | exit 0 |

The `simplified-schema` and `biscuit-file` gates ran before the one-line test fix below,
which is in neither package. The `darkmatter` gates ran after it, on the final source.

The 8 new doctests passed through `cargo test --doc`, filtered to the new symbols,
because `just test` does not run doctests.

### Dependent packages

This phase only adds symbols, so no existing test in a dependent exercises the change.
That made a compile check the right evidence:
`cargo check --all-targets` exited 0 with no warnings for `claudine`, `claudine-cli`,
`claudine-gen`, `messenger`, `messenger-cli`, `biscuit-file-cli`, `biscuit-terminal`,
`content-policy`, `model-citizen`, `playa-cli`, `research`, `schematic-define`,
`schematic-gen`, `sniff`, `sniff-detection-cli`, `tree-hugger`, `tree-hugger-cli`, and
`worktree`. Their test suites were not run.

`test-toolkit`, which the scope check selected, had 1 failure:
`archive_path_guard::no_archive_executed_target_bakes_in_a_producer_path`. It was not
caused by this phase. It flagged an `env!("CARGO_MANIFEST_DIR")` at
`darkmatter/cli/tests/l1/output_commands.rs:687`, a file this branch had not touched.
It reproduced when run alone. I replaced it with `biscuit_test_harness::manifest_dir!()`.
After the fix, 479 passed and 2 skipped, and darkmatter's
`every_leaf_has_a_schema_file_and_a_conformance_test` passes.

### Load-sensitive tests

Five existing tests passed but went over the 5 s L1 budget in the final darkmatter run,
with host load around 15 to 25. None of them calls code this phase added. Each is well
inside the budget alone, so I left them as they are.

| Test | Full suite | Alone |
|---|---|---|
| `compose_remote_caching::test_compose_denied_host_never_reads_a_seeded_cache_entry` | 5.902 s | 2.369 s |
| `compose_report_diagnostics::other_families_report_their_positions_and_no_reference_details` | 5.955 s | 2.301 s |
| `compose_shell_template::every_suggested_rule_approves_the_command` | 6.486 s | 2.824 s |
| `schema_triggers::dialect_family_has_identical_compose_validate_and_trace_activation` | 5.108 s | 2.057 s |
| `claudine_editor_parity::the_shared_corpus_valid_generated_verb_fixtures_raise_no_schema_diagnostic` | 5.158 s | 2.114 s |

### Cross-OS check

Not run. This phase adds string and in-memory JSON code, with no file paths, processes,
or OS-specific crates. A `\` in a path is property-key syntax here, not a separator.
Other operating systems are unexecuted and run under scheduled CI.

### Skills and spec

No skill or `docs/` page changed in this phase, since no command behaves differently
yet; Phase 9 owns both. This phase names no spec frontmatter to set.

## Phase 3

Added the nested text editor (rules 8 and 9). No command calls it yet, so `md` behaves
as it did; Phase 4 connects it.

### What landed

| Item | Where |
|---|---|
| `PathEdit`, `PathEditRefusal`, `edit_frontmatter_paths`, `edited_document_text` | `darkmatter/lib/src/markdown/hash/path_edit.rs` |
| Twelve helpers made `pub(super)`, with no behavior change | `darkmatter/lib/src/markdown/hash/write.rs` |

The editor reads node positions from `locate_frontmatter_value`, one top-level property
at a time. After each edit it reloads the text through `Markdown::try_from_content`, the
loader that produced `expected`, so lenient forms such as a bare `{{ name }}` compare
correctly. Whole-value comparison is the safety net: a wrong position becomes a refusal,
never a bad write.

### Defects the tests found in my first version

- **Fallback damaged the body.** `Markdown::as_string` dropped the final line break and
  normalized CRLF. The fallback now keeps the source's fences and body bytes.
- **A comment became content.** Growing `name: Alice # c` to a block scalar put `# c` on
  the scalar's last line. It now goes on the first line.
- **A new key cut into a `|+` scalar.** The located span stops before the blank lines that
  scalar owns. The append point now moves past them.
- **Unreachable branch.** Removing the inner item of `- - x` could never run, because the
  locator cannot read that form. I deleted it and pinned the refusal.

### Behavior worth knowing

- An indented `  ---` inside a block scalar closes the frontmatter for every reader,
  because the fence scan compares trimmed lines. This is existing behavior.
- A flow bracket closing at column 0 and a compact `- - x` are unsupported forms, so they
  fall back and lose comments.

### Requirement-to-test mapping

All tests are L1 in `darkmatter/lib/tests/l1/frontmatter_path_edit.rs` (50 tests,
`frontmatter_path_edit::`), in process through the crate's public functions, with
`expected` made by the real `set_at` and `RemovalPlan` (fidelity F1). The boundary is the
library's text-to-text contract. No file or CLI boundary is crossed yet, so persistence,
exit status, and the no-write-on-refusal behavior belong to Phase 4's CLI tests.

| Requirement | Tests |
|---|---|
| Comments inside a shared parent; sibling scalar spelling | `a_nested_edit_keeps_comments_and_spelling_in_the_shared_parent`, `a_replaced_scalar_keeps_the_comment_after_it`, control `control_an_unedited_document_is_returned_unchanged` |
| Appending to a block mapping at its indent; created parents | `a_new_key_appends_at_its_parents_indent`, `missing_parents_are_created_inside_the_nearest_block_mapping`, `a_new_top_level_key_goes_at_the_end_of_the_frontmatter`, `an_empty_block_or_absent_frontmatter_gains_the_key`, `an_empty_flow_mapping_takes_a_new_key_as_a_block` |
| Replacing in a block sequence | `an_item_in_a_block_sequence_is_replaced_in_place`, `a_key_inside_a_sequence_item_is_replaced_and_appended_at_the_items_indent`, `a_whole_item_can_become_a_mapping_or_a_scalar`, `an_item_with_no_value_or_on_its_own_line_can_be_replaced`, `a_sequence_nested_in_a_sequence_is_edited_on_its_own_lines_only` |
| Replacing across shapes and spellings | `a_scalar_becomes_a_container_and_keeps_its_comment`, `a_container_becomes_a_scalar_and_keeps_the_key_lines_comment`, `a_replaced_container_keeps_its_childrens_column`, `an_empty_value_is_filled_in_after_its_colon`, `multi_line_scalars_are_replaced_whole`, `a_multi_line_value_is_written_inside_its_parent`, `a_value_that_would_read_as_another_type_stays_a_string`, `a_comment_stays_on_the_first_line_of_a_value_that_grows`, `a_flow_collection_named_whole_is_replaced_in_place`, `replacing_an_alias_itself_changes_only_that_value` |
| Removal, including the last child and original indexes | `removing_a_node_takes_its_own_lines_and_leaves_its_neighbors`, `removing_the_last_child_leaves_an_empty_parent`, `removing_the_only_top_level_key_leaves_an_empty_block`, `removals_use_original_indexes_in_either_order`, `removing_a_key_that_shares_a_line_with_a_dash_keeps_the_dash`, `an_item_whose_dash_is_on_its_own_line_is_removed_with_it` |
| LF and CRLF (LF throughout) | `a_crlf_document_keeps_its_line_endings`, `a_crlf_document_with_no_frontmatter_gets_a_crlf_block`, `the_fallback_keeps_the_fences_and_body_bytes_of_a_crlf_document`, `a_lone_cr_document_is_edited_exactly_or_refused` |
| Flow collection falling back | `walking_into_a_flow_collection_is_refused_and_falls_back` |
| Anchor whose edit changes an alias: refused, no fallback | `an_edit_an_alias_reads_is_refused_with_no_fallback`, `removing_an_anchor_an_alias_needs_is_refused_with_no_fallback`, `an_anchor_redeclared_earlier_cannot_silently_redirect_an_alias`, `an_expected_value_the_text_does_not_hold_is_refused`; negative control `an_anchored_node_with_no_alias_reading_the_change_can_be_edited` |
| Other refused forms | `an_indented_root_is_refused_and_serialization_writes_it`, `walking_through_an_alias_or_tag_is_refused_and_serialization_keeps_the_values`, `frontmatter_only_the_lenient_loader_reads_falls_back` |
| Source forms that could mislead the locator | `a_bare_template_beside_the_edit_is_kept`, `keys_that_need_quoting_are_found_and_written_as_keys`, `unusual_key_spellings_are_matched_by_their_decoded_text`, `a_merge_key_is_an_ordinary_key_beside_the_edit`, `text_that_only_looks_like_structure_is_not_mistaken_for_it`, `a_scalar_keeping_its_trailing_blank_lines_is_not_cut_short` |
| Read/write/read round trip | `only_the_named_node_changes_across_a_read_write_read_round_trip`, `a_document_built_in_memory_is_serialized` |

### Input Robustness Matrix

Not applicable: this phase adds a writer, not a reader of a file format. It does reread its
own output, through the existing document loader. Code-smell grep over `path_edit.rs`: no
`filter_map` or `#[serde(default)]`. Two intentional exceptions:

- `split_whitespace().next().unwrap_or_default()` in `keeps_trailing_lines`: empty text has no
  block header.
- `item_dash(...).ok()` in `first_child_column`: with no dash, the replacement uses the
  default indent, and verification still judges the result.

### Gates (macOS, 2026-11-06)

The scope check does not allow skipping the gates. It reported
`packages: [biscuit-file, darkmatter, darkmatter-cli, simplified-schema, test-toolkit]`,
`change_class: package`. The uncommitted tree still holds Phase 2's files, which is why
`biscuit-file`, `simplified-schema`, and `test-toolkit` appear; this phase changed none of
them, so their gates were not rerun.

| Gate | Result |
|---|---|
| `just lint` in `darkmatter/` | exit 0 |
| `just test` in `darkmatter/` | exit 0: 14029 passed, 10 skipped by the recipe's selection; 13979 at baseline plus these 50 |
| `just check-tier-coverage darkmatter` (from the root) | exit 0, 0 stranded |

`just lint` and `just test` ran before the plan, log, and skill text were finished, and no
source file changed after them.

### Dependent packages

This phase only adds symbols and narrows no visibility, so a compile check was the right
evidence: `cargo check --all-targets` exited 0 with no warnings for `biscuit-icon-cli`,
`biscuit-speaks-cli`, `biscuit-terminal-cli`, `claudine`, `claudine-cli`, `claudine-gen`,
`messenger`, `messenger-cli`, `playa-cli`, `research`, and `sniff-detection-cli`. Their test
suites were not run. `darkmatter-cli` and `darkmatter-lsp` ran in full under `just test`.

### Load-sensitive tests

Four existing tests passed but went over the 5 s L1 budget in the full run. None of them
calls the new code, and I left them as they are.

| Test | Full suite | Alone |
|---|---|---|
| `compose_report_diagnostics::other_families_report_their_positions_and_no_reference_details` | 5.035 s | 2.291 s |
| `compose_shell_template::every_suggested_rule_approves_the_command` | 7.607 s | 2.830 s |
| `claudine_editor_parity::the_shared_corpus_valid_depth_fixtures_raise_no_schema_diagnostic` | 5.456 s | 2.381 s |
| `content_policy_baseline::every_shared_declaration_publishes_the_baseline_verdict_at_its_anchor` | 5.331 s | 2.161 s |

### Cross-OS check

Not run. The code is string editing with no file paths, processes, or OS-specific crates, and
CRLF and lone-CR text are covered by tests on every OS. Other operating systems are unexecuted
and run under scheduled CI.

### Skills, docs, and spec

Updated `.claude/skills/darkmatter/frontmatter.md`, because it told agents not to add a YAML
writer and now has to name this one. No `docs/` page changed, since no command behaves
differently yet; Phase 9 owns those. This phase names no spec frontmatter to set.

## Phase 4

Connected `md get`, `md set`, and `md rm` to property paths, the shared value parser, and
the nested text editor (rules 4 and 9 to 12).

### What landed

| Item | Where |
|---|---|
| `run_get`, `run_set`, `run_rm` on `PropertyPath`; `set_checked`, `blocking_problems`, `set_schema_key`; `edited_text` removed in favor of `edited_document_text` | `darkmatter/cli/src/commands/frontmatter.rs` |
| `declared_shape` | `darkmatter/lib/src/markdown/schemas/completion.rs` |
| `frontmatter_as_json` made public | `darkmatter/lib/src/markdown/schemas/mod.rs` |
| `CommandRun::add_diagnostics` | `darkmatter/cli/src/output/command.rs` |
| `request_schemas` | `darkmatter/cli/src/commands/schema/validate.rs` |
| Unquoted `true`/`false` under `boolish` | `simplified-schema/lib/src/setter_value.rs` |

`restore_properties_text` no longer has a caller in these commands. It stays because it is
a public library function that was out of this phase's scope.

### Defect the tests found in my first version

`md set doc.md draft true` wrote `draft: 'true'`, because the baseline declares `draft:
boolish`. `test_set_boolean_value`, an existing test, caught it. It is fixed in `bind` and
listed in `decisions_for_review`.

### Behavior worth knowing

- `3` under `string | number` is written as the string `'3'`, as the spec's union rule
  requires. Baseline keys with that type, such as some `style` values, do this.
- A failed `md set` reports `dm.schema.*` codes with a null line, so a command-line value is
  never attributed to the old value's position.
- `md set` now prepares a schema on every run, taking about 0.25 s per test run in a debug
  build against 0.04 s for `md get`. This is an observation from test timings, not a
  benchmark.

### Requirement-to-test mapping

All new tests are L1 and spawn the real `md` through `CliProcessFixture` with fixture-owned
documents and schemas (fidelity F2). The boundary is the CLI's exit status, output envelope,
and file bytes. Tests are in `darkmatter/cli/tests/l1/get_set_rm.rs` under the module named
in each row, unless another file is named.

| Requirement | Tests |
|---|---|
| Rule 4 and rule 10: a malformed path exits 2 with no read | `malformed_paths::each_command_fails_as_usage_before_reading_anything`, `one_malformed_path_among_good_ones_stops_get_and_rm`, `stdin_is_not_consumed_for_a_malformed_path` |
| Rule 10: nested reads, top-level unchanged | `get_paths::a_nested_value_is_read_and_an_ordinary_key_is_unchanged`; existing `test_get_*` |
| Grammar: dots, `\.`, `\\`, `C:\dir`, `''`, `$schema`, Unicode | `get_paths::a_dot_always_separates_and_an_escaped_dot_names_a_literal_key`, `backslashes_follow_the_two_escapes`, `the_empty_argument_special_keys_and_unicode_are_ordinary_keys` |
| Grammar: index versus key, `items.01`, signs, `-`, huge index, scalar and null traversal | `get_paths::a_number_is_an_index_on_an_array_and_a_key_on_a_mapping`, `an_unreachable_path_is_missing_and_prints_empty`; `set_paths::a_number_on_a_mapping_is_a_key` |
| Out-of-range rows: `get`, `set`, `rm` | `get_paths::an_unreachable_path_is_missing_and_prints_empty`, `set_paths::an_array_element_is_edited_and_an_index_out_of_range_names_the_length`, `rm_paths::an_index_out_of_range_or_a_scalar_parent_is_an_error` |
| Explicit null; flat multi-path `get` | `get_paths::an_explicit_null_stays_null_in_data_output`, `several_paths_give_a_flat_object_keyed_as_written`; `output_commands::conforms_get_nested_paths_stay_flat` |
| `a.b` never literal; `a\.b` is, through `get` and `rm` | `set_paths::a_dotted_path_never_writes_a_literal_key_and_an_escaped_one_does` |
| Parents table | `set_paths::a_missing_parent_is_created`, `a_number_under_a_missing_parent_is_refused`, `a_parent_that_cannot_hold_the_key_is_named_with_its_type`; `rm_paths::removing_the_last_child_leaves_the_parent` |
| Replacing a whole value | `set_paths::a_whole_scalar_or_null_can_be_replaced_by_naming_it`, `the_empty_top_level_key_can_be_set` |
| Rule 9: comments, read/write/read, flow fallback, alias refusal, stdin | `set_paths::a_nested_edit_keeps_comments_and_spelling_in_the_shared_parent`, `repeated_edits_round_trip_through_the_file`, `a_flow_collection_falls_back_and_still_edits_the_value`, `an_edit_an_alias_reads_is_refused_with_no_fallback`, `stdin_without_save_prints_the_edited_document`; `rm_paths::removing_an_anchor_an_alias_needs_is_refused` |
| Rule 5 and 7: `string`, `number`, `numberlike`, `boolish`, `string \| number`, array elements, undeclared guess, `{a:1}`, YAML flow, quoted, `null`, empty | `set_values::a_declared_type_reads_the_text`, `numberlike_boolish_and_unions_follow_the_shared_rules`, `an_array_element_reads_through_the_element_type`, `an_undeclared_key_keeps_the_guess`, `containers_read_the_same_in_json5_and_yaml_flow`, `a_quoted_string_decodes_once_and_null_is_null`; `boolish` rows in `setter_values::text_goes_through_the_declared_type` (F1) |
| Rule 6: admission failures, with no write | `set_values::an_inadmissible_container_fails_and_never_becomes_text` |
| Pending values stay unevaluated | `set_values::a_pending_value_is_written_unevaluated` |
| Rule 11, the three rows of the spec's table | `set_validation::a_valid_value_writes_despite_a_problem_elsewhere`, `a_value_the_type_rejects_writes_nothing`, `setting_the_missing_key_repairs_the_document` |
| Rule 11: broken schema, separate file, `$schema`, ancestors, closed versus open, union narrowing, triggers, skipped-trigger warning | `set_validation::a_broken_schema_fails_with_an_explanation_and_no_write`, `a_schema_in_a_separate_file_applies`, `setting_schema_checks_only_the_reference`, `a_bare_schema_name_stays_a_string_and_an_inline_schema_is_read`, `a_required_child_of_a_new_or_replaced_object_blocks`, `a_nested_key_a_closed_object_does_not_declare_is_refused`, `a_pattern_key_opens_the_object`, `a_literal_tag_narrows_the_declared_type`, `a_trigger_selected_by_the_edit_reads_the_value_and_a_broken_trigger_is_only_a_warning` |
| Rule 12: conflicts, original indexes, missing target, `removed` order, `remaining`, required key removal | `rm_paths::a_duplicate_or_an_ancestor_with_its_descendant_cancels_the_command`, `array_removals_use_original_indexes_in_either_order`, `a_missing_target_cancels_the_whole_command`, `removing_the_last_child_leaves_the_parent`, `the_empty_key_and_a_required_key_can_be_removed`, `removing_a_nested_key_keeps_its_siblings_and_their_comments` |
| Source bytes unchanged after a failure | `assert_refused` compares the file's bytes in every refusal above |
| Output contracts, no schema change | `output_commands::conforms_get_*`, `conforms_set_*`, `conforms_rm_*`, including new `conforms_set_nested_path_is_written_as_given`, `conforms_set_rejected_value_has_no_document`, `conforms_rm_nested_path_keeps_top_level_remaining`, `conforms_rm_conflict_fails`; `set_values::a_failed_value_reports_through_the_data_output_with_no_document`, `set_paths::the_result_carries_the_path_as_written_and_the_value_as_bound` |

### Input Robustness Matrix

Phase 2's parser and its matrix are unchanged, so this phase only wires them; the CLI proves
that with a control value and invalid shapes (`set_values` above). Code-smell grep over
`frontmatter.rs` and `declared_shape`: no `unwrap_or_default` or `#[serde(default)]`. Two
intentional exceptions:

- `.ok()` in `declared_shape`: a declaration with no conversion shape reads as undeclared, and
  validation still judges the value.
- `filter_map` in `element_shape`: a union arm that is not a collection has no element, so it
  is not a candidate.

The `.ok()` in `resolved_input` is existing code.

### Gates (macOS, 2026-11-06)

The scope check does not allow skipping the gates. It reported
`packages: [darkmatter, darkmatter-cli, simplified-schema, test-toolkit]`,
`change_class: package`.

| Gate | Result |
|---|---|
| `just lint` in `darkmatter/` | exit 0, rerun after the last source edit |
| `just test` in `darkmatter/` | exit 0: 14088 passed, 10 skipped by the recipe's selection; none at or above 5 s |
| `just check-tier-coverage darkmatter` | exit 0 |
| `just lint` in `simplified-schema/` | exit 0 |
| `just test` in `simplified-schema/` | 1479 passed, 0 skipped; second run 1291 passed, 0 skipped |

After the full `darkmatter` run, the last edits were `--help` text and one test rename. I reran
lint and `get_set_rm::` plus the help tests for those: 127 passed.

### Dependent packages

`darkmatter-cli` and `darkmatter-lsp` ran in full under `just test`. `cargo check --all-targets`
exited 0 for `claudine` and `claudine-cli`, whose code uses only additive changes here; their
test suites were not run.

`test-toolkit`, which the scope check selected: `just test test-toolkit` exited 0 with 477 passed
and 4 skipped, and two existing tests ran past 5 s (`archive_path_guard::no_archive_executed_target_bakes_in_a_producer_path`
6.278 s, `kache_host_contracts::report_resolves_the_worktree_base_by_the_wt_contract` 5.724 s). Neither calls this
phase's code, and I left them as they are.

### Cross-OS check

Not run. The change adds no OS-specific code: a `\` here is key syntax, and file opening uses the
existing `open_argument` and `canonicalize_simplified`. Other operating systems are unexecuted
and run under scheduled CI.

### Skills, docs, and spec

Updated `.claude/skills/darkmatter/frontmatter.md`. `darkmatter/docs/cli/get.md`, `set.md`, and
`rm.md` still describe top-level keys, because the plan gives them to Phase 9. This phase names no
spec frontmatter to set, and the spec is unchanged.

## Phase 5

Replaced `md compose`'s early setter map with an ordered plan, bound per document through
that document's schema (rules 13 to 18).

### What landed

| Item | Where |
|---|---|
| `classify_token`, `RawSetter`, `SetterPlan`, `apply_setter_plan`, `apply_setter_plan_for_preflight`, `SetterBindings` | new `darkmatter/lib/src/markdown/compose/setters.rs` |
| `ComposeOptions::with_setters`, `setters` and `setter_bindings` fields, setters in both identity encoders | `compose/context/options.rs` |
| Plan applied after `prepare_schemas`, before caller-file projection | `compose/pipeline/mod.rs`, and both walks in `compose/preflight/collect.rs` |
| `DataPaths::clear_path`, `FrontmatterProvenance::set_authored_source` | `compose/value_origin.rs` |
| Nested file setters projected from the launch directory | `compose/schema_validation.rs` |
| `MarkdownError::Setter`, `dm.setter.invalid` with `source: null`, status block | `types.rs`, `compose/report_diagnostics.rs`, `errors/blocks.rs` |
| `parse_compose_setter`, `parse_shorthand_value`, and the map parameter removed | `cli/src/commands/compose.rs`, `cli/src/commands/mod.rs` |
| `ValidationProblem::offending_properties` | `simplified-schema/lib/src/validation.rs`, `validate.rs` |

### Defects the tests found

- A schema problem on a setter's value pointed at the old on-disk scalar. It now names the
  setter and has a null position.
- `dm.setter.invalid` picked up a file from `with_document_fallback`. That code is now exempt.
- `md set` wrote a second undeclared key into a closed object, from Phase 4: the validator
  reports an object's undeclared keys as one problem, and only the first was visible. Fixed
  in `blocking_problems`, with `a_second_undeclared_key_is_refused_though_the_object_already_has_one`.

### Requirement-to-test mapping

CLI tests are L1 and spawn the real `md` through `CliProcessFixture` with fixture-owned
documents, schemas, and repositories (fidelity F2). The boundary is the exit status, the
output envelope, and the composed text. They are in `darkmatter/cli/tests/l1/compose_setters.rs`
under the module named in each row. A successful `md compose` on a file crosses the reference
check, the pre-flight walk, and the pipeline.

| Requirement | Tests |
|---|---|
| Rule 13: classification | `setters::tests::a_word_that_fits_the_grammar_is_a_setter`, `a_word_that_does_not_fit_is_a_path` (includes `.\dir\.config=1.md`), `an_empty_key_is_its_own_class` (F1); `classifying::a_dotted_key_is_a_setter_and_dot_slash_makes_it_a_path`, `two_paths_are_still_refused`, `an_empty_key_is_a_setter_error` |
| Rule 14: overlaps | `setters::tests::ancestors_come_first_and_independent_paths_keep_their_order`, `the_last_value_of_a_path_wins_and_decoded_paths_are_compared`, `an_overridden_value_is_never_read`, `a_scalar_or_null_ancestor_fails_in_both_orders`, `a_replaced_scalar_ancestor_does_not_block_the_value_that_won` (F1); `precedence::whole_object_plus_descendant_is_the_same_in_both_orders`, `a_scalar_ancestor_fails_in_both_orders_and_composes_nothing`, `a_scalar_ancestor_is_found_before_the_document_is_read`, `a_repeated_path_takes_the_last_value`, `an_overridden_value_needs_only_valid_syntax` |
| `author.name=Alice` keeps siblings, reaches expressions; undotted as before; no write | `applying::a_nested_setter_keeps_its_siblings_and_reaches_expressions`, `a_setter_may_come_before_the_document`, `a_missing_parent_is_created_and_shows_in_the_composed_frontmatter`, `an_undotted_setter_behaves_as_before`, `a_setter_applies_to_stdin_input`, `an_array_element_is_overridden_for_this_run`, `the_file_on_disk_is_not_changed`, `an_escaped_dot_names_a_literal_key` |
| Precedence over `--set`, `--state`; data overrides win | `precedence::a_setter_outranks_set_and_state`, `a_whole_object_setter_outranks_a_more_specific_set_value`, `a_dotted_set_key_stays_a_literal_key`; `darkmatter/lib/tests/l1/compose_setters.rs`: `a_setter_applies_after_set_overrides`, `a_data_override_keeps_the_key_it_names`, `an_empty_plan_changes_nothing` |
| `items.9=x`, parents, admission | `failing::an_index_out_of_range_names_the_length_and_has_no_source`, `bounds_are_checked_against_an_ancestor_setter`, `a_scalar_parent_and_a_number_under_a_missing_parent_fail`, `an_inadmissible_container_never_becomes_text`, `a_failure_prints_to_stderr_in_markdown_mode` |
| Rule 7 through compose: `string`, `number`, `numberlike`, `boolish`, unions, undeclared, quoted, containers, nested type, null, wrong value | `values::a_declared_type_reads_the_text`, `an_undeclared_key_keeps_the_guess`, `a_quoted_string_decodes_once_and_containers_read_alike`, `a_nested_value_reads_through_its_declared_type`, `null_is_accepted_only_where_the_type_permits`, `a_wrong_value_fails_schema_validation_and_is_not_placed_on_disk` |
| Closed versus open object, required child | `values::a_closed_object_refuses_an_undeclared_child_and_a_pattern_opens_it`, `an_undeclared_child_beside_an_authored_one_is_still_given_to_its_setter`, `a_required_child_a_more_specific_setter_supplies_is_satisfied` |
| Rule 15: schema selection | `selecting_a_schema::a_setter_selects_the_trigger_that_reads_another_in_either_order`, `a_set_that_replaces_the_schema_selects_the_same_way`, `a_trigger_on_a_converted_type_fails_clearly_with_no_retry` |
| Rule 16: children | `reaching_children::a_child_reads_a_setter_through_its_own_schema`, `a_nested_setter_keeps_a_childs_siblings`, `a_path_a_child_cannot_hold_fails_the_run_and_names_the_child`, `a_setter_chooses_a_transclusion_target` (reference check) |
| Rule 17: origins and provenance | `origins::an_authored_file_value_beside_a_setter_keeps_its_own_origin`, `a_nested_file_setter_resolves_from_the_launch_directory`, `a_nested_file_setter_does_not_fall_back_to_the_documents_directory`, `a_top_level_file_setter_keeps_resolving_from_the_launch_directory`; `value_origin::tests::clearing_a_path_keeps_the_values_beside_it`, `clearing_a_path_drops_marks_below_it_and_leaves_others` (F1) |
| Pending values, failure naming the setter, pre-flight | `pending_values::a_template_in_a_setter_is_authored_at_any_depth`, `a_template_under_a_number_stays_pending_and_binds_after`, `a_failing_template_names_its_setter`, `the_shell_report_sees_a_command_a_setter_supplies`, `the_shell_report_fails_on_a_setter_the_document_cannot_hold` |
| Rule 18: `dm.setter.invalid`, envelope | `assert_setter_failure` on every failure above; `a_data_output_reports_success_with_the_setter_applied` |
| Cache identity | `options::tests::setters_participate_in_graph_and_cache_identity` (F1) |
| Unknown-key naming | `validate::tests::every_unknown_key_of_an_object_is_named` (F1); `get_set_rm::set_validation::a_second_undeclared_key_is_refused_though_the_object_already_has_one` |

The checkpoint holds: `compose_state_set`, the composition-errors tests, and every other existing
test pass, and none needed a changed expectation. The only edits to existing tests moved the
setter parser's unit tests and added the new field to struct literals.

### Input Robustness Matrix

Phase 2's parser and its matrix are unchanged; this phase wires them, shown by a control value
and invalid shapes (`failing::an_inadmissible_container_never_becomes_text`). Code-smell grep over
`setters.rs` and the new `schema_validation.rs` code: no `unwrap_or_default`, `.ok()`, or
`#[serde(default)]`. The `filter_map` calls read schema shape or a value just written; none is on
a load-bearing parse.

### Gates (macOS, 2026-11-06)

The scope check does not allow skipping the gates. It reported
`packages: [darkmatter, darkmatter-cli, simplified-schema, test-toolkit]`, `change_class: package`.

| Gate | Result |
|---|---|
| `just lint` in `darkmatter/` | passed, including the Zed WASM check |
| `just test` in `darkmatter/` | exit 0: 14151 passed, 10 skipped by the recipe's selection; none slow |
| `just check-tier-coverage darkmatter` | exit 0 |
| `just lint` in `simplified-schema/` | passed |
| `just test` in `simplified-schema/` | exit 0: 1480 passed; second run 1292 passed; 0 skipped |

No source changed after these runs; only the skill, this log, and the plan did.

### Dependent packages

- `claudine` and `claudine-cli`: `cargo check --all-targets` passed. `claudine` has over 200 tests,
  so I ran those touching overrides and caller records: `test(caller) | test(set_override) |
  test(override)`, 99 passed.
- `messenger` reads `offending_property`: `test(research)`, 1 passed.
- `test-toolkit`, which the scope check selected: `just test test-toolkit`, 477 passed, 4 skipped.
  `archive_path_guard::no_archive_executed_target_bakes_in_a_producer_path` took 5.295 s. It was
  slow in Phase 4 too, does not call this phase's code, and is left as it is.

### Cross-OS check

Not run. The change adds no OS-specific code: `\` in a setter key is key syntax on every OS, and
file values go through the existing `FileReference` projection. Other operating systems are
unexecuted and run under scheduled CI.

### Docs, skills, and spec

Rule 18 added `dm.setter.invalid` to `docs/cli/data-contracts.md` and
`docs/errors/composition-problems.md`. `docs/cli/compose.md` belongs to Phase 9. Updated
`.claude/skills/darkmatter/compose.md`. This phase names no spec frontmatter to set, and the spec
is unchanged.

## Phase 6

Added the one neutral candidate function that Tab will read (rule 19). Nothing calls it from
the CLI until Phase 7, so this phase changes no observable behavior of `md`.

### What landed

| Item | Where |
|---|---|
| `path_candidates`, `path_value_takes_files`, `PathQuery`, `PathCommand`, `PathPosition`, `PathCandidate`, `PathCandidateKind` | `darkmatter/lib/src/markdown/schemas/completion.rs` |
| In-process tests of the public API | new `darkmatter/lib/tests/l1/path_candidates.rs`, declared in `tests/l1/main.rs` |

The function is passive: it reads only its query, so it opens no file, evaluates nothing, and
adds no ambient read for the context-construction and path-lookup guards to judge.

### Findings

- `effective_for` returns `Ok(None)` for a document with no `$schema` or trigger, so `effective:
  None` is the ordinary case for such a document, not only a failure. The baseline answers
  through `effective_property_shape`, whose base shape is definition-free.
- The slot grammar from `2026-11-05-dynamic-schema-types` is not built: `enum({{ list() }})` fails
  to resolve today, so a slot reaches completion as `None` and offers no value.
  `a_dynamic_slot_offers_nothing` asks the same question once the schema resolves.
- `for_path` reads only the document's layer. See `decisions_for_review`.

### Requirement-to-test mapping

All are L1, in-process through the public `path_candidates` (fidelity F1): the boundary is the
crate's public result, and CLI wiring is Phase 7's and Phase 8's. Modules are under
`darkmatter/lib/tests/l1/path_candidates.rs`.

| Spec requirement | Tests |
|---|---|
| Keys per command and their order | `keys_per_command::compose_offers_schema_keys_with_required_first_then_keys_present`, `set_offers_schema_keys_with_required_first_then_keys_present`, `get_offers_keys_present_in_document_order_then_absent_schema_keys`, `rm_offers_only_keys_present_in_document_order`, `a_document_schema_still_sees_the_baseline`, `rm_ignores_declared_children_that_are_absent` |
| Spelling: object, union with object arm | `spelling::an_object_in_a_setter_command_descends_and_assigns`, `an_existing_object_selects_and_descends_in_a_path_command`, `an_absent_object_only_descends_in_a_path_command`, `a_union_with_an_object_arm_descends_and_keeps_the_whole_value` |
| Spelling: anything else, in `key=value` | `spelling::a_scalar_in_a_setter_command_assigns` |
| Spelling: array | `spelling::an_array_descends_through_its_indexes_and_assigns_whole`, `a_container_item_descends_and_a_scalar_item_does_not`, `an_empty_or_absent_array_has_no_indexes_to_descend` |
| Spelling: open object, pattern key | `spelling::an_open_object_invents_no_keys`, `a_pattern_key_is_never_offered` |
| Help text | `spelling::a_description_is_the_help_text_and_only_keys_have_it` |
| Path escaping, grammar per command | `spelling::a_literal_dot_is_escaped_and_found_through_its_escape`, `a_backslash_in_a_key_round_trips`, `the_typed_parent_is_kept_as_written`, `only_the_receiving_grammar_is_offered`, `a_word_that_is_not_a_setter_offers_nothing_after_its_equals`, `a_scalar_or_null_cannot_be_descended`, `a_path_that_could_only_fail_offers_nothing` |
| Deduplication by decoded path, earlier setters | `deduplication::a_declared_and_present_key_appears_once_with_its_description`, `an_earlier_setter_is_not_offered_again_by_decoded_path`, `a_parent_whose_child_was_set_stays_available`, `a_whole_key_given_earlier_goes_but_its_children_stay` |
| Values: enum, literal, suggest, boolean, union, open types | `values::enum_members_arrive_through_a_setter_and_through_set`, `a_value_is_marked_as_finished`, `a_literal_offers_its_value`, `suggestions_are_offered`, `a_boolean_offers_both`, `a_union_offers_each_arm_once_in_order`, `open_types_offer_nothing`, `a_value_below_an_unreachable_path_offers_nothing`, `a_nested_array_element_reads_its_own_type` |
| Values: file | `values::a_file_type_asks_the_caller_for_files` |
| Values with spaces, quotes, `$`, backslashes, `=`, read back through `SetterValue::parse` and `bind` | `value_spelling::a_space_stays_plain`, `a_dollar_stays_plain`, `a_backslash_stays_plain`, `an_equals_stays_plain`, `an_inner_quote_stays_plain`, `a_fully_quoted_member_is_quoted_again`, `words_the_parser_would_read_as_another_type_are_quoted`, `a_string_that_looks_numeric_binds_through_its_declared_type` |
| Literal tag narrowing, including through an overlay | `narrowing::a_literal_tag_narrows_keys_to_its_arm`, `without_a_tag_every_arm_contributes`, `a_setter_applied_to_the_overlay_narrows_the_same_way`, `a_tag_narrows_a_siblings_values`, `the_tag_being_replaced_does_not_narrow_itself`, `a_root_union_narrows_too` |
| Replaced baseline property, never unioned | `schema_sources::a_replaced_baseline_property_loses_the_baseline_declaration` |
| Separate `$schema` file, recursive types | `schema_sources::a_separate_schema_file_supplies_keys_values_and_recursive_types` |
| Baseline fallback, document keys supplement | `schema_sources::with_no_effective_schema_the_baseline_answers`, `with_no_effective_schema_document_keys_still_supplement`, `value_spelling::with_no_effective_schema_the_baseline_answers_for_values` |
| Dynamic slot, undeclared child | `schema_sources::a_dynamic_slot_offers_nothing`, `a_child_the_schema_does_not_declare_offers_no_values` |

Not covered here, by the plan's split: shell word shapes, the overlay, no-evaluation and no-read
proofs, and the real `md` path. These are Phases 7 and 8. A missing-definition reference cannot be
built through the public schema API; `alternatives_for_def_in` already documents and tests it.

### Input Robustness Matrix

Not applicable: this phase adds no reader of a file format or configuration. The value parser is
unchanged, and the spelling tests only call it. Code-smell grep over the new code: the
`unwrap_or_else` in `intended_value` keeps a label as text when it is not JSON, which only ever
reads schema-authored scalars.

### Gates (macOS, 2026-11-06)

The scope check does not allow skipping the gates. It reported
`packages: [darkmatter, test-toolkit, darkmatter-cli]`, `change_class: package`.

| Gate | Result |
|---|---|
| `just lint` in `darkmatter/` | passed, including the Zed WASM check |
| `just test` in `darkmatter/` | exit 0: 14210 passed, 10 skipped by the recipe's selection; none slow |
| `just check-tier-coverage darkmatter` (repo root) | exit 0; stranded 0 |

No source changed after these runs; only the skill, this log, and the plan did.

### Dependent packages

- `claudine` and `claudine-cli`: `cargo check --all-targets` passed. The change only adds to the API,
  so no `claudine` test exercises it.
- `test-toolkit`, which the scope check selected: `just test test-toolkit`, 477 passed, 4 skipped,
  with 4 slow tests in `archive_path_guard` and the `kache_*` contracts, the largest 8.174 s. None
  calls this phase's code.

### Cross-OS check

Not run. The code has no OS-specific behavior: it reads no paths, and `\` is key syntax on every OS.
Other operating systems are unexecuted and run under scheduled CI.

### Docs, skills, and spec

Nothing a user can see changed, so `docs/` waits for Phase 9. Added a note to
`.claude/skills/darkmatter/schema.md`. This phase names no spec frontmatter to set, and the spec is
unchanged.

## Phase 7

Wired the candidates to the shell: `md compose`, `get`, `set`, and `rm` now complete property
paths and values from the document on the line (rules 20 and 21).

### What landed

| Item | Where |
|---|---|
| Value completers, reading the line through the command's own clap declarations | new `darkmatter/cli/src/args/path_completion.rs` |
| `recover_words`, the bash and zsh adapters, `SHELLS` | new `darkmatter/cli/src/completion_shells.rs` |
| `completion_overlay`, plus `settle_overlaps`, `plan_setter`, and `apply_guess` split out of `SetterPlan::new` and `apply_setter_plan` | `darkmatter/lib/src/markdown/compose/setters.rs` |
| `document_schemas`, the assembly `md set` used, now shared | `darkmatter/cli/src/commands/schema/validate.rs`, called from `commands/frontmatter.rs` |
| `complete_any_files`, ordinary files for a `file` value | `darkmatter/cli/src/args/completion.rs` |
| `completion_request`, skipping a `--magic-root` that does not exist | `darkmatter/cli/src/request.rs` |

`CommandLine::read` walks the words through `Cli::command()`, so a flag's value (`--output json`)
is never taken for the input document. The earlier `SetterPlan` and `apply_setter_plan` behavior
is unchanged: the same helpers run, and the existing `setters` tests and `compose_setters` suites
pass untouched.

### Findings

- On bash 5, `x=<Tab>` gives `COMP_WORDS` ending `x`, `=`, with `COMP_CWORD` on `=` and `$2` empty.
  The built-in script's `words[COMP_CWORD]="$2"` erases the `=`. Bash 3.2 keeps `x=ce` as one word
  and still replaces only what follows the `=`. `$2` keeps backslashes but drops an opening quote.
  All observed by pressing Tab in `/bin/bash` 3.2.57 and Homebrew bash 5.3.20.
- `CompleteEnv` removes the `COMPLETE` variable before any completer runs, so a completer cannot
  read which shell is active. The adapter records it in `WORD_FORM` before calling the engine.
- A schema file that fails to parse gives no diagnostic and leaves the document's keys and the
  baseline, as the spec requires. The first probe fixture did this by accident.

### Per-shell results (real Tab, macOS, 2026-11-06)

Run by hand in a private detached tmux server with the debug build, with a marker typed after Tab
to reveal any space. This is not yet an automated test; the L2 version is Phase 8's.

| Typed, then Tab | bash 3.2.57 | bash 5.3.20 | zsh | fish |
|---|---|---|---|---|
| `md compose doc.md style.page.al` | `...alignment=`, no space | same | same | same |
| `md compose doc.md style.bl` (Tab twice) | `style.block-quote`, no space | same | same | same |
| `md compose doc.md style.page.alignment=ce` | `...=center `, one space | same | same | same |
| `md get doc.md author.` | `author.name ` | same | same | same |
| `md set doc.md style.page.alignment ce` | `center ` | same | same | same |
| `md compose schemaed.md mode=two` | `mode=two\ words ` | same | same | same |
| `md compose schemaed.md inc=note` | `inc=notes.txt ` | same | same | same |
| `md compose schemaed.md mode="pl` | `mode="plain"` | `mode="plain" ` | `mode=plain ` | **nothing offered** |

Elvish and PowerShell are not installed on this host: unproven beyond their word shape, and
PowerShell's real behavior needs native Windows. Elvish's built-in has no way to suppress a space,
so `style.` likely takes one there; Phase 8 should confirm where it can and Phase 9 document it.

### Release measurement

One sample, release build, this host: about 40 ms for `md compose schemaed.md <Tab>` outside any
repository (a document with a separate `$schema` file, measured 0.04 s real on repeat runs), and
about 80 ms for `md compose README.md <Tab>` in `darkmatter/` inside this monorepo, where
repository discovery and trigger discovery run. Not a gate.

### Requirement-to-test mapping

The boundary is the shell's words to `md`'s printed candidates, so the wiring is L1 subprocess
(fidelity F2) through `COMPLETE=<shell> md -- ...`, in
`darkmatter/cli/tests/l1/path_completion.rs`; pure logic is unit-tested beside it.

| Requirement | Tests |
|---|---|
| Keys and order, one level at a time | `compose_offers_schema_keys_document_keys_and_files`, `compose_walks_the_schema_one_level_at_a_time`, `get_set_and_rm_offer_their_own_keys`, `set_offers_values_as_its_own_argument` |
| Finding the input with rule 13 | `the_input_may_come_after_setters_and_flag_values_are_skipped`, `a_word_that_is_a_path_with_equals_completes_files`, `no_input_offers_only_the_baseline_and_files`; unit `the_compose_input_can_be_anywhere_on_the_line`, `flag_values_are_not_positionals`, `an_inline_flag_value_consumes_no_word`, `words_after_the_escape_are_positionals`, `a_word_naming_a_subcommand_is_only_special_once`, `a_dangling_flag_is_not_an_error` |
| Words after the cursor ignored | `words_after_the_cursor_are_ignored`; unit `words_after_the_cursor_stay_out_of_the_earlier_words` |
| Earlier setters: not offered again, narrow, select a schema | `paths_already_given_are_not_offered_again`, `earlier_setters_and_overrides_choose_the_schema_the_keys_come_from`, `an_earlier_setter_selects_a_trigger_schema`, `a_document_schema_supplies_keys_and_values` |
| Invalid earlier setters dropped one at a time | `invalid_earlier_setters_are_ignored_one_at_a_time`; unit `an_overlay_drops_only_the_setter_composition_would_refuse`, `an_inadmissible_repeat_keeps_the_earlier_valid_value`, `an_overlay_applies_the_plan_in_its_settled_order`, `an_overlay_layers_defaults_under_and_overrides_over_the_document` |
| Every failure means fewer candidates and no output | `a_broken_document_or_schema_leaves_the_baseline_without_a_word` (broken YAML, missing and invalid `$schema`, remote `$schema`, absent document, absent magic root, bad `--set`); every test's helper asserts empty stderr |
| No stdin read | `stdin_is_never_read`; unit `stdin_is_never_opened` |
| `--magic-root` from the line | `a_magic_root_on_the_line_finds_the_document`; unit `magic_roots_are_read_before_the_subcommand_only` |
| `file` values take ordinary files | `a_file_value_offers_ordinary_files`; unit `any_file_completion_admits_what_markdown_completion_skips`, `the_stdin_marker_is_offered_only_for_input_documents` |
| Bash word shapes and last-piece replies | `bash_gets_only_the_last_piece_in_every_word_shape`, `bash_escapes_a_value_so_it_arrives_as_one_word`; unit `bash_pieces_rejoin_mid_value`, `the_cursor_right_after_equals_means_an_empty_value`, `bash_3_passes_a_whole_word_through`, `earlier_setters_rejoin_and_the_index_follows`, `every_break_in_a_value_rejoins`, `the_program_name_is_never_joined`, `a_reply_replaces_only_the_last_piece`, `a_reply_is_escaped_unless_a_quote_is_open`, `a_finished_reply_carries_its_own_space_when_bash_cannot_add_one`, `raw_words_lose_their_shell_quoting` |
| Other shells keep the whole word | `the_shells_that_pass_a_whole_word_get_a_whole_word`, `a_lone_equals_word_is_only_rejoined_for_bash`; unit `other_shells_keep_a_lone_equals_word`, `no_index_means_the_last_word`, `a_missing_current_word_is_empty` |
| Registration scripts | `registration_scripts_print_for_every_shell`; unit `every_registration_edit_applies`, `the_shells_keep_their_names` |

Not covered yet, by the plan's split: the sentinel proof of no evaluation, proof of no network
request for a remote `$schema` (only its candidates are asserted), the read limit through
`work_counters`, and automated real-Tab tests. These are Phase 8's.

### Input Robustness Matrix

The value parser is unchanged, so its matrix still stands. The new readers are advisory, and this
is the one stated exception: every invalid shape gives fewer candidates and no output, rather than
a located error. A `--state` or `--set` that is not a JSON5 object is ignored
(`an_object_argument_ignores_anything_else`).

Code-smell grep over the changed code, all intentional because completion must stay silent:
`.ok()` on the request, `open_argument`, `Markdown::try_from`, `document_context`, `document_schemas`,
`effective_for`, and the `--state`/`--set` parse in `path_completion.rs`; `filter_map(... .ok())` on
earlier path arguments, where an incomplete word is not a path; `.is_ok()` filters in
`SetterPlan::tolerant` and `completion_overlay`, which drop a setter composition would refuse. No
`unwrap_or_default` or `#[serde(default)]` was added. `SetterPlan::new` still fails on all of them.

### Gates (macOS, 2026-11-06)

The scope check does not allow skipping the gates. It reported
`packages: [darkmatter, darkmatter-cli, test-toolkit]`, `change_class: package`.

| Gate | Result |
|---|---|
| `just lint` in `darkmatter/` | exit 0, including the Zed WASM check |
| `just test` in `darkmatter/` | exit 0: 14264 passed, 10 skipped by the recipe's selection; none slow |
| `just check-tier-coverage darkmatter` (repo root) | exit 0; stranded 0 |

The first full `just test` stopped on three of this phase's own unit tests, which built their
document with `Markdown::new`, which takes body text and parses no frontmatter. Each was rerun
alone, the tests were corrected to `Markdown::from`, and the full run then passed. No
load-sensitive or pre-existing failure occurred. No source changed after these runs; only the
skill, this log, and the plan did.

### Dependent packages

- `claudine` and `claudine-cli`: `cargo check --all-targets` passed. The public API only gained
  items, and `apply_setter_plan` keeps its behavior.
- `darkmatter-lsp` and `zed-dmls-cli`: compiled and linted by `just lint`.
- `test-toolkit`, which the scope check selected: not run. It has more than 200 tests (477 at
  Phase 6) and none uses a symbol this phase changed; it does not depend on `completion_shells`,
  `path_completion`, or the overlay.

### Cross-OS check

Run, because this phase handles file paths and process arguments.
`just cross-check darkmatter-cli --os windows`: 2050 passed, 76 skipped, exit 0, on the working tree
patched onto the rig, so it publishes no CI observation. Linux and WSL2 were not run; they run under
scheduled CI. PowerShell's real Tab behavior is unexecuted.

### Docs, skills, and spec

`docs/` waits for Phase 9, as the plan assigns, including the per-shell limits above. Added
completion notes to `.claude/skills/darkmatter/cli.md`. This phase names no spec frontmatter to set,
and the spec is unchanged. The departures from the plan are in `decisions_for_review`.

## Phase 8

Proved the input and execution contracts with real processes and real shells. No product
behavior changed. The one production edit is a seam: `DocumentSite::load` now takes its
request, so a unit test can measure one Tab press in its own temp directory instead of
scanning this checkout.

### What landed

| Item | Where |
|---|---|
| No evaluation, no command, no request, no write, with controls | `darkmatter/cli/tests/l1/path_completion.rs` |
| The read limit, through `work_counters` and `observed::collect` | unit test in `darkmatter/cli/src/args/path_completion.rs` |
| `work-counters` on the dev-dependency, so only test builds compile it | `darkmatter/cli/Cargo.toml` |
| Registration scripts loading in real shells | `darkmatter/cli/tests/l1/path_completion.rs` |
| Real Tab round trips, ending by running the command | new `darkmatter/cli/tests/level2/level2_path_completion.rs` |
| Evidence that spans commands | new `darkmatter/cli/tests/l1/property_paths.rs` |
| "Only `PropertyPath` splits a path" guards | `property_paths.rs`, `darkmatter/lib/tests/l1/compose_setters.rs` |

### Findings

- The sentinel is the `darkmatter-argv-recorder` program, not `touch`, which native Windows
  lacks. It records which commands ran, and the control asserts composing runs exactly
  `frontmatter`, then `body`, while a Tab leaves the whole fixture workspace unchanged.
- The fixture's first draft had a duplicate `title` key, so it did not parse. A document that
  fails to parse also runs nothing, so "no sentinel" alone would have passed; the assertion that
  local keys are offered is what caught it. A no-effect test needs a positive assertion and a
  control.
- Interpolation is proven through a union: `kind: a` narrows to one arm, and
  `kind: "{{ 'a' }}"` offers both, so the value was not evaluated.
- A Tab with `--allow host=127.0.0.1` on the line still makes no request. A remote `$schema`
  and a remote import in a local schema both leave `server.requests()` empty, and the control
  shows the same server does answer an allowed composition.
- `ExpressionParse` is 648 and `ValidatorCompile` 19 on the first press, whatever the document:
  that is the baseline schema. The test therefore compares warmed presses and asserts the
  document's own expressions add nothing, rather than asserting zero.
- A YAML list under `$schema` is a union of object arms, not an array. `md set items.0.count abc`
  under such a schema saved, because the document was already invalid at the parent `items` and
  rule 11 does not blame an edit for that. This looked like a bug and is not.
- The first draft's no-diagnostics test took 5.3 s as one test across five shells. It is now
  one test per shell. `run_shell` first set a directory and `HOME` on the shell's command, which
  `spawn_site_guard` rejected; the shells read and write no profile, so both were removed rather
  than adding a file-wide allow-list entry.
- On this host, a bare `ls` in `/tmp` hung for over 90 s twice during probing, while `/bin/ls` did
  not. The shell aliases `ls`. The cause was not investigated; it is not related to `md`.

### Per-shell results

"Real Tab" is the automated L2 round trip on this Mac, 2026-11-06: `style.pa<Tab>` to `style.page`,
type `.`, Tab, type `al`, Tab to `style.page.alignment=`, type `ce`, Tab, type `zz=1`, assert the
whole line is `md compose doc.md style.page.alignment=center zz=1`, press Enter, and assert the
composed output says `Align: center!`. A stray space at any step fails the whole-line assertion.

| Shell | Word shape (L1) | Script loads (L1) | Real Tab (L2) |
|---|---|---|---|
| bash 5.3.20 | proven | proven | proven |
| bash 3.2.57 | proven | proven | proven |
| zsh | proven | proven | proven |
| fish | proven | proven | proven |
| elvish | proven | **unproven**: not installed, no script test | **unproven** |
| PowerShell | proven | proven on native Windows | **unproven**: needs native Windows |

Mutation check of the L2 test: with bash's no-space rule disabled, it failed on the line
`md compose doc.md style.page.alignment= center`, then the source was restored. Linux and WSL2 run
these under scheduled CI, where a bash 3 and usually zsh and fish are absent and report unproven.

### Acceptance criteria to tests

CLI tests are in `darkmatter/cli/tests/l1/` unless a crate is named. `get_set_rm` and
`compose_setters` are the earlier phases' files; `property_paths` is this phase's.

| Criterion | Tests |
|---|---|
| Read, edit, and remove a nested value; siblings and bytes kept | `property_paths::an_edit_survives_every_later_command_and_what_was_not_named_survives_the_edits`, `get_set_rm::set_paths::a_nested_edit_keeps_comments_and_spelling_in_the_shared_parent`, `get_set_rm::rm_paths::removing_a_nested_key_keeps_its_siblings_and_their_comments` |
| `a.b` never writes a literal key; `a\.b` does, and `get`/`rm` reach it | `get_set_rm::set_paths::a_dotted_path_never_writes_a_literal_key_and_an_escaped_one_does`, `property_paths::an_edit_survives_every_later_command_and_what_was_not_named_survives_the_edits` |
| `a.b=1` overrides one value, visible to expressions and validation | `compose_setters::applying::a_nested_setter_keeps_its_siblings_and_reaches_expressions`, `compose_setters::values::a_nested_value_reads_through_its_declared_type` |
| Top-level contracts kept; differences intentional | `output_commands::conforms_get_*`, `conforms_set_*`, `conforms_rm_*`; `get_set_rm::test_set_save_*`, `test_rm_keeps_every_other_byte`, `test_set_then_rm_round_trip_restores_original_bytes`; `compose_state_set` |
| `./a.b=c.md` is a path, `a.b=c.md` a setter, `'$schema'` works | `compose_setters::classifying::a_dotted_key_is_a_setter_and_dot_slash_makes_it_a_path`, `get_set_rm::get_paths::the_empty_argument_special_keys_and_unicode_are_ordinary_keys` |
| `items.1.name`, numbers on mappings, every out-of-range row | `get_set_rm::get_paths::a_number_is_an_index_on_an_array_and_a_key_on_a_mapping`, `get_set_rm::set_paths::a_number_on_a_mapping_is_a_key`, `property_paths::an_index_that_does_not_exist_is_the_same_case_for_every_command` |
| Each Parents row; a failed edit writes nothing | `get_set_rm::set_paths::a_missing_parent_is_created`, `a_number_under_a_missing_parent_is_refused`, `a_parent_that_cannot_hold_the_key_is_named_with_its_type`, `get_set_rm::rm_paths::removing_the_last_child_leaves_the_parent`, `property_paths::a_null_or_scalar_parent_refuses_every_mutation_and_can_be_replaced_by_name` |
| `1.10` under `string`; `count=5` declared and not; `{a:1}` alike in both commands | `property_paths::a_declared_string_keeps_its_spelling`, `a_declared_number_is_a_number`, `an_undeclared_number_is_a_number`, `a_json5_object_is_an_object` |
| Both orders of `author={...} author.name=Bob` | `compose_setters::precedence::whole_object_plus_descendant_is_the_same_in_both_orders` |
| `dm.schema.unknown_key` under a closed object, none under a pattern | `compose_setters::values::a_closed_object_refuses_an_undeclared_child_and_a_pattern_opens_it` |
| `{ max-width: 50 }`; rejected syntax stays text; admission errors fail | `property_paths::a_yaml_flow_object_is_an_object`, `a_rejected_syntax_stays_text`, `compose_setters::failing::an_inadmissible_container_never_becomes_text`, `get_set_rm::set_values::an_inadmissible_container_fails_and_never_becomes_text`; simplified-schema `setter_values` |
| Output schemas conform | `output_commands::conforms_get_nested_paths_stay_flat`, `conforms_set_nested_path_is_written_as_given`, `conforms_set_rejected_value_has_no_document`, `conforms_rm_nested_path_keeps_top_level_remaining`, `conforms_rm_conflict_fails` |
| Only `PropertyPath` parses; typed paths in diagnostics | `property_paths::only_property_path_splits_a_path_in_the_commands`, lib `compose_setters::only_property_path_splits_a_path_in_the_library`, `property_paths::a_rejected_value_names_its_place_whether_a_number_was_an_index_or_a_key`; simplified-schema `property_paths::the_pointer_comes_from_the_typed_walk_and_escapes_its_keys` |
| `parse(print(segments)) == segments` | simplified-schema `property_paths::round_trip_holds_for_the_hard_spellings`, `parse_of_print_is_the_segments` |
| Empty key, syntax errors, Unicode, escapes, huge indexes, scalar/null, removal order and conflicts, scalar ancestor in both orders | `property_paths::whitespace_and_unicode_in_a_key_are_kept_by_every_command`, `get_set_rm::malformed_paths::each_command_fails_as_usage_before_reading_anything`, `get_set_rm::rm_paths::array_removals_use_original_indexes_in_either_order`, `a_duplicate_or_an_ancestor_with_its_descendant_cancels_the_command`, `compose_setters::precedence::a_scalar_ancestor_fails_in_both_orders_and_composes_nothing` |
| Flat `get`; `rm.remaining`; comments; alias refused; read/write/read | `get_set_rm::get_paths::several_paths_give_a_flat_object_keyed_as_written`, `get_set_rm::rm_paths::removing_the_last_child_leaves_the_parent`, `get_set_rm::set_paths::an_edit_an_alias_reads_is_refused_with_no_fallback`, `repeated_edits_round_trip_through_the_file` |
| `md set` validation rows; broken schema; setting `$schema` | `get_set_rm::set_validation::a_valid_value_writes_despite_a_problem_elsewhere`, `a_value_the_type_rejects_writes_nothing`, `setting_the_missing_key_repairs_the_document`, `a_broken_schema_fails_with_an_explanation_and_no_write`, `setting_schema_checks_only_the_reference` |
| Trigger in either order; `--set` replacing `$schema`; converted trigger explained | `compose_setters::selecting_a_schema::a_setter_selects_the_trigger_that_reads_another_in_either_order`, `a_set_that_replaces_the_schema_selects_the_same_way`, `a_trigger_on_a_converted_type_fails_clearly_with_no_retry` |
| Quotes decode once; shared conversion; pending and caller origins kept | `property_paths::a_quoted_string_decodes_once`, `get_set_rm::set_values::numberlike_boolish_and_unions_follow_the_shared_rules`, `a_pending_value_is_written_unevaluated`, `compose_setters::origins::an_authored_file_value_beside_a_setter_keeps_its_own_origin`, `a_template_in_a_setter_is_authored_at_any_depth` |
| A setter failure names its argument and path, with no authored position | `compose_setters::assert_setter_failure` (used by its failure tests), `property_paths::a_rejected_value_names_its_place_whether_a_number_was_an_index_or_a_key` |
| Completion table rows, separate `$schema`, required first, tag narrowing | lib `path_candidates` (every row); `path_completion::a_document_schema_supplies_keys_and_values`, `earlier_setters_and_overrides_choose_the_schema_the_keys_come_from` |
| No evaluation, command, or request; no diagnostic text | `path_completion::a_tab_runs_no_command_and_writes_nothing_while_composing_the_same_document_does`, `a_remote_schema_is_never_requested_and_local_keys_remain`, `a_remote_import_inside_a_local_schema_is_never_requested`, `an_interpolated_tag_is_not_evaluated_to_narrow_a_union`, `a_failure_adds_no_text_in_{bash,zsh,fish,elvish,powershell}` |
| What a Tab may read | unit `args::path_completion::tests::one_tab_prepares_once_and_reads_only_the_documents_schema_files` |
| Unparseable document, unresolved schema, no input each still complete | `path_completion::a_broken_document_or_schema_leaves_the_baseline_without_a_word`, `no_input_offers_only_the_baseline_and_files` |
| The Tab round trip in each shell, or its documented limit | level2 `level2_path_completion::a_tab_round_trip_in_{modern_bash,bash_3,zsh,fish}`; the table above for elvish and PowerShell, with the limits written in Phase 9 |

Fidelity: the shell boundary is F3 in L2 (real shell, real terminal, detached tmux, no focus); word
shapes and effects are F2 subprocess in L1; the read limit is in-process, because counters live in
the process that does the work.

### Input Robustness Matrix

No reader was added or changed, so the matrix stands from Phase 7. `property_paths` adds
control rows through the real CLI: a good value at the same path succeeds where `abc` fails.

### Gates (macOS, 2026-11-06)

The scope check does not allow skipping, because source files changed.

| Gate | Result |
|---|---|
| `just test` in `darkmatter/` | exit 0: 14296 passed, 10 skipped by the recipe's selection; none slow |
| `just lint` in `darkmatter/` | exit 0, including the Zed WASM check |
| `just check-tier-coverage darkmatter` | exit 0; stranded 0 |
| `just test-l2 level2_path_completion` | 4 passed |

The first full run stopped on `spawn_site_guard` (described above), and it passed after the fix.
No pre-existing or load-sensitive failure occurred, and nothing was skipped beyond the recipe's selection.

### Dependent packages

`claudine` and `claudine-cli`: `cargo check --all-targets` passed. No library code changed in this
phase, so the other dependents have nothing new to exercise and their tests were not run.

### Cross-OS check

`just cross-check darkmatter-cli --os windows`, run because a `#[cfg(windows)]` test was added and
the new tests spawn programs. The first run failed on two of this phase's tests, both test defects:
`touch` does not exist there, and Windows PowerShell has no `--version` flag. After the fixes: 2079
passed, 80 skipped, exit 0, on a patched tree, so it publishes no CI observation. Linux and WSL2 were
not run; they run under scheduled CI.

### Docs, skills, and spec

`docs/` waits for Phase 9, as the plan assigns. Added the Tab-evidence patterns to
`.claude/skills/darkmatter/testing.md` and the two Windows facts to `.claude/skills/os/windows.md`.
This phase names no spec frontmatter to set, and the spec is unchanged.

## Phase 9

Wrote the documentation and guidance. No compiled code changed, so `source_files_during_phase_9`
is empty. Every example and error text in the new docs was run against the debug `md` in a scratch
directory first.

### What landed

| Item | Where |
|---|---|
| Shared sections: Property Paths, Command-Line Values, Shell Completions (paths, values, what it reads, per-shell limits) | `darkmatter/docs/cli/index.md` |
| Paths, migration tables; validation flow for `set`, conflict rules for `rm`, setter classification and overlaps for `compose` | `darkmatter/docs/cli/{get,set,rm,compose}.md` |
| Removed "does not evaluate dotted paths", the old key grammar, `compose.md`'s claim that `--set` takes dot notation, and "setters merge into the `--set` map" | same |
| Editing Nested Values: what is kept, the fallback, the alias refusal | `darkmatter/docs/topics/frontmatter.md` |
| Paths as written and exit 2 for a malformed path; the setter family and examples | `darkmatter/docs/cli/data-contracts.md`, `darkmatter/docs/errors/composition-problems.md` |
| A setter's non-finite number now fails inside a container (drift) | `darkmatter/docs/topics/numbers.md` |
| New pages for the two library types, linked from the README | `simplified-schema/docs/topics/{property-paths,setter-values}.md`, `simplified-schema/README.md`, `foundations.md` |
| Guidance | `.claude/skills/darkmatter/cli.md`, `.claude/skills/simplified-schema/SKILL.md`, `.claude/skills/biscuit-file/references/api.md`, `.claude/skills/os/macos.md` |

`.claude/skills/darkmatter/frontmatter.md` and `schema.md` already describe the final behavior
from Phases 3, 4, and 6, so they did not change.

### Findings

- My first value table was wrong in two columns. Running every cell showed `numberlike` refuses
  `true` and `On`, and `boolish` refuses `5`, `1.10`, and `02134`. The table now reports what ran.
- `[x, y]` as a value is text, because only a value starting with `{` is tried as YAML. The docs
  say so and show `'["x", "y"]'`.
- `md compose p.md v=7` beats a child's `::file ch.md set.v=9`. `--set` does the same, so the docs
  say a setter reaches children "just as `--set` does".
- **A probe touched another repository.** A scratch script ran `mkdir repo; cd repo; git init`.
  This host's `CDPATH` sent the `cd` to `/Users/ken/coding/personal/repo`, an existing repository
  with uncommitted work, so `git init` re-initialized it. Checked afterward: its commits and
  working-tree changes are untouched, `.git/config` was rewritten and holds only `[core]`, and
  `hooks/sendemail-validate.sample` is new. Nothing was reverted there. The trap is now in
  `.claude/skills/os/macos.md`.

### Requirement-to-check mapping

Documentation has no test to add, so each requirement maps to how it was checked.

| Requirement | Check |
|---|---|
| Examples and messages are real | ran against `target/debug/md` in a scratch directory |
| Links and anchors resolve | `md validate refs` on the seven pages it can judge; a script over all thirteen changed pages for the rest |
| No page names this feature | grep for dated names and `features/`/`fixes/` over the changed pages: none |
| Elvish and PowerShell not over-claimed | per-shell table follows the Phase 8 results |

`md validate refs` fails on `compose.md`, `data-contracts.md`, `numbers.md`,
`composition-problems.md`, `foundations.md`, and the simplified-schema README, because it evaluates
their illustrative `{{ ... }}` text. The same files fail the same way at `HEAD`, so this is
pre-existing. The script's only misses are three old repo-rooted links in that README
(`darkmatter/README.md`, `claudine/README.md`, `darkmatter/dmls/README.md`), left as they were.

### Gates (macOS, 2026-11-06)

The scope check does not allow skipping. Over the working tree it reported
`packages: [darkmatter, darkmatter-cli, test-toolkit]`, `change_class: package`, because Phase 8's
uncommitted sources are still in the tree. Run over this phase's Markdown files alone, it still
reports `packages: [darkmatter]`, with the reason "changed test input read by package(s) darkmatter",
as one Linux L1 cell. So a test reads these pages, and the run below, made after the edits, covers it.

| Gate | Result |
|---|---|
| `just lint` in `darkmatter/` | exit 0, including the Zed WASM check |
| `just test` in `darkmatter/` | exit 0: 14296 passed, 10 skipped by the recipe's selection; none slow |
| `just check-tier-coverage darkmatter` | exit 0; stranded 0 |

### Dependent packages and cross-OS

No compiled code changed, so dependents have nothing new to exercise and were not run, and no
cross-OS check was needed. The Windows notes in the docs (`.\dir\.config=1.md`) come from the spec
and the Phase 8 cross-check.

No spec frontmatter was named for this phase, and the spec is unchanged.

## Phase 10

Ran the integrated validation and closed the plan. The review found one coverage gap: the plan
asks for the `.\dir\.config=1.md` classification under `#[cfg(windows)]`, and the only coverage was
an all-OS assertion on the classifier, so nothing proved the real command on native Windows.

### What landed

| Item | Where |
|---|---|
| `classifying::a_windows_path_with_a_dot_folder_needs_its_leading_dot_backslash`, `#[cfg(windows)]`: `.\dir\.config=1.md` opens that file, and `dir\.config=1.md` beside `doc.md` is a setter rather than a second input | `darkmatter/cli/tests/l1/compose_setters.rs` |

No product code changed. The test only asserts behavior the spec already states.

### Requirement-to-test mapping

| Requirement | Test or check | Boundary and fidelity |
|---|---|---|
| A file named like a setter opens with `.\` on native Windows | the test above, first assertion | CLI process, native Windows, F2 |
| `dir\.config=1.md` reads as a setter there | the test above, second assertion: a second input would exit 1 with "expected at most one input path" | CLI process, native Windows, F2 |
| Every spec acceptance criterion | the table under Phase 8, unchanged | as listed there |

### Gates (macOS, 2026-11-06)

The scope check does not allow skipping, because a test source changed. Over that file it reports
`packages: [darkmatter-cli, test-toolkit]`.

| Gate | Result |
|---|---|
| `just test` in `darkmatter/`, rerun after the new test | exit 0: 14296 passed, 10 skipped by the recipe's selection; none at or above 5 s |
| `just lint` in `darkmatter/`, rerun after the new test | exit 0, including the Zed WASM check |
| `just test` in `simplified-schema/` | exit 0: 1480 passed in the first run, 1292 in the second; 0 skipped |
| `just lint` in `simplified-schema/` | exit 0 |
| `just test` in `biscuit-file/` | exit 0: 1198 passed; then 6 passed, 67 skipped by selection |
| `just lint` in `biscuit-file/` | exit 0 |
| `just test-l2 level2_path_completion` in `darkmatter/` | exit 0: 4 passed. The full `just test-l2` was not run; the plan names the new target |
| `just check-tier-coverage darkmatter` (from the root) | exit 0; stranded 0 |
| `just test test-toolkit` (selected by the scope check) | exit 0: 477 passed, 4 skipped. Two tests took over the 5 s budget: `kache_host_contracts::report_resolves_the_worktree_base_by_the_wt_contract` (5.03 s) and `archive_path_guard::no_archive_executed_target_bakes_in_a_producer_path` (5.56 s). This feature did not touch either; recorded, not fixed |
| `cargo --color=never check --all-targets -p claudine -p claudine-cli` | exit 0 |
| `git diff --check` over the working tree | exit 0 |

The new test is `#[cfg(windows)]`, so the macOS count stays 14296 and its reruns add no macOS
coverage; they confirm the edit left the build clean. The two `simplified-schema` and `biscuit-file`
runs were made before that edit, which touches neither package. No test failed in any run, and no
test was load-sensitive apart from the two slow `test-toolkit` tests noted above. `git diff --check`
over each of this feature's six implementation and docs commits reports nothing. Against `main`, it
reports trailing whitespace in ten files from other commits on this branch, including older lines of
`simplified-schema/README.md`, which this feature also edits; those were left alone.

### Guards

All passed in the runs above: `schema_language_ownership_guard` and `dependency_direction_guard`
(schema ownership), `context_construction_guard` in lib and CLI, `path_lookup_guard` in lib and CLI,
and in `output_commands`, `every_leaf_has_a_schema_file_and_a_conformance_test` with the 14
`conforms_get_*`, `conforms_set_*`, and `conforms_rm_*` cases.

### Comment review

Reviewed the doc comments on the changed behavior in `cli/src/commands/frontmatter.rs` and the `get`,
`set`, and `rm` help text in `cli/src/args/command.rs`; they match the code. `edited_text`, which the
plan names, no longer exists, so its comment has nothing to drift. `parse_compose_setter`,
`parse_shorthand_value`, and the removed doc claims appear nowhere in `darkmatter/lib/src`, `darkmatter/cli/src`,
`darkmatter/docs`, `simplified-schema/docs`, or the Darkmatter skill. The new modules have no `TODO`, `FIXME`, `todo!`, or `unimplemented!`.
The robustness grep matches only the `.ok()` calls already recorded as intentional under Phase 2.

### Where the closure evidence lives

- Test names per acceptance criterion: "Acceptance criteria to tests" under Phase 8.
- Release measurement: "Release measurement" under Phase 7, one sample, about 40 ms outside a
  repository and about 80 ms inside this monorepo. Not remeasured; no product code changed since.
- Per-shell results: "Per-shell results" under Phase 8. Bash 5.3.20, bash 3.2.57, zsh, and fish are
  proven with real Tab. Elvish has its word shape proven only, because it is not installed.
  PowerShell has its word shape proven and its script loading on native Windows, with real Tab unproven.
- Departures from the plan and spec: the `decisions_for_review` list, which this phase did not
  add to.

### Operating systems

| OS | Status in this phase |
|---|---|
| macOS | full gates above |
| native Windows | `just cross-check darkmatter-cli --os windows compose_setters::classifying`: exit 0, 4 passed, 2156 filtered out, on a patched tree, so it publishes no CI observation. Targeted evidence only. The last full `darkmatter-cli` Windows run is the one under Phase 8, 2079 passed |
| Linux | **unexecuted**; runs under the repository's scheduled CI |
| WSL2 | **unexecuted**; runs under the repository's scheduled CI |

`simplified-schema`, `biscuit-file`, and the `darkmatter` library have no native Windows run recorded
in this plan, only `darkmatter-cli` does, so scheduled CI is their first evidence there.

### Dependent packages

`claudine` and `claudine-cli` compile, as above. No library code changed in this phase, so no
dependent's tests were run.

### Docs, skills, and spec

No docs or skills changed. The `-E` quoting trap hit while filtering the Windows run is already in
`.claude/skills/os/build-hosts.md`, and the Darkmatter skill needed no change. The feature stays in
the active directory: implementation complete, ready for review.
