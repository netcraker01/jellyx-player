//! Focus session persistence — simple CRUD operations.

use rusqlite::OptionalExtension;
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::sqlite::SqliteHandle;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusSessionRow {
    pub id: String,
    pub intention: String,
    pub goal: String,
    pub first_action: String,
    pub workflow: String,
    pub work_duration_ms: i64,
    pub break_duration_ms: i64,
    pub rounds: i32,
    pub round: i32,
    pub phase: String,
    pub state: String,
    pub phase_started_at: Option<i64>,
    pub phase_deadline_at: Option<i64>,
    pub paused_remaining_ms: Option<i64>,
    pub revision: i64,
    pub music_strategy: String,
    pub music_value: Option<String>,
    pub degradation_reason: Option<String>,
    pub outcome: Option<String>,
    pub updated_at: i64,
    pub captures: Vec<FocusCaptureRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusCaptureRow {
    pub id: i64,
    pub session_id: String,
    pub kind: String,
    pub body: String,
    pub created_at: i64,
}

const SESSION_SELECT: &str = "SELECT id, intention, goal, first_action, workflow, work_duration_ms, \
     break_duration_ms, rounds, round, phase, state, phase_started_at, \
     phase_deadline_at, paused_remaining_ms, revision, music_strategy, \
     music_value, degradation_reason, outcome, updated_at FROM focus_sessions";

pub struct FocusSessionRepository {
    db: SqliteHandle,
}

impl FocusSessionRepository {
    pub fn new(db: SqliteHandle) -> Self {
        Self { db }
    }

    pub fn get_session(&self, id: &str) -> Result<Option<FocusSessionRow>, String> {
        let conn = self.db.lock().map_err(|e| e.to_string())?;
        let mut result = conn
            .query_row(
                &format!("{SESSION_SELECT} WHERE id = ?1"),
                params![id],
                Self::row_to_session,
            )
            .optional()
            .map_err(|e| e.to_string())?;
        if let Some(ref mut session) = result {
            session.captures = Self::load_captures(&conn, &session.id)?;
        }
        Ok(result)
    }

    pub fn get_nonterminal_session(&self) -> Result<Option<FocusSessionRow>, String> {
        let conn = self.db.lock().map_err(|e| e.to_string())?;
        let mut result = conn
            .query_row(
                &format!("{SESSION_SELECT} WHERE state NOT IN ('completed', 'discarded') LIMIT 1"),
                [],
                Self::row_to_session,
            )
            .optional()
            .map_err(|e| e.to_string())?;
        if let Some(ref mut session) = result {
            session.captures = Self::load_captures(&conn, &session.id)?;
        }
        Ok(result)
    }

    pub fn list_sessions(&self, limit: u32) -> Result<Vec<FocusSessionRow>, String> {
        let conn = self.db.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(&format!(
                "{SESSION_SELECT} WHERE state IN ('completed', 'discarded') ORDER BY updated_at DESC LIMIT ?1"
            ))
            .map_err(|e| e.to_string())?;
        let mut sessions: Vec<FocusSessionRow> = stmt
            .query_map(params![limit], Self::row_to_session)
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        for session in &mut sessions {
            session.captures = Self::load_captures(&conn, &session.id)?;
        }
        Ok(sessions)
    }

    pub fn delete_session(&self, id: &str) -> Result<(), String> {
        let conn = self.db.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "DELETE FROM focus_sessions WHERE id = ?1 AND state IN ('completed', 'discarded')",
            params![id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn insert_capture(
        &self,
        session_id: &str,
        kind: &str,
        body: &str,
        created_at: i64,
    ) -> Result<FocusCaptureRow, String> {
        let conn = self.db.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO focus_captures (session_id, kind, body, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![session_id, kind, body, created_at],
        )
        .map_err(|e| e.to_string())?;
        let id = conn.last_insert_rowid();
        Ok(FocusCaptureRow {
            id,
            session_id: session_id.to_string(),
            kind: kind.to_string(),
            body: body.to_string(),
            created_at,
        })
    }

    pub fn get_operation_result(&self, request_id: &str) -> Result<Option<String>, String> {
        let conn = self.db.lock().map_err(|e| e.to_string())?;
        let result = conn
            .query_row(
                "SELECT result_json FROM focus_operations WHERE request_id = ?1",
                params![request_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        Ok(result)
    }

    pub fn is_playback_directive(&self, request_id: &str) -> Result<bool, String> {
        let conn = self.db.lock().map_err(|e| e.to_string())?;
        conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM focus_operations WHERE request_id = ?1 AND operation_kind = 'playbackDirective')",
            params![request_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())
    }

    pub fn mark_playback_directive(&self, request_id: &str) -> Result<(), String> {
        let conn = self.db.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE focus_operations SET operation_kind = 'playbackDirective' WHERE request_id = ?1",
            params![request_id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn load_captures(
        conn: &rusqlite::Connection,
        session_id: &str,
    ) -> Result<Vec<FocusCaptureRow>, String> {
        let mut stmt = conn
            .prepare(
                "SELECT id, session_id, kind, body, created_at FROM focus_captures \
                 WHERE session_id = ?1 ORDER BY created_at, id",
            )
            .map_err(|e| e.to_string())?;
        let captures = stmt
            .query_map(params![session_id], |row| {
                Ok(FocusCaptureRow {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    kind: row.get(2)?,
                    body: row.get(3)?,
                    created_at: row.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        Ok(captures)
    }

    fn row_to_session(row: &rusqlite::Row<'_>) -> rusqlite::Result<FocusSessionRow> {
        let music_strategy: String = row.get(15)?;
        let music_value: Option<String> = row.get(16)?;
        Ok(FocusSessionRow {
            id: row.get(0)?,
            intention: row.get(1)?,
            goal: row.get(2)?,
            first_action: row.get(3)?,
            workflow: row.get(4)?,
            work_duration_ms: row.get(5)?,
            break_duration_ms: row.get(6)?,
            rounds: row.get(7)?,
            round: row.get(8)?,
            phase: row.get(9)?,
            state: row.get(10)?,
            phase_started_at: row.get(11)?,
            phase_deadline_at: row.get(12)?,
            paused_remaining_ms: row.get(13)?,
            revision: row.get(14)?,
            music_strategy,
            music_value,
            degradation_reason: row.get(17)?,
            outcome: row.get(18)?,
            updated_at: row.get(19)?,
            captures: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn fresh_handle() -> SqliteHandle {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE focus_sessions (
                id TEXT PRIMARY KEY,
                intention TEXT NOT NULL,
                goal TEXT NOT NULL,
                first_action TEXT NOT NULL,
                workflow TEXT NOT NULL,
                work_duration_ms INTEGER NOT NULL,
                break_duration_ms INTEGER NOT NULL,
                rounds INTEGER NOT NULL,
                round INTEGER NOT NULL,
                phase TEXT NOT NULL,
                state TEXT NOT NULL,
                phase_started_at INTEGER,
                phase_deadline_at INTEGER,
                paused_remaining_ms INTEGER,
                revision INTEGER NOT NULL DEFAULT 0,
                music_strategy TEXT NOT NULL DEFAULT 'none',
                music_value TEXT,
                degradation_reason TEXT,
                outcome TEXT,
                created_at INTEGER NOT NULL DEFAULT 0,
                updated_at INTEGER NOT NULL DEFAULT 0,
                completed_at INTEGER
            );
            CREATE TABLE focus_captures (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL REFERENCES focus_sessions(id),
                kind TEXT NOT NULL,
                body TEXT NOT NULL,
                created_at INTEGER NOT NULL
            );
            CREATE TABLE focus_operations (
                operation_id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                request_id TEXT NOT NULL UNIQUE,
                operation_kind TEXT NOT NULL DEFAULT 'none',
                result_json TEXT,
                created_at INTEGER NOT NULL
            );",
        )
        .unwrap();
        SqliteHandle::new(conn)
    }

    fn insert_session(h: &SqliteHandle, id: &str, state: &str, updated_at: i64) {
        let conn = h.lock().unwrap();
        conn.execute(
            "INSERT INTO focus_sessions (id, intention, goal, first_action, workflow, work_duration_ms, break_duration_ms, rounds, round, phase, state, revision, music_strategy, updated_at) VALUES (?1, 't', 't', 't', 'pomodoro', 25000, 5000, 4, 1, 'work', ?2, 0, 'none', ?3)",
            params![id, state, updated_at],
        )
        .unwrap();
    }

    #[test]
    fn get_session_returns_none_for_unknown() {
        let h = fresh_handle();
        assert!(
            FocusSessionRepository::new(h)
                .get_session("nonexistent")
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn get_session_returns_captures() {
        let h = fresh_handle();
        insert_session(&h, "s1", "active", 1000);
        let conn = h.lock().unwrap();
        conn.execute(
            "INSERT INTO focus_captures (session_id, kind, body, created_at) VALUES (?1, ?2, ?3, ?4)",
            params!["s1", "note", "test note", 1001],
        )
        .unwrap();
        drop(conn);

        let repo = FocusSessionRepository::new(h);
        let session = repo.get_session("s1").unwrap().unwrap();
        assert_eq!(session.captures.len(), 1);
        assert_eq!(session.captures[0].body, "test note");
    }

    #[test]
    fn list_sessions_orders_by_updated_at() {
        let h = fresh_handle();
        insert_session(&h, "s1", "completed", 2000);
        insert_session(&h, "s2", "completed", 1000);
        insert_session(&h, "s3", "discarded", 3000);
        let repo = FocusSessionRepository::new(h);
        let sessions = repo.list_sessions(10).unwrap();
        assert_eq!(sessions.len(), 3);
        assert_eq!(sessions[0].id, "s3");
    }

    #[test]
    fn delete_session_only_deletes_terminal() {
        let h = fresh_handle();
        insert_session(&h, "s1", "active", 1000);
        insert_session(&h, "s2", "completed", 2000);
        let repo = FocusSessionRepository::new(h);
        repo.delete_session("s1").unwrap();
        assert!(repo.get_session("s1").unwrap().is_some());
        repo.delete_session("s2").unwrap();
        assert!(repo.get_session("s2").unwrap().is_none());
    }

    #[test]
    fn insert_capture_and_mark_playback() {
        let h = fresh_handle();
        insert_session(&h, "s1", "active", 1000);
        let conn = h.lock().unwrap();
        conn.execute(
            "INSERT INTO focus_operations (operation_id, session_id, request_id, operation_kind, result_json, created_at) VALUES ('op1', 's1', 'req1', 'none', '{}', 1000)",
            [],
        )
        .unwrap();
        drop(conn);

        let repo = FocusSessionRepository::new(h);
        let capture = repo.insert_capture("s1", "note", "hello", 1001).unwrap();
        assert_eq!(capture.kind, "note");
        assert!(!repo.is_playback_directive("req1").unwrap());
        repo.mark_playback_directive("req1").unwrap();
        assert!(repo.is_playback_directive("req1").unwrap());
    }

    #[test]
    fn get_operation_result_returns_raw_json() {
        let h = fresh_handle();
        let conn = h.lock().unwrap();
        conn.execute(
            "INSERT INTO focus_operations (operation_id, session_id, request_id, operation_kind, result_json, created_at) VALUES ('op1', 's1', 'req1', 'none', '{\"id\":\"s1\"}', 1000)",
            [],
        )
        .unwrap();
        drop(conn);
        let repo = FocusSessionRepository::new(h);
        assert_eq!(
            repo.get_operation_result("req1").unwrap().unwrap(),
            "{\"id\":\"s1\"}"
        );
    }
}
