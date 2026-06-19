---
last_updated: 2026-04-27
status: draft
related:
  - spec.md
  - desktop-notification-helpers.md
---

# Leveraging Desktop Notification Helpers — Technical Design

## 1. Overview

The desktop provider currently delivers through native APIs only: `notify-rust` (Linux D-Bus), AppleScript / `UserNotifications.framework` (macOS), and `winrt-notification` (Windows). Several capabilities the public `Dispatch` API already models — interactive action buttons, reply input, sound selection, image attachments on Windows, group / replace semantics on macOS — are weakly supported or absent on those native paths.

A small set of well-known third-party CLIs fill those gaps. This feature wires them in opportunistically:

- **detect** which helpers are present on the host (via `sniff`)
- **report** detection results through `messenger info`
- **install** missing helpers through `messenger install`
- **leverage** detected helpers at send time, falling back to the existing native path

No configuration is required for the value-add; an optional `prefer_helpers` override lets callers shape election order.

## 2. Goals & Non-Goals

### Goals

1. Detect six helpers across three OSes:
    - macOS: `terminal-notifier`, `alerter`
    - Windows: `snoretoast`, `burnttoast`
    - Linux: `dunstify`, `notify-send`
2. Elect the right helper per `Dispatch`, honoring per-OS `prefer_helpers` and dispatch shape (interactive vs notice-only).
3. Send via the elected helper, falling through to the next preferred helper, then native, on failure.
4. Capture activation results (action key / reply text) synchronously into `SendReceipt.metadata`.
5. Preserve existing public types and behavior — no breaking changes to `DesktopConfig`, `Dispatch`, or `SendReceipt`.
6. Ship `messenger info` and `messenger install` as the user-facing surface.

### Non-Goals

1. Long-lived helper child processes (BurntToast pwsh pooling). Every send spawns one-shot. Revisit later if perf demands it.
2. Asynchronous activation callbacks. Activation is captured synchronously in `metadata`; no `tokio::mpsc` channel surface.
3. `kdialog` integration. Research deems it strictly less capable than `notify-send` for the notification path. The separate "messenger dialog" command — also out of scope here — is the right home for kdialog later.
4. Bundling helper binaries with the messenger CLI. Users install via `messenger install` (which delegates to `sniff`).
5. Replacing the existing native backends. They remain the universal fallback.

## 3. Architecture Overview

```
                       Dispatch + PreparedMessage
                                  │
                                  ▼
                      DesktopNotificationProvider
                                  │
                                  ▼
                          {Linux,MacOs,Windows}Backend
                                  │
                       ┌──────────┴──────────┐
                       │ elect_helper(req)?  │
                       │  ─ preferred order  │
                       │  ─ dispatch shape   │
                       │  ─ helper presence  │
                       └──────────┬──────────┘
                                  ▼
                  ┌──── HelperBackend trait ────┐
                  │                              │
   ┌──────────────┼─────────┬──────────┬────────┼──────────────┐
   ▼              ▼         ▼          ▼        ▼              ▼
TerminalNotifier  Alerter  SnoreToast  BurntToast  Dunstify  NotifySend
   │              │         │          │        │              │
   └──────────────┴─────────┴──────────┴────────┴──────────────┘
                                  │
                          on failure / unsupported
                                  ▼
                       native API (existing path)
```

`HelperBackend` is the new abstraction. Each platform backend gains an `helpers: Vec<Arc<dyn HelperBackend>>` field plus an election method. Native delivery remains the floor.

## 4. Detection Layer (`sniff` Changes)

### 4.1 New programs subcategory

Add `notification_helpers` to sniff's `ProgramsInfo`, alongside `editors`, `agents`, etc.

```rust
// sniff/lib/src/programs/notification_helpers.rs
pub struct NotificationHelpersInfo {
    pub helpers: Vec<NotificationHelper>,
    pub active_daemon: Option<NotificationDaemon>,  // Linux only
}

pub struct NotificationHelper {
    pub name: NotificationHelperName,
    pub installed: bool,
    pub path: Option<PathBuf>,
    pub version: Option<String>,
    pub install_hint: InstallHint,
}

pub enum NotificationHelperName {
    TerminalNotifier,
    Alerter,
    SnoreToast,
    BurntToast,
    Dunstify,
    NotifySend,
}

pub struct NotificationDaemon {
    pub name: String,         // "dunst", "GNOME Shell", "mako", "Plasma"
    pub vendor: Option<String>,
    pub version: Option<String>,
}
```

### 4.2 Detection mechanics

- **PATH probe** via the existing `ExecutableIndex`. Cost is amortized across all sniff program categories.
- **macOS bundle probe** is unnecessary — terminal-notifier and alerter are CLI binaries, not `.app` bundles.
- **Version probe** invokes the binary with the canonical version flag. Failures degrade to `installed: true, version: None`:

| Helper | Version command |
|---|---|
| `terminal-notifier` | `terminal-notifier -help` (parse first line) |
| `alerter` | `alerter -help` (parse trailing `version` line) |
| `snoretoast` | `snoretoast -v` |
| `dunstify` | `dunstify --version` |
| `notify-send` | `notify-send --version` |
| `burnttoast` | `pwsh -NoProfile -Command "(Get-Module -ListAvailable BurntToast).Version.ToString()"` |

- **BurntToast presence** is a PowerShell module check, not a binary on PATH:
  `pwsh -NoProfile -Command "if (Get-Module -ListAvailable BurntToast) { 'yes' } else { 'no' }"`. Cache result per process (cold start matters).
- **Active Linux daemon** uses zbus to call `org.freedesktop.Notifications.GetServerInformation`. Returns `(name, vendor, version, spec_version)`. Required to elect dunstify safely (dunstify's `--wait`/`-A` only work end-to-end when dunst owns the bus). If zbus fails, `active_daemon = None` and dunstify is not elected.

### 4.3 InstallHint

```rust
pub struct InstallHint {
    pub method: InstallMethod,           // Brew, AptGet, PacmanS, Choco, PSGet, …
    pub package: String,                 // "terminal-notifier", "BurntToast", …
    pub command: String,                 // pre-rendered for display
    pub elevation: ElevationRequired,    // None / Sudo / Admin
}
```

Sniff already emits these for editors and agents; this is just another consumer.

### 4.4 Sniff CLI

```
sniff software notification-helpers           # text output
sniff software notification-helpers --json    # JSON
```

Exposed in the software enum so `sniff software` shows it alongside editors / agents.

### 4.5 Sniff API surface used by messenger

```rust
use sniff::programs::notification_helpers::{
    detect_notification_helpers, NotificationHelpersInfo,
};

let info: NotificationHelpersInfo = detect_notification_helpers()?;
```

Messenger does not duplicate the catalog. The `NotificationHelperName` enum is **defined in sniff** and re-exported from messenger so callers can `use messenger::desktop::HelperName`.

## 5. The `HelperBackend` Trait

Lives at `messenger/lib/src/provider/desktop/helpers/mod.rs`:

```rust
#[async_trait::async_trait]
pub(crate) trait HelperBackend: Send + Sync {
    fn name(&self) -> HelperName;

    /// What the helper can deliver. Used by `elect_helper` to filter on
    /// dispatch shape (interactivity, image, replace).
    fn capabilities(&self) -> HelperCapabilities;

    /// Per-send fitness for this dispatch.
    /// `0` = cannot serve, higher = better fit.
    fn score(&self, request: &DesktopNotificationRequest) -> u8;

    async fn send(
        &self,
        request: &DesktopNotificationRequest,
    ) -> Result<DesktopNotificationReceipt, HelperError>;

    /// Default impl returns HelperError::Unsupported.
    async fn replace(
        &self,
        _id: &str,
        _request: &DesktopNotificationRequest,
    ) -> Result<DesktopNotificationReceipt, HelperError> {
        Err(HelperError::Unsupported("replace"))
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct HelperCapabilities {
    pub actions: bool,
    pub reply: bool,
    pub image: bool,
    pub sound: bool,
    pub replace: bool,
    pub group: bool,
    pub blocking: bool,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum HelperError {
    #[error("helper not present on PATH")]
    NotPresent,
    #[error("helper does not support {0}")]
    Unsupported(&'static str),
    #[error("helper exited with status {status}: {stderr}")]
    Exited { status: i32, stderr: String },
    #[error("helper invocation timed out after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },
    #[error("helper output unparseable: {0}")]
    Parse(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
```

Each helper is a `pub(crate) struct` implementing the trait. Construction takes presence info from sniff and any helper-specific config.

### 5.1 Per-helper specifications

#### 5.1.1 `terminal-notifier` (macOS)

```rust
pub(crate) struct TerminalNotifierHelper {
    path: PathBuf,
}
```

| Concern | Detail |
|---|---|
| Capabilities | `image, sound, replace, group, blocking=false`. **No** `actions`/`reply`. |
| `score` | Returns `0` if `request.actions.is_some_and(non_empty) || request.reply_hint`. Else `80` (high, default macOS choice for notice-only). |
| Argv | `-title`, `-subtitle`, `-message`, `-appIcon`, `-contentImage`, `-sound`, `-group`, `-remove` (replace), `-ignoreDnD` (when `urgency = Critical`), `-open URL` when `metadata.click_url` present |
| Sound | `request.sound.as_deref().unwrap_or("default")`. Map `urgency=Critical` → `"Basso"`, `Low` → suppress with no `-sound` flag. |
| Replace | `terminal-notifier -group <id> -remove <id>` then re-send. Returns the same `id`. |
| Receipt id | The `-group` value, or a generated UUID if absent. |
| Timeout | 5s. |

#### 5.1.2 `alerter` (macOS)

```rust
pub(crate) struct AlerterHelper {
    path: PathBuf,
}
```

| Concern | Detail |
|---|---|
| Capabilities | `actions, reply, image, sound, blocking=true`. **No** `replace`/`group`. |
| `score` | Returns `90` when `request.actions.is_some_and(non_empty) || request.reply_hint`. Else `30` (still usable but blocking). |
| Argv | `--title`, `--subtitle`, `--message`, `--appIcon`, `--contentImage`, `--sound`, `--actions "id1\|Label1,id2\|Label2"` (we encode id+label, then split on `|` after parse), `--reply "Reply…"`, `--closeLabel "Dismiss"`, `--json` |
| JSON parse | `{"activationType": "actionClicked"\|"replied"\|"closed"\|"contentClicked"\|"timeout", "activationValue": "<id-or-text>"}` |
| Receipt | `notification_id` = generated UUID; `metadata`: `activation_type`, `activation_key` (for actions), `reply_text` (for replies). |
| Replace | Returns `HelperError::Unsupported("replace")`. |
| Timeout | None at the `tokio::time::timeout` layer when `actions`/`reply` are present (user controls duration). 60s ceiling for notice-only. |

#### 5.1.3 `snoretoast` (Windows)

```rust
pub(crate) struct SnoreToastHelper {
    path: PathBuf,
    app_id: String,                 // required, supplied by WindowsDesktopConfig.app_id
}
```

| Concern | Detail |
|---|---|
| Capabilities | `actions, reply, image, sound, replace, group, blocking=true` |
| `score` | Returns `90` (default Windows choice). |
| Argv | `-appID <id>`, `-t <title>`, `-m <body>`, `-p <png-path>`, `-s <sound-name>`, `-b "Yes;No"` (semicolon-separated; ids and labels collapsed to label-only — see below), `-tb` (text input — mutually exclusive with `-b`), `-id <replace-id>`, `-d short\|long` |
| Action id ↔ label | snoretoast returns the **button text** on stdout, not an id. Since we need stable ids, we rebuild the mapping client-side: we send `-b "Label1;Label2"` and remember the `id_by_label: BTreeMap<&str, &str>` table. After exit, look up the stdout label to recover the id. Ambiguous duplicate labels are rejected at `score()` time (return `0` so the next helper takes over). |
| Image | PNG only, ≤ 1024×1024, ≤ 200 KB. We pre-validate; oversize → drop image with a `metadata["dropped"]="image_too_large"` marker, do not fail the send. |
| Exit code | `0` activated, `1` hidden, `2` dismissed, `3` timed out, `4` button pressed (label on stdout), `5` text entered (text on stdout), `-1` failure. |
| Receipt | `notification_id = request.replace_id.clone().unwrap_or_else(uuid)`; metadata: `exit_code`, `activation_type` (mapped enum), `activation_key`, `reply_text`. |
| Timeout | 5s for notice-only; none for interactive. |
| AppID registration | See §5.2. |

#### 5.1.4 `burnttoast` (Windows)

```rust
pub(crate) struct BurntToastHelper {
    pwsh_path: PathBuf,
    app_id: String,
}
```

| Concern | Detail |
|---|---|
| Capabilities | `actions, reply, image, sound, replace, group, blocking=true` |
| `score` | Returns `40` — used when SnoreToast is absent OR when the dispatch needs hero image / scenario / progress that SnoreToast lacks. |
| Argv | `pwsh -NoProfile -NonInteractive -Command -` then we pipe a generated PowerShell script over stdin. The script is templated from `request` fields (escape via single-quote-doubling for safety). |
| Activation capture | We embed an `OnActivated` handler in the script that writes `__MESSENGER_ACTIVATION__\t<json>` to stdout, then exits the runspace. Our parser scans stdout lines, picks the marker line, decodes the JSON. |
| Receipt | UUID; metadata: `helper="burnttoast"`, `activation_type`, `activation_key`, `reply_text`. |
| Replace | BurntToast `-UniqueIdentifier <id>` + `Update-BTNotification` route. Returns same id. |
| Cold start | ~300ms. Acceptable v1 cost. Future: long-lived `pwsh` host with stdin command framing (out of scope). |
| Timeout | 10s for notice-only; none for interactive. |

#### 5.1.5 `dunstify` (Linux)

```rust
pub(crate) struct DunstifyHelper {
    path: PathBuf,
    daemon_is_dunst: bool,         // populated from sniff::active_daemon
}
```

| Concern | Detail |
|---|---|
| Capabilities | `actions, image, sound (via dunst rules), replace, group (via stack-tag), blocking=optional` |
| `score` | Returns `0` if `!self.daemon_is_dunst` (active daemon is not dunst — actions won't round-trip). Else `90` when actions present, `70` for notice-only (still usable, gives `--wait` close-reason if interesting). |
| Argv | `--appname`, `--icon`, `--urgency low\|normal\|critical`, `--timeout`, `--hint string:sound-name:<n>`, `--stack-tag <group>`, `--action "id,Label"` (one flag per action), `--replace <id>`, `--printid` (capture id on stdout), `--wait` when actions present |
| stdout parse | First line: notification id (`--printid`). Then on `--wait`: a second line with the chosen action key, OR exit code (1 dismissed, 2 timeout, 3 closed by call, 4 undefined). |
| Receipt | `notification_id` = id from `--printid`; metadata: `activation_key`, `close_reason` when `--wait` was used. |
| Replace | `dunstify --replace <id>`. Returns same id. |
| Timeout | None when `--wait` (interactive); 3s otherwise (it's local D-Bus). |

#### 5.1.6 `notify-send` (Linux)

```rust
pub(crate) struct NotifySendHelper {
    path: PathBuf,
    libnotify_version: Option<semver::Version>,
}
```

| Concern | Detail |
|---|---|
| Capabilities | `actions` (only when `libnotify_version >= 0.7.8` and active daemon advertises actions), `image, sound, replace`. **No** `reply`. |
| `score` | Returns `60` (universal Linux default). Drops to `40` if dispatch has actions but version is < 0.7.8 (still usable, actions get dropped with a warning metadata flag). |
| Argv | `--app-name`, `--icon`, `--urgency`, `--expire-time`, `--hint string:sound-name:<n>`, `-A "id=Label"` (one flag per action; libnotify 0.7.8+), `-r <id>`, `-p` (print id), `--selected-action-fd 1` |
| Pango markup | `body` may include `<b>`, `<i>`, `<a href>` from the existing markdown→plain renderer. Daemon-dependent. |
| Receipt | id from `-p` stdout; metadata: `activation_key` if `--selected-action-fd 1` returned a key. |
| Replace | `--replace-id <id>`. |
| Timeout | 5s. |

### 5.2 Windows AppID auto-registration

`WindowsBackend::new(config)` runs once at construction time:

1. Require `config.app_id` to be set when ANY Windows helper is going to be elected. If unset, log a warning and force the election to skip helpers (native winrt-notification path used).
2. If snoretoast is present:
    - Run `snoretoast.exe -appID <id> -install <id>.lnk %~dp0\\snoretoast.exe <id>` (mirrors the canonical snoretoast install pattern).
    - Cache "registered" state in `OnceCell<bool>` keyed by `(app_id, helper)` so reconstructing the provider in the same process is free.
3. If BurntToast is present:
    - Run `pwsh -NoProfile -Command "New-BTAppId -AppId '<id>'"` once.
    - Same caching.

Errors during registration are non-fatal: they downgrade the helper's `score()` to `0` for the lifetime of the provider and emit a `tracing::warn!`.

### 5.3 Helper-side argv-builder responsibility

Each helper struct owns argv construction in a single `fn build_args(&self, request: &DesktopNotificationRequest) -> Vec<OsString>` method, separated from `send()`. This is the unit-testable seam: argv is asserted in unit tests; `send()` is exercised in the stub-helper integration tests (§9.2).

## 6. Backend Integration

### 6.1 Backend extension

The three platform backends already implement `DesktopBackend`. We extend each to hold a `Vec<Arc<dyn HelperBackend>>` plus the platform's `prefer_helpers` order:

```rust
pub(crate) struct LinuxBackend {
    config: LinuxDesktopConfig,
    native: Box<dyn LinuxNativeAdapter>,         // existing notify-rust path
    helpers: Vec<Arc<dyn HelperBackend>>,        // populated at construction
}
```

Same shape for `MacOsBackend` and `WindowsBackend`.

### 6.2 Construction

Each backend's `new()`:

1. Calls `sniff::detect_notification_helpers()` once.
2. Filters to helpers relevant to the OS.
3. Constructs each helper from its detection record (path, version, daemon-info).
4. Sorts the resulting `Vec` by `prefer_helpers` first (callers take precedence), then by the static default order from §7.1.

`sniff` detection happens in a blocking call here (provider construction is sync in `Messenger::register`). Detection takes O(20ms) on a warm `ExecutableIndex` — acceptable for a one-time cost. We do **not** call sniff again per send.

### 6.3 Send path

```rust
impl DesktopBackend for LinuxBackend {
    async fn send(&self, request: DesktopNotificationRequest)
        -> Result<DesktopNotificationReceipt, MessengerError>
    {
        let mut attempts = Vec::<HelperAttempt>::new();

        for helper in self.helpers.iter() {
            if helper.score(&request) == 0 { continue; }
            match helper.send(&request).await {
                Ok(mut receipt) => {
                    receipt.metadata.insert("helper_used".into(), helper.name().to_string());
                    if !attempts.is_empty() {
                        receipt.metadata.insert(
                            "helper_fallbacks".into(),
                            attempts.iter().map(|a| a.summary()).collect::<Vec<_>>().join(","),
                        );
                    }
                    return Ok(receipt);
                }
                Err(err) => {
                    tracing::warn!(helper = %helper.name(), %err, "helper send failed, falling through");
                    attempts.push(HelperAttempt::from(helper.name(), &err));
                }
            }
        }

        // Native fallback
        let mut receipt = self.native.send(request).await?;
        receipt.metadata.insert("helper_used".into(), "native".into());
        if !attempts.is_empty() {
            receipt.metadata.insert(
                "helper_fallbacks".into(),
                attempts.iter().map(|a| a.summary()).collect::<Vec<_>>().join(","),
            );
        }
        Ok(receipt)
    }

    async fn replace(&self, id: &str, request: DesktopNotificationRequest)
        -> Result<DesktopNotificationReceipt, MessengerError>
    {
        let helper_used = request.replace_helper_hint.as_deref();
        // route to the same helper that produced the original receipt when known;
        // else iterate helpers that advertise `replace`; else native.
        ...
    }
}
```

`HelperAttempt` is a tiny struct: `{ name: HelperName, error: String }` with `summary()` returning `"alerter:timeout"` etc.

### 6.4 `replace_helper_hint` on the request

So that `provider.replace(receipt, …)` routes to the same helper that produced the original receipt, we add to `DesktopNotificationRequest`:

```rust
pub struct DesktopNotificationRequest {
    // … existing fields …
    /// Hint set by the caller via SendReceipt.metadata["helper_used"];
    /// the backend prefers this helper for replace() routing.
    pub replace_helper_hint: Option<HelperName>,
}
```

`build_request()` (in `desktop/mod.rs`) reads `dispatch.overrides.metadata` if present; the higher-level `provider.replace()` injects `helper_used` from the caller's receipt.

`dismiss()` is unchanged. Helpers do not currently support cross-process dismissal of an already-delivered notification; we keep the existing macOS-native dismiss path and continue returning `UnsupportedFeature` elsewhere. See §11.

## 7. Election Algorithm

### 7.1 Default election order

| OS | Order (highest first) | Notes |
|---|---|---|
| macOS | `terminal-notifier`, `alerter` | `alerter` jumps to first when `actions` or reply hint present |
| Windows | `snoretoast`, `burnttoast` | snoretoast preferred for cold-start + exit-code simplicity |
| Linux | `dunstify` (only if `daemon == dunst`), `notify-send` | dunstify gated on daemon; notify-send always works |

### 7.2 Scoring

Election is `argmax score(helper, request)` across present helpers, breaking ties by the configured `prefer_helpers` order, then by the default order above. Score `0` means "cannot serve this request" and is filtered out.

Key score rules:

- **Interactive dispatches** (`!request.actions.is_empty() || request.reply_hint`):
    - alerter → `90`; snoretoast → `90`; burnttoast → `80`; dunstify → `90` (if daemon==dunst); notify-send → `60` (libnotify ≥ 0.7.8) or `0` (older).
    - terminal-notifier → `0` (cannot serve).
- **Notice-only**:
    - terminal-notifier → `80`; alerter → `30` (works but blocks).
    - snoretoast → `90`; burnttoast → `40`.
    - dunstify → `70` (if dunst); notify-send → `60`.

A helper that explicitly fails detection (binary missing, wrong daemon) is omitted from the vec entirely — `score()` is never called on it.

### 7.3 Caller override

`prefer_helpers` is consulted **before** scoring: if the caller lists `[alerter, terminal-notifier]`, alerter wins ties (and beats terminal-notifier when both score equally). It does **not** override `score == 0` — a caller asking for `alerter` on a notice-only path still gets it (alerter's notice-only score is non-zero).

`prefer_helpers` may name helpers from other OSes; those entries are silently ignored on the wrong host.

## 8. Configuration Changes

### 8.1 Per-OS struct extensions

```rust
// LinuxDesktopConfig
pub struct LinuxDesktopConfig {
    pub desktop_entry: Option<String>,             // existing
    pub prefer_helpers: Vec<HelperName>,           // NEW
}

// MacOsDesktopConfig
pub struct MacOsDesktopConfig {
    pub bundle_id: Option<String>,                  // existing
    pub strategy: MacOsNotificationStrategy,        // existing
    pub prefer_helpers: Vec<HelperName>,           // NEW
}

// WindowsDesktopConfig
pub struct WindowsDesktopConfig {
    pub app_id: Option<String>,                     // existing
    pub prefer_helpers: Vec<HelperName>,           // NEW
}
```

`HelperName` re-exported from `messenger::desktop::HelperName` (defined in sniff, shared type).

### 8.2 CLI config schema (TOML)

```toml
[desktop.linux]
desktop_entry = "messenger"
prefer_helpers = ["dunstify", "notify-send"]

[desktop.macos]
bundle_id = "net.ken.messenger"
strategy = "auto"
prefer_helpers = ["alerter", "terminal-notifier"]

[desktop.windows]
app_id = "net.ken.messenger"
prefer_helpers = ["snoretoast"]
```

The CLI's existing config loader (`messenger/cli/src/config.rs`) gains three optional fields with `serde(default)`. Empty `prefer_helpers` means "use library default order".

### 8.3 Environment variable override

For one-off overrides without touching the config file:

```
MESSENGER_DESKTOP_PREFER_HELPERS="dunstify,notify-send"
```

Read once at CLI startup; merged into the per-OS list ahead of the config-file value. Library callers do not see this — it lives in the CLI layer only.

## 9. CLI Commands

### 9.1 `messenger info`

```
messenger info [--json | --plain]
```

Output sections (text mode, rendered with `biscuit-terminal` `Prose`):

```
Host
  OS              macOS 14.4 (Sonoma)
  Active daemon   —                              # Linux only
  bundle id       net.ken.messenger              # macOS only
  app id          net.ken.messenger              # Windows only

Notification Helpers
  ┌────────────────────┬───────────┬──────────┬───────────────────────────────────┐
  │ helper             │ installed │ version  │ install hint                      │
  ├────────────────────┼───────────┼──────────┼───────────────────────────────────┤
  │ terminal-notifier  │   yes     │ 2.0.0    │ —                                 │
  │ alerter            │   no      │ —        │ brew install vjeantet/tap/alerter │
  └────────────────────┴───────────┴──────────┴───────────────────────────────────┘

Election order (this host)
  1. terminal-notifier   (notice-only, default)
  2. alerter             (interactive dispatches)
  3. native AppleScript  (fallback)

Configured Routes
  desktop                local desktop notifications
  slack:dev              SLACK_BOT_TOKEN_DEV → #dev
  …
```

JSON mode is a flat record matching the same data; consumed by tests for stable assertions.

### 9.2 `messenger install`

```
messenger install [--yes] [--helper <name>]…
```

Behavior:

1. Run sniff detection; build a list of helpers with `installed: false`.
2. If `--helper` given, restrict to those names.
3. If `--yes` not given, present an interactive `inquire::MultiSelect` of installable helpers, with capability summaries shown beside each name.
4. Print the install plan: each line is the rendered `InstallHint.command`, with elevation badges (`[sudo]`, `[admin]`).
5. Confirm with `inquire::Confirm` (skipped under `--yes`).
6. Execute via sniff's existing install-with-consent pipeline. Stream stdout/stderr to the user.
7. On completion, re-detect and print the updated `messenger info` table.

`messenger install` does not modify the messenger config file. Once installed, election picks helpers up automatically on the next provider construction.

### 9.3 Help / discoverability

Both subcommands appear in the top-level `messenger --help`. `messenger info` is also invoked implicitly at the end of `messenger setup desktop` so users see what they just gained.

## 10. Receipt & Activation Surface

`SendReceipt.metadata` (a `BTreeMap<String, String>`) carries the helper-related signal. Keys:

| Key | When set | Meaning |
|---|---|---|
| `helper_used` | always | `"terminal-notifier"`, `"alerter"`, …, or `"native"` |
| `helper_fallbacks` | when ≥ 1 helper failed before success | Comma-separated `helper:reason` summaries |
| `activation_type` | interactive sends only | `"action"`, `"reply"`, `"dismissed"`, `"timeout"`, `"content_clicked"` |
| `activation_key` | when user picked an action | The `NotificationAction.id` mapped from helper output |
| `reply_text` | when user typed a reply | The raw text the user entered |
| `close_reason` | dunstify `--wait` only | `"dismissed"`, `"timeout"`, `"closed_by_call"`, `"undefined"` |
| `dropped` | best-effort feature drop | e.g. `"image_too_large"`, `"actions_libnotify_old"` |
| `platform` | always (existing behavior) | `"linux"`, `"macos"`, `"windows"` |

Existing receipt structure is unchanged — we are only adding new keys to the existing `metadata` map.

### 10.1 Public helpers on `SendReceipt`

To avoid making callers stringly-typed, add three convenience accessors:

```rust
impl SendReceipt {
    pub fn helper_used(&self) -> Option<&str> { self.metadata.get("helper_used").map(String::as_str) }
    pub fn activation(&self) -> Option<Activation> { /* parse activation_type + value */ }
    pub fn reply_text(&self) -> Option<&str> { self.metadata.get("reply_text").map(String::as_str) }
}

pub enum Activation<'a> {
    Action(&'a str),
    Reply(&'a str),
    Dismissed,
    Timeout,
    ContentClicked,
}
```

These are the public surface for activation; metadata stays the source of truth.

## 11. Replace / Dismiss Behavior

### 11.1 Replace

- The original `SendReceipt` carries `metadata["helper_used"]`. The provider's `replace()` reads it and routes the new request to the same helper instance via the platform backend.
- If the original used a helper that doesn't support replace (e.g. alerter), `replace()` returns `MessengerError::UnsupportedFeature { provider: Desktop, feature: "notification replacement (alerter)" }`.
- If the original used `"native"`, behavior is unchanged from today.
- If the helper used to send the original is no longer present at replace time (rare — user uninstalled mid-session), we error with `UnsupportedFeature`. We do not silently re-elect a different helper, because `notification_id` semantics are helper-specific.

### 11.2 Dismiss

`dismiss()` is **not** extended in this feature. terminal-notifier, snoretoast, dunstify, and notify-send do not expose a "dismiss already-delivered notification" API in the way the provider's `dismiss()` semantics demand. dismiss continues to:

- macOS native (`UserNotifications.framework`) — works.
- All other paths — return `MessengerError::UnsupportedFeature`.

This matches the pre-feature behavior and keeps the surface honest.

## 12. Error Handling & Fallback

### 12.1 Fallback ladder

```
elected helper → next preferred helper → … → native backend → MessengerError::ProviderError
```

Fallback triggers only on:

- `HelperError::NotPresent` (race: detected at construction, gone at send time)
- `HelperError::Exited { status: nonzero }`
- `HelperError::Timeout`
- `HelperError::Io(_)`

It does **not** trigger on `HelperError::Unsupported(_)` or `HelperError::Parse(_)` — those are programmer errors and surface to the caller as `MessengerError::ProviderError` after exhausting the ladder. (Parse errors mean we're misreading helper output; better to fail loudly than silently downgrade.)

### 12.2 User-visible errors

`MessengerError::ProviderError { provider: Desktop, message }` carries a multi-line `message` summarizing every attempt: `"alerter: timed out after 60000ms; terminal-notifier: exit 1: <stderr first line>; native: …"`. Consumers (CLI especially) render this verbatim.

### 12.3 Tracing

Each helper send emits a `tracing::debug` span with `helper`, `request.title.len`, `request.body.len`, `interactive`, `image_present`. Failures emit `tracing::warn`. `tracing::info` is reserved for the final outcome (which helper succeeded).

## 13. File / Module Inventory

### 13.1 New files (messenger)

```
messenger/lib/src/provider/desktop/
  helpers/
    mod.rs                    # HelperBackend trait, HelperName, HelperCapabilities, HelperError
    terminal_notifier.rs      # TerminalNotifierHelper
    alerter.rs                # AlerterHelper
    snoretoast.rs             # SnoreToastHelper + AppID registration
    burnttoast.rs             # BurntToastHelper + pwsh script template
    dunstify.rs               # DunstifyHelper
    notify_send.rs            # NotifySendHelper
    election.rs               # elect_helper(), HelperAttempt, default order tables
    process.rs                # spawn_helper(), output parsing utilities, timeout wrapper
```

### 13.2 Modified files (messenger)

```
messenger/lib/src/provider/desktop/
  mod.rs                      # provider construction: detect helpers, assemble per-OS lists
  backend.rs                  # (no signature change; doc updates only)
  request.rs                  # add `replace_helper_hint: Option<HelperName>` field
  linux.rs                    # LinuxBackend gains helpers vec; send/replace iterate
  macos.rs                    # MacOsBackend gains helpers vec; send/replace iterate
  windows.rs                  # WindowsBackend gains helpers vec; AppID registration

messenger/lib/src/dispatch.rs # (no public change; private DesktopOverrides untouched)
messenger/lib/src/receipt.rs  # add SendReceipt::helper_used / activation / reply_text accessors

messenger/cli/src/main.rs     # wire `info`, `install` subcommands
messenger/cli/src/info.rs     # NEW — implements `messenger info`
messenger/cli/src/install.rs  # NEW — implements `messenger install`
messenger/cli/src/config.rs   # parse prefer_helpers from TOML
```

### 13.3 New files (sniff)

```
sniff/lib/src/programs/notification_helpers/
  mod.rs                      # NotificationHelpersInfo, detect_notification_helpers
  daemon.rs                   # zbus probe of org.freedesktop.Notifications.GetServerInformation
  helpers.rs                  # per-helper detection (path, version, install hint)

sniff/cli/src/commands/notification_helpers.rs  # NEW — `sniff software notification-helpers`
```

### 13.4 Modified files (sniff)

```
sniff/lib/src/programs/mod.rs # register notification_helpers as a category
sniff/lib/src/lib.rs          # re-export NotificationHelperName, NotificationHelpersInfo
sniff/cli/src/main.rs         # subcommand wiring
```

## 14. Testing Strategy

### 14.1 Unit tests (per helper)

For each `*Helper`:

- `build_args` — given a fully-populated `DesktopNotificationRequest`, assert the exact argv (snapshot). Cover:
    - Notice-only
    - With image (path, oversized PNG for snoretoast → dropped)
    - With actions
    - With reply hint
    - With replace_id
    - With urgency=Critical / Low / Normal
- `score` — table-driven: each combination of `(actions empty/non-empty, reply_hint, daemon)` → expected score.
- `parse_output` (where applicable) — feed canned stdout/exit-code, assert `DesktopNotificationReceipt` shape (alerter JSON, snoretoast exit codes, dunstify printid+key).

### 14.2 Election tests

In `desktop/helpers/election.rs`:

- Given a vec of helpers and a request, returns the right order.
- `prefer_helpers` reorders ties.
- Score-zero helpers are filtered out.
- Empty helper vec falls through to native.

### 14.3 Stub-helper integration tests

Under `messenger/lib/tests/desktop_helpers.rs`. Each test:

1. Builds a tiny Rust binary (`tests/bin/stub_terminal_notifier/main.rs`, etc.) at compile time that prints canned output and exits with a configured code. Stub binaries are placed under `target/test-helpers/<name>/` via a `build.rs`.
2. Constructs a `LinuxBackend`/`MacOsBackend`/`WindowsBackend` with the test PATH pointing at the stub directory.
3. Sends a request, asserts `SendReceipt.metadata["helper_used"]` and any activation fields.
4. Tests cover: success path, non-zero exit → fallback to next helper, timeout → fallback, parse error → surfaced.

CI runs these on Linux only (cross-OS PATH sandboxing is finicky); macOS and Windows runners run them when we have those CI runners. The stubs are platform-agnostic Rust binaries — there is no real OS notification call.

### 14.4 CLI snapshot tests

Under `messenger/cli/tests/info_snapshot.rs` and `install_snapshot.rs`, using `insta` (already in the workspace). Stub the sniff result with a fixed JSON fixture; assert `messenger info --plain --json` matches the snapshot.

### 14.5 Daemon-detection test

`sniff/lib/tests/notification_daemon.rs`:

- Uses `zbus`'s test fixture to stand up a fake `org.freedesktop.Notifications` service.
- Verifies that `detect_notification_helpers()` reports the right `active_daemon` name.
- Linux-only `#[cfg(target_os = "linux")]`.

### 14.6 What we do *not* test

- Real OS notification center delivery. The existing native-backend tests already cover that.
- BurntToast on a CI runner. We unit-test the script template; integration test only via stub binaries.

## 15. Implementation Phases

Each phase is independently mergeable. Reviews after each phase.

### Phase 1 — Sniff detection

- New `sniff::programs::notification_helpers` module.
- `sniff software notification-helpers` CLI subcommand.
- Tests for path/version/daemon detection.

### Phase 2 — Helper trait + Linux helpers

- `HelperBackend` trait, `HelperError`, `HelperCapabilities`, `process.rs`.
- `dunstify`, `notify-send` helpers + unit tests.
- `LinuxBackend` extended with helpers vec and election.
- Linux stub-helper integration test.

### Phase 3 — macOS helpers

- `terminal-notifier`, `alerter` helpers + unit tests.
- `MacOsBackend` extended.
- macOS-flagged stub-helper test (skipped on non-macOS CI).

### Phase 4 — Windows helpers

- `snoretoast`, `burnttoast` helpers + AppID registration.
- `WindowsBackend` extended.
- Windows-flagged stub-helper test.

### Phase 5 — CLI surface

- `messenger info` with text/JSON/plain modes.
- `messenger install` with sniff install integration.
- TOML config parsing for `prefer_helpers`.
- `MESSENGER_DESKTOP_PREFER_HELPERS` env var.
- Snapshot tests.

### Phase 6 — Receipt convenience API

- `SendReceipt::helper_used`, `activation`, `reply_text` methods.
- `Activation` enum.
- Documentation pass on `messenger/docs/user-guide.md`.

## 16. Risks & Open Items

### 16.1 Risks

| Risk | Mitigation |
|---|---|
| BurntToast pwsh cold start makes interactive Windows sends sluggish | Phase 4 scope is one-shot only; document the 200-400ms cost. Pooling deferred. |
| dunstify mis-electing on a non-dunst host (user has dunst installed but a different daemon is the bus owner) | Daemon check via zbus `GetServerInformation` is gating; helper score returns 0 unless daemon is dunst. |
| AppID auto-registration interferes with apps that already register the same id | Cache registration in `OnceCell` keyed by `(app_id, helper)`. Failures non-fatal. Document the behavior. |
| snoretoast PNG size limit drops images silently from the user's POV | Surface via `metadata["dropped"]="image_too_large"`; CLI `messenger info` documents the limit. |
| sniff detection adding noticeable startup cost when many helpers are checked | Detection runs once per provider construction; uses sniff's shared `ExecutableIndex`. Measured cost target: ≤ 50ms on a warm cache. |
| Action labels with duplicate text on snoretoast lose id mapping | snoretoast helper's `score()` returns 0 in this case; election picks BurntToast or native. |
| Helper version drift (e.g. alerter changes from `-` to `--` flags between releases) | Pin tested behavior in unit tests; helper-specific argv builders are the single change point. |

### 16.2 Open items deferred to follow-ups

1. Long-lived BurntToast pwsh host with stdin-framed commands.
2. `kdialog` integration as part of a separate `messenger dialog` interactive-prompt command.
3. Asynchronous activation surface (`subscribe_activations() -> Receiver<Activation>`).
4. Headless / tty-only fallback (no notification center available — currently we just fail).
5. Helper auto-update via `messenger install --upgrade`.
