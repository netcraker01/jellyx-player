use jellyx_engine::updater::{RepositoryError, UpdatePreferencesRepository, UpdatePrefs};

use super::db::Database;

impl UpdatePreferencesRepository for Database {
    fn update_preferences(&self) -> Result<UpdatePrefs, RepositoryError> {
        self.engine.update_preferences()
    }

    fn save_update_preferences(&self, prefs: &UpdatePrefs) -> Result<(), RepositoryError> {
        self.engine.save_update_preferences(prefs)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::updater::prefs::UpdatePrefsService;

    #[test]
    fn sqlite_adapter_is_compatible_with_legacy_database_access() {
        let path = std::env::temp_dir().join(format!(
            "jellyx-updater-preferences-{}.db",
            uuid::Uuid::new_v4()
        ));
        let db = Arc::new(Database::open(&path).unwrap());
        let service = UpdatePrefsService::new(db.clone());
        assert_eq!(service.get().unwrap(), UpdatePrefs::default());

        service.set_skipped_version("v0.4.4").unwrap();
        let expected = service.set_remind_later("2026-08-04T10:00:00Z").unwrap();
        assert_eq!(db.get_update_prefs().unwrap(), expected);
        drop(service);
        drop(db);

        let reopened = Arc::new(Database::open(&path).unwrap());
        assert_eq!(
            UpdatePrefsService::new(reopened.clone()).get().unwrap(),
            expected
        );
        assert_eq!(reopened.get_update_prefs().unwrap(), expected);
        drop(reopened);
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{}", path.display(), suffix));
        }
    }
}
