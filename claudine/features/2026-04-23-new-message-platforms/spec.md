# Feature: New Message & Notification Platforms

This feature expands Claudine's outbound communication capabilities by introducing new messaging webhooks and integrating native desktop notifications.

Crucially, **Desktop Notifications** and **Messaging Platforms** are treated as distinct, separate concerns.

## 1. Messaging Platforms (Discord & Slack Webhooks)

The `messenger` package is gaining support for webhook-based providers, which are handled as remote messaging platforms.

- **Providers:**
  - Discord Webhook
  - Slack Webhook
- **Configuration (`claudine config` TUI):**
  - **Masked Input:** Webhook URLs and secrets must use password-style masked input to prevent accidental exposure during configuration.
  - **URL Validation:** The TUI must perform regex validation on the provided URLs to ensure they match the expected provider format (e.g., `https://discord.com/api/webhooks/...`).
  - **Test Connection:** A 'Test Connection' button must be available in the configuration UI to verify the webhook's validity before saving.
- **Trigger:**
  - Activated via the existing `message` field in composition lifecycle frontmatter or via `HookAction::Message`.

## 2. Desktop Notifications (Notifications Concern)

Desktop notifications are a separate concern from messaging providers and are used for local user alerts.

- **Lifecycle Frontmatter Integration:**
  - A new `notify` field (`Option<String>`) is added to the lifecycle notification structs (used in `start`, `success`, `blocked`, and `failure` states).
  - This allows concurrent routing: a remote message can be sent to a team while a local notification alerts the user.
  - Example:
    ```yaml
    ---
    success:
      say: "Workflow complete"
      message: "Production deployment finished" # Remote (Discord/Slack)
      notify: "Deployment Successful"           # Local Desktop Notification
    ---
    ```
- **Configuration (`claudine config` TUI):**
  - **No TUI Controls:** There is explicitly no "Enable" toggle or driver selection in the `claudine config` TUI.
  - **Zero-Config:** Desktop notifications require no user configuration to function.
- **Implementation:**
  - Logic for driver auto-detection (e.g., AppleScript vs. Alerter on macOS) lives entirely within the `messenger` library. `claudine` is agnostic to the underlying OS implementation.

## 3. Lifecycle Integration & Delegation

`claudine`'s lifecycle management is updated to support the new `notify` field:

1.  **Parsing:** `claudine` parses the `notify` field from composition frontmatter into its internal lifecycle structs.
2.  **Delegation:** During lifecycle transitions, `claudine` delegates the execution of `notify` actions to the `messenger` library's desktop notification capability.
3.  **Error Handling:** Failures in desktop notifications (e.g., missing OS binaries) are treated as non-fatal and should not interrupt the composition lifecycle.

## 4. System Boundaries

| Concern | `claudine` Responsibility | `messenger` Responsibility |
| :--- | :--- | :--- |
| **Messaging** | Frontmatter parsing; TUI configuration (masked inputs, validation); Secret storage. | Actual transport logic (HTTP calls to webhooks); Provider-specific payload formatting. |
| **Notifications** | Frontmatter parsing; Delegation of `notify` calls. | OS-level interaction (AppleScript/Alerter detection); Notification rendering. |
| **Config** | Providing the TUI and 'Test Connection' workflow. | Providing the underlying 'ping' capability for 'Test Connection'. |

## Technical Integration

This section details the internal wiring required to support the new messaging platforms and notifications.

### 1. `claudine/lib/src/messaging` Updates

- **`MessagingRouteConfig` Enum:** Update to include `DiscordWebhook` and `SlackWebhook` variants.
- **Payload Handling (`send.rs`):** 
  - Update `build_payload` and `send_payload` to handle these new variants.
  - Register the corresponding `messenger` providers for these variants.
- **Desktop Notification Export:** Export a new `execute_notification` function that delegates to `messenger`'s desktop notification capability.

### 2. `claudine/lib/src/composition/lifecycle.rs` Updates

- **`LifecycleNotification` Struct:** Add the `notify: Option<String>` field.
- **`LifecycleEmitter` Trait:** Update to include an `emit_notification` method.
- **`DefaultLifecycleEmitter`:** Implement `emit_notification` by calling the `execute_notification` function.
- **`LifecycleRunGuard::emit_signal`:** Update to call `emit_notification` when the `notify` field is present.
- **Configuration Parsing:** Update `parse_lifecycle_config` to extract and normalize the `notify` field.

### 3. `claudine/cli` Updates (TUI)

- **`claudine config` TUI:** 
  - Update the TUI to handle the new `MessagingRouteConfig` variants.
  - Ensure masked input is provided for sensitive webhook URLs.
  - Implement the 'Test Connection' workflow for on-demand verification of settings.
