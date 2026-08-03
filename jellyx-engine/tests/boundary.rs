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

#[test]
fn sqlite_handle_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}

    assert_send_sync::<jellyx_engine::sqlite::SqliteHandle>();
}

#[test]
fn sqlite_handle_clones_share_one_connection() {
    let handle =
        jellyx_engine::sqlite::SqliteHandle::new(rusqlite::Connection::open_in_memory().unwrap());
    let clone = handle.clone();

    handle
        .lock()
        .unwrap()
        .execute_batch("CREATE TABLE shared (value INTEGER); INSERT INTO shared VALUES (42);")
        .unwrap();
    let value = clone
        .lock()
        .unwrap()
        .query_row("SELECT value FROM shared", [], |row| row.get::<_, i64>(0))
        .unwrap();

    assert_eq!(value, 42);
}
