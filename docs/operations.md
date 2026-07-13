# Operations

## Optional Sentry telemetry and privacy

Remote telemetry is **disabled by default**. Jellyx transmits nothing unless both conditions are true:

1. The user explicitly enables **Settings → Privacy → Share anonymous failure signals**. This choice is stored locally and can be turned off at any time.
2. The packaged build contains a non-empty `JELLYX_SENTRY_DSN`, or the application operator supplies one at launch. A non-empty runtime value overrides the packaged default, which is useful for development.

A DSN does not enable consent, and consent without a DSN does not create a Sentry client or make a network request. Disabling the setting first closes an atomic consent gate, then unbinds the Sentry Hub and drops its transport guard. The gate rejects queued envelopes at transport send, flush, and shutdown time, so the guard's close-triggered flush cannot transmit after opt-out. Deleting `JELLYX_SENTRY_DSN` disables it for all users on the next start.

The Sentry client is initialized with the Jellyx release and `desktop` environment only after both gates pass. Its `ClientInitGuard` is retained for the application lifetime. A defensive `before_send` callback rebuilds every outgoing event from a strict allowlist and drops unsafe identifiers. Consequently events contain only stable `component:event` failure identifiers, threshold alert identifiers, and a latency value bounded to 60,000 ms. They never contain paths, URLs, titles, media metadata, usernames, request bodies, breadcrumbs, raw backend errors, device identifiers, exceptions, stack traces, contexts, or SDK-added extras.

### Operator setup

- **Setup:** Create a Sentry project for Jellyx and set the repository Actions secret **`JELLYX_SENTRY_DSN`** to its non-empty project DSN. The Release workflow fails before any platform build if it is absent and passes it only as a build environment variable. Never echo or commit the DSN.
- **Packaged default:** The release build embeds the secret as its default DSN. An empty or absent runtime `JELLYX_SENTRY_DSN` uses that packaged default.
- **Runtime override:** A non-empty `JELLYX_SENTRY_DSN` at application launch overrides the packaged default, which is useful for development and controlled deployments.
- **Rotation:** Create a replacement client key in Sentry, update the Actions secret, release a new build, then revoke the old key after supported builds have been replaced.

Create aggregate/count-based alerts for stable event messages (for example `updater:periodic_check_failed`), threshold identifiers `observability:error_rate_1_percent`, `observability:error_rate_2_percent`, and `observability:error_rate_5_percent`, and `updater:latest_release_fetch:latency_high`. Error-rate threshold events are emitted at most once per identifier per rolling hour to prevent recursion and alert floods. Do not add server-side enrichment that collects user data.

## Local operational alerts and diagnostics

Jellyx keeps a bounded, durable local record of expected failures at its app-data location. It survives restart, has no external telemetry dependency, and never stores paths, URLs, track titles, or backend error text. A failed diagnostic write is non-fatal and does not affect playback.

Call the Tauri command `get_failure_diagnostics` from a trusted local diagnostic session. It returns:

- `counters`: cumulative counts by stable `component:event` identifier.
- `recentEvents`: the latest 64 identifiers, oldest first.
- `eventsLastHour`: bounded recent failure volume (not a rate).
- `errorRatePercent` and `operationRates`: the rolling-hour failure percentage and per-operation `{ attempts, failures, errorRatePercent }`. The denominator contains only the latest 64 completed audio output, proxy forward, and updater fetch operations.
- `latency`: aggregate operation timings (`observations`, `averageMs`, `maxMs`). The updater records its GitHub latest-release fetch latency; its aggregates are lifetime-of-diagnostics values, not rolling-window percentiles.
- `alerts`: redacted, in-app health alerts. The UI or a local health command can retrieve and display these through the same command.

Alerts have explicit local thresholds: rolling error rates at or above 1%, 2%, and 5% produce `observability:error_rate_1_percent`, `observability:error_rate_2_percent`, and `observability:error_rate_5_percent`. An operation with a recorded latency of at least 2,000 ms produces `<component>:<operation>:latency_high`. Counters, event history, operation history, and metric identifiers are each capped at 64 entries. Remote delivery remains subject to the separate consent-and-DSN gate above.

Use repeated identifiers to triage a failing component (for example, `proxy:forward_error`). The existing structured stderr records remain useful for local logs; do not add sensitive values to either interface.

## Public release recovery

If a release is already public and must be withdrawn, first stop distribution by running **Release Recovery** with the affected `release_tag` and the exact confirmation `REVOKE_PUBLIC_RELEASE`. It only marks the existing release as a draft and verifies that state. GitHub's latest-release updater endpoint excludes drafts, so this revokes updater visibility without deleting evidence or assets.

Then investigate, build a higher-version patch, validate all artifacts and checksums, and publish the patch through the normal Release workflow. Communicate the withdrawal and patch notes to users. Do not delete assets automatically or as an incident reflex: after review, a maintainer may explicitly delete an unrecoverable draft with `gh release delete <tag> --yes`; that destructive action is intentionally outside automation.
