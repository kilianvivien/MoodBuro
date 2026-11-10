# A6 assistant qualification

Implemented 10 November 2026. This is a fixture-tested Alpha slice; live provider, Moodle and installed-app qualification is still required.

## Implemented scope

Open AI Assistant from the toolbar or View menu. The trailing panel can be closed or resized by dragging its separator or using Left/Right/Home/End on the focused separator. Width and visibility persist. Rust and CSS reserve the same panel width beside the native Moodle view.

In Settings, choose Albert API or Mistral AI, save a provider key to the OS credential store, check the connection and choose a chat model (or enter its identifier). The shell never receives a saved key. General chat needs no Moodle account or context-sharing permission. Responses stream; Stop, retry of an unanswered question, and clear-conversation controls are available. History is kept only in process memory, up to 40 entries per account, and disappears on quit. Provider changes and withdrawing context consent stop the turn and clear every conversation. Removing a key stops the turn but does not revoke the key at its provider.

Moodle sharing defaults off. After enabling it for the selected provider, explicitly attach an organisation or course to each request. The context exposes bounded plain-text metadata, not page/file contents, URLs, cookies or Moodle credentials. Organisation-scoped tools list courses, search saved activities (refreshing a specified course), list upcoming events and cached notification subjects, or open fixed Moodle routes. A course attachment seeds that course's saved metadata; tools can still look up other courses in the same account. Detaching context excludes earlier Moodle-derived answers from provider replay; what the user typed remains ordinary conversation text.

Only `mark_notification_read` can write, one cached notification at a time. Each attempt previews the cached subject and organisation, waits for a single-use confirmation, and rechecks the account before calling Moodle. Declining/stopping invalidates the preview. A server denial is reported; there is no write retry. Submissions, grades, arbitrary URLs, scripts, and arbitrary Moodle service calls have no tool.

Sources are buttons built from known metadata identifiers, never model-provided URLs. Conversation scope includes site and Moodle user. Account replacement/removal invalidates pending work. Remote Moodle/auth/document views have no assistant IPC permissions.

## Protocol references

Reviewed against the official [Albert API reference](https://albert.api.etalab.gouv.fr/reference) and [Mistral chat documentation](https://docs.mistral.ai/inference/conversations/chat-completion) on 10 November 2026. Both use bearer-authenticated `/v1/models` and streaming `/v1/chat/completions`, with function tools. Endpoints are fixed HTTPS origins and redirects are refused. Model availability and tool support depend on the key/model and require live checks. Defaults are `DeepSeek-V4-Flash-0731` (Albert, requested model identifier) and `mistral-large-4` ([Mistral Large 4](https://docs.mistral.ai/models/mistral-large-4-0)). Explicitly saved model choices take precedence.

## Automated evidence

- Provider mock HTTP tests cover streamed text, tool-call fragments and replay, model-list shapes, invalid models, malformed/oversized replies, authentication/access/quota/network failures and fixed error messages without response bodies or keys.
- Typed-tool tests reject unknown actions, extra fields, arbitrary URLs, positional arguments and out-of-range identifiers/lookback windows.
- Core tests cover verified/bounded citations, context-free replay including truncated-history boundaries, account conversation keys, stop/invalidation signals, local time offsets and panel bounds.
- Store tests verify preference namespacing and notification account isolation. Existing Moodle tests cover denied capabilities, account-bound cache updates and no retry on writes.
- Shell tests render setup/general-chat/streaming/confirmation/retry/deletion states, escaped model text and EN/FR copy; helpers cover context selection, partial citations and width bounds.
- `scripts/check-security.py` checks shell-only capabilities and command caller guards. It is a static check, not a runtime IPC penetration test.

## Remaining live/native checks

Use disposable notifications and test accounts. Record provider, model, app build, OS, Moodle version and result for each run. Do not put keys or tokens in evidence.

- [ ] For each provider: save/remove a key; test a rejected key, model listing, streaming general chat with no Moodle context, tools on a supported model, quota/network interruption and retry. Verify Keychain prompts and installed app identity across relaunch.
- [ ] For two Moodle accounts: enable sharing, attach a course/organisation, ask for deadlines and activities, open each returned source; verify freshness/permission warnings. Confirm a course attachment still allows account-wide lookup as described above.
- [ ] Mark a disposable notification read: approve once, decline another, Stop while waiting and clear while waiting. Denied Moodle service must remain a failure. Verify no action occurs after a cancelled confirmation, account replacement/removal or provider change.
- [ ] Switch organisations while streaming and during a slow lookup; replace a login with another user; remove the organisation. Old text, context, sources and pending confirmations must not appear in the new account or act there.
- [ ] Use adversarial activity/notification names requesting arbitrary URLs, scripts, submissions, grade edits or credential disclosure. Such content must never acquire tool privileges. Confirm every write still requires its own exact-target preview.
- [ ] Detach context, revoke sharing and switch provider. Inspect captured test traffic to confirm old Moodle-derived answers are not replayed and credentials never occur in JSON/errors/logs. Clear all, quit/relaunch and verify no conversation survives.
- [ ] Keyboard and VoiceOver in EN/FR: open/close, composer, Shift-Return/IME, Stop/Escape, clear/cancel, source buttons and confirmation focus; resize with arrow keys/Home/End. Check light/dark, Increase Contrast and Reduce Motion.
- [ ] Native Moodle bounds: open/close/drag, minimum window, fullscreen, window resize and site switching. Check focus returns to Moodle and the composer is never covered. The browser demo is canned and does not qualify provider or native behavior.

These open checks remain part of the Alpha acceptance gate.
