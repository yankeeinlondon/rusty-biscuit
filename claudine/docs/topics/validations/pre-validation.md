# Pre Validation in Claudine

Pre-validation in **Claudine** are rule(s) which validate whether or not a given page should be executed / started. Each Validation has precisely three states it can emit:

- `pass` - _the validation rules trigger but the rule is able to take _actions_ to move it back into a passing state_
- `skip` - _the validation expresses that it's utility is not needed and can be safely skipped_
- `fail` - _the process is not ready to start_

If a `skip` or `fail` state is specified in a rule, we immediately give up on the execution of the current document and:

- `skip`
    - We provide status to the caller: `Status::with_prose("").state(StatusState::Skip)`
    - If the document is part of a **Sequence** then the next state of the sequence will be started
    - Otherwise the execution will end and a 0/ok exit code will be returned
- `fail`
    - We provide status to the caller: `Status:with_prose("").state(StatusState::Error)`

The `success` state is the case where _all_ pre-validation rules.

The rule(s) are created from a enumerated set of [rules](./validation-rules.md) but do allow for a shell command to be run as "safety valve" if none of the operations are adequate for what needs to be tested.

> **Note:** like all other shell command execution in **Claudine** any shell commands defined will be added to the [pre-flight process](../pre-flight-checks.md) to ensure that the command is "allowed" via the whitelisting process.


