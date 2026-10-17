# Push Notifications Design

**Date:** 2026-10-17  
**Status:** Approved 2026-10-17 (including the Pi → APNs outbound call); open questions taken at their defaults  
**Repos:** todo-app (this spec: server, schema, APNs), ai-session-remote (app, hook scripts)  
**Builds on:** [2026-10-15 todo AI conversations](2026-10-15-todo-ai-conversations-design.md), [2026-10-16 projects/memories](https://github.com/tsetsoo/todo-app/pull/5) (schema v3, not merged yet)

## Goal

When a todo's Claude/Codex conversation needs the user (status → `waiting`), the user's iPhone gets a push notification, even with the app closed. Tapping it opens that todo's conversation. Optionally, a long busy run going idle ("done") also notifies.

Success looks like this: you walk away from a long turn, your phone buzzes once when Claude asks for permission, one tap puts you in the terminal, and it never buzzes while you're already looking at that conversation.

## Principles

- **The Pi still only stores data.** It still opens no SSH connections and relays no terminals. **New:** it makes one kind of outbound call, an HTTPS/HTTP/2 POST to Apple's push service (`api.push.apple.com` / `api.sandbox.push.apple.com`, port 443). Nothing else leaves the tailnet. *This needs the user's explicit approval.*
- **What leaves the tailnet** (it goes to Apple, TLS-encrypted, and Apple can read it): the todo title, the section code, the computer name, the todo id and the conversation id. Never conversation content: the hook's `message` (for example "Claude needs your permission to use Bash") stays on the Pi. No tokens or secrets.
- Section codes `Sp` / `I` / `Si` / `P` are opaque and never expanded.
- **Push is best-effort.** Losing a push (Pi restart, Apple outage) is acceptable, but a duplicate or a buzz while you're watching is not. When unsure, don't send.

## Anti-goals

Even if everything technically works, these count as failures:
- a buzz for every tool call or every turn;
- a buzz while you're typing in that conversation, on any device or terminal;
- two alerts for the same event on one device (local + push);
- any conversation text in a notification.

## Context (current)

- `POST /api/conversations/status` (hooks on three computers) → `set_status_db` → WebSocket `refresh`. The Claude hook already maps:
  - `UserPromptSubmit` → `busy`;
  - `PostToolUse` → `busy`, only when the last status the hook sent wasn't already busy, i.e. after an answered prompt;
  - `Notification` → `waiting` (permission/input prompts only; `idle_prompt` is reported as `idle`);
  - `Stop` → `idle`.

  Codex's `notify` is only confirmed to send `agent-turn-complete` → `idle` (codex-cli 0.163.0). The script maps `approval-requested` → `waiting` "if Codex sends it", which is unconfirmed. Codex never sends `busy`.
- So one Claude turn looks like `busy → (waiting → busy)* → idle`. Note that `PostToolUse` fires **after the approved tool finishes**, so after you answer a prompt, the status stays `waiting` for as long as that tool runs.
- **Codex in v1 is best-effort:** whatever `waiting` the existing script delivers is pushed like Claude's. There is no Codex-specific work; "done" pushes never fire for Codex (no `busy`).
- The app has `AttentionNotifier` (local alert after a session-list refresh finds `waiting`), and asks for notification permission on first launch.
- The Pi runs Raspbian Buster on armv7l with glibc 2.28, clock NTP-synced, timezone Europe/London. It already reaches `api.sandbox.push.apple.com` over HTTP/2 (checked 2026-10-17).
- The iOS app is `com.aisessionremote.app`, team `233QTK6KCL`, installed as a **Release build signed with a development profile** (`xcodebuild … -allowProvisioningUpdates` + `devicectl`). Such a build gets **sandbox** APNs tokens, even though it is "Release".

## Product decisions

| Topic | Decision |
|---|---|
| Who gets pushes | iPhone (iOS app). The Mac app gets **no remote push** in v1 (see Mac). |
| What notifies | `waiting`: on by default. Busy run → `idle` after ≥ N minutes ("done"): **off by default**; N is set in Settings (default 5 when turned on). |
| Debounce | At most **one `waiting` push sent per turn**, and at most one "done" push per turn. A turn starts at the first `busy` after `idle`/`unknown` and ends at `idle`. Only a push that was actually **sent** uses up the turn's one push. A `waiting` that was suppressed (you were there, muted, or answered within the grace) doesn't, so a later prompt in the same turn, after you walked away, still pushes. *(My call.)* |
| Grace delay | A `waiting` push is held for **20 s** (configurable). It is dropped if, by then, the status has left `waiting`, someone has started viewing the conversation, or an `attended` re-check (below) says you touched the terminal after the prompt appeared. That covers "I answered it at the desk within seconds", including when the approved tool then runs for minutes. "Done" pushes have no grace delay. |
| Already looking | No push if any of these holds at send time:<br>• **any** device reports that it is viewing that todo's conversation (app presence);<br>• the hook reported `attended` when the prompt appeared: a tmux client attached to that session had keyboard activity in the last 60 s (covers plain terminals);<br>• the hook's **re-check ~12 s later** reports activity since the prompt (you answered it). |
| Mute | Per section (`Sp`/`I`/`Si`/`P`) and per todo, stored on the server. A muted todo/section never pushes. A muted todo stays muted until unmuted. Deleting the todo removes its mute. |
| Quiet hours | Optional window (e.g. 22:30–07:30) in an IANA timezone that the app sends (the phone's). Inside the window, pushes are still sent but as `interruption-level: passive`: no sound, no lighting up, they wait in Notification Center. *(See Open questions.)* |
| Grouping | `thread-id` = todo id (iOS groups per todo). `apns-collapse-id` = `todo-<todo id>`, so a newer alert for a todo **replaces** the older one instead of stacking. Local alerts use the same identifier. |
| Text | Title = todo title (≤ 120 chars, ellipsised). Subtitle = `<section code> · <computer name>`. Body = `Needs you` (waiting) / `Done after 14 min` (finished). |
| Tap | Opens that todo's conversation (deep link via the payload, not a URL scheme). |
| Long-press actions | `Mute this todo` (runs in the background, calls the mute endpoint). |
| Device toggle | Settings → Notifications → "Notify me when a conversation needs me" registers/unregisters this device. Off by default. The permission prompt appears only when it's switched on, never at first launch. |
| Badges, clearing stale alerts from the server | Not in v1. Alerts are cleared on the device whenever the app becomes active (see App). |

## Architecture

```
hook (Claude/Codex, 3 computers)
   │ POST /api/conversations/status {session_id, status, attended?}
   ▼
todo-server (Pi)
   set_status_db  ── returns (todo_id, old_status, new_status)
   │
   ├─► broadcaster.send()                (unchanged: WS refresh)
   └─► PushNotifier::on_transition(...)  (non-blocking mpsc send)
          │  background task (one per server)
          │   1. TurnTracker: is this a new waiting/done event for the turn?
          │   2. Policy: settings, mutes, presence, attended, quiet hours
          │   3. waiting: sleep(grace) → re-read status, presence, attended re-check → still due?
          │   4. ApnsClient: JWT ES256, HTTP/2, one POST per registered device
          ▼
        Apple APNs ──► iPhone ──tap──► app opens todo's conversation
```

App presence: the app calls `POST /api/push/presence` while a conversation is on screen (Mac too). The server keeps it **in memory** with a 90 s TTL.

### Server modules (todo-server)

- `push/mod.rs`: `PushNotifier` (handle with an `mpsc::Sender`, cloned into `web::Data`), plus the background task.
- `push/policy.rs`: **pure** decision functions with no IO: `TurnTracker`, `should_notify(event, settings, mutes, presence, now)`, quiet-hours check. Most tests live here.
- `push/apns.rs`: JWT signer, payload builder, a `PushTransport` trait with an HTTP/2 implementation, response classification.
- `push/store.rs`: device / settings / mute queries.
- `push/handlers.rs`: the endpoints below.
- Shared request/response types go in `todo-shared`.
- `conversations.rs` changes:
  - `set_status_db` reads `todo_id` + old status and updates in one transaction, returning `Option<StatusChange>`;
  - `post_conversation_status` adds one line that calls the notifier;
  - `ConversationStatusRequest` gains `attended: Option<bool>`.

  The existing 404 for unknown/closed ids and the rule that `closed` is never overwritten stay as they are (agreed with the projects session).

Turn state lives in memory (`HashMap<todo_id, TurnState { started_at, waiting_sent, done_sent }>`). After a restart (a deploy), the worst case is one missed "done" or one repeated `waiting` push in a turn already in flight. That is acceptable, and it keeps the tables free of per-turn churn.

## Data model: schema v4

**Migration number agreed with the projects/memories session (2026-10-17):** v3 is theirs (projects, sessions, session_days, memories, todos.project_id) and they have no further version planned; **v4 is push**.
- Same ladder: `schema_version()` sniffs `push_devices` → 4 when `user_version` is 0.
- The `v3 → v4` step comes after theirs, with one backup per startup.
- Fresh databases create the tables with `IF NOT EXISTS` in `init_pool`.
- v4 only **creates** tables, so an older binary after a rollback ignores them.

```sql
CREATE TABLE IF NOT EXISTS push_devices (
  token        TEXT PRIMARY KEY,               -- APNs device token, lowercase hex
  platform     TEXT NOT NULL CHECK(platform IN ('ios','macos')),
  environment  TEXT NOT NULL CHECK(environment IN ('sandbox','production')),
  bundle_id    TEXT NOT NULL,                  -- apns-topic
  app_version  TEXT NOT NULL,
  device_name  TEXT,
  created_at   TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at   TEXT NOT NULL DEFAULT (datetime('now')),
  last_sent_at TEXT,
  last_error   TEXT                            -- APNs reason, e.g. 'BadDeviceToken'; never a secret
);

CREATE TABLE IF NOT EXISTS push_settings (     -- exactly one row, id = 1
  id                     INTEGER PRIMARY KEY CHECK(id = 1),
  notify_waiting         INTEGER NOT NULL DEFAULT 1,
  done_after_minutes     INTEGER,              -- NULL = no "done" pushes
  grace_seconds          INTEGER NOT NULL DEFAULT 20,
  quiet_start            TEXT,                 -- 'HH:MM', NULL = no quiet hours
  quiet_end              TEXT,
  quiet_tz               TEXT                  -- IANA, e.g. 'Europe/Sofia'
);

CREATE TABLE IF NOT EXISTS push_muted_sections (
  section TEXT PRIMARY KEY CHECK(section IN ('Sp','I','Si','P'))
);

CREATE TABLE IF NOT EXISTS push_muted_todos (
  todo_id    TEXT PRIMARY KEY REFERENCES todos(id) ON DELETE CASCADE,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

No secrets go in the database. The APNs key is read from a file (see Secrets).

## API

All endpoints need the bearer token, like the rest.

| Method | Path | Purpose |
|---|---|---|
| `PUT` | `/api/push/devices` | Upsert `{token, platform, environment, bundle_id, app_version, device_name?}`. Validates: token is hex, 32–200 bytes; enums. Returns the device. |
| `DELETE` | `/api/push/devices/{token}` | Unregister (toggle off). 204, or 404 if unknown. |
| `GET` | `/api/push/devices` | List, for Settings/debugging; the response carries only the first 8 hex chars of each token. |
| `GET` / `PUT` | `/api/push/settings` | `{notify_waiting, done_after_minutes, grace_seconds, quiet_hours: {start, end, tz} \| null, muted_sections: [..], muted_todo_ids: [..]}`. `PUT` replaces the scalar settings and section mutes; todo mutes use the endpoints below. Unknown tz → 400. |
| `PUT` / `DELETE` | `/api/todos/{id}/push-mute` | Mute / unmute one todo. 404 for an unknown todo. |
| `POST` | `/api/push/presence` | `{client_id, todo_id \| null}`: "this client is viewing todo X" (or nothing). In memory, 90 s TTL; clients refresh every 30 s while visible. |
| `GET` | `/api/push/status` | `{configured, environment_hosts_ok, last_error, last_sent_at, devices}`, so Settings can show "server push not configured". Never contains key material. |
| `POST` | `/api/push/test` | Sends `Test notification` to all devices (or `{token}`), returns per-device result. For Settings → "Send test" and the e2e step. |

- Every settings/mute/device mutation triggers the WS broadcast. Presence doesn't: it would cause a refresh storm.
- The status hook body gets an optional `attended: bool`. Old hooks omit it, which counts as `false`.
- A `waiting` post for a conversation that is **already** `waiting` is the hook's re-check:
  - with `attended: true`, it cancels a pending (grace-held) push for that conversation;
  - otherwise it is a no-op apart from the usual timestamp update.

  It is never a new transition, so it never pushes.
- `/api/describe` lists the new endpoints.

## Push payload

```json
{
  "aps": {
    "alert": { "title": "Fix invoice export", "subtitle": "Sp · MacBook", "body": "Needs you" },
    "sound": "default",
    "thread-id": "<todo_id>",
    "category": "TODO_CONVERSATION",
    "interruption-level": "active"
  },
  "todo_id": "<todo_id>",
  "session_id": "<conversation session id>",
  "kind": "waiting"
}
```

Headers:
- `apns-push-type: alert`
- `apns-topic: <bundle_id>`
- `apns-priority: 10`, or `5` when passive
- `apns-expiration: now + 1 h` (Apple holds it that long while the phone is offline; older than that is stale)
- `apns-collapse-id: todo-<todo_id>`

`kind` is `waiting`, `done` or `test`. During quiet hours: `interruption-level: passive` and no `sound`.

## APNs client

- **Transport:** `reqwest = "0.12"` (0.13 switched to aws-lc by default; don't bump without re-running the cross-compile spike) with `default-features = false`, `features = ["http2", "rustls-tls", "json"]`.
  - rustls with the **ring** provider and bundled webpki roots. No OpenSSL, no aws-lc (aws-lc needs cmake and is painful under zigbuild), no reliance on the Pi's CA store.
  - One client per host, kept for the process lifetime, so one long-lived HTTP/2 connection is reused across pushes.
  - Base URL is injectable: tests point it at a local mock.
- **JWT:** ES256, signed with `ring::signature::EcdsaKeyPair` (`ECDSA_P256_SHA256_FIXED_SIGNING`, PKCS#8 from the `.p8` PEM, base64-decoded with `rustls-pki-types`' PEM parser or `base64`). ring is already compiled in through rustls; it becomes a direct dependency, but adds no new build.
  - Header `{alg: ES256, kid: KEY_ID}`; claims `{iss: TEAM_ID, iat}`.
  - Cached and re-signed after 40 min. Apple rejects tokens older than 60 min, and refreshes faster than every 20 min (`TooManyProviderTokenUpdates`).
- **Cross-compile:** the first implementation step is a spike: `cargo zigbuild --release -p todo-server --target armv7-unknown-linux-gnueabihf.2.28` with the new dependencies. If ring or rustls fail there, stop and report back before going on.
- **Response handling** (per device):

| APNs answer | Action |
|---|---|
| 200 | `last_sent_at = now`, clear `last_error` |
| 410 `Unregistered` | Delete the device, unless it was re-registered (`updated_at`) after Apple's timestamp |
| 400 `BadDeviceToken` | Retry once against the **other** environment. If that works, store the corrected environment; otherwise delete the device |
| 400 `DeviceTokenNotForTopic` | Delete the device |
| 403 `ExpiredProviderToken` / `InvalidProviderToken` | Re-sign the JWT and retry once, but **never re-sign more than once per 20 min** (Apple answers faster refreshes with `TooManyProviderTokenUpdates`). If it still fails, record `last_error`, show it in `/api/push/status`, and drop pushes until the next allowed re-sign |
| 429, 500, 503, connect/timeout errors | Retry after 2 s, 10 s, 30 s, then give up and log |
| other 4xx | Record `last_error`, log, no retry |

- **Logging** (journald, `-t todo` / stdout): event, todo id, token prefix (8 chars), APNs status/reason. Never the full token, the JWT, or the key.
- **Not configured** (no key env): the notifier runs, decides, and logs `push: not configured, skipped`. Registration endpoints still work, so the server can deploy before secrets exist.

## Secrets (user steps; agents may not write them to the Pi)

1. **Apple Developer portal** → Certificates, Identifiers & Profiles → **Keys** → **+**.
   - Name it `todo-server APNs` and tick **Apple Push Notifications service (APNs)**.
   - If asked, choose environment **Sandbox & Production** and the **team-scoped** key.
   - Download `AuthKey_<KEYID>.p8` (it can only be downloaded once) and note the **Key ID**.
   - Team ID is `233QTK6KCL`.
2. **Also in the portal**, if the first app build fails to sign: Identifiers → `com.aisessionremote.app` → tick **Push Notifications**. Automatic signing with `-allowProvisioningUpdates` normally does this by itself.
3. **On the Mac**, write the key and its env file to the Pi. Commands to paste, with `<KEYID>` replaced:
   ```sh
   KEY=~/Downloads/AuthKey_<KEYID>.p8
   ssh pi@100.118.255.23 'umask 077; cat > /opt/todo/data/apns-key.p8' < "$KEY"
   ssh pi@100.118.255.23 'umask 077; cat > /opt/todo/data/apns.env' <<'EOF'
   APNS_KEY_PATH=/opt/todo/data/apns-key.p8
   APNS_KEY_ID=<KEYID>
   APNS_TEAM_ID=233QTK6KCL
   EOF
   ssh -t pi@100.118.255.23 'sudo mkdir -p /etc/systemd/system/todo.service.d &&
     printf "[Service]\nEnvironmentFile=-/opt/todo/data/apns.env\n" | sudo tee /etc/systemd/system/todo.service.d/apns.conf >/dev/null &&
     sudo systemctl daemon-reload && sudo systemctl restart todo &&
     ls -l /opt/todo/data/apns-key.p8 /opt/todo/data/apns.env'
   ```
   - This doesn't touch `todo.env` or `token.conf`.
   - Both files are `-rw------- pi pi`. The service runs as `pi`, and systemd reads `EnvironmentFile` as root.
   - The leading `-` keeps the service starting if the file is ever missing.
   - The topic is not in the env: it comes from each device's `bundle_id`.
4. Keep the `.p8` somewhere safe (a password manager), then delete it from `~/Downloads`.

`deploy/pi/bootstrap.sh` and the docs get the same steps, for rebuilding the Pi.

## App (ai-session-remote)

### Ownership, agreed with the terminal-UI-redesign session (2026-10-17)

- **This work owns** the plumbing:
  - `NotificationCenterCoordinator` (`@MainActor`, `.shared`): the `UNUserNotificationCenter` delegate, the permission request, categories/actions, tap routing, local-alert posting, presence reporting and push registration;
  - `AttentionNotifier` (folded into the coordinator);
  - `NotificationSettingsView` (quiet hours, section mutes, "done" threshold, muted todos, Send test, status line);
  - the entitlements file;
  - `AISessionCore/Push/*`;
  - additive `TodoHubClient` endpoints.
- **The UI session owns** the visuals:
  - the Settings → Notifications screen and its toggle (`@AppStorage("notify.needsYou")`), which embeds `NotificationSettingsView`;
  - status-flip detection (`StatusTransitions.newlyWaiting`, which never fires on the first snapshot after launch/reconnect);
  - list context menus;
  - calling `setVisibleConversation` from the conversation frame (iOS: on appear/disappear and scene inactive; Mac: only while the conversation is in the key window).
- **API between the two:**
  ```swift
  func postWaiting(todoId: String, sessionId: String, title: String, body: String)
  var onOpenTodo: ((String) -> Void)?          // a tap from a cold launch is buffered and replayed when set
  func setVisibleConversation(todoId: String?)
  func setNeedsYouEnabled(_ on: Bool) async -> Bool   // permission + registration; false → toggle reverts
  func setMuted(todoId: String, _ muted: Bool) async throws
  var pushState: PushState { get }              // .off / .localOnly(reason) / .registered
  ```
- **No duplicates:**
  - `postWaiting` is a no-op when `pushState == .registered` (the server push is the alert), or when that todo is visible. Otherwise it posts a local alert with identifier `todo-<todoId>` and thread-id = todoId, the same identity as the push.
  - The delegate's `willPresent` drops a push for the visible todo. For other todos it shows a banner in-app.
  - The first-launch `requestAuthorizationIfNeeded()` call is removed.

### iOS push specifics

- **Entitlements:** `App/AISessionRemote/AISessionRemote.entitlements` with `aps-environment = development`, wired via `CODE_SIGN_ENTITLEMENTS` on the iOS target only.
  - Development-signed builds (all current device installs, Debug or Release) get sandbox tokens.
  - An App Store/TestFlight export rewrites the value to `production` automatically. There is no TestFlight today; this is noted for later.
- **Environment detection:** the app reads `aps-environment` from `embedded.mobileprovision` at runtime (the plist sits as plain text inside the CMS blob). No profile (TestFlight/App Store) means `production`. This goes in `AISessionCore/Push/ApsEnvironment.swift`, unit-tested with fixture text. The server's BadDeviceToken fallback covers a wrong guess.
- **App delegate (shared, owned here):** SwiftUI allows only one adaptor, so `@UIApplicationDelegateAdaptor(AppDelegate.self)` (iOS only) is the one-line addition to `AISessionRemoteApp.swift` agreed with the UI session.
  - `AppDelegate` returns a scene configuration whose `delegateClass` is a tiny `AppSceneDelegate`. It only forwards Home-screen quick actions (`windowScene(_:performActionFor:)` and the launch shortcut item) to `static var onShortcutItem: ((UIApplicationShortcutItem) -> Bool)?`. A launch item is buffered until that is set.
  - The UI session sets the hook for its "New Todo" quick action and owns no delegate code.
- **Registration:**
  - While the toggle is on, `registerForRemoteNotifications()` runs on every launch.
  - `didRegisterForRemoteNotificationsWithDeviceToken` → `PUT /api/push/devices`, also re-sent when the hub URL/token changes in Settings.
  - Toggle off → `DELETE /api/push/devices/{token}` + `unregisterForRemoteNotifications()`.
  - `PUT` answers 404 (older server) → `pushState = .localOnly("server has no push support")`, and local alerts keep working while the app runs.
- **Tap / deep link:** `didReceive response` → `payload.todo_id` → `onOpenTodo`, buffered until set (cold launch). The `Mute this todo` action → `setMuted` in a background task.
- **Presence:**
  - `setVisibleConversation(todoId)` → `POST /api/push/presence {client_id, todo_id}`, then every 30 s while it is unchanged and the scene is active.
  - `nil` / background → one `POST` with `todo_id: null`, inside `beginBackgroundTask`.
  - `client_id` is a random UUID kept in `UserDefaults`.
- **Clearing stale alerts:** on becoming active, delivered notifications whose todo is no longer `waiting` (from the conversations list) are removed.

### Mac

- **Recommendation: no remote push on the Mac in v1.** The Mac app is unsandboxed and ad-hoc signed. Push needs:
  - signing with team `233QTK6KCL` and a provisioning profile for a `com.aisessionremote.mac` App ID with Push enabled;
  - `com.apple.developer.aps-environment`;
  - changes to `scripts/build-mac-release.sh`.

  Moving off ad-hoc signing changes the app's code identity, so the login-keychain items it created (SSH keys, hub token) would prompt or need re-adding.
- **Benefit:** small. While the Mac app runs it already sees status flips over the WebSocket and posts local alerts through the same coordinator. When it isn't running, the iPhone push covers it, and with iPhone Mirroring on macOS 15+ the iPhone's notifications show on the Mac anyway.
- The Mac app still **reports presence**, so viewing a conversation on the Mac suppresses the iPhone push.
- **Mac local alerts vs. iPhone mirroring** *(my call)*: on the Mac, `postWaiting` posts a local alert only when the server has no registered iPhone (`GET /api/push/status` → `devices == 0` or `configured == false`), or when the Mac-only switch "Alert on this Mac too" (in `NotificationSettingsView`, default off) is on. Otherwise the iPhone push is the single alert, shown on the Mac anyway if iPhone notifications are mirrored.

### Hook scripts (ai-session-remote/scripts/todo)

- `ai-todo-hook` (Claude) adds `"attended": true|false` to its body. True when any tmux client attached to its session had activity in the last 60 s: `tmux list-clients -t "=$session" -F '#{client_activity}'` compared with `date +%s`.
- **Re-check:** for `waiting` only, the hook's existing background job also sleeps ~12 s and, if a client's `client_activity` is newer than the prompt time, re-posts `{status: waiting, attended: true}`. A keystroke in the app's terminal counts too, since the app attaches through tmux.
  - Without it, answering a prompt whose tool then runs for minutes would still push, because `busy` only comes from `PostToolUse` after the tool.
  - It stays inside the hook's "never block Claude" rule: backgrounded, 3 s curl limit.
  - The 12 s re-check must land inside the 20 s grace.
- `ai-todo-codex-notify` does the same for the `ai/codex/todo-*` session.
- Old hooks keep working; they just never suppress. Rolling this out means re-running `install.sh` on each computer (user's go).

## Testing

**Rust** (`cargo test -p todo-server -p todo-shared`, `cargo clippy … --all-targets -- -D warnings`). Temporary databases only (`TestDb`); never the Pi DB. No real Apple calls: every test uses a fake `PushTransport` or a local mock server.
- `policy.rs`:
  - turn tracking (`busy→waiting` pushes, a second `waiting` in the same turn doesn't, `idle` resets);
  - "done" threshold;
  - Codex (no `busy`, so `waiting` still pushes and "done" never does);
  - `attended` at the prompt and via the re-check (cancels a grace-held push), presence present/expired;
  - a suppressed `waiting` doesn't use up the turn; a sent one does;
  - section/todo mute;
  - quiet hours (overnight wrap, DST day, tz ≠ Pi tz).
- Grace delay: with a paused tokio clock, status leaves `waiting` → no push; presence appears → no push.
- `apns.rs`:
  - JWT verifies with the matching public key (test key generated in-test with ring), and is cached and refreshed;
  - payload JSON and headers;
  - every row of the response table, including the BadDeviceToken environment fallback and the 410 timestamp rule.
- HTTP transport against a local HTTP mock: request path `/3/device/<token>`, headers, body.
- `db.rs`: `v3 → v4` keeps rows and takes one backup; a fresh DB is v4; deleting a todo cascades its mute.
- Handlers: device validation, settings round-trip, unknown tz → 400, presence TTL, status endpoint never leaks the key path's contents.

**Swift** (`swift test`; both Xcode builds with `DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer`):
- `ApsEnvironment` parsing (development / production / no profile);
- payload → `todoId` parsing;
- the `postWaiting` decision (registered / local-only / visible) as a pure function in `AISessionCore`;
- `TodoHubClient` push endpoints with the existing `StubURLProtocol`, including 404 → local-only.

**Shell:** `test-todo-scripts.sh` covers `attended` true/false/no-tmux and the delayed re-check (with a test-only override for the 12 s delay), using `AI_TODO_TMUX_SESSION` and a stub `tmux` on PATH.

**E2E with one real sandbox push**, before anything merges. It only runs with the user present, after approval, and with the phone unlocked:
1. Run the branch's `todo-server` on the Mac: temp DB under the job's tmp dir, a throwaway API token, bound to the Mac's Tailscale IP on port **18190** (not 18090, which the projects session uses), with `APNS_KEY_PATH` pointing at the user's downloaded `.p8` (read in place, not copied).
2. Build and install the app on the phone. Launch it with `--hub-url http://<mac-tailscale-ip>:18190 --hub-token <throwaway>`. The user switches the toggle on and taps **Allow**.
3. `POST /api/push/test`: one sandbox push. The user confirms it arrived, then taps it: the app opens.
4. Seed a todo + conversation in the temp DB and post `waiting` for it: one push titled with that todo, which opens that todo. Post `waiting` again in the same turn: no second push.
5. Restore the phone to the Pi: relaunch with `--hub-url http://100.118.255.23:8080 --hub-token <from ~/.ai-remote/todo-hub.conf>`, passed straight from the file without echoing it.
6. Stop the server and delete the temp DB.

## Rollout

0. The projects/memories session's v3 merges and deploys first (their gate). My server branch is stacked on `feature/projects-memories-server`; its draft PR targets that branch and gets retargeted to `main` after PR #5 merges.
1. Pre-merge e2e (above), with the user.
2. **Merge the server PR (user's go)** → the Pi auto-deploys and migrates v3 → v4 (backup `todos.db.pre-v4-*`). No secrets yet → push is "not configured"; nothing changes for anyone.
3. **User:** portal key + the Pi secret commands → restart. `GET /api/push/status` shows `configured: true`.
4. **Merge the app PR (user's go)** → build, install on the phone (unlocked), toggle on, Send test.
5. **Re-run `install.sh`** on the three computers for `attended` and the re-check (user's go). It is strongly recommended: until then, at-desk suppression relies on app presence and the grace delay only, and a plain-terminal user answering a slow-tool prompt still gets a buzz.

Older builds:
- An app without push never registers, so the server sends it nothing, and its first-launch local alerts keep working as before.
- A new app on an old server gets 404 on registration → local-only.
- A rollback of the server binary leaves the v4 tables untouched, and registrations survive a re-deploy.

## Open questions (resolved 2026-10-17: the user approved without answering, so the defaults below apply; they can be revisited)

1. **Second prompt after a push you answered.** A push was sent and you answered from the phone, so the turn's push is used up, and a later permission prompt in the same turn is silent. The alternative: re-arm once you answer (the status went back to `busy`), with presence/`attended` still suppressing while you're there. Default in this doc: strict one *sent* push per turn, as the handover asked.
2. **Quiet hours.** Passive delivery (silent, waits in Notification Center), which this doc assumes, or drop entirely? iOS Sleep/Focus already silences the phone at night. Should server quiet hours exist at all? If not, the `quiet_*` columns and the timezone crate (`chrono-tz`; the server has no date/time crate today) go away.
3. **Approve the new outbound call** from the Pi to Apple, carrying the todo title, section code and computer name.

## Non-goals (v1)

- Remote push to the Mac.
- Badges.
- Server-side removal of stale alerts (background pushes).
- Notification Service Extension.
- TestFlight/App Store.
- Pushes for anything other than conversation status.
- Multiple users.
