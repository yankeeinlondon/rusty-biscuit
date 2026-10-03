# Secret Recognition

Claudine keeps credentials out of text it persists or displays. Four places do
this, and they share one recognizer, `claudine::secrets`:

| Consumer | What it protects | Rules it applies | Replacement |
| --- | --- | --- | --- |
| Capture-time scrubbing (`protect::scrub`) | unmatched-event harvest files | credential tokens, then email addresses; sensitive JSON keys; home path → `~` | `<redacted>` per whole match |
| Messaging errors (`messaging::send`) | route warnings and test-connection errors | webhook URLs | `<redacted-webhook-url>` |
| Wrapper sanitization (CLI `wrap::env`) | the child environment, and every display of the provider argv (`AGENT_PARAMS`, dry runs, traces) | sensitive env-var names (stripped); in argv, sensitive flags, credential-token prefixes, and every rule family inside a token (`mask_argument_token`) | variable removed; argument value `****` |
| Steering audit (`steering::audit`) | steering log records, errors, provider echoes | every rule family, plus values already recognized in the message | `****` per merged span |

Recognition is shared; what a consumer does with a match is not. Only
scrubbing hides email addresses and home paths. Steering deliberately keeps
them, because a steering message is prose written for an agent and must stay
readable in its log.

Recognition is a heuristic. It catches common credential shapes and contexts;
it cannot promise that every secret is found.

## What is recognized

The catalog (`SECRET_CATALOG`) groups rules into three families. A consumer
selects the families it needs.

**Credential tokens** identify themselves by shape:

```text
sk-ant-api03-AbCdEf0123456789XyZ        API key (sk-)
AKIAIOSFODNN7EXAMPLE                    AWS access key ID
ghp_… gho_… ghs_… github_pat_…          GitHub tokens
xoxb-… xoxp-…                           Slack tokens
Bearer abc.DEF-123_xyz~9                bearer token (the value after "Bearer")
eyJhbGciOi….eyJzdWIi….SflKxw…           JWT
```

**Webhook URLs** carry their credential in the path:

```text
https://discord.com/api/webhooks/123/abc-DEF
https://hooks.slack.com/services/T000/B000/XXXX
```

**Contextual** rules recognize a value by what surrounds it:

| Context | Example input | Steering log |
| --- | --- | --- |
| assignment to a sensitive key | `OPENAI_API_KEY=abc123` | `OPENAI_API_KEY=****` |
| quoted or JSON value | `{"client_secret": "s3cr3t"}` | `{"client_secret": "****"}` |
| `key: value` field | `password: hunter2` | `password: ****` |
| long flag | `--password hunter2` | `--password ****` |
| authorization header | `Authorization: Basic dXNlcjpwYXNz` | `Authorization: Basic ****` |
| URL password | `https://ken:pw123@git.example.com` | `https://ken:****@git.example.com` |
| URL query parameter | `?api_key=abc123&page=2` | `?api_key=****&page=2` |
| private-key block | `-----BEGIN … PRIVATE KEY-----` … | body replaced by `****` |

A `key: value` field or a flag value is masked only when it looks like a
credential: 16 or more bytes, or containing a character that is not a letter.
That keeps prose readable:

```text
Token: expired. Password: required!     unchanged
password: hunter2                        password: ****
```

### Sensitive key names

`is_sensitive_key_name` decides whether a key, header, flag, or environment
variable names a secret. It ignores case and treats `-` and `_` alike, so
`x-api-key`, `API_KEY`, and `apiKey` all match. A name is sensitive when it
contains `API_KEY`, `APIKEY`, `AUTHORIZATION`, `TOKEN`, `PASSWORD`, `SECRET`,
`PRIVATE_KEY`, `CREDENTIAL`, `ACCESS_KEY`, or `PASSPHRASE`, or ends with
`_KEY` (but not `PUBLIC_KEY`), `_AUTH`, `_PAT`, `_PWD`, or `_PEM`.
`SSH_AUTH_SOCK`, `OLDPWD`, and `PUBLIC_KEY` are not sensitive.

The same function strips wrapper environment variables and redacts harvested
JSON values, so both consumers agree on what a secret key is.

## Steering masking

`mask_secrets` finds every span from every family, merges spans that overlap
or touch, and replaces each merged span with `****`. Span bounds are always
UTF-8 character boundaries, so multibyte text around a secret is intact.

```text
Authorization: Bearer sk-ant-api03-AbCdEf0123456789XyZ
Authorization: Bearer ****        (three rules matched; one mask)
```

Masking is idempotent: masking `password=****` again yields `password=****`.

A `Redactor` is built from the original message. Besides the recognized spans,
it remembers each recognized value of at least six bytes and masks every
later occurrence of it — in the message and in any error or provider response
that echoes it:

```text
message:  set password=hunter22 and then log in with hunter22
logged:   set password=**** and then log in with ****
error:    login failed: invalid credential 'hunter22'
logged:   login failed: invalid credential '****'
```

The redactor holds those values in memory only; its `Debug` output shows just
their count. Masked text has its own type, `RedactedText`, which is the only
text type a steering audit record accepts.

## Adding a rule

Add a `SecretRule` to `SECRET_CATALOG` in the family it belongs to, with a
case to the `CORPUS` in `lib/src/secrets/tests.rs` and, if the rule could
match ordinary text, a case to `ORDINARY`. Credential-token rules also change
what scrubbing redacts, and a new literal prefix belongs in
`CREDENTIAL_TOKEN_PREFIXES`, which the wrapper's argument sanitizer uses; a
test checks that every prefix names a catalog token shape.
