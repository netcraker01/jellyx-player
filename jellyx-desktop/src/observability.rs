//! Bounded, redacted local diagnostics and operational alerts.
//!
//! The durable file stores only stable component/event identifiers and event
//! timestamps. It never contains paths, URLs, titles, or backend error text.

use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

const MAX_RECENT_EVENTS: usize = 64;
const MAX_COUNTERS: usize = 64;
const RATE_WINDOW_SECONDS: u64 = 60 * 60;
/// Cooldown for re-sending the same operational alert to Sentry.
/// Independent from RATE_WINDOW_SECONDS so a sustained error rate doesn't
/// spam Sentry every time the window shifts. One alert per identifier per
/// cooldown period.
const ALERT_COOLDOWN_SECONDS: u64 = 6 * 60 * 60;
/// Alert thresholds for the bounded rolling operation window, in percent.
const ERROR_RATE_ALERT_THRESHOLDS_PERCENT: [f64; 3] = [1.0, 2.0, 5.0];
/// Alert when a measured operation takes at least three seconds.
const LATENCY_ALERT_THRESHOLD_MS: u64 = 3_000;
const DIAGNOSTICS_FILE: &str = "failure-diagnostics.json";
const MAX_REPORTED_LATENCY_MS: u64 = 60_000;

/// The only remote payload shape Jellyx permits. It deliberately has no free
/// text field, so callers cannot accidentally attach user or backend data.
#[derive(Debug, Clone, PartialEq, Eq)]
struct TelemetryEvent {
    identifier: String,
    value: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RemoteTelemetryConfig {
    consented: bool,
    dsn_configured: bool,
}

trait TelemetrySink {
    fn capture(&self, event: TelemetryEvent);
}

struct SentrySink;

impl TelemetrySink for SentrySink {
    fn capture(&self, event: TelemetryEvent) {
        let mut sentry_event = sentry::protocol::Event {
            message: Some(event.identifier),
            level: sentry::Level::Error,
            ..Default::default()
        };
        if let Some(value) = event.value {
            // The bounded value is operational timing only, never a user value.
            sentry_event
                .tags
                .insert("latency_ms".into(), value.to_string());
        }
        sentry::capture_event(sentry_event);
    }
}

/// Applies the live consent gate at the transport boundary, including flush and
/// shutdown.  This is intentionally below Sentry's client queue: dropping a
/// `ClientInitGuard` flushes that queue, so an opt-out must be able to reject
/// envelopes that were accepted before consent was withdrawn.
struct ConsentAwareTransport {
    enabled: Arc<AtomicBool>,
    inner: Arc<dyn sentry::Transport>,
}

impl sentry::Transport for ConsentAwareTransport {
    fn send_envelope(&self, envelope: sentry::protocol::Envelope) {
        if self.enabled.load(Ordering::Acquire) {
            self.inner.send_envelope(envelope);
        }
    }

    fn flush(&self, timeout: Duration) -> bool {
        self.enabled.load(Ordering::Acquire) && self.inner.flush(timeout)
    }

    fn shutdown(&self, timeout: Duration) -> bool {
        self.enabled.load(Ordering::Acquire) && self.inner.shutdown(timeout)
    }
}

#[derive(Clone)]
struct ConsentAwareTransportFactory {
    enabled: Arc<AtomicBool>,
}

impl sentry::TransportFactory for ConsentAwareTransportFactory {
    fn create_transport_with_options(
        &self,
        options: sentry::TransportOptions,
    ) -> Arc<dyn sentry::Transport> {
        let inner =
            sentry::transports::DefaultTransportFactory.create_transport_with_options(options);
        Arc::new(ConsentAwareTransport {
            enabled: self.enabled.clone(),
            inner,
        })
    }
}

struct RemoteTelemetryState {
    config: RemoteTelemetryConfig,
    transport_enabled: Arc<AtomicBool>,
    guard: Option<sentry::ClientInitGuard>,
    alert_last_sent: BTreeMap<String, u64>,
}

static REMOTE_TELEMETRY: OnceLock<Mutex<RemoteTelemetryState>> = OnceLock::new();

fn remote_telemetry() -> &'static Mutex<RemoteTelemetryState> {
    REMOTE_TELEMETRY.get_or_init(|| {
        Mutex::new(RemoteTelemetryState {
            config: RemoteTelemetryConfig {
                consented: false,
                dsn_configured: false,
            },
            transport_enabled: Arc::new(AtomicBool::new(false)),
            guard: None,
            alert_last_sent: BTreeMap::new(),
        })
    })
}

fn configured_dsn() -> Option<String> {
    std::env::var("JELLYX_SENTRY_DSN")
        .ok()
        .filter(|dsn| !dsn.trim().is_empty())
        .or_else(|| {
            #[cfg(jellyx_sentry_dsn)]
            {
                include!(concat!(env!("CARGO_MANIFEST_DIR"), "/.sentry-dsn.rs"))
            }
            #[cfg(not(jellyx_sentry_dsn))]
            {
                None
            }
        })
}

/// Configure optional remote telemetry after reading persisted user consent.
/// A DSN alone never enables collection. Withdrawal unbinds the current Hub
/// and drops the guard while holding the same lock used by capture, so no new
/// capture can race past an opt-out. The transport gate is switched off before
/// guard disposal, so its close-triggered flush cannot transmit queued data.
pub fn configure_remote_telemetry(consented: bool) {
    let dsn = configured_dsn();
    let config = RemoteTelemetryConfig {
        consented,
        dsn_configured: dsn.is_some(),
    };
    let Ok(mut state) = remote_telemetry().lock() else {
        return;
    };
    state.config = config;
    if !consented || dsn.is_none() {
        state.transport_enabled.store(false, Ordering::Release);
        state.alert_last_sent.clear();
        sentry::Hub::current().bind_client(None);
        drop(state.guard.take());
        return;
    }
    if state.guard.is_none() {
        state.transport_enabled.store(true, Ordering::Release);
        let transport_enabled = state.transport_enabled.clone();
        state.guard = Some(sentry::init(sentry::ClientOptions {
            dsn: dsn.and_then(|value| value.parse().ok()),
            release: sentry::release_name!(),
            environment: Some("desktop".into()),
            before_send: Some(Arc::new(sanitize_sentry_event)),
            transport: Some(Arc::new(ConsentAwareTransportFactory {
                enabled: transport_enabled,
            })),
            ..Default::default()
        }));
    }
}

fn capture_if_opted_in(
    sink: &dyn TelemetrySink,
    config: RemoteTelemetryConfig,
    event: TelemetryEvent,
) {
    if config.consented && config.dsn_configured && is_telemetry_identifier(&event.identifier) {
        sink.capture(event);
    }
}

fn capture_remote(event: TelemetryEvent) {
    // Keep the lock through capture: consent withdrawal cannot switch off the
    // Hub until this capture completes, and no capture starts after it does.
    if let Ok(state) = remote_telemetry().lock() {
        capture_if_opted_in(&SentrySink, state.config, event);
    }
}

fn capture_alerts_if_opted_in(
    sink: &dyn TelemetrySink,
    config: RemoteTelemetryConfig,
    alerts: &[OperationalAlert],
    last_sent: &mut BTreeMap<String, u64>,
    now: u64,
) {
    if !config.consented || !config.dsn_configured {
        return;
    }
    for alert in alerts {
        if last_sent
            .get(&alert.identifier)
            .is_some_and(|previous| now.saturating_sub(*previous) <= ALERT_COOLDOWN_SECONDS)
        {
            continue;
        }
        if last_sent.len() == MAX_COUNTERS && !last_sent.contains_key(&alert.identifier) {
            continue;
        }
        capture_if_opted_in(
            sink,
            config,
            TelemetryEvent {
                identifier: alert.identifier.clone(),
                value: None,
            },
        );
        last_sent.insert(alert.identifier.clone(), now);
    }
}

fn capture_remote_alerts(alerts: &[OperationalAlert], now: u64) {
    if let Ok(mut state) = remote_telemetry().lock() {
        let config = state.config;
        let mut last_sent = std::mem::take(&mut state.alert_last_sent);
        capture_alerts_if_opted_in(&SentrySink, config, alerts, &mut last_sent, now);
        state.alert_last_sent = last_sent;
    }
}

/// Drop every SDK-populated field and rebuild the event from the allowlist.
/// This removes requests, users, breadcrumbs, contexts, exceptions, stack
/// traces, extras, device details, and any SDK integrations' added data.
fn sanitize_sentry_event(
    event: sentry::protocol::Event<'static>,
) -> Option<sentry::protocol::Event<'static>> {
    let identifier = event.message?;
    if !is_telemetry_identifier(&identifier) {
        return None;
    }
    Some(sentry::protocol::Event {
        message: Some(identifier),
        level: sentry::Level::Error,
        ..Default::default()
    })
}

fn is_telemetry_identifier(identifier: &str) -> bool {
    identifier.split(':').all(|part| {
        !part.is_empty()
            && part.len() <= 64
            && part
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    }) && identifier.matches(':').count() >= 1
        && identifier.len() <= 128
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FailureDiagnostics {
    pub counters: BTreeMap<String, u64>,
    pub recent_events: Vec<String>,
    pub events_last_hour: u64,
    pub error_rate_percent: f64,
    pub operation_rates: BTreeMap<String, OperationRate>,
    pub latency: BTreeMap<String, LatencyMetric>,
    pub alerts: Vec<OperationalAlert>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LatencyMetric {
    pub observations: u64,
    pub average_ms: u64,
    pub max_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationRate {
    pub attempts: u64,
    pub failures: u64,
    pub error_rate_percent: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationalAlert {
    pub identifier: String,
    pub threshold_percent: f64,
    pub observed_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RecordedEvent {
    identifier: String,
    occurred_at: u64,
}

#[derive(Clone, Serialize, Deserialize)]
struct RecordedOperation {
    identifier: String,
    succeeded: bool,
    occurred_at: u64,
}

#[derive(Clone, Default, Serialize, Deserialize)]
struct FailureCollector {
    counters: BTreeMap<String, u64>,
    recent_events: VecDeque<RecordedEvent>,
    operation_results: VecDeque<RecordedOperation>,
    latency: BTreeMap<String, LatencyAccumulator>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
struct LatencyAccumulator {
    observations: u64,
    total_ms: u64,
    max_ms: u64,
}

impl FailureCollector {
    fn record(&mut self, identifier: String, occurred_at: u64) {
        if self.counters.contains_key(&identifier) || self.counters.len() < MAX_COUNTERS {
            *self.counters.entry(identifier.clone()).or_default() += 1;
        }
        if self.recent_events.len() == MAX_RECENT_EVENTS {
            self.recent_events.pop_front();
        }
        self.recent_events.push_back(RecordedEvent {
            identifier,
            occurred_at,
        });
    }

    fn record_latency(&mut self, identifier: String, elapsed_ms: u64) {
        if self.latency.contains_key(&identifier) || self.latency.len() < MAX_COUNTERS {
            let metric = self.latency.entry(identifier).or_default();
            metric.observations = metric.observations.saturating_add(1);
            metric.total_ms = metric.total_ms.saturating_add(elapsed_ms);
            metric.max_ms = metric.max_ms.max(elapsed_ms);
        }
    }

    fn record_operation(&mut self, identifier: String, succeeded: bool, occurred_at: u64) {
        if self.operation_results.len() == MAX_RECENT_EVENTS {
            self.operation_results.pop_front();
        }
        self.operation_results.push_back(RecordedOperation {
            identifier,
            succeeded,
            occurred_at,
        });
    }

    fn diagnostics(&self, now: u64) -> FailureDiagnostics {
        let events_last_hour = self
            .recent_events
            .iter()
            .filter(|event| now.saturating_sub(event.occurred_at) <= RATE_WINDOW_SECONDS)
            .count() as u64;
        let latency = self
            .latency
            .iter()
            .map(|(identifier, metric)| {
                let average_ms = if metric.observations == 0 {
                    0
                } else {
                    metric.total_ms / metric.observations
                };
                (
                    identifier.clone(),
                    LatencyMetric {
                        observations: metric.observations,
                        average_ms,
                        max_ms: metric.max_ms,
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();
        let mut rates = BTreeMap::<String, (u64, u64)>::new();
        for operation in self
            .operation_results
            .iter()
            .filter(|entry| now.saturating_sub(entry.occurred_at) <= RATE_WINDOW_SECONDS)
        {
            let rate = rates.entry(operation.identifier.clone()).or_default();
            rate.0 += 1;
            if !operation.succeeded {
                rate.1 += 1;
            }
        }
        let attempts = rates.values().map(|(attempts, _)| attempts).sum::<u64>();
        let failures = rates.values().map(|(_, failures)| failures).sum::<u64>();
        let error_rate_percent = if attempts == 0 {
            0.0
        } else {
            failures as f64 * 100.0 / attempts as f64
        };
        let operation_rates = rates
            .into_iter()
            .map(|(identifier, (attempts, failures))| {
                let error_rate_percent = failures as f64 * 100.0 / attempts as f64;
                (
                    identifier,
                    OperationRate {
                        attempts,
                        failures,
                        error_rate_percent,
                    },
                )
            })
            .collect();
        let mut alerts = ERROR_RATE_ALERT_THRESHOLDS_PERCENT
            .iter()
            .filter(|threshold| error_rate_percent >= **threshold)
            .map(|threshold| OperationalAlert {
                identifier: format!("observability:error_rate_{}_percent", *threshold as u64),
                threshold_percent: *threshold,
                observed_percent: error_rate_percent,
            })
            .collect::<Vec<_>>();
        for (identifier, metric) in &latency {
            if metric.max_ms >= LATENCY_ALERT_THRESHOLD_MS {
                alerts.push(OperationalAlert {
                    identifier: format!("{identifier}:latency_high"),
                    threshold_percent: LATENCY_ALERT_THRESHOLD_MS as f64,
                    observed_percent: metric.max_ms as f64,
                });
            }
        }
        FailureDiagnostics {
            counters: self.counters.clone(),
            recent_events: self
                .recent_events
                .iter()
                .map(|event| event.identifier.clone())
                .collect(),
            events_last_hour,
            error_rate_percent,
            operation_rates,
            latency,
            alerts,
        }
    }
}

static FAILURE_COLLECTOR: OnceLock<Mutex<FailureCollector>> = OnceLock::new();

fn diagnostics_path() -> PathBuf {
    jellyx_core::shared::utils::data_dir().join(DIAGNOSTICS_FILE)
}

fn load_collector(path: &Path) -> FailureCollector {
    let mut collector: FailureCollector = std::fs::read(path)
        .ok()
        .and_then(|contents| serde_json::from_slice(&contents).ok())
        .unwrap_or_default();
    while collector.recent_events.len() > MAX_RECENT_EVENTS {
        collector.recent_events.pop_front();
    }
    while collector.operation_results.len() > MAX_RECENT_EVENTS {
        collector.operation_results.pop_front();
    }
    while collector.counters.len() > MAX_COUNTERS {
        let Some(identifier) = collector.counters.keys().next().cloned() else {
            break;
        };
        collector.counters.remove(&identifier);
    }
    while collector.latency.len() > MAX_COUNTERS {
        let Some(identifier) = collector.latency.keys().next().cloned() else {
            break;
        };
        collector.latency.remove(&identifier);
    }
    collector
}

fn collector() -> &'static Mutex<FailureCollector> {
    FAILURE_COLLECTOR.get_or_init(|| Mutex::new(load_collector(&diagnostics_path())))
}

fn persist_collector(path: &Path, collector: &FailureCollector) -> std::io::Result<()> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    std::fs::create_dir_all(parent)?;
    let temporary = path.with_extension("json.tmp");
    let mut bounded = collector.clone();
    while bounded.operation_results.len() > MAX_RECENT_EVENTS {
        bounded.operation_results.pop_front();
    }
    let bytes = serde_json::to_vec(&bounded).map_err(std::io::Error::other)?;
    std::fs::write(&temporary, bytes)?;
    std::fs::rename(temporary, path)
}

fn stable_identifier(component: &str, event: &str) -> String {
    let is_stable = |part: &str| {
        !part.is_empty()
            && part.len() <= 64
            && part
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    };
    if is_stable(component) && is_stable(event) {
        format!("{component}:{event}")
    } else {
        "observability:invalid_identifier".to_string()
    }
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |time| time.as_secs())
}

/// Record a redacted failure signal. Persistence failures are deliberately
/// non-fatal: diagnostics must never interrupt playback or application startup.
pub fn expected_failure(component: &str, event: &str) {
    let identifier = stable_identifier(component, event);
    if let Ok(mut collector) = collector().lock() {
        collector.record(identifier.clone(), unix_seconds());
        if persist_collector(&diagnostics_path(), &collector).is_err() {
            eprintln!("{{\"component\":\"observability\",\"event\":\"persist_failed\",\"outcome\":\"expected_failure\"}}");
        }
    }
    let (component, event) = identifier
        .split_once(':')
        .unwrap_or(("observability", "invalid_identifier"));
    eprintln!(
        "{{\"component\":\"{component}\",\"event\":\"{event}\",\"outcome\":\"expected_failure\"}}"
    );
    capture_remote(TelemetryEvent {
        identifier,
        value: None,
    });
}

/// Record a completed relevant operation. The bounded rolling denominator is
/// made only from these records; failures alone are never presented as a rate.
pub fn record_operation(component: &str, operation: &str, succeeded: bool) {
    let identifier = stable_identifier(component, operation);
    let now = unix_seconds();
    let alerts = if let Ok(mut collector) = collector().lock() {
        collector.record_operation(identifier, succeeded, now);
        if persist_collector(&diagnostics_path(), &collector).is_err() {
            eprintln!("{{\"component\":\"observability\",\"event\":\"persist_failed\",\"outcome\":\"operation\"}}");
        }
        collector.diagnostics(now).alerts
    } else {
        Vec::new()
    };
    capture_remote_alerts(&alerts, now);
}

/// Record a redacted latency metric for an operation. The operation identifier
/// follows the same stable identifier rules as failures, and only aggregate
/// millisecond values are persisted.
pub fn record_latency(component: &str, operation: &str, elapsed_ms: u64) {
    let identifier = stable_identifier(component, operation);
    if let Ok(mut collector) = collector().lock() {
        collector.record_latency(identifier.clone(), elapsed_ms);
        if persist_collector(&diagnostics_path(), &collector).is_err() {
            eprintln!("{{\"component\":\"observability\",\"event\":\"persist_failed\",\"outcome\":\"latency\"}}");
        }
    }
    // Report only a thresholded, bounded aggregate signal. The value is never
    // a path, URL, title, metadata field, device identifier, or raw error.
    if elapsed_ms >= LATENCY_ALERT_THRESHOLD_MS {
        capture_remote(TelemetryEvent {
            identifier: format!("{identifier}:latency_high"),
            value: Some(elapsed_ms.min(MAX_REPORTED_LATENCY_MS)),
        });
    }
}

pub fn failure_diagnostics() -> FailureDiagnostics {
    collector()
        .lock()
        .map(|collector| collector.diagnostics(unix_seconds()))
        .unwrap_or_default()
}

impl Default for FailureDiagnostics {
    fn default() -> Self {
        Self {
            counters: BTreeMap::new(),
            recent_events: Vec::new(),
            events_last_hour: 0,
            error_rate_percent: 0.0,
            operation_rates: BTreeMap::new(),
            latency: BTreeMap::new(),
            alerts: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sentry::Transport;

    fn temporary_path() -> PathBuf {
        std::env::temp_dir().join(format!("jellyx-diagnostics-{}", uuid::Uuid::new_v4()))
    }

    #[test]
    fn collector_is_bounded_redacted_and_reports_failure_volume() {
        let mut collector = FailureCollector::default();
        for _ in 0..(MAX_RECENT_EVENTS + 1) {
            collector.record(stable_identifier("audio", "stream_error"), 100);
        }
        let diagnostics = collector.diagnostics(100);
        assert_eq!(diagnostics.recent_events.len(), MAX_RECENT_EVENTS);
        assert_eq!(diagnostics.events_last_hour, MAX_RECENT_EVENTS as u64);
        assert_eq!(diagnostics.error_rate_percent, 0.0);
        assert_eq!(
            diagnostics.counters["audio:stream_error"],
            (MAX_RECENT_EVENTS + 1) as u64
        );
        assert_eq!(
            stable_identifier("audio", "error /secret/path"),
            "observability:invalid_identifier"
        );
    }

    #[test]
    fn operational_error_rate_uses_a_bounded_rolling_operation_denominator() {
        let mut collector = FailureCollector::default();
        collector.record_operation("updater:latest_release_fetch".into(), true, 100);
        collector.record_operation("updater:latest_release_fetch".into(), false, 100);
        collector.record_operation("proxy:forward_request".into(), true, 100);
        let diagnostics = collector.diagnostics(100);
        assert_eq!(diagnostics.error_rate_percent, 100.0 / 3.0);
        assert_eq!(
            diagnostics.operation_rates["updater:latest_release_fetch"].attempts,
            2
        );
        assert_eq!(
            diagnostics.operation_rates["updater:latest_release_fetch"].failures,
            1
        );
        assert!(diagnostics
            .alerts
            .iter()
            .any(|alert| alert.identifier == "observability:error_rate_1_percent"));
        assert!(diagnostics
            .alerts
            .iter()
            .any(|alert| alert.identifier == "observability:error_rate_2_percent"));
        assert!(diagnostics
            .alerts
            .iter()
            .any(|alert| alert.identifier == "observability:error_rate_5_percent"));
        assert_eq!(
            collector
                .diagnostics(100 + RATE_WINDOW_SECONDS + 1)
                .error_rate_percent,
            0.0
        );
    }

    #[test]
    fn latency_metrics_are_bounded_redacted_and_surface_threshold_alerts() {
        let mut collector = FailureCollector::default();
        collector.record_latency(stable_identifier("updater", "latest_release_fetch"), 3_001);
        let diagnostics = collector.diagnostics(100);
        assert_eq!(
            diagnostics.latency["updater:latest_release_fetch"].average_ms,
            3_001
        );
        assert!(diagnostics.alerts.iter().any(|alert| {
            alert.identifier == "updater:latest_release_fetch:latency_high"
                && alert.threshold_percent == LATENCY_ALERT_THRESHOLD_MS as f64
        }));
    }

    #[test]
    fn durable_diagnostics_survive_restart_and_ignore_corrupt_files() {
        let path = temporary_path();
        let mut collector = FailureCollector::default();
        collector.record("proxy:forward_error".into(), 100);
        persist_collector(&path, &collector).unwrap();
        let reloaded = load_collector(&path);
        assert_eq!(reloaded.diagnostics(100).counters["proxy:forward_error"], 1);
        std::fs::write(&path, b"not json").unwrap();
        assert!(load_collector(&path).counters.is_empty());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn load_and_save_cap_oversized_operation_history() {
        let path = temporary_path();
        let mut oversized = FailureCollector::default();
        for index in 0..(MAX_RECENT_EVENTS + 10) {
            oversized.operation_results.push_back(RecordedOperation {
                identifier: "audio:output_stream".into(),
                succeeded: index % 2 == 0,
                occurred_at: 100,
            });
        }
        std::fs::write(&path, serde_json::to_vec(&oversized).unwrap()).unwrap();

        let reloaded = load_collector(&path);
        assert_eq!(reloaded.operation_results.len(), MAX_RECENT_EVENTS);

        persist_collector(&path, &oversized).unwrap();
        let persisted: FailureCollector =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(persisted.operation_results.len(), MAX_RECENT_EVENTS);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn immediate_output_start_failure_persists_one_external_attempt_and_outcome() {
        let path = temporary_path();
        let mut collector = FailureCollector::default();
        let terminal = AtomicBool::new(false);

        // This mirrors a synchronous start error racing an immediate callback:
        // only the first terminal path may write the persisted operation result.
        if !terminal.swap(true, Ordering::AcqRel) {
            collector.record_operation("audio:output_stream".into(), false, 100);
        }
        if !terminal.swap(true, Ordering::AcqRel) {
            collector.record_operation("audio:output_stream".into(), false, 100);
        }
        persist_collector(&path, &collector).unwrap();
        let diagnostics = load_collector(&path).diagnostics(100);
        let rate = &diagnostics.operation_rates["audio:output_stream"];
        assert_eq!((rate.attempts, rate.failures), (1, 1));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn counter_and_latency_identifier_limits_hold_at_64_and_65() {
        let mut collector = FailureCollector::default();
        for index in 0..65 {
            collector.record(format!("audio:event_{index}"), 100);
            collector.record_latency(format!("audio:latency_{index}"), index);
        }
        let diagnostics = collector.diagnostics(100);
        assert_eq!(diagnostics.counters.len(), 64);
        assert_eq!(diagnostics.latency.len(), 64);
        assert!(!diagnostics.counters.contains_key("audio:event_64"));
        assert!(!diagnostics.latency.contains_key("audio:latency_64"));
    }

    #[derive(Default)]
    struct TestSink(Mutex<Vec<TelemetryEvent>>);

    impl TelemetrySink for TestSink {
        fn capture(&self, event: TelemetryEvent) {
            self.0.lock().unwrap().push(event);
        }
    }

    #[derive(Default)]
    struct QueuedTransport {
        queued: Mutex<usize>,
        sent: Mutex<usize>,
    }

    impl sentry::Transport for QueuedTransport {
        fn send_envelope(&self, _envelope: sentry::protocol::Envelope) {
            *self.queued.lock().unwrap() += 1;
        }

        fn flush(&self, _timeout: Duration) -> bool {
            let mut queued = self.queued.lock().unwrap();
            *self.sent.lock().unwrap() += *queued;
            *queued = 0;
            true
        }
    }

    #[test]
    fn remote_capture_is_default_off_and_requires_both_consent_and_dsn() {
        let event = TelemetryEvent {
            identifier: "updater:periodic_check_failed".into(),
            value: None,
        };
        let sink = TestSink::default();
        capture_if_opted_in(
            &sink,
            RemoteTelemetryConfig {
                consented: false,
                dsn_configured: true,
            },
            event.clone(),
        );
        capture_if_opted_in(
            &sink,
            RemoteTelemetryConfig {
                consented: true,
                dsn_configured: false,
            },
            event.clone(),
        );
        assert!(sink.0.lock().unwrap().is_empty());
        capture_if_opted_in(
            &sink,
            RemoteTelemetryConfig {
                consented: true,
                dsn_configured: true,
            },
            event.clone(),
        );
        assert_eq!(*sink.0.lock().unwrap(), vec![event]);
    }

    #[test]
    fn compiled_dsn_still_requires_explicit_consent() {
        let event = TelemetryEvent {
            identifier: "updater:periodic_check_failed".into(),
            value: None,
        };
        let sink = TestSink::default();
        capture_if_opted_in(
            &sink,
            RemoteTelemetryConfig {
                consented: false,
                // Model a build that contains JELLYX_SENTRY_DSN.
                dsn_configured: true,
            },
            event,
        );
        assert!(sink.0.lock().unwrap().is_empty());
    }

    #[test]
    fn mock_transport_sends_nothing_after_consent_is_withdrawn() {
        let enabled = Arc::new(AtomicBool::new(true));
        let queued = Arc::new(QueuedTransport::default());
        let transport = ConsentAwareTransport {
            enabled: enabled.clone(),
            inner: queued.clone(),
        };

        transport.send_envelope(sentry::protocol::Envelope::new());
        assert_eq!(*queued.queued.lock().unwrap(), 1);
        enabled.store(false, Ordering::Release);
        assert!(!transport.flush(Duration::ZERO));
        assert_eq!(*queued.sent.lock().unwrap(), 0);
    }

    #[test]
    fn error_rate_alerts_use_the_remote_gate_and_are_rate_limited() {
        let sink = TestSink::default();
        let alerts = vec![OperationalAlert {
            identifier: "observability:error_rate_1_percent".into(),
            threshold_percent: 1.0,
            observed_percent: 50.0,
        }];
        let mut last_sent = BTreeMap::new();
        let enabled = RemoteTelemetryConfig {
            consented: true,
            dsn_configured: true,
        };
        capture_alerts_if_opted_in(&sink, enabled, &alerts, &mut last_sent, 100);
        capture_alerts_if_opted_in(&sink, enabled, &alerts, &mut last_sent, 101);
        capture_alerts_if_opted_in(
            &sink,
            RemoteTelemetryConfig {
                consented: false,
                dsn_configured: true,
            },
            &alerts,
            &mut last_sent,
            100 + ALERT_COOLDOWN_SECONDS + 1,
        );
        assert_eq!(sink.0.lock().unwrap().len(), 1);
        capture_alerts_if_opted_in(
            &sink,
            enabled,
            &alerts,
            &mut last_sent,
            100 + ALERT_COOLDOWN_SECONDS + 1,
        );
        assert_eq!(sink.0.lock().unwrap().len(), 2);
    }

    #[test]
    fn remote_capture_rejects_sensitive_or_unbounded_identifiers() {
        let sink = TestSink::default();
        capture_if_opted_in(
            &sink,
            RemoteTelemetryConfig {
                consented: true,
                dsn_configured: true,
            },
            TelemetryEvent {
                identifier: "updater:https://example.invalid/private".into(),
                value: None,
            },
        );
        assert!(sink.0.lock().unwrap().is_empty());
    }

    #[test]
    fn sentry_before_send_rebuilds_event_without_sensitive_fields() {
        let event = sentry::protocol::Event {
            message: Some("updater:periodic_check_failed".into()),
            breadcrumbs: vec![Default::default()].into(),
            ..Default::default()
        };
        let sanitized = sanitize_sentry_event(event).unwrap();
        assert_eq!(
            sanitized.message.as_deref(),
            Some("updater:periodic_check_failed")
        );
        assert!(sanitized.breadcrumbs.is_empty());
        assert!(sanitized.request.is_none());
        assert!(sanitize_sentry_event(sentry::protocol::Event {
            message: Some("updater:https://example.invalid/private".into()),
            ..Default::default()
        })
        .is_none());
    }
}
