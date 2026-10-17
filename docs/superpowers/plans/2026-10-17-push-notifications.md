# Push Notifications Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** APNs push to the iPhone when a todo's conversation flips to `waiting` (and optionally "done"), with presence/attended suppression, mutes and quiet hours.

**Architecture:**
- **todo-server:** a `push` module.
  - `policy` is pure; `apns` holds the JWT, payload and transport; `notifier` is the background task; `store` holds the SQL; `handlers` holds the endpoints.
  - It is fed by `set_status_db` returning the transition.
- **ai-session-remote:**
  - a `NotificationCenterCoordinator` plus an `AppDelegate` in the app;
  - testable logic in `AISessionCore/Push`;
  - an `attended` flag plus a ~12 s re-check in the hook scripts.

**Tech Stack:** Rust (Actix 4, rusqlite, reqwest 0.12 + rustls/ring, chrono-tz), Swift 5.9 / SwiftUI / UserNotifications, POSIX sh.

**Spec:** `docs/superpowers/specs/2026-10-17-push-notifications-design.md` (approved 2026-10-17)

## Global Constraints

- **Branches:**
  - server: `feature/push-notifications-server` (stacked on `feature/projects-memories-server`, v3);
  - app/scripts: `feature/push-notifications-app` (from `ai-session-remote` main).

  Draft PRs only; never merge, never push to main.
- Schema version **4**, following v3's ladder: sniff `push_devices` → 4; one backup per startup; `if version == 3 { … }` after the v2→v3 step, which must now set `version = 3`.
- `reqwest = { version = "0.12", default-features = false, features = ["http2", "rustls-tls", "json"] }`. No OpenSSL, no aws-lc.
- Must build with `cargo zigbuild --release -p todo-server --target armv7-unknown-linux-gnueabihf.2.28`.
- APNs hosts `https://api.push.apple.com` (production), `https://api.sandbox.push.apple.com` (sandbox); path `/3/device/<token>`.
- **Env:** `APNS_KEY_PATH`, `APNS_KEY_ID`, `APNS_TEAM_ID`. All three present → configured; otherwise "not configured" (log once, never fail startup).
- **Timings:**
  - JWT re-signed after 40 min, never more than once per 20 min;
  - presence TTL 90 s (client refresh 30 s);
  - `attended` window 60 s; hook re-check after 12 s;
  - grace default 20 s;
  - retries after 2 s, 10 s, 30 s;
  - `apns-expiration` now + 3600.
- **Payload text:** title = todo title ≤ 120 chars (ellipsis `…`); subtitle `<section> · <computer name>`; body `Needs you` / `Done after <N> min` / `Test notification`.
- **Identity:** `apns-collapse-id` = local identifier = `todo-<todo_id>`; `thread-id` = todo_id; category `TODO_CONVERSATION`; payload keys `todo_id`, `session_id`, `kind`.
- **What leaves the device or the Pi:**
  - never the hook `message`/`status_message` in a push or a local alert;
  - never log a full device token (8-char prefix), the JWT or key bytes.
- Section codes `Sp`/`I`/`Si`/`P` are never expanded.
- **Tests:**
  - temp DBs only (`TestDb`), never the Pi DB;
  - no network to Apple;
  - any test server goes on 127.0.0.1 with an ephemeral port (never 18090).
- **Gates:**
  - `cargo test -p todo-server -p todo-shared`;
  - `cargo clippy -p todo-shared -p todo-server --all-targets -- -D warnings`;
  - `swift test`;
  - both Xcode builds with `DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer`.
- Commits end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Review Focus

1. **Server restart mid-turn:** the in-memory turn state is empty, so a `waiting` arrives as a transition from DB `busy`/`waiting`. Expect at most one push; never a crash or a double from a repeated `waiting` post. → Task 4 test `repeat_waiting_post_is_not_a_transition`.
2. **APNs not configured** (no env) while devices are registered: status posts still 204 quickly, nothing is sent, and `/api/push/status` says `configured: false`. → Task 6 test `unconfigured_notifier_skips_and_status_reports_it`.
3. **Todo deleted** between the `waiting` and the end of the grace: no push, no panic, its mute row is gone. → Task 5 test `todo_deleted_during_grace_sends_nothing`.
4. **A device token re-registered** after Apple's 410 timestamp is kept. → Task 3 test `unregistered_410_keeps_device_reregistered_later`.
5. **App on an old server** (registration 404): the toggle stays on with `.localOnly`, local alerts still post, no retry loop. → Task A1 test `registerDevice404MapsToUnsupported`.

---

## Part 1 — todo-server (`todo-app`, worktree `.claude/worktrees/push-notifications-server`)

### Task 1: Dependencies + armv7 cross-compile spike

**Files:** Modify `crates/todo-server/Cargo.toml`, `Cargo.lock`.

- [ ] Add:
  - `reqwest` (as in Global Constraints);
  - `ring = "0.17"`, `base64 = "0.22"`, `chrono = { version = "0.4", default-features = false, features = ["clock", "std"] }`, `chrono-tz = "0.10"`;
  - tokio features `["sync", "rt", "time", "macros"]`;
  - dev-dep `tokio = { version = "1", features = ["test-util"] }`.
- [ ] `cargo build -p todo-server` → builds.
- [ ] `cargo tree -p todo-server -i aws-lc-sys` → "did not match" (absent), and `cargo tree -i openssl-sys` → absent.
- [ ] `cargo zigbuild --release -p todo-server --target armv7-unknown-linux-gnueabihf.2.28` → `Finished`. **If this fails, stop and report.**
- [ ] Commit `Add APNs client dependencies (armv7 cross-compile verified).`

### Task 2: Schema v4 + push store

**Files:**
- Modify `crates/todo-server/src/db.rs`
- Create `crates/todo-server/src/push/mod.rs` (module root, `pub mod store;` for now), `crates/todo-server/src/push/store.rs`
- Modify `main.rs` (`mod push;`)

**Interfaces — Produces** (`push::store`, all `fn(&Connection, …) -> rusqlite::Result<_>`):
- `upsert_device(conn, &PushDeviceRequest) -> PushDevice`
- `delete_device(conn, token) -> bool`
- `list_devices(conn) -> Vec<PushDevice>`
- `devices_for_push(conn) -> Vec<DeviceTarget>`, where `DeviceTarget { token: String, environment: ApnsEnvironment, bundle_id: String, updated_at: String }`
- `update_device_environment(conn, token, ApnsEnvironment)`
- `record_result(conn, token, Option<&str> /*error*/)`, which sets `last_sent_at` on `None`
- `load_settings(conn) -> PushSettings` (defaults when no row)
- `save_settings(conn, &PushSettingsUpdate)`
- `set_todo_muted(conn, todo_id, bool) -> bool /*todo exists*/`
- `is_muted(conn, todo_id, section) -> bool`
- `push_context(conn, todo_id) -> Option<PushContext>`, where `PushContext { todo_id, title, section: String, computer_name: Option<String>, session_id: String, status: ConversationStatus }`

- [ ] Write the tests in `db.rs`:
  - `migrates_v3_to_v4_keeping_rows`: a v3 DB with a todo and a project → after `init_pool` it has `user_version` 4, the `push_*` tables exist, the rows are intact, and there is exactly one `pre-v4` backup;
  - `fresh_database_is_v4_without_backup`;
  - `schema_version_sniffs_push_devices_as_4`.
- [ ] Write the tests in `store.rs`:
  - `upsert_device_updates_existing_token`;
  - `deleting_todo_cascades_its_mute`;
  - `section_mute_and_todo_mute`;
  - `settings_default_and_round_trip`;
  - `push_context_joins_todo_conversation_computer`.
- [ ] Run → FAIL.
- [ ] Implement: DDL exactly as in the spec, `migrate_v3_to_v4`, `SCHEMA_VERSION = 4`, the doc-comment line `- v4: push_devices, push_settings, push_muted_sections, push_muted_todos`, and the store functions.
- [ ] `cargo test -p todo-server` → PASS.
- [ ] Commit `Add schema v4 push tables and store.`

### Task 3: Shared types + APNs client

**Files:**
- Modify `crates/todo-shared/src/lib.rs`
- Create `crates/todo-server/src/push/apns.rs`

**Interfaces — Produces:**
- todo-shared (serde, snake_case):
  - `ApnsEnvironment { Sandbox, Production }`; `PushPlatform { Ios, Macos }`;
  - `PushDeviceRequest { token, platform, environment, bundle_id, app_version, device_name: Option }`;
  - `PushDevice { token_prefix, platform, environment, bundle_id, app_version, device_name, created_at, updated_at, last_sent_at, last_error }`;
  - `QuietHours { start: String, end: String, tz: String }`;
  - `PushSettings { notify_waiting: bool, done_after_minutes: Option<u32>, grace_seconds: u32, quiet_hours: Option<QuietHours>, muted_sections: Vec<Section>, muted_todo_ids: Vec<String> }`;
  - `PushSettingsUpdate` (the same minus `muted_todo_ids`);
  - `PresenceRequest { client_id, todo_id: Option<String> }`;
  - `PushStatus { configured, last_error: Option, last_sent_at: Option, devices: usize }`;
  - `PushTestRequest { token: Option }`; `PushTestResult { token_prefix, ok: bool, reason: Option<String> }`;
  - `ConversationStatusRequest.attended: Option<bool>`.
- apns.rs:
  - `ApnsConfig { key_id, team_id, key_pkcs8_der: Vec<u8> }` with `ApnsConfig::from_env() -> Result<Option<Self>, String>`, which decodes the PEM;
  - `JwtSigner::new(&ApnsConfig)`; `JwtSigner::token(&mut self, now: SystemTime) -> String` (cached, re-signed after 40 min); `JwtSigner::force_refresh(&mut self, now) -> bool` (false if < 20 min since the last sign);
  - `PushKind { Waiting, Done { minutes: u32 }, Test }`; `Alert { todo_id, session_id, title, subtitle, kind, passive: bool }`;
  - `build_payload(&Alert) -> serde_json::Value`; `headers(&Alert, bundle_id, now) -> Vec<(&'static str, String)>`;
  - `trait PushTransport: Send + Sync { async fn post(&self, env: ApnsEnvironment, token: &str, headers: Vec<(&'static str, String)>, body: Vec<u8>) -> Result<ApnsResponse, String> }`, where `ApnsResponse { status: u16, reason: Option<String>, timestamp_ms: Option<i64> }`;
  - `HttpTransport::new(production_base: String, sandbox_base: String)`;
  - `Outcome { Sent, Retry, RefreshToken, WrongEnvironment, DeleteDevice { unless_updated_after_ms: Option<i64> }, Failed(String) }`; `classify(&ApnsResponse) -> Outcome`;
  - `ApnsSender::new(Arc<dyn PushTransport>, JwtSigner)`; `ApnsSender::send(&self, pool: &DbPool, target: &DeviceTarget, alert: &Alert) -> PushTestResult`, which applies the response table (retries via `tokio::time::sleep`, the env fallback, 410 with timestamp vs `updated_at`, 403 with the rate-limited refresh).

- [ ] Write the tests:
  - `jwt_verifies_with_public_key_and_is_cached` (generate a P-256 key with `ring::rand` + `EcdsaKeyPair::generate_pkcs8`, verify with `UnparsedPublicKey` / `ECDSA_P256_SHA256_FIXED`);
  - `force_refresh_is_rate_limited_to_20_minutes`;
  - `payload_has_ids_thread_category_and_no_message`;
  - `passive_payload_has_no_sound_and_priority_5`;
  - `title_truncated_to_120_chars`;
  - `classify_covers_response_table` (200, 410+ts, 400 BadDeviceToken, 400 DeviceTokenNotForTopic, 403 ExpiredProviderToken, 429, 500, 503, 413);
  - `bad_device_token_retries_other_environment_and_stores_it`;
  - `unregistered_410_deletes_device`;
  - `unregistered_410_keeps_device_reregistered_later`;
  - `retries_then_gives_up` (paused clock; 4 attempts);
  - `http_transport_sends_path_headers_body` (std `TcpListener` on 127.0.0.1:0, HTTP/1.1, assert `/3/device/<token>`, `authorization: bearer …`, `apns-topic`, `apns-collapse-id`).

  All of these use a `FakeTransport` with scripted responses.
- [ ] Run → FAIL.
- [ ] Implement.
- [ ] Run → PASS.
- [ ] Commit `Add APNs client: ES256 JWT, payload, HTTP/2 transport, response handling.`

### Task 4: Policy (pure)

**Files:** Create `crates/todo-server/src/push/policy.rs`.

**Interfaces — Produces:**
- `Transition { todo_id, old: ConversationStatus, new: ConversationStatus, attended: bool, at: SystemTime }`
- `TurnTracker::default()`; `TurnTracker::on_transition(&mut self, &Transition) -> Option<Event>`, where `Event { Waiting, Done { minutes } , AttendedRecheck }`
  - A turn starts on `busy` from anything but `busy`/`waiting`.
  - `waiting` from non-`waiting` gives `Waiting` if not `waiting_sent`.
  - `waiting → waiting` with `attended` gives `AttendedRecheck`.
  - `→ idle` gives `Done` if the turn started, `!done_sent`, and the elapsed time is ≥ the threshold (checked later). It resets the turn.
- `TurnTracker::mark_sent(&mut self, todo_id, Event)`
- `Suppress { Disabled, Muted, Attended, Viewing, DoneBelowThreshold }`; `decide(event, &Ctx) -> Result<Delivery /*{passive: bool}*/, Suppress>`, where `Ctx { settings: &PushSettings, muted: bool, attended: bool, viewing: bool, local_now: … }`
- `in_quiet_hours(&QuietHours, now: DateTime<Utc>) -> bool`, which handles windows that cross midnight and is tz-aware.

- [ ] Write the tests:
  - `busy_then_waiting_pushes_once_per_turn`;
  - `suppressed_waiting_does_not_use_up_turn`;
  - `idle_resets_turn`;
  - `repeat_waiting_post_is_not_a_transition`;
  - `done_only_after_threshold_and_never_for_codex_without_busy`;
  - `attended_and_viewing_suppress`;
  - `muted_suppresses`;
  - `quiet_hours_overnight_wrap`;
  - `quiet_hours_respect_tz_not_pi_tz` (Europe/Sofia vs UTC);
  - `quiet_hours_dst_day`;
  - `quiet_hours_mark_passive_not_drop`.
- [ ] Run → FAIL.
- [ ] Implement.
- [ ] Run → PASS.
- [ ] Commit `Add push policy: per-turn debounce, suppression, quiet hours.`

### Task 5: Notifier + status hook integration

**Files:**
- Modify `crates/todo-server/src/push/mod.rs`
- Modify `crates/todo-server/src/conversations.rs` (`set_status_db`, `post_conversation_status`)

**Interfaces:**
- **Consumes:** Tasks 2–4.
- **Produces:**
  - `Presence::default()`; `Presence::set(client_id, Option<todo_id>, now)`; `Presence::viewing(todo_id, now) -> bool` (TTL 90 s). `Arc<Mutex<…>>` inside, `Clone`.
  - `PushNotifier::start(pool: DbPool, sender: Option<ApnsSender>, presence: Presence) -> PushNotifier` (spawns the task); `PushNotifier::disabled()` for tests and handlers that don't care; `PushNotifier::notify(&self, Transition)` (non-blocking `try_send` on a bounded 256 queue; drops and logs when full); `PushNotifier::status(&self) -> (bool configured, Option<String> last_error, Option<String> last_sent_at)`.
  - `set_status_db(conn, tool, session_id, req) -> rusqlite::Result<Option<StatusChange>>`, where `StatusChange { todo_id, old: ConversationStatus }`. It runs a SELECT then an UPDATE in one transaction; it still never updates `closed` (returns `None` for unknown or closed).
- **Waiting path:**
  1. `decide` at arrival.
  2. If due, spawn: `sleep(grace)`.
  3. Re-load `push_context` (`None` → drop) and require `status == Waiting`, `!viewing`, no `AttendedRecheck` seen for this turn.
  4. Send to `devices_for_push`, then `mark_sent`.
- `Done`/`Test` are sent immediately.

- [ ] Write the tests (`#[tokio::test(start_paused = true)]`, with `FakeTransport`):
  - `waiting_pushes_after_grace`;
  - `answered_within_grace_sends_nothing`;
  - `attended_recheck_cancels_pending`;
  - `presence_during_grace_cancels`;
  - `todo_deleted_during_grace_sends_nothing`;
  - `second_waiting_in_turn_is_silent_after_send`;
  - in `conversations.rs`: `status_change_reports_old_status` and the existing `status_updates_and_reopen_resets_closed`, still passing.
- [ ] Run → FAIL.
- [ ] Implement. `post_conversation_status` gains a `web::Data<PushNotifier>` and calls `notify` after `broadcaster.send()`. 404/400 behaviour is unchanged.
- [ ] Run → PASS.
- [ ] Commit `Notify push on conversation status transitions.`

### Task 6: Endpoints, wiring, describe, deploy notes

**Files:**
- Create `crates/todo-server/src/push/handlers.rs`
- Modify `main.rs` (routes in `api_routes`; build `ApnsConfig::from_env`, `HttpTransport` with the Apple hosts, `PushNotifier::start`, `Presence`; app_data)
- Modify `test_support.rs` (register `PushNotifier::disabled()` + `Presence`)
- Modify `describe.rs`
- Modify `deploy/pi/bootstrap.sh` (comment block with the spec's secret steps)

- **Routes** (literal before `{…}`):
  - `PUT /api/push/devices`, `GET /api/push/devices`, `DELETE /api/push/devices/{token}`;
  - `GET|PUT /api/push/settings`;
  - `POST /api/push/presence`;
  - `GET /api/push/status`;
  - `POST /api/push/test`;
  - `PUT|DELETE /api/todos/{id}/push-mute`.
- **Validation:**
  - token is lowercase hex, 64–200 chars (normalize case);
  - `bundle_id` / `app_version` non-empty;
  - unknown tz, bad `HH:MM`, or `grace_seconds > 300` → 400;
  - `done_after_minutes` in 1..=600.
- **Broadcast** on device/settings/mute mutations, not on presence.

- [ ] Write the tests (`test_support::call`):
  - `register_list_unregister_device` (the list carries only `token_prefix`);
  - `register_rejects_bad_token_and_enum`;
  - `settings_round_trip_and_bad_tz_400`;
  - `todo_mute_404_for_unknown_todo`;
  - `presence_accepts_and_clears`;
  - `unconfigured_notifier_skips_and_status_reports_it` (status body has `configured: false`, devices 1);
  - `test_push_returns_per_device_result` (fake sender);
  - `describe_lists_push_endpoints`.
- [ ] Run → FAIL.
- [ ] Implement.
- [ ] Run → PASS.
- [ ] Run the full gates: `cargo test -p todo-server -p todo-shared`, clippy `--all-targets -D warnings`, and the zigbuild armv7 release.
- [ ] Commit `Add push endpoints and wire the notifier.`
- [ ] Push the branch and open a draft PR with base `feature/projects-memories-server`.

## Part 2 — ai-session-remote (worktree `.claude/worktrees/push-notifications-app`)

### Task A1: Core push logic + hub client endpoints

**Files:**
- Create `Sources/AISessionCore/Push/ApsEnvironment.swift`, `PushPayload.swift`, `NotificationPolicy.swift`
- Modify `Sources/AISessionCore/TodoHub/TodoHubClient.swift`, `TodoHubModels.swift`
- Test `Tests/AISessionCoreTests/PushTests.swift`, `TodoHubClientTests.swift`

**Interfaces — Produces:**
- `enum ApsEnvironment: String { case sandbox, production }`; `static func fromProvisioningProfile(_ data: Data?) -> ApsEnvironment` (`nil` → `.production`; `<key>aps-environment</key>\s*<string>development</string>` → `.sandbox`).
- `struct PushPayload { todoId, sessionId: String?, kind: String }`; `init?(userInfo: [AnyHashable: Any])` (keys `todo_id`, `session_id`, `kind`).
- `static func notificationIdentifier(todoId:) -> String` = `"todo-\(todoId)"`.
- `enum PushState: Equatable { case off, localOnly(String), registered }`.
- `NotificationPolicy.shouldPostLocal(pushState:, visibleTodoId:, todoId:, isMac:, iPhoneRegisteredOnServer:, macAlertsToo:) -> Bool`.
- `TodoHubClient`:
  - `registerPushDevice(_ PushDeviceRegistration) async throws`; maps 404 → `TodoHubError.unsupported`;
  - `unregisterPushDevice(token:) async throws`;
  - `pushStatus() async throws -> PushServerStatus`;
  - `pushSettings() async throws -> PushSettings`; `savePushSettings(_:) async throws -> PushSettings`;
  - `setTodoPushMuted(todoId:, _ muted: Bool) async throws`;
  - `reportPresence(clientId:, todoId: String?) async throws`;
  - `sendTestPush() async throws -> [PushTestResult]`.
- Models mirror todo-shared (snake_case via the client's existing decoder settings).

- [ ] Write the tests:
  - `apsEnvironmentDevelopmentIsSandbox`, `apsEnvironmentProductionAndMissingProfile`;
  - `payloadParsesTodoId`, `payloadWithoutTodoIdIsNil`;
  - `policyNoLocalWhenRegistered`, `policyNoLocalWhenVisible`, `policyMacDefersToRegisteredIPhone`, `policyMacAlertsTooOverrides`;
  - `registerDeviceSendsBody`, `registerDevice404MapsToUnsupported`;
  - `presenceSendsNullTodo`;
  - `settingsRoundTrip`.
- [ ] `swift test` → FAIL.
- [ ] Implement.
- [ ] `swift test` → PASS.
- [ ] Commit.

### Task A2: Coordinator, app delegate, entitlements

**Files:**
- Create `App/AISessionRemote/Services/Notifications/NotificationCenterCoordinator.swift`, `AppDelegate.swift` (iOS only, `#if os(iOS)`), `App/AISessionRemote/AISessionRemote.entitlements` (`aps-environment` = `development`)
- Delete `App/AISessionRemote/Services/AttentionNotifier.swift`; its callers in `SessionsListView` use `postWaiting`-style local alerts via the coordinator.
- Modify `AISessionRemoteApp.swift`: add the adaptor and `NotificationCenterCoordinator.shared.install()`; remove the first-launch prompt.
- Modify `project.yml` (`CODE_SIGN_ENTITLEMENTS: App/AISessionRemote/AISessionRemote.entitlements` under the iOS target's settings), then regenerate with `xcodegen`.

**Interfaces — Produces:** exactly the spec's API:
- `postWaiting`, `onOpenTodo`, `setVisibleConversation`, `setNeedsYouEnabled`, `setMuted`, `pushState`;
- `install()`;
- `AppDelegate.onShortcutItem`, `AppSceneDelegate`.

Behaviour:
- **Install:** the category `TODO_CONVERSATION` gets the action `MUTE_TODO` "Mute this todo".
- **Presence:** a 30 s heartbeat `Task` while a todo is visible and the scene is active; `nil` on background.
- **Registration:**
  - re-register on launch when `notify.needsYou` is on;
  - re-upload when the token or hub settings change (`TodoHubSettings` posts a `Notification.Name.todoHubSettingsChanged`);
  - `.unsupported` → `.localOnly("Server has no push support")`.
- **On becoming active:** remove delivered notifications whose todo isn't `waiting`.

- [ ] Both Xcode builds pass (iOS generic device with `CODE_SIGNING_ALLOWED=NO`, plus Mac).
- [ ] `swift test` passes.
- [ ] Commit.

### Task A3: NotificationSettingsView

**Files:** Create `App/AISessionRemote/Views/Notifications/NotificationSettingsView.swift`.

The view loads `pushSettings()` and shows:
- a "done" toggle + minutes stepper (default 5);
- quiet hours: a toggle, two `DatePicker(.hourAndMinute)`, tz = `TimeZone.current.identifier`;
- four section mute toggles labelled with the raw codes;
- the muted todos list with unmute;
- the Mac-only "Alert on this Mac too" (`@AppStorage("notify.macAlertsToo")`);
- "Send test" (iOS) with per-device results;
- a status line from `pushStatus()` + `pushState`.

It saves with `savePushSettings` on change (debounced 0.5 s).

- [ ] Add a `#Preview`.
- [ ] Both Xcode builds pass.
- [ ] Commit.

### Task A4: Hook `attended` + re-check

**Files:**
- Modify `scripts/todo/ai-todo-hook`, `scripts/todo/ai-todo-codex-notify`
- Test `scripts/todo/test-todo-scripts.sh`

- `attended`: any `tmux list-clients -t "=$session" -F '#{client_activity}'` value ≥ now − 60 → true. With no tmux or no clients → false.
- Re-check (`waiting` only): the existing background job sleeps `${AI_TODO_RECHECK_DELAY:-12}` s. If any `client_activity` is greater than the prompt epoch, it posts the same body with `"attended":true`.
- Tests use a stub `tmux` on PATH and `AI_TODO_HOOK_SYNC=1`, `AI_TODO_RECHECK_DELAY=0`, with a local `nc`/python capture server already used by the script tests (or the existing capture mechanism):
  - `attended true when recent activity`;
  - `attended false without clients`;
  - `recheck posts attended after activity`;
  - `no recheck for busy/idle`.
- [ ] `sh scripts/todo/test-todo-scripts.sh` → all pass.
- [ ] Commit.
- [ ] Push and open a draft PR against main.

## Part 3 — E2E (user present, phone unlocked; after the user's go)

Follow the spec's six e2e steps exactly. Port 18190; temp DB under `$CLAUDE_JOB_DIR/tmp`. The `.p8` is read in place from the user's path. Restore the phone to the Pi afterwards.
