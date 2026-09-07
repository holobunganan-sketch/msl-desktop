//! provider_settings + app_settings repository（指南 §6.11 / app_settings）。

use rusqlite::{params, Connection, OptionalExtension, Row};

use super::{now_unix, DbError, DbResult};

pub const AI_TASK_KINDS: &[&str] = &[
    "workbench_qa",
    "kol_analysis",
    "workspace_analysis",
    "work_draft",
    "global_analysis",
    "daily_brief",
    "weekly_report",
    "monthly_report",
    "translation",
    "general",
];
pub const AI_PROTOCOLS: &[&str] = &["chat_completions", "responses", "anthropic_messages"];
pub const AI_AUTH_MODES: &[&str] = &["bearer", "api_key", "none"];
pub const PROVIDER_TEMPLATES: &[&str] = &["deepseek", "opencode_go", "custom"];

pub fn validate_task_kind(value: &str) -> DbResult<()> {
    if AI_TASK_KINDS.contains(&value) {
        Ok(())
    } else {
        Err(DbError::Migration(format!("invalid AI task kind: {value}")))
    }
}

pub fn validate_protocol(value: &str) -> DbResult<()> {
    if AI_PROTOCOLS.contains(&value) {
        Ok(())
    } else {
        Err(DbError::Migration(format!("invalid AI protocol: {value}")))
    }
}

pub fn validate_auth_mode(value: &str) -> DbResult<()> {
    if AI_AUTH_MODES.contains(&value) {
        Ok(())
    } else {
        Err(DbError::Migration(format!("invalid auth mode: {value}")))
    }
}

pub fn validate_template_kind(value: &str) -> DbResult<()> {
    if PROVIDER_TEMPLATES.contains(&value) {
        Ok(())
    } else {
        Err(DbError::Migration(format!(
            "invalid provider template: {value}"
        )))
    }
}

// ---------- provider_settings ----------

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProviderSetting {
    pub id: i64,
    pub display_name: String,
    pub provider_type: String, // deepseek|openai_compatible
    pub base_url: String,
    pub model: String,
    pub enabled: bool,
    pub credential_ref: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProviderConnection {
    pub id: i64,
    pub display_name: String,
    pub provider_type: String,
    pub base_url: String,
    pub legacy_model: String,
    pub enabled: bool,
    pub credential_ref: String,
    pub template_kind: String,
    pub auth_mode: String,
    pub models_endpoint: Option<String>,
    pub last_models_refresh_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProviderModel {
    pub id: i64,
    pub provider_id: i64,
    pub model_id: String,
    pub display_name: String,
    pub protocol: String,
    pub endpoint_path: String,
    pub capabilities_json: String,
    pub source: String,
    pub enabled: bool,
    pub available: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AiTaskRoute {
    pub task_kind: String,
    pub provider_model_id: Option<i64>,
    pub updated_at: i64,
}

fn row_to_connection(row: &Row) -> rusqlite::Result<ProviderConnection> {
    Ok(ProviderConnection {
        id: row.get(0)?,
        display_name: row.get(1)?,
        provider_type: row.get(2)?,
        base_url: row.get(3)?,
        legacy_model: row.get(4)?,
        enabled: row.get::<_, i64>(5)? != 0,
        credential_ref: row.get(6)?,
        template_kind: row.get(7)?,
        auth_mode: row.get(8)?,
        models_endpoint: row.get(9)?,
        last_models_refresh_at: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

fn row_to_model(row: &Row) -> rusqlite::Result<ProviderModel> {
    Ok(ProviderModel {
        id: row.get(0)?,
        provider_id: row.get(1)?,
        model_id: row.get(2)?,
        display_name: row.get(3)?,
        protocol: row.get(4)?,
        endpoint_path: row.get(5)?,
        capabilities_json: row.get(6)?,
        source: row.get(7)?,
        enabled: row.get::<_, i64>(8)? != 0,
        available: row.get::<_, i64>(9)? != 0,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

fn row_to_route(row: &Row) -> rusqlite::Result<AiTaskRoute> {
    Ok(AiTaskRoute {
        task_kind: row.get(0)?,
        provider_model_id: row.get(1)?,
        updated_at: row.get(2)?,
    })
}

fn row_to_provider(row: &Row) -> rusqlite::Result<ProviderSetting> {
    Ok(ProviderSetting {
        id: row.get(0)?,
        display_name: row.get(1)?,
        provider_type: row.get(2)?,
        base_url: row.get(3)?,
        model: row.get(4)?,
        enabled: row.get::<_, i64>(5)? != 0,
        credential_ref: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
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
        let credential_ref = crate::ai::provider::new_credential_ref();
        self.conn.execute(
            "INSERT INTO provider_settings (display_name, provider_type, base_url, model, enabled, credential_ref, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
            params![display_name, provider_type, base_url, model, enabled as i64, credential_ref, now],
        )?;
        let id = self.conn.last_insert_rowid();
        self.get(id)?
            .ok_or_else(|| DbError::NotFound("provider_setting".into()))
    }

    pub fn get(&self, id: i64) -> DbResult<Option<ProviderSetting>> {
        self.conn
            .query_row(
                "SELECT id, display_name, provider_type, base_url, model, enabled, credential_ref, created_at, updated_at
                 FROM provider_settings WHERE id = ?1",
                [id],
                row_to_provider,
            )
            .optional()
            .map_err(DbError::from)
    }

    pub fn list(&self) -> DbResult<Vec<ProviderSetting>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, display_name, provider_type, base_url, model, enabled, credential_ref, created_at, updated_at
             FROM provider_settings ORDER BY id",
        )?;
        let rows = stmt.query_map([], row_to_provider)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(DbError::from)
    }

    /// 启用的 Provider（最多一个用于当前会话）。
    pub fn list_enabled(&self) -> DbResult<Vec<ProviderSetting>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, display_name, provider_type, base_url, model, enabled, credential_ref, created_at, updated_at
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
            params![
                display_name,
                provider_type,
                base_url,
                model,
                enabled as i64,
                now_unix(),
                id
            ],
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

/// Provider connection、model catalog 与 task route 的独立 repository。
pub struct ProviderCatalogRepo<'a> {
    conn: &'a Connection,
}

impl<'a> ProviderCatalogRepo<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    fn connection_select() -> &'static str {
        "SELECT id, display_name, provider_type, base_url, model, enabled, credential_ref,
                template_kind, auth_mode, models_endpoint, last_models_refresh_at, created_at, updated_at
         FROM provider_settings"
    }

    pub fn list_connections(&self) -> DbResult<Vec<ProviderConnection>> {
        let mut stmt = self
            .conn
            .prepare(&format!("{} ORDER BY id", Self::connection_select()))?;
        let rows = stmt.query_map([], row_to_connection)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(DbError::from)
    }

    pub fn get_connection(&self, id: i64) -> DbResult<Option<ProviderConnection>> {
        self.conn
            .query_row(
                &format!("{} WHERE id = ?1", Self::connection_select()),
                [id],
                row_to_connection,
            )
            .optional()
            .map_err(DbError::from)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn insert_connection(
        &self,
        display_name: &str,
        provider_type: &str,
        base_url: &str,
        legacy_model: &str,
        template_kind: &str,
        auth_mode: &str,
        models_endpoint: Option<&str>,
        enabled: bool,
    ) -> DbResult<ProviderConnection> {
        validate_template_kind(template_kind)?;
        validate_auth_mode(auth_mode)?;
        let now = now_unix();
        let credential_ref = crate::ai::provider::new_credential_ref();
        self.conn.execute(
            "INSERT INTO provider_settings
             (display_name, provider_type, base_url, model, enabled, credential_ref,
              template_kind, auth_mode, models_endpoint, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
            params![
                display_name,
                provider_type,
                base_url,
                legacy_model,
                enabled as i64,
                credential_ref,
                template_kind,
                auth_mode,
                models_endpoint,
                now
            ],
        )?;
        let id = self.conn.last_insert_rowid();
        self.get_connection(id)?
            .ok_or_else(|| DbError::NotFound("provider_connection".into()))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_connection(
        &self,
        id: i64,
        display_name: &str,
        provider_type: &str,
        base_url: &str,
        legacy_model: &str,
        template_kind: &str,
        auth_mode: &str,
        models_endpoint: Option<&str>,
        enabled: bool,
    ) -> DbResult<ProviderConnection> {
        validate_template_kind(template_kind)?;
        validate_auth_mode(auth_mode)?;
        let changed = self.conn.execute(
            "UPDATE provider_settings
             SET display_name = ?1, provider_type = ?2, base_url = ?3, model = ?4,
                 template_kind = ?5, auth_mode = ?6, models_endpoint = ?7,
                 enabled = ?8, updated_at = ?9
             WHERE id = ?10",
            params![
                display_name,
                provider_type,
                base_url,
                legacy_model,
                template_kind,
                auth_mode,
                models_endpoint,
                enabled as i64,
                now_unix(),
                id
            ],
        )?;
        if changed == 0 {
            return Err(DbError::NotFound("provider_connection".into()));
        }
        self.get_connection(id)?
            .ok_or_else(|| DbError::NotFound("provider_connection".into()))
    }

    pub fn mark_models_refreshed(&self, provider_id: i64, at: i64) -> DbResult<()> {
        let changed = self.conn.execute(
            "UPDATE provider_settings SET last_models_refresh_at = ?1, updated_at = ?1 WHERE id = ?2",
            params![at, provider_id],
        )?;
        if changed == 0 {
            return Err(DbError::NotFound("provider_connection".into()));
        }
        Ok(())
    }

    fn model_select() -> &'static str {
        "SELECT id, provider_id, model_id, display_name, protocol, endpoint_path,
                capabilities_json, source, enabled, available, created_at, updated_at
         FROM provider_models"
    }

    pub fn list_models(&self, provider_id: Option<i64>) -> DbResult<Vec<ProviderModel>> {
        let (sql, params): (String, Vec<Box<dyn rusqlite::ToSql>>) = match provider_id {
            Some(id) => (
                format!(
                    "{} WHERE provider_id = ?1 ORDER BY id",
                    Self::model_select()
                ),
                vec![Box::new(id)],
            ),
            None => (
                format!("{} ORDER BY provider_id, id", Self::model_select()),
                Vec::new(),
            ),
        };
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(params.iter()), row_to_model)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(DbError::from)
    }

    pub fn get_model(&self, id: i64) -> DbResult<Option<ProviderModel>> {
        self.conn
            .query_row(
                &format!("{} WHERE id = ?1", Self::model_select()),
                [id],
                row_to_model,
            )
            .optional()
            .map_err(DbError::from)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn upsert_model(
        &self,
        provider_id: i64,
        model_id: &str,
        display_name: &str,
        protocol: &str,
        endpoint_path: &str,
        capabilities_json: &str,
        source: &str,
        enabled: bool,
        available: bool,
    ) -> DbResult<ProviderModel> {
        validate_protocol(protocol)?;
        if !matches!(source, "legacy" | "template" | "remote" | "manual") {
            return Err(DbError::Migration(format!(
                "invalid model source: {source}"
            )));
        }
        let now = now_unix();
        self.conn.execute(
            "INSERT INTO provider_models
             (provider_id, model_id, display_name, protocol, endpoint_path, capabilities_json,
              source, enabled, available, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)
             ON CONFLICT(provider_id, model_id) DO UPDATE SET
               display_name = excluded.display_name,
               protocol = excluded.protocol,
               endpoint_path = excluded.endpoint_path,
               capabilities_json = excluded.capabilities_json,
               source = excluded.source,
               enabled = excluded.enabled,
               available = excluded.available,
               updated_at = excluded.updated_at",
            params![
                provider_id,
                model_id,
                display_name,
                protocol,
                endpoint_path,
                capabilities_json,
                source,
                enabled as i64,
                available as i64,
                now
            ],
        )?;
        self.conn
            .query_row(
                &format!(
                    "{} WHERE provider_id = ?1 AND model_id = ?2",
                    Self::model_select()
                ),
                params![provider_id, model_id],
                row_to_model,
            )
            .map_err(DbError::from)
    }

    pub fn set_model_enabled(&self, id: i64, enabled: bool) -> DbResult<()> {
        let changed = self.conn.execute(
            "UPDATE provider_models SET enabled = ?1, updated_at = ?2 WHERE id = ?3",
            params![enabled as i64, now_unix(), id],
        )?;
        if changed == 0 {
            return Err(DbError::NotFound("provider_model".into()));
        }
        Ok(())
    }

    pub fn list_routes(&self) -> DbResult<Vec<AiTaskRoute>> {
        let mut stmt = self
            .conn
            .prepare("SELECT task_kind, provider_model_id, updated_at FROM ai_task_routes ORDER BY task_kind")?;
        let rows = stmt.query_map([], row_to_route)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(DbError::from)
    }

    pub fn get_route(&self, task_kind: &str) -> DbResult<Option<AiTaskRoute>> {
        validate_task_kind(task_kind)?;
        self.conn
            .query_row(
                "SELECT task_kind, provider_model_id, updated_at FROM ai_task_routes WHERE task_kind = ?1",
                [task_kind],
                row_to_route,
            )
            .optional()
            .map_err(DbError::from)
    }

    pub fn upsert_route(&self, task_kind: &str, provider_model_id: Option<i64>) -> DbResult<()> {
        validate_task_kind(task_kind)?;
        self.conn.execute(
            "INSERT INTO ai_task_routes (task_kind, provider_model_id, updated_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(task_kind) DO UPDATE SET provider_model_id = excluded.provider_model_id,
               updated_at = excluded.updated_at",
            params![task_kind, provider_model_id, now_unix()],
        )?;
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

        let p = repo
            .insert(
                "DeepSeek",
                "openai_compatible",
                "https://api.deepseek.com",
                "deepseek-v4-flash",
                true,
            )
            .unwrap();
        repo.insert(
            "Local",
            "openai_compatible",
            "http://localhost:11434/v1",
            "qwen",
            false,
        )
        .unwrap();

        assert_eq!(repo.list().unwrap().len(), 2);
        let enabled = repo.list_enabled().unwrap();
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].display_name, "DeepSeek");
        assert!(enabled[0].credential_ref.starts_with("provider-"));

        repo.update(
            p.id,
            "DeepSeek V4",
            "openai_compatible",
            "https://api.deepseek.com",
            "deepseek-v4-flash",
            false,
        )
        .unwrap();
        assert_eq!(repo.list_enabled().unwrap().len(), 0);
    }

    #[test]
    fn independent_databases_do_not_share_new_credential_ref() {
        let db_a = Database::open_in_memory().unwrap();
        let db_b = Database::open_in_memory().unwrap();
        let a = ProviderRepo::new(db_a.conn())
            .insert("A", "openai_compatible", "http://a", "model", false)
            .unwrap();
        let b = ProviderRepo::new(db_b.conn())
            .insert("B", "openai_compatible", "http://b", "model", false)
            .unwrap();
        assert_eq!(a.id, 1);
        assert_eq!(b.id, 1);
        assert_ne!(a.credential_ref, b.credential_ref);
    }

    #[test]
    fn app_settings_upsert() {
        let db = Database::open_in_memory().unwrap();
        let repo = AppSettingsRepo::new(db.conn());

        repo.set("main_workspace", "C:\\Work").unwrap();
        assert_eq!(
            repo.get("main_workspace").unwrap().as_deref(),
            Some("C:\\Work")
        );

        // upsert 覆盖
        repo.set("main_workspace", "D:\\Medical").unwrap();
        assert_eq!(
            repo.get("main_workspace").unwrap().as_deref(),
            Some("D:\\Medical")
        );

        assert!(repo.get("missing").unwrap().is_none());
    }

    #[test]
    fn provider_catalog_models_and_routes_are_isolated() {
        let db = Database::open_in_memory().unwrap();
        let repo = ProviderCatalogRepo::new(db.conn());
        let deepseek = repo
            .insert_connection(
                "DeepSeek",
                "custom",
                "https://api.deepseek.com",
                "",
                "deepseek",
                "bearer",
                Some("https://api.deepseek.com/models"),
                true,
            )
            .unwrap();
        let custom = repo
            .insert_connection(
                "Custom",
                "custom",
                "http://localhost:9999",
                "",
                "custom",
                "bearer",
                None,
                true,
            )
            .unwrap();
        let a = repo
            .upsert_model(
                deepseek.id,
                "same-model",
                "Same",
                "chat_completions",
                "/chat/completions",
                "{}",
                "manual",
                true,
                true,
            )
            .unwrap();
        let b = repo
            .upsert_model(
                custom.id,
                "same-model",
                "Same",
                "responses",
                "/responses",
                "{}",
                "manual",
                true,
                true,
            )
            .unwrap();
        assert_ne!(a.id, b.id);
        assert!(repo
            .upsert_model(
                deepseek.id,
                "same-model",
                "Same updated",
                "chat_completions",
                "/chat/completions",
                "{}",
                "manual",
                true,
                false,
            )
            .is_ok());
        assert_eq!(repo.list_models(Some(deepseek.id)).unwrap().len(), 1);
        repo.upsert_route("general", Some(a.id)).unwrap();
        assert_eq!(
            repo.get_route("general")
                .unwrap()
                .unwrap()
                .provider_model_id,
            Some(a.id)
        );
        repo.upsert_route("general", None).unwrap();
        assert_eq!(
            repo.get_route("general")
                .unwrap()
                .unwrap()
                .provider_model_id,
            None
        );
    }

    #[test]
    fn provider_catalog_rejects_unknown_enums() {
        assert!(validate_protocol("unknown").is_err());
        assert!(validate_task_kind("unknown").is_err());
        assert!(validate_template_kind("openai").is_err());
    }
}
