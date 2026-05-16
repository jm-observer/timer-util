// Database module for alarm-server
use crate::models::AlarmRecord;
use chrono::Utc;
use rusqlite::{params, params_from_iter, Connection};
use std::sync::{Arc, Mutex, MutexGuard};

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

    fn lock_conn(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Create the alarms table if it does not exist.
    pub fn initialize(&self) -> Result<(), rusqlite::Error> {
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
        CREATE TABLE IF NOT EXISTS notification_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            alarm_id TEXT NOT NULL,
            alarm_name TEXT NOT NULL DEFAULT '',
            callback_url TEXT NOT NULL,
            status TEXT NOT NULL,
            http_status INTEGER,
            error_message TEXT,
            attempt INTEGER NOT NULL DEFAULT 1,
            triggered_at TEXT NOT NULL,
            completed_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_notification_logs_alarm_id ON notification_logs(alarm_id);
        CREATE INDEX IF NOT EXISTS idx_notification_logs_triggered_at ON notification_logs(triggered_at);
        "#;
        self.lock_conn().execute_batch(sql)
    }

    pub fn insert_alarm(&self, alarm: &AlarmRecord) -> Result<(), rusqlite::Error> {
        let now = Utc::now().naive_utc();
        let sql = r#"
            INSERT INTO alarms (id, name, alarm_type, cron_expr, once_at, callback_url, callback_body, status, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
        "#;
        let conn = self.lock_conn();
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
        let conn = self.lock_conn();
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
        let conn = self.lock_conn();
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
        let conn = self.lock_conn();
        let affected = conn.execute(sql, params![id])?;
        Ok(affected > 0)
    }

    pub fn update_status(&self, id: &str, status: &str) -> Result<bool, rusqlite::Error> {
        let sql = "UPDATE alarms SET status = ?1, updated_at = ?2 WHERE id = ?3";
        let now = Utc::now()
            .naive_utc()
            .format("%Y-%m-%dT%H:%M:%S")
            .to_string();
        let conn = self.lock_conn();
        let affected = conn.execute(sql, params![status, now, id])?;
        Ok(affected > 0)
    }

    /// Count alarms, optionally filtered by status.
    pub fn count_alarms(&self, status: Option<&str>) -> Result<usize, rusqlite::Error> {
        let conn = self.lock_conn();
        match status {
            Some(s) => conn.query_row(
                "SELECT COUNT(*) FROM alarms WHERE status = ?1",
                params![s],
                |row| row.get(0),
            ),
            None => conn.query_row("SELECT COUNT(*) FROM alarms", [], |row| row.get(0)),
        }
    }

    /// Count alarms by status (e.g., "active", "completed")
    pub fn count_by_status(&self, status: &str) -> Result<usize, rusqlite::Error> {
        self.count_alarms(Some(status))
    }

    // Insert a notification log entry
    pub fn insert_notification_log(
        &self,
        log: &crate::models::NotificationLog,
    ) -> Result<(), rusqlite::Error> {
        let sql = r#"
            INSERT INTO notification_logs (
                alarm_id, alarm_name, callback_url, status, http_status, error_message, attempt, triggered_at, completed_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        "#;
        let conn = self.lock_conn();
        conn.execute(
            sql,
            params![
                log.alarm_id,
                log.alarm_name,
                log.callback_url,
                log.status,
                log.http_status.map(|s| s as i64),
                log.error_message,
                log.attempt,
                log.triggered_at,
                log.completed_at,
            ],
        )?;
        Ok(())
    }

    // List notification logs with optional filters and pagination
    pub fn list_notification_logs(
        &self,
        alarm_id_opt: Option<&str>,
        status_opt: Option<&str>,
        page: usize,
        per_page: usize,
    ) -> Result<(Vec<crate::models::NotificationLog>, usize), rusqlite::Error> {
        let mut sql = String::from("SELECT id, alarm_id, alarm_name, callback_url, status, http_status, error_message, attempt, triggered_at, completed_at FROM notification_logs");
        let mut conditions = Vec::new();
        let mut params_vec: Vec<rusqlite::types::Value> = Vec::new();
        if let Some(aid) = alarm_id_opt {
            conditions.push("alarm_id = ?");
            params_vec.push(rusqlite::types::Value::from(aid.to_string()));
        }
        if let Some(st) = status_opt {
            conditions.push("status = ?");
            params_vec.push(rusqlite::types::Value::from(st.to_string()));
        }
        if !conditions.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&conditions.join(" AND "));
        }
        // Count total
        let count_sql = format!("SELECT COUNT(*) FROM ({})", sql);
        let conn = self.lock_conn();
        let total: usize =
            conn.query_row(&count_sql, params_from_iter(params_vec.iter()), |row| {
                row.get(0)
            })?;

        // Add ordering and pagination
        sql.push_str(" ORDER BY triggered_at DESC LIMIT ? OFFSET ?");
        params_vec.push(rusqlite::types::Value::from(per_page as i64));
        let offset = ((page - 1) * per_page) as i64;
        params_vec.push(rusqlite::types::Value::from(offset));
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(params_vec.iter()), |row| {
            Ok(crate::models::NotificationLog {
                id: row.get(0)?,
                alarm_id: row.get(1)?,
                alarm_name: row.get(2)?,
                callback_url: row.get(3)?,
                status: row.get(4)?,
                http_status: row.get::<_, Option<i64>>(5)?.map(|v| v as u16),
                error_message: row.get(6)?,
                attempt: row.get(7)?,
                triggered_at: row.get(8)?,
                completed_at: row.get(9)?,
            })
        })?;
        let mut vec = Vec::new();
        for r in rows {
            vec.push(r?);
        }
        Ok((vec, total))
    }

    // Get notification stats
    pub fn notification_stats(&self) -> Result<crate::models::NotificationStats, rusqlite::Error> {
        let sql = "SELECT status, COUNT(*) FROM notification_logs GROUP BY status";
        let conn = self.lock_conn();
        let mut stmt = conn.prepare(sql)?;
        let mut total = 0usize;
        let mut success = 0usize;
        let mut failed = 0usize;
        let mut retrying = 0usize;
        let mut cancelled = 0usize;
        let rows = stmt.query_map([], |row| {
            let status: String = row.get(0)?;
            let cnt: usize = row.get(1)?;
            Ok((status, cnt))
        })?;
        for r in rows {
            let (status, cnt) = r?;
            match status.as_str() {
                "success" => success += cnt,
                "failed" => failed += cnt,
                "retrying" => retrying += cnt,
                "cancelled" => cancelled += cnt,
                _ => {}
            }
            total += cnt;
        }
        Ok(crate::models::NotificationStats {
            total,
            success,
            failed,
            retrying,
            cancelled,
        })
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
