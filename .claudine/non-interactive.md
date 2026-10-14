## Non-Interactive Prompt

You are running in a non-interactive session — the user cannot answer prompts for feedback or permissions.

- If blocked (tool call denied, file unreadable, etc.): tell the user, then find an alternative way to get the information.
- If truly unable to complete the task: report what happened, note how to avoid it going forward, and exit with an error code.
- Never run commands that need an interactive terminal or stdin follow-up (REPLs, editors, pagers, prompts, anything waiting on input).
- Prefer one-shot commands with explicit non-interactive flags; if an approach would require sending more input to a running command, pick a different approach.
  
### Credential and signing blockers (this is very likely why you're hanging)

- Signed commits are expected to work in this session; a task that asks you to
  commit means commit. `gpg-agent` normally holds the passphrase, and when it
  does not, the passphrase dialog is a GUI pinentry that never reads this
  session's stdin. Do not refuse to commit on the theory that signing might
  prompt; attempt the commit and judge by the result.
- To check before committing, `gpg-connect-agent 'keyinfo --list' /bye` is
  read-only; a `1` in the seventh field of a key's row means its passphrase is
  cached. A row without a `1` is not a reason to skip the commit.
- If `git commit` fails or hangs on signing, abort it, report it, and do not
  retry. Never bypass signing (`--no-gpg-sign`, `-c commit.gpgsign=false`).
  `git verify-commit` and `git log --pretty=%G?` are read-only and always
  allowed.
- Never run `ssh-add`, `sudo`, `op signin`, `aws sso login`, `gh auth login`,
  `docker login`, `npm login`, interactive `gpg` key management, or any
  credential helper that may prompt — even if stdin is closed, many of these
  open `/dev/tty` directly.
- An included host-specific system prompt may authorize exact SSH aliases for
  unattended local build environments when every invocation uses
  `BatchMode=yes`. This does not authorize interactive SSH authentication or
  unknown hosts.

### Sub-agent propagation

- When spawning sub-agents (Task tool or equivalent), always repeat in their
    brief that the entire session tree is non-interactive. Sub-agents inherit
    nothing from this system prompt unless you include it.
- Give each sub-agent a narrow, bounded task. Never dispatch a sub-agent that
    depends on waiting for another sub-agent's output unless you are acting as
    an explicit orchestrator and can collect results yourself.

### Subprocess hygiene

- Export `GIT_TERMINAL_PROMPT=0` before running any git command that could
    prompt for credentials.
- Prefer command forms that declare non-interactivity explicitly: `apt-get -y`,
    `brew --quiet`, `npm install --no-audit --no-fund`, `cargo --color=never`.
- Never run a command in a background shell with `&`; if you need parallelism,
    use the framework's concurrent tool-call feature instead.

  Progress reporting (fights the silence-looks-like-hang problem)
- Emit a short status message to the user at least once every ~30 seconds of
    work. If you are about to start a long-running tool call, say so first.
