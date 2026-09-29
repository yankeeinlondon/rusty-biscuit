use super::*;

/// `(input, masked)` pairs covering every rule family and the prose that must
/// survive. Each input is realistic steering or echo text.
const CORPUS: &[(&str, &str)] = &[
    // Token prose.
    (
        "please retry with sk-ant-api03-AbCdEf0123456789XyZ now",
        "please retry with **** now",
    ),
    (
        "use AKIAIOSFODNN7EXAMPLE and ghp_0123456789abcdefghij0123 then xoxb-1234567890-abcdef",
        "use **** and **** then ****",
    ),
    (
        "jwt eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.SflKxwRJSMeKKF2QT4fwpM expired",
        "jwt **** expired",
    ),
    // Assignments.
    ("export OPENAI_API_KEY=abc123xyz", "export OPENAI_API_KEY=****"),
    ("password=secret", "password=****"),
    (r#"password = "correct horse battery""#, r#"password = "****""#),
    ("db.password='p@ss w0rd'", "db.password='****'"),
    (
        r#"{"client_secret": "s3cr3t-value", "name": "demo"}"#,
        r#"{"client_secret": "****", "name": "demo"}"#,
    ),
    ("password: hunter2.", "password: ****."),
    ("x-api-key: abcdef123456", "x-api-key: ****"),
    ("run tool --password hunter2 --verbose", "run tool --password **** --verbose"),
    ("run tool --token=abc --verbose", "run tool --token=**** --verbose"),
    // Authentication headers.
    ("Authorization: Basic dXNlcjpwYXNz", "Authorization: Basic ****"),
    (
        "curl -H 'Authorization: Bearer abc.DEF-123_xyz~9' https://api.example.com",
        "curl -H 'Authorization: Bearer ****' https://api.example.com",
    ),
    ("Proxy-Authorization: Negotiate YIIabc123", "Proxy-Authorization: Negotiate ****"),
    // Credential-bearing URLs.
    (
        "clone https://ken:hunter2pass@git.example.com/repo.git",
        "clone https://ken:****@git.example.com/repo.git",
    ),
    (
        "GET https://api.example.com/v1?api_key=abc123&page=2",
        "GET https://api.example.com/v1?api_key=****&page=2",
    ),
    (
        "post to https://hooks.slack.com/services/T000/B000/XXXX failed",
        "post to **** failed",
    ),
    (
        "hook https://discord.com/api/webhooks/123/abc-DEF_ghi rejected",
        "hook **** rejected",
    ),
    // Overlapping rules merge into one mask.
    (
        "Authorization: Bearer sk-ant-api03-AbCdEf0123456789XyZ",
        "Authorization: Bearer ****",
    ),
    (
        "GITHUB_TOKEN=ghp_0123456789abcdefghij0123",
        "GITHUB_TOKEN=****",
    ),
    // Unicode around and inside secrets.
    (
        "✓ déploiement: token=café😀123 terminé 🚀",
        "✓ déploiement: token=**** terminé 🚀",
    ),
    ("秘密 api_key=ключ42 完了", "秘密 api_key=**** 完了"),
    // Multiline text.
    (
        "line one\nGITHUB_TOKEN=abc\nline three\n",
        "line one\nGITHUB_TOKEN=****\nline three\n",
    ),
    (
        "key:\n-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXk\nAAAA\n-----END OPENSSH PRIVATE KEY-----\ndone",
        "key:\n-----BEGIN OPENSSH PRIVATE KEY-----****-----END OPENSSH PRIVATE KEY-----\ndone",
    ),
];

/// Ordinary text that recognition must leave byte-for-byte unchanged.
const ORDINARY: &[&str] = &[
    "Please refactor the tokenizer module before 10:30.",
    "Email ken@example.com and read /Users/ken/project/secret_handling.md",
    r"Open C:\Users\ken\secrets.txt and https://example.com/docs?page=2",
    "The API key rotation is done; risk-assessment-for-the-quarterly-plan next.",
    "Token: expired. Password: required!",
    "--tokens are cheap, the authorization: pending",
    "ssh://git@github.com/org/repo.git",
    "",
];

#[test]
fn corpus_masks_each_secret_and_keeps_the_rest() {
    for (input, expected) in CORPUS {
        assert_eq!(mask_secrets(input), *expected, "input: {input}");
    }
}

#[test]
fn ordinary_prose_emails_and_paths_are_unchanged() {
    for input in ORDINARY {
        assert!(matches!(mask_secrets(input), Cow::Borrowed(_)), "changed: {input}");
    }
}

#[test]
fn masking_is_idempotent() {
    for (input, _) in CORPUS {
        let once = mask_secrets(input).into_owned();
        assert_eq!(mask_secrets(&once), once, "input: {input}");
        let redactor = Redactor::for_message(input);
        let redacted = redactor.redact(input);
        assert_eq!(redactor.redact(redacted.as_str()), redacted, "input: {input}");
    }
}

#[test]
fn spans_are_merged_sorted_and_on_char_boundaries() {
    for (input, _) in CORPUS {
        let spans = find_secret_spans(input);
        for pair in spans.windows(2) {
            assert!(pair[0].end < pair[1].start, "unmerged spans in {input}: {spans:?}");
        }
        for span in &spans {
            assert!(input.is_char_boundary(span.start) && input.is_char_boundary(span.end));
        }
    }
    // Three rules recognize this token; one span remains.
    assert_eq!(
        find_secret_spans("Authorization: Bearer sk-ant-api03-AbCdEf0123456789XyZ"),
        vec![22..54]
    );
}

#[test]
fn redactor_masks_repeated_values_and_provider_echoes() {
    let message = "set password=hunter22 and then log in with hunter22";
    let redactor = Redactor::for_message(message);
    assert_eq!(
        redactor.redact(message).as_str(),
        "set password=**** and then log in with ****"
    );
    // An echo that no rule would recognize on its own.
    assert_eq!(
        redactor.redact("login failed: invalid credential 'hunter22'").as_str(),
        "login failed: invalid credential '****'"
    );
    assert_eq!(format!("{redactor:?}"), "Redactor(1 known values)");
}

#[test]
fn short_known_values_are_not_masked_elsewhere() {
    let redactor = Redactor::for_message("password=abc");
    assert_eq!(redactor.redact("abc is the alphabet").as_str(), "abc is the alphabet");
}

#[test]
fn catalog_ids_are_unique_and_token_order_is_stable() {
    let mut seen = std::collections::HashSet::new();
    for rule in SECRET_CATALOG {
        assert!(seen.insert(rule.id), "duplicate: {}", rule.id);
    }
    // Capture-time scrubbing applies these sequentially in this order.
    let token_ids: Vec<&str> = SECRET_CATALOG
        .iter()
        .filter(|rule| rule.family == SecretFamily::CredentialToken)
        .map(|rule| rule.id)
        .collect();
    assert_eq!(
        token_ids,
        [
            "openai_anthropic_key",
            "aws_access_key_id",
            "github_token",
            "slack_token",
            "bearer_token",
            "jwt"
        ]
    );
    assert_eq!(family_regexes(SecretFamily::CredentialToken).count(), 6);
    assert_eq!(family_regexes(SecretFamily::WebhookUrl).count(), 2);
}

#[test]
fn every_credential_prefix_names_a_catalog_token_shape() {
    for prefix in CREDENTIAL_TOKEN_PREFIXES {
        let matched = ["ABCDEFGHIJKLMNOP", "abcdefghij0123456789ABCD"].iter().any(|filler| {
            let value = format!("{prefix}{filler}");
            family_regexes(SecretFamily::CredentialToken).any(|regex| regex.is_match(&value))
        });
        assert!(matched, "prefix {prefix} matches no credential token rule");
        assert!(has_credential_prefix(&format!("{prefix}x")));
    }
    assert!(!has_credential_prefix("skip-this"));
    assert!(!has_credential_prefix("--model"));
}

#[test]
fn sensitive_key_names_cover_payload_and_environment_spellings() {
    for key in [
        "Authorization",
        "api_key",
        "API-KEY",
        "apiKey",
        "session_token",
        "client_secret",
        "OPENAI_API_KEY",
        "STRIPE_KEY",
        "NPM_AUTH",
        "GITHUB_PAT",
        "DB_PWD",
        "SSL_PEM",
        "DB_PASSWORD",
        "AWS_ACCESS_KEY_ID",
        "GPG_PASSPHRASE",
        "x-api-key",
        "--token",
    ] {
        assert!(is_sensitive_key_name(key), "{key}");
    }
    for key in ["PUBLIC_KEY", "SSH_AUTH_SOCK", "OLDPWD", "CWD", "PATH", "HOME", "name", "model"] {
        assert!(!is_sensitive_key_name(key), "{key}");
    }
}
