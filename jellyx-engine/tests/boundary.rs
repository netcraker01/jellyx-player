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

#[test]
fn sqlite_connection_file_profile_is_wal_foreign_keyed_and_waits_five_seconds() {
    let path = std::env::temp_dir().join(format!(
        "jellyx-engine-file-profile-{}-{:?}.db",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let handle = jellyx_engine::sqlite::SqliteHandle::open_file(&path).unwrap();
    let conn = handle.lock().unwrap();

    let journal: String = conn
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .unwrap();
    let foreign_keys: i64 = conn
        .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
        .unwrap();
    let busy_timeout: i64 = conn
        .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
        .unwrap();

    assert_eq!(journal, "wal");
    assert_eq!(foreign_keys, 1);
    assert_eq!(busy_timeout, 5_000);
    drop(conn);
    drop(handle);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn sqlite_connection_in_memory_profile_does_not_apply_file_settings() {
    let handle = jellyx_engine::sqlite::SqliteHandle::open_in_memory().unwrap();
    let conn = handle.lock().unwrap();

    let journal: String = conn
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .unwrap();
    let foreign_keys: i64 = conn
        .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
        .unwrap();
    assert_eq!(journal, "memory");
    assert_eq!(foreign_keys, 1);
}

#[test]
fn sqlite_connection_error_preserves_stage_and_rusqlite_source() {
    use jellyx_engine::sqlite::{SqliteHandle, SqliteOpenStage};

    let error = match SqliteHandle::open_file(&std::env::temp_dir()) {
        Err(error) => error,
        Ok(_) => panic!("opening a directory as SQLite unexpectedly succeeded"),
    };

    assert_eq!(error.stage(), SqliteOpenStage::Open);
    assert!(matches!(
        error.source_error(),
        rusqlite::Error::SqliteFailure(_, _)
    ));

    let corrupt_path = std::env::temp_dir().join(format!(
        "jellyx-engine-corrupt-profile-{}-{:?}.db",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::write(&corrupt_path, b"not a sqlite database").unwrap();
    let error = match SqliteHandle::open_file(&corrupt_path) {
        Err(error) => error,
        Ok(_) => panic!("opening corrupt SQLite unexpectedly succeeded"),
    };
    assert_eq!(error.stage(), SqliteOpenStage::Configure);
    assert_eq!(
        error.source_error().sqlite_error_code(),
        Some(rusqlite::ErrorCode::NotADatabase)
    );
    std::fs::remove_file(corrupt_path).unwrap();
}

#[test]
fn sqlite_quick_check_classifies_valid_database() {
    use jellyx_engine::sqlite::{SqliteHandle, SqliteIntegrityClassification};

    let handle = SqliteHandle::open_in_memory().unwrap();
    handle
        .lock()
        .unwrap()
        .execute_batch("CREATE TABLE ok (x INTEGER); INSERT INTO ok VALUES (1);")
        .unwrap();

    assert_eq!(
        handle.quick_check().unwrap(),
        SqliteIntegrityClassification::Valid
    );
}

#[test]
fn sqlite_quick_check_classifies_corrupt_database_as_corrupt() {
    use jellyx_engine::sqlite::{SqliteHandle, SqliteIntegrityClassification};
    use std::io::{Read, Seek, SeekFrom, Write};

    let path = std::env::temp_dir().join(format!(
        "jellyx-engine-quick-check-corrupt-{}-{:?}.db",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));

    let handle = SqliteHandle::open_file(&path).unwrap();
    handle
        .lock()
        .unwrap()
        .execute_batch("CREATE TABLE broken (x INTEGER); INSERT INTO broken VALUES (1);")
        .unwrap();
    drop(handle);

    let size = std::fs::metadata(&path).unwrap().len();
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .unwrap();
    file.seek(SeekFrom::Start(size / 2)).unwrap();
    let mut byte = [0u8; 1];
    file.read_exact(&mut byte).unwrap();
    byte[0] = byte[0].wrapping_add(1);
    file.seek(SeekFrom::Start(size / 2)).unwrap();
    file.write_all(&byte).unwrap();

    let handle = SqliteHandle::open_file(&path).unwrap();
    assert_eq!(
        handle.quick_check().unwrap(),
        SqliteIntegrityClassification::Corrupt
    );

    drop(handle);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(format!("{}-wal", path.display()));
    let _ = std::fs::remove_file(format!("{}-shm", path.display()));
}
