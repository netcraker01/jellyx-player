#[test]
fn crate_exposes_the_engine_boundary() {
    assert!(jellyx_engine::BOUNDARY_ESTABLISHED);
}

#[test]
fn manifest_has_no_tauri_dependency() {
    let manifest = include_str!("../Cargo.toml").to_ascii_lowercase();
    assert!(!manifest.contains("tauri"));
}

#[test]
fn preferences_contract_is_platform_neutral() {
    let repository: Option<&dyn jellyx_engine::preferences::PreferencesRepository> = None;
    assert!(repository.is_none());
}

#[test]
fn updater_preferences_contract_is_platform_neutral_and_object_safe() {
    let repository: Option<&dyn jellyx_engine::updater::UpdatePreferencesRepository> = None;
    assert!(repository.is_none());
}
