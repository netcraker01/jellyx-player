//! Tests for Tauri command handlers.
//!
//! These tests focus on pure, side-effect-free validation logic that does not
//! require a running Tauri runtime.

use super::*;
use std::sync::Arc;

#[test]
fn settings_ipc_shapes_remain_unchanged() {
    let source = SourceSettingDto {
        source: "YouTube".into(),
        enabled: true,
        label: "YouTube".into(),
    };
    let audio = AudioSettingsDto {
        normalize_audio: true,
    };
    let telemetry = crate::persistence::models::TelemetrySettings { enabled: false };

    assert_eq!(
        serde_json::to_string(&source).unwrap(),
        r#"{"source":"YouTube","enabled":true,"label":"YouTube"}"#
    );
    assert_eq!(
        serde_json::to_string(&audio).unwrap(),
        r#"{"normalizeAudio":true}"#
    );
    assert_eq!(
        serde_json::to_string(&telemetry).unwrap(),
        r#"{"enabled":false}"#
    );
}

#[test]
fn focus_command_errors_are_stable_and_redacted() {
    let stale = focus_error(FocusServiceError::Persistence(
        "stale focus revision; sqlite at /private/path".into(),
    ));
    let unavailable = focus_error(FocusServiceError::Persistence(
        "database is locked at /private/path".into(),
    ));

    assert_eq!(stale.code, "FOCUS_REVISION_CONFLICT");
    assert_eq!(unavailable.code, "FOCUS_UNAVAILABLE");
    assert_eq!(
        serde_json::to_string(&unavailable).unwrap(),
        "{\"code\":\"FOCUS_UNAVAILABLE\",\"details\":null}"
    );
}

#[test]
fn focus_start_delegates_and_replays_request_ids() {
    let service = FocusService::new(
        Arc::new(crate::persistence::db::Database::open_in_memory().unwrap()),
        crate::focus::service::SystemClock,
    );
    let cadence = FocusCadence {
        work_duration_ms: 1_000,
        break_duration_ms: 0,
        rounds: 1,
    };
    let started = start_focus(
        &service,
        "focus-ipc-start",
        0,
        "Write command tests".into(),
        "Ship recovery".into(),
        "Open the editor".into(),
        FocusWorkflow::Custom,
        cadence.clone(),
        FocusMusicStrategy::None,
    )
    .unwrap();
    let replay = start_focus(
        &service,
        "focus-ipc-start",
        0,
        "Write command tests".into(),
        "Ship recovery".into(),
        "Open the editor".into(),
        FocusWorkflow::Custom,
        cadence,
        FocusMusicStrategy::None,
    )
    .unwrap();

    assert_eq!(replay, started);
}

#[test]
fn focus_events_include_snapshot_phase_and_playback_directive() {
    let service = FocusService::new(
        Arc::new(crate::persistence::db::Database::open_in_memory().unwrap()),
        crate::focus::service::SystemClock,
    );
    let result = start_focus(
        &service,
        "focus-event-start",
        0,
        "Test events".into(),
        String::new(),
        String::new(),
        FocusWorkflow::Custom,
        FocusCadence {
            work_duration_ms: 1_000,
            break_duration_ms: 0,
            rounds: 1,
        },
        FocusMusicStrategy::ContinueCurrent,
    )
    .unwrap();
    let mut events = Vec::new();
    emit_focus_result(&result, true, |event| {
        events.push(event.clone());
        Ok(())
    });

    assert!(events
        .iter()
        .any(|event| matches!(event.kind, FocusEventKind::SessionMutation(_))));
    assert!(events
        .iter()
        .any(|event| matches!(event.kind, FocusEventKind::PhaseChange { .. })));
    assert!(events
        .iter()
        .any(|event| matches!(event.kind, FocusEventKind::PlaybackDirective(_))));
}

#[test]
fn source_contract_accepts_supported_source_names() {
    assert_eq!(
        parse_source("YouTube").unwrap(),
        jellyx_core::models::source::Source::YouTube
    );
    assert_eq!(
        parse_source("SoundCloud").unwrap(),
        jellyx_core::models::source::Source::SoundCloud
    );
    assert_eq!(
        parse_source("Local").unwrap(),
        jellyx_core::models::source::Source::Local
    );
}

#[test]
fn source_contract_rejects_unknown_names_instead_of_defaulting_to_youtube() {
    let error = parse_source("Unknown").unwrap_err();
    assert_eq!(error.code, "VALIDATION_ERROR");
    assert_eq!(
        error.details.as_deref(),
        Some("unsupported source: Unknown")
    );
}

#[test]
fn open_release_page_accepts_jellyx_repo_url() {
    let url = "https://github.com/netcraker01/jellyx-player/releases/tag/v0.3.3";
    assert!(
        is_release_url_allowed(url),
        "expected Jellyx release URL to be allowed: {}",
        url
    );
}

#[test]
fn open_release_page_accepts_legacy_helix_repo_url() {
    let url = "https://github.com/netcraker01/helix/releases/tag/v0.3.3";
    assert!(
        is_release_url_allowed(url),
        "expected legacy Helix release URL to be allowed (GitHub redirects): {}",
        url
    );
}

#[test]
fn open_release_page_rejects_non_github_url() {
    let url = "https://example.com/evil";
    assert!(!is_release_url_allowed(url));
}

#[test]
fn open_release_page_rejects_malformed_github_path() {
    let url = "https://github.com/netcraker01/jellyx-player/issues/1";
    assert!(!is_release_url_allowed(url));
}
