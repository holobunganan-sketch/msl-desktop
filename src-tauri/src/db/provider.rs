//! provider_settings + app_settings repository（指南 §6.11 / app_settings）。

use rusqlite::{Connection, OptionalExtension, Row, params};

use super::{DbError, DbResult, now_unix};

// ---------- provider_settings ----------

#[derive(Debug, Clone)]
pub struct ProviderSetting {
    pub id: i64,
    pub display_name: String,
    pub provider_type: String, // deepseek|openai_compatible
    pub base_url: String,
    pub model: String,
    pub enabled: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

fn row_to_provider(row: &Row) -> rusqlite::Result<ProviderSetting> {
    Ok(ProviderSetting {
        id: row.get(0)?,
        display_name: row.get(1)?,
        provider_type: row.get(2)?,
        base_url: row.get(3)?,
        model: row.get(4)?,
        enabled: row.get::<_, i64>(5)? != 0,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

pub struct ProviderRepo<'a> {
    conn: &'a Connection,
}

impl<'a> ProviderRepo<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// 新增 Provider（API Key 不落库，只存 keyring reference 于应用层）。
    pub fn insert(
        &self,
        display_name: &str,
        provider_type: &str,
        base_url: &str,
        model: &str,
        enabled: bool,
    ) -> DbResult<ProviderSetting> {
        let now = now_unix();
        self.conn.execute(
            "INSERT INTO provider_settings (display_name, provider_type, base_url, model, enabled, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
            params![display_name, provider_type, base_url, model, enabled as i64, now],
        )?;
        let id = self.conn.last_insert_rowid();
        self.get(id)?
            .ok_or_else(|| DbError::NotFound("provider_setting".into()))
    }

    pub fn get(&self, id: i64) -> DbResult<Option<ProviderSetting>> {
        self.conn
            .query_row(
                "SELECT id, display_name, provider_type, base_url, model, enabled, created_at, updated_at
                 FROM provider_settings WHERE id = ?1",
                [id],
                row_to_provider,
            )
            .optional()
            .map_err(DbError::from)
    }

    pub fn list(&self) -> DbResult<Vec<ProviderSetting>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, display_name, provider_type, base_url, model, enabled, created_at, updated_at
             FROM provider_settings ORDER BY id",
        )?;
        let rows = stmt.query_map([], row_to_provider)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(DbError::from)
    }

    /// 启用的 Provider（最多一个用于当前会话）。
    pub fn list_enabled(&self) -> DbResult<Vec<ProviderSetting>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, display_name, provider_type, base_url, model, enabled, created_at, updated_at
             FROM provider_settings WHERE enabled = 1 ORDER BY id",
        )?;
        let rows = stmt.query_map([], row_to_provider)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(DbError::from)
    }

    /// 更新 Provider（不含 key）。
    pub fn update(
        &self,
        id: i64,
        display_name: &str,
        provider_type: &str,
        base_url: &str,
        model: &str,
        enabled: bool,
    ) -> DbResult<()> {
        let affected = self.conn.execute(
            "UPDATE provider_settings
             SET display_name = ?1, provider_type = ?2, base_url = ?3, model = ?4,
                 enabled = ?5, updated_at = ?6
             WHERE id = ?7",
            params![display_name, provider_type, base_url, model, enabled as i64, now_unix(), id],
        )?;
        if affected == 0 {
            return Err(DbError::NotFound("provider_setting".into()));
        }
        Ok(())
    }

    pub fn delete(&self, id: i64) -> DbResult<()> {
        let affected = self
            .conn
            .execute("DELETE FROM provider_settings WHERE id = ?1", [id])?;
        if affected == 0 {
            return Err(DbError::NotFound("provider_setting".into()));
        }
        Ok(())
    }
}

// ---------- app_settings ----------

pub struct AppSettingsRepo<'a> {
    conn: &'a Connection,
}

impl<'a> AppSettingsRepo<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn set(&self, key: &str, value: &str) -> DbResult<()> {
        self.conn.execute(
            "INSERT INTO app_settings (key, value, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![key, value, now_unix()],
        )?;
        Ok(())
    }

    pub fn get(&self, key: &str) -> DbResult<Option<String>> {
        self.conn
            .query_row(
                "SELECT value FROM app_settings WHERE key = ?1",
                [key],
                |row| row.get(0),
            )
            .optional()
            .map_err(DbError::from)
    }

    pub fn delete(&self, key: &str) -> DbResult<()> {
        let affected = self
            .conn
            .execute("DELETE FROM app_settings WHERE key = ?1", [key])?;
        if affected == 0 {
            return Err(DbError::NotFound("app_setting".into()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn provider_crud_and_enabled() {
        let db = Database::open_in_memory().unwrap();
        let repo = ProviderRepo::new(db.conn());

        let p = repo.insert("DeepSeek", "openai_compatible", "https://api.deepseek.com", "deepseek-v4-flash", true)
            .unwrap();
        repo.insert("Local", "openai_compatible", "http://localhost:11434/v1", "qwen", false)
            .unwrap();

        assert_eq!(repo.list().unwrap().len(), 2);
        let enabled = repo.list_enabled().unwrap();
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].display_name, "DeepSeek");

        repo.update(p.id, "DeepSeek V4", "openai_compatible", "https://api.deepseek.com", "deepseek-v4-flash", false)
            .unwrap();
        assert_eq!(repo.list_enabled().unwrap().len(), 0);
    }

    #[test]
    fn app_settings_upsert() {
        let db = Database::open_in_memory().unwrap();
        let repo = AppSettingsRepo::new(db.conn());

        repo.set("main_workspace", "C:\\Work").unwrap();
        assert_eq!(repo.get("main_workspace").unwrap().as_deref(), Some("C:\\Work"));

        // upsert 覆盖
        repo.set("main_workspace", "D:\\Medical").unwrap();
        assert_eq!(repo.get("main_workspace").unwrap().as_deref(), Some("D:\\Medical"));

        assert!(repo.get("missing").unwrap().is_none());
    }
}
