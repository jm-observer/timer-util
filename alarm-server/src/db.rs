// Database module for alarm-server
use crate::models::AlarmRecord;
use chrono::Utc;
use rusqlite::{params, Connection};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>, // thread‑safe connection
}

impl Database {
    pub fn new(path: &str) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Create the alarms table if it does not exist.
    pub fn initialize(&self) {
        let sql = r#"
        CREATE TABLE IF NOT EXISTS alarms (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL DEFAULT '',
            alarm_type TEXT NOT NULL,
            cron_expr TEXT,
            once_at TEXT,
            callback_url TEXT NOT NULL,
            callback_body TEXT,
            status TEXT NOT NULL DEFAULT 'active',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        "#;
        let _ = self.conn.lock().unwrap().execute(sql, []);
    }

    pub fn insert_alarm(&self, alarm: &AlarmRecord) -> Result<(), rusqlite::Error> {
        let now = Utc::now().naive_utc();
        let sql = r#"
            INSERT INTO alarms (id, name, alarm_type, cron_expr, once_at, callback_url, callback_body, status, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
        "#;
        let conn = self.conn.lock().unwrap();
        conn.execute(
            sql,
            params![
                alarm.id,
                alarm.name,
                alarm.alarm_type,
                alarm.cron_expr,
                alarm.once_at,
                alarm.callback_url,
                alarm.callback_body,
                alarm.status,
                now.format("%Y-%m-%dT%H:%M:%S").to_string(),
                now.format("%Y-%m-%dT%H:%M:%S").to_string(),
            ],
        )?;
        Ok(())
    }

    pub fn get_alarm(&self, id: &str) -> Result<Option<AlarmRecord>, rusqlite::Error> {
        let sql = "SELECT id, name, alarm_type, cron_expr, once_at, callback_url, callback_body, status, created_at, updated_at FROM alarms WHERE id = ?1";
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(sql)?;
        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(AlarmRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                alarm_type: row.get(2)?,
                cron_expr: row.get(3)?,
                once_at: row.get(4)?,
                callback_url: row.get(5)?,
                callback_body: row.get(6)?,
                status: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn list_alarms(&self, status: Option<&str>) -> Result<Vec<AlarmRecord>, rusqlite::Error> {
        let sql = "SELECT id, name, alarm_type, cron_expr, once_at, callback_url, callback_body, status, created_at, updated_at FROM alarms WHERE (?1 IS NULL OR status = ?1)";
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map(params![status], |row| {
            Ok(AlarmRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                alarm_type: row.get(2)?,
                cron_expr: row.get(3)?,
                once_at: row.get(4)?,
                callback_url: row.get(5)?,
                callback_body: row.get(6)?,
                status: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?;
        let mut vec = Vec::new();
        for r in rows {
            vec.push(r?);
        }
        Ok(vec)
    }

    pub fn delete_alarm(&self, id: &str) -> Result<bool, rusqlite::Error> {
        let sql = "DELETE FROM alarms WHERE id = ?1";
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute(sql, params![id])?;
        Ok(affected > 0)
    }

    pub fn update_status(&self, id: &str, status: &str) -> Result<bool, rusqlite::Error> {
        let sql = "UPDATE alarms SET status = ?1, updated_at = ?2 WHERE id = ?3";
        let now = Utc::now()
            .naive_utc()
            .format("%Y-%m-%dT%H:%M:%S")
            .to_string();
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute(sql, params![status, now, id])?;
        Ok(affected > 0)
    }

    pub fn count_by_status(&self, status: &str) -> Result<usize, rusqlite::Error> {
        let sql = "SELECT COUNT(*) FROM alarms WHERE status = ?1";
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(sql)?;
        let count: usize = stmt.query_row(params![status], |row| row.get(0))?;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_initialize() {
        let db = Database::new(":memory:").unwrap();
        db.initialize();
    }

    #[test]
    fn test_insert_and_get() {
        let db = Database::new(":memory:").unwrap();
        db.initialize();
        let rec = AlarmRecord {
            id: "test-id".to_string(),
            name: "test".to_string(),
            alarm_type: "once".to_string(),
            cron_expr: None,
            once_at: Some("2099-01-01T00:00:00".to_string()),
            callback_url: "http://example.com".to_string(),
            callback_body: None,
            status: "active".to_string(),
            created_at: "".to_string(),
            updated_at: "".to_string(),
        };
        db.insert_alarm(&rec).unwrap();
        let fetched = db.get_alarm("test-id").unwrap().unwrap();
        assert_eq!(fetched.id, rec.id);
    }

    #[test]
    fn test_list_and_filter() {
        let db = Database::new(":memory:").unwrap();
        db.initialize();
        let rec1 = AlarmRecord {
            id: "id1".to_string(),
            name: "a".to_string(),
            alarm_type: "once".to_string(),
            cron_expr: None,
            once_at: Some("2099-01-01T00:00:00".to_string()),
            callback_url: "http://example.com".to_string(),
            callback_body: None,
            status: "active".to_string(),
            created_at: "".to_string(),
            updated_at: "".to_string(),
        };
        db.insert_alarm(&rec1).unwrap();
        let mut rec2 = rec1.clone();
        rec2.id = "id2".to_string();
        rec2.status = "completed".to_string();
        db.insert_alarm(&rec2).unwrap();
        let active = db.list_alarms(Some("active")).unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, "id1");
    }
}
