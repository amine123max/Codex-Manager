# Changelog

This file records externally visible changes to CodexManager and serves as the single source of truth for version history.
It follows Keep a Changelog with a lightweight adaptation for this repository.

## [Unreleased]

## [0.4.9] - 2026-10-27

### Added
- Added admin RPC `quota/modelPriceRules/syncOpenai` and a Sync OpenAI prices button in model management. Fetches flagship/general text Standard/Fast/Flex/Ultrafast and long-context rates from official Markdown, atomically updates prices, preserves custom and other-provider rules, and displays sync time and rates. Failures retain the previous snapshot.
- Added [official GPT-6.1 Terra pricing](https://developers.openai.com/api/docs/models/gpt-6.1-terra): $1 input / $0.05 cached input / $5 output per million tokens; above 272,000 input tokens, $2 / $0.10 / $7.50 for the entire request. Fast / Priority uses 2x pricing; Flex uses 0.5x.
- Support provider prefixes, dated names and underscore aliases. Upgrade database price seeds while preserving custom rules. New requests use the updated prices; historical usage is unchanged.
- Released Windows 0.4.9 installer and standalone executable.

## [0.4.8] - 2026-10-11

### Added
- Added immediate/scheduled warmup actions. Schedules target selected accounts, support fractional-hour delays, one-shot or repeating execution and cancellation, and persist the next run across service restarts.
- Added reset-credit hover/focus details showing individual expiration dates, following sub2api.
- Fetch expiry metadata from the read-only reset-credit endpoint, with explicit unknown, partial and expired-data states. Queries do not consume credits.
- Released Windows 0.4.8 installers and standalone executable.

## [0.3.8] - 2026-05-30

### Fixed
- Fixed local prompt-cache account binding not being reused when a proxy preserved only `x-codex-turn-state` while the request still carried `prompt_cache_key`, and added `anchor_mode` diagnostics.
- Fixed `/v1/messages/count_tokens` requests potentially being forwarded as real Responses generations by using local token estimation.
- Hardened Anthropic-native and Claude Code compatibility against Cloudflare challenges by disabling `zstd` on compatibility streams, adding challenge cooldown and candidate switching, and keeping low-quota accounts as tail fallbacks.
- Normalized Claude Code 2.x mid-conversation `system` messages to the Responses `developer` role to reduce ChatGPT Codex backend Cloudflare challenges.

### Changed
- Bumped the release version to `0.3.8` and synchronized workspace, frontend package, Tauri desktop metadata, and lockfiles.

## [0.3.7] - 2026-05-27

### Added
- Added Linux arm64 as a `release-all.yml` publishing target so it is covered by the unified multi-platform release pipeline.

### Fixed
- Fixed unstable local account routing for OpenAI-compatible `/v1/responses` requests with `prompt_cache_key`; matching client keys now reuse the same upstream account while preserving the original client key upstream.
- Fixed short `prompt_cache_key` values, such as `pc_1`, being skipped by local account binding and spread across balanced rotation.
- Fixed ChatGPT upstream transport errors not continuing failover across candidate accounts.
- Fixed missing beta marker headers during Responses WebSocket upstream handshakes.
- Fixed inconsistent `message_stop` terminal detection for `/v1/messages` compatibility streams.
- Fixed account upserts potentially clearing existing tokens.

### Changed
- Bumped the release version to `0.3.7` and synchronized workspace, frontend package, Tauri desktop metadata, and lockfiles.

## [0.3.6] - 2026-05-25

### Added
- Added scheduled account warmup so configured accounts can be warmed periodically, reducing cold-start failures on first use.
- Added an experimental ChatGPT `/v1/responses` WebSocket upstream transport. When `CODEXMANAGER_USE_WEBSOCKET_UPSTREAM=1` is set, the gateway tries WebSocket first, falls back to HTTP streaming on failure, and keeps upstream proxy, connect-timeout, and protocol-frame validation behavior.

### Changed
- Request logs now show API key names instead of key IDs in the key column, making call-source debugging easier.
- Bumped the release version to `0.3.6` and synchronized workspace, frontend package, Tauri desktop metadata, and lockfiles.

## [0.3.5] - 2026-05-24

### Added
- Added compact model forwarding rules in Settings, scoped to `/v1/responses/compact` requests.
- Added request-gate wait, first-response latency, and slow-trace stdout diagnostics for high-concurrency streaming investigations.

### Fixed
- Fixed Anthropic SSE completed snapshots being replayed after tool calls, which could duplicate assistant text in clients.
- Fixed Chat Completions compatibility when Responses only returns reasoning summaries; streaming and non-streaming responses now expose compatible reasoning fields.
- Suspended automatic token refresh for region-blocked accounts to avoid repeated background retries.
- Cleaned stale Aggregate API/model source routes and orphan automatic catalog models after model sync.
- Added an opt-in request-gate wait cap via `CODEXMANAGER_REQUEST_GATE_WAIT_TIMEOUT_MS=5000` to avoid long head-of-line blocking for matching key/path/model streaming requests.

### Changed
- Bumped the release version to `0.3.5` and synchronized workspace, frontend package, Tauri desktop metadata, and lockfiles.

### Added
- Aggregate API balance checks now refresh automatically on the same interval as account-pool usage polling, controlled by the Settings usage polling worker.
- Added Codex image-generation compatibility: `/v1/responses` now auto-injects the official `image_generation` tool by default to match Codex behavior, explicit tools are forwarded unchanged, and compatible `/v1/images/generations` plus `/v1/images/edits` endpoints are available with `gpt-image-2` as the default image tool model.
- Added an `auth.json` step to the Codex CLI first-time setup guide, clarifying how the platform key, `auth.json`, and `config.toml` fit together.

### Fixed
- Fixed missing Spark dedicated quota display by continuing to parse official `additional_rate_limits[].rate_limit` buckets.
- Adjusted the usage-details dialog so multiple additional quota windows can be shown in two columns with scrolling.

### Changed
- Bumped the release version to `0.3.2` and synchronized workspace, frontend package, Tauri desktop metadata, and lockfiles.
- Bumped the release version to `0.2.6` and synchronized workspace, frontend package, Tauri desktop metadata, and lockfiles.
- Removed recent-commit blocks from README entry pages so they only keep stable feature and documentation entry points.
- Restored the upstream total-timeout setting in the Settings gateway transport card. `CODEXMANAGER_UPSTREAM_TOTAL_TIMEOUT_MS` can now be viewed and changed there; default `0` means no service-side total-duration cutoff.
- Added a dedicated `/v1/images/` block to the Nginx example config for image uploads, large `b64_json` responses, and long-running image generation.
- Synchronized request-log cost estimates with official `gpt-5.5`, `gpt-5.5-pro`, and `gpt-image-*` pricing. Because current usage logs do not include modality buckets, image models are conservatively estimated with official Image token prices.

## [0.2.3] - 2026-04-15

### Fixed
- Restored the native Codex thread-anchor priority for resume flows: when a request already carries `conversation_id` or `x-codex-turn-state`, the gateway no longer lets request-body `prompt_cache_key` take over, reducing compatibility-mode regressions in native Codex context continuation.
- Added targeted regression coverage for native anchors, explicit `prompt_cache_key`, conflicting anchors, and Anthropic-native paths so compatibility fields cannot silently outrank native Codex semantics again.

### Changed
- Bumped the release version to `0.2.3` and synchronized the workspace, frontend package, Tauri desktop metadata, and lockfiles.

## [0.2.0] - 2026-04-12

### Added
- Top-level pages now use background keep-alive caching with a more visible full-area loading overlay, improving revisit latency across desktop, web, and Docker deployments.
- Added targeted regression coverage for `service_tier`, Cloudflare challenge handling, and compatibility forwarding across native Codex, Claude Code, and Gemini CLI request paths.

### Fixed
- Tightened the native Codex passthrough path so the default behavior stays close to the official request shape, keeping only account selection, auth replacement, routing, session affinity, and required internal-field cleanup; Claude and Gemini still use protocol adapters.
- Aligned upstream Codex behavior with the official client: requests sent to `chatgpt.com/backend-api/codex/responses` now map `service_tier=fast` to upstream `priority`, while `/responses/compact` no longer carries `service_tier`.
- Fixed the Claude compatibility path so `fast` service-tier mapping and error-stream model echoing are stable, reducing misleading diagnostics during `403` investigations.

### Changed
- The release version is now `0.2.0`, with workspace, frontend package, Tauri desktop, and external version notes updated together.

## [0.1.19] - 2026-04-08

### Added
- Aggregate API now supports multiple authentication types and custom `action` settings, and passthrough routing can hit the correct upstream based on auth mode and action.
- Internationalization now persists the selected locale and uses split message catalogs, with the remaining dashboard, modal, sidebar, and usage labels localized across languages.

### Fixed
- Fixed Aggregate API passthrough streams so Anthropic `message_stop` events are recognized correctly, reducing premature stream termination and state misreads.
- Fixed the gateway forwarding unsupported `service_tier` values upstream; standard Responses requests now keep only supported values to avoid upstream rejection.
- Changed the collaboration and security entry pages to show Chinese content by default and cleaned up the multilingual doc entry flow to make release docs easier to open from the repository root.

### Changed
- Formal documentation is now organized by language folders, and the root entry documents plus multilingual landing pages were refreshed together.
- The release version is upgraded to `0.1.19`, and the version notes of workspace, front-end package, Tauri desktop, lock file, README, and CHANGELOG are updated simultaneously.

## [0.1.18] - 2026-04-06

### Added
- Added direct sort controls for the account list and a quota-first sorting mode, making it easier to inspect accounts with tight remaining quota.
- Added the initial Gemini CLI compatibility layer, including key request paths such as streaming `tools`, MCP tool names, and SSE `tool_call` handling.

### Fixed
- Fixed the misaligned "quota details" popup on the account page by anchoring the right-hand panel to the center line of the quota overview card.
- Fixed Gemini issues around completed tool output being treated as plain text, streaming cached-token logging, request adaptation compatibility, and token refresh edge cases.

### Changed
- Aligned the Gemini → Codex / Responses request path toward CPA-compatible behavior, including developer messages, tool-name mapping, FIFO `call_id`, `reasoning`, `include`, and `parallel_tool_calls`.
- Removed unused Gemini-path code and refreshed the CPA acknowledgement plus version-related documentation notes.
- The release version is upgraded to `0.1.18`, and the version notes of workspace, front-end package, Tauri desktop, lock file, README, and CHANGELOG are updated simultaneously.

## [0.1.17] - 2026-04-05

### Added
- The request log has a new "final effective service level" caliber. The HTTP/WS log will now retain both the client's explicit `service_tier` and the final value after the request is rewritten, making it easier to check whether the platform Key's default `Fast` is actually sent to the upstream.
- A new global "model forwarding rule" has been added to the settings page, which supports the use of `pattern=target` format for model name rewriting, and takes effect during the runtime request rewriting phase.

### Changed
- The protocol type of common platform keys converges to "wildcard compatibility (Codex / Claude Code)". By default, Claude or Codex / OpenAI semantics are automatically selected according to the request path, reducing the cost of repeatedly maintaining multiple sets of Keys.
- The release version is upgraded to `0.1.17`, and the version notes of workspace, front-end package, Tauri desktop, lock file, README and CHANGELOG are updated simultaneously.

## [0.1.16] - 2026-04-05

### Added
- Added `/v1/responses` WebSocket request support, completed transmission type identification, request header normalization, proxy runtime and request log link.
- A new additional quota window display is added to the account page and usage pop-up window; after refreshing, the remaining quota and reset time of the standard quota and additional quotas such as Code Review/Spark will be displayed uniformly.
- The gateway trace adds a new `CLIENT_SERVICE_TIER` event to record whether the HTTP/WS original request explicitly carries `service_tier`, the original value and the log normalized value, making it easy to quickly distinguish the client's explicit `fast` from the platform Key's default service level.

### Fixed
- Fixed the issue of inconsistent calibers of `service_tier` in HTTP and WS request logs; now `fast` will only be recorded when the client requests that it explicitly carries `service_tier`, and the platform Key default value will no longer be mistakenly recorded as a request to explicitly enable it.
- Fixed the issue where the service level display on the log page is inconsistent with the gateway on-wire value; `priority` will be uniformly displayed as `fast`, and requests that do not explicitly carry a service level will continue to be displayed as `auto`.

### Changed
- The release version is upgraded to `0.1.16`, and the version notes of workspace, front-end package, Tauri desktop, lock file, README and CHANGELOG are updated simultaneously.

## [0.1.15] - 2026-04-03

### Changed
- The release version is upgraded to `0.1.15`, and the version notes of workspace, front-end package, Tauri desktop, running documentation and README are updated simultaneously.

## [0.1.14] - 2026-03-30

### Added
- A new "System Derivation" button and "Single Account Concurrency Limit" are added to the settings page, which can be backfilled according to the current machine resources with one click and take effect immediately.
- The entry layer adds short queue waiting and rapid overload degradation to prevent high concurrency from directly dragging down the service process.

### Changed
- README、workspace、前端包、Tauri 桌面端与版本一致性校验脚本统一提升到 `0.1.14`。

## [0.1.13] - 2026-03-25

### Added
- Added "Aggregation API" management page to support supplier name, sequence priority, classification according to `Codex / Claude`, connectivity testing and minimum forwarding upstream management.
- The platform key has a new `账号轮转 / 聚合 API 轮转` policy, and the aggregation API rotation will hit the corresponding suppliers first in order before continuing to the next channel.

### Fixed
- Fixed the automatic recovery behavior when starting the desktop service and switching between pages, to avoid being paged and restarted after shutting down, and to avoid accidentally clearing data on the dashboard when disconnected.

### Changed
- README, workspace, front-end package, Tauri desktop and version consistency verification scripts have been upgraded to `0.1.13`.

## [0.1.12] - 2026-03-20

### Fixed
- Fixed the issue where the platform key name editing link was not fully transparently transmitted on the desktop; now both Web and the desktop can correctly save and echo the name, and support Chinese names.
- Fixed the issue where the key ID in the platform key list was truncated by default; now it will be directly displayed in full for easy verification and troubleshooting.

### Changed
- README adds sponsorship support entrance and sponsorship area jump, making it easier to locate sponsorship instructions directly from the top of the document.
- The release version is upgraded to `0.1.12`, and workspace, front-end package, Tauri desktop, version consistency verification script and README are updated simultaneously.

## [0.1.11] - 2026-03-20

### Added
- Account management has added new ban identification, ban filtering and "one-click cleanup of banned accounts" portals; `account_deactivated` and `workspace_deactivated` will be automatically recognized as unavailable signals and can be filtered and cleared directly in the list.
- The 5-hour/7-day quota column of the account list will now display the reset time of the respective window; free accounts that only return the 7-day window will also display the reset time in the 7-day column.
- The platform key has new service level configurations: `跟随请求`, `Fast`, and `Flex`, among which `Fast` will be mapped to the upstream `priority`, and `Flex` will be directly transmitted to `flex`.

### Fixed
- Fixed the issue where `serviceTier` was not transparently transmitted when creating/editing the desktop platform key, causing the "service level" to not take effect or be echoed after being saved.
- Fixed the problem of Web occasionally downloading wrong files when refreshing other than the home page, and fixed the copy failure caused by `navigator.clipboard.writeText` being unavailable when copying API Key/login link in some operating environments.
- Fixed an issue where the "Check for Updates" button on the settings page continued to rotate incorrectly when automatically and silently checking for updates; now the loading status is only displayed when manually clicked.

### Changed
- The main link of the gateway continues to close to Codex-first: outbound semantics such as session binding, automatic number and thread switching, `originator` / `User-Agent` / request compression have been further aligned, and the upstream cookie link left behind by the old compatibility path has been removed.
- The setting page replenishment service listening address switch can be switched between `localhost` and `0.0.0.0`; README and documents have also been synchronized to the current mainline path.
- The release version is upgraded to `0.1.11`, and workspace, front-end package, Tauri desktop, version consistency verification script and README are updated simultaneously.

## [0.1.10] - 2026-03-18

### Fixed
- Fixed the problem that the Web / Docker version mistakenly entered the desktop-specific command branch, account enable/disable was missing the `sort` parameter, resulting in the inability to switch states, and the status column was not refreshed in time after the account details failed to be refreshed.
- Fixed an issue where disabled accounts still participate in manual batch refresh and background usage polling; batch refresh and background polling now skip manually disabled accounts and are executed as concurrent workers.
- Fixed the semantic confusion of account status: manual disablement is unified into `disabled`, quota exhaustion and `usage endpoint 401` are unified into `unavailable`, `refresh token 401` related links are also unified into `unavailable`, and the front-end status display is synchronously closed to "Disabled/Unavailable".
- Fixed the problem of Windows local Web `codexmanager-service` / `codexmanager-web` still remaining in the background after the launcher closes the console window; the launcher will now recycle the child process together through the Job Object.

### Changed
- The release version is upgraded to `0.1.10`, and workspace, Tauri desktop version, front-end package version, README latest version description and version consistency test are simultaneously updated.

## [0.1.9] - 2026-03-18

### Added
- The request log now supports backend paging, backend statistics, first try account and try link display, making it easier to distinguish the actual hit account from the final account after failover.
- A new free / 7-day single-window account usage model configuration is added to the settings page. Free accounts will initiate requests according to the setting model.

### Fixed
- Fixed stability issues such as desktop startup misjudgment, `/rpc` empty response, refresh failure caused by missing `spawn_blocking`, out-of-sync refresh of the usage pop-up window, lag on the first page cut, and Hydration inconsistency.
- Fixed issues such as incorrect refresh token extraction, free account request model not being rewritten correctly, unstable priority account behavior, and `503 no available account` missing contextual diagnosis.
- Fixed the issue of verify failure caused by the mismatch between the pnpm version and the current lock file in the release workflow.

### Changed
- The old front-end has been removed, and the desktop and Web management interfaces have been unified to the new `apps` front-end; account management, platform keys, request logs, settings pages, and navigation layouts have all undergone a desktop-first restructuring.
- Codex Request links continue to be aligned with actual on-wire behavior: login / callback / workspace validation, refresh semantics, `/v1/responses` and `/v1/responses/compact` rewrites, thread anchors, request compression, error summaries, and fallback diagnostics all continue to be aligned.
- Gateway failure diagnostics and disk logging continue to converge, with compact false success bodies, HTML/challenge pages, `401 refresh` subclasses, and exhausted candidate links all outputting clearer summaries.
- Unifiedly upgrade the release version to `0.1.9`, and simultaneously update the workspace, Tauri desktop, `tauri.conf.json` and front-end package versions.
- The fixed Tauri CLI versions in the GitHub Release workflow have been aligned to the actual version currently used on the Rust side, reducing the risk of CLI / crate drift in the packaging phase.
- The release documentation and README have been updated to `v0.1.9`, and the front-end static export directory description has been corrected to `apps/out`.

## [0.1.8] - 2026-03-11

### Fixed
- Removed the default `https://api.openai.com/v1` fallback path for ChatGPT-backed requests; upstream `challenge` and `403` outcomes are now returned from the primary login-account path instead of being rewritten into local fallback errors.
- ChatGPT login-account requests now recover from `401` by refreshing the local `access_token` with the stored `refresh_token` and retrying the current request once.

### Changed
- ChatGPT login-account turns now use `access_token` directly on the primary upstream path and no longer mix in `api_key_access_token` semantics.
- Synthetic gateway terminal failures now return structured OpenAI-style `error.message / error.type / error.code` payloads while keeping the existing trace and error-code headers.

## [0.1.7] - 2026-03-11

### Added
- Added gateway transmission parameters to the settings page: supports direct configuration of upstream streaming timeout and SSE keepalive interval, and takes effect hot when the service is running.
- Desktop startup snapshot completion: Dashboard statistics, account usage status, and request log first screen will restore the most recent snapshot first, reducing all 0/unknown status after source code running or service restart.

### Fixed
- Fixed an issue where `codexmanager-web`'s access password session could continue to be used across reboots; after closing and reopening the Web process, the old login cookie would become invalid and the password would need to be re-verified.
- Fix the startup and root routing compatibility issue when the source code runs `codexmanager-web`, and reduce the inconsistent behavior of Web static resources and root paths under Axum routing.
- Fixed the SSE idle disconnection and reconnection problem in long output scenarios to reduce the probability of long-term streaming responses being interrupted by misjudgment.
- Fixed desktop interaction issues such as saving the upstream agent on the settings page, closing the platform key creation pop-up window and submitting it repeatedly, and the account form not refreshing after successful login.
- Fixed some upstream compatibility issues caused by the default additional version parameters when pulling models, and changed the model request to not include a version number by default.
- Fixed the problem of inconsistent account merging logic between account import and login callback, and unified new or updated accounts according to the same identity rule.
- Fixed the problem of tool truncation in Claude / Anthropic `/v1/messages` when adapted to multiple MCP server scenarios; tools for subsequent servers will no longer be lost due to the first 16 tools being full.
- Fixed the problem of missing long tool name shortening and response restoration in Claude / Anthropic `/v1/messages` to avoid mapping instability when the MCP tool name is too long.

### Changed
- The gateway failure response adds structured `errorCode` / `errorDetail` fields, and simultaneously adds `X-CodexManager-Error-Code`, `X-CodexManager-Trace-Id` response headers to facilitate the client and log system to track failed links.
- Protocol adaptation continues to align the Codex / OpenAI compatible ecology: further unifying the forwarding semantics of `/v1/chat/completions`, `/v1/responses`, Claude `/v1/messages`, and stabilizing `tools` / `tool_calls`, thinking/reasoning, streaming bridging, and response restoration links.
- The settings page and runtime configuration continue to converge: high-frequency configurations such as background tasks, gateway transmission, upstream proxy, Web security, etc. are unified and persisted by `app_settings` and backfilled into the current process.
- The desktop and service startup links will continue to be managed, the startup boundaries and startup sequences between Web/service/desktop will be converged, and the behavior bifurcation between source code running and packaged running will be reduced.
- The project continues to promote long-term maintenance-oriented reconstruction management: the front-end main entrance, settings page, request log view, Tauri command registration, service life cycle, gateway protocol adapter, HTTP bridge, upstream attempt flow and other areas have further split module boundaries to reduce the coupling between large files and the root-level facade.
- The service/gateway directory structure continues to converge, and more wildcard imports, cross-layer direct connections, and extremely long facade lists have been replaced by explicit dependencies and layered modules, making subsequent maintenance and protocol regression positioning costs lower.
- The release link continues to converge to `release-all.yml` single entry, and the front-end construction products and protocol regression baselines are reused to reduce the risk of protocol regression during repeated builds and releases.

## [0.1.6] - 2026-03-07

### Fixed
- Fix the problem that `release-all.yml` still relies heavily on pre-built front-end artifacts when `run_verify` is manually closed; each platform task will automatically fall back to local `pnpm install + build` when `codexmanager-frontend-dist` is missing.

### Changed
- Windows Desktop release products continue to converge, only `CodexManager-portable.exe` portable version will be retained, and no additional `CodexManager-windows-portable.zip` will be generated.
- Improve SOCKS5 upstream proxy support and normalization, and supplement the proxy protocol prompt copy in the settings page.

## [0.1.5] - 2026-03-06

### Added
- Added "Import by folder": On the desktop, you can directly select a directory, recursively scan the `.json` files and import accounts in batches.
- Added OpenAI upstream proxy configuration and request header convergence policy switches, which can be saved directly on the settings page and take effect immediately.
- Supplement the chat tools hit probe script to facilitate local verification of whether the tool call is actually hit and transparently transmitted.

### Fixed
- Fix `tool_calls` / `tools` related regressions: Complete the tool call retention, tool name shortening and response restoration link in the chat aggregation path to avoid tool calls being lost or having names confused in OpenAI compatible returns, streaming increments and adaptation transformations.
- Improve the OpenClaw / Anthropic compatibility return adaptation to ensure that tool calls, SSE deltas, and non-streaming JSON responses are all correctly restored in a compatible format.
- Request log tracing is enhanced to supplement the original path, adaptation path and more context to facilitate locating `/v1/chat/completions -> /v1/responses` forwarding and protocol adaptation issues.

### Changed
- Gateway protocol adaptation further aligns Codex CLI: `/v1/chat/completions` and `/v1/responses` two links uniformly converge to Codex `responses` semantics, the upstream streaming/non-streaming behavior is closer to the official one, and is compatible with OpenAI compatible calls of clients such as Cherry Studio.
- The commonly used configurations at the top of the settings page have been changed to a unified three-column row layout, and the agent configuration remains consistent with it; it also supports hiding in the system tray after closing the window.
- The publishing process is integrated into a single one-click multi-platform workflow, and the desktop product form is converged; Windows directly provides portable exe, macOS uniformly uses DMG distribution.

## [0.1.4] - 2026-03-03

### Added
- Added "One-click removal of unavailable Free accounts": clean up "Unavailable + free plan" accounts in batches and return scan/skip/deletion statistics.
- Added "Export User": Supports selecting a local directory and exporting according to "one account, one JSON file".
- Import compatibility enhancement: support automatic identification of `tokens.*`, top-level `*_token`, camelCase fields (such as `accessToken` / `idToken` / `refreshToken`).

### Fixed
- Compatible with old services: The front-end will automatically normalize the top-level token format before importing it to avoid reporting `missing field: tokens` in the old version of the back-end.

### Changed
- The operation area of ​​the account management page is integrated into a single "Account Operation" drop-down menu, replacing the stack of multiple buttons on the right, making the interface more concise.

[Unreleased]: https://github.com/qxcnm/Codex-Manager/compare/v0.3.8...HEAD
[0.3.8]: https://github.com/qxcnm/Codex-Manager/compare/v0.3.7...v0.3.8
[0.3.7]: https://github.com/qxcnm/Codex-Manager/compare/v0.3.6...v0.3.7
[0.3.6]: https://github.com/qxcnm/Codex-Manager/compare/v0.3.5...v0.3.6
[0.3.5]: https://github.com/qxcnm/Codex-Manager/compare/v0.3.4...v0.3.5
[0.2.6]: https://github.com/qxcnm/Codex-Manager/compare/v0.2.3...v0.2.6
[0.2.3]: https://github.com/qxcnm/Codex-Manager/compare/v0.2.0...v0.2.3
[0.2.0]: https://github.com/qxcnm/Codex-Manager/releases/tag/v0.2.0
[0.1.19]: https://github.com/qxcnm/Codex-Manager/releases/tag/v0.1.19
[0.1.17]: https://github.com/qxcnm/Codex-Manager/releases/tag/v0.1.17
[0.1.16]: https://github.com/qxcnm/Codex-Manager/releases/tag/v0.1.16
[0.1.15]: https://github.com/qxcnm/Codex-Manager/releases/tag/v0.1.15
[0.1.14]: https://github.com/qxcnm/Codex-Manager/releases/tag/v0.1.14
[0.1.13]: https://github.com/qxcnm/Codex-Manager/releases/tag/v0.1.13
[0.1.12]: https://github.com/qxcnm/Codex-Manager/releases/tag/v0.1.12
[0.1.11]: https://github.com/qxcnm/Codex-Manager/compare/v0.1.10...v0.1.11
[0.1.10]: https://github.com/qxcnm/Codex-Manager/releases/tag/v0.1.10
[0.1.9]: https://github.com/qxcnm/Codex-Manager/releases/tag/v0.1.9
[0.1.8]: https://github.com/qxcnm/Codex-Manager/releases/tag/v0.1.8
[0.1.7]: https://github.com/qxcnm/Codex-Manager/releases/tag/v0.1.7
[0.1.6]: https://github.com/qxcnm/Codex-Manager/releases/tag/v0.1.6
[0.1.5]: https://github.com/qxcnm/Codex-Manager/releases/tag/v0.1.5
[0.1.4]: https://github.com/qxcnm/Codex-Manager/releases/tag/v0.1.4
