use crate::db::{Database, DbResult};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct StorageCategory {
    pub category: String,
    pub bytes: u64,
    pub count: u64,
    pub protected: u64,
}
#[derive(Debug, Clone, Serialize)]
pub struct StorageUsage {
    pub categories: Vec<StorageCategory>,
    pub cache_bytes: u64,
    pub database_bytes: u64,
    pub cache_limit_bytes: u64,
    pub high_water_percent: u8,
    pub target_percent: u8,
    pub last_cleanup_at: Option<i64>,
}

pub fn get_usage(db: &Database) -> DbResult<StorageUsage> {
    let entries = crate::db::documents::CacheEntryRepo::new(db.conn()).list()?;
    let mut categories = std::collections::BTreeMap::<String, (u64, u64)>::new();
    for entry in entries {
        let path = crate::storage::paths::existing_cache_path(&entry.relative_path).ok();
        let bytes = path
            .and_then(|p| std::fs::symlink_metadata(p).ok().map(|m| m.len()))
            .unwrap_or(entry.size_bytes.max(0) as u64);
        let value = categories.entry(entry.category).or_default();
        value.0 += bytes;
        value.1 += 1;
    }
    let settings = crate::db::provider::AppSettingsRepo::new(db.conn());
    let setting = |key: &str, default: &str| {
        settings
            .get(key)
            .ok()
            .flatten()
            .unwrap_or_else(|| default.into())
    };
    let cats = categories
        .into_iter()
        .map(|(category, (bytes, count))| StorageCategory {
            category,
            bytes,
            count,
            protected: 0,
        })
        .collect::<Vec<_>>();
    let cache_bytes = cats.iter().map(|c| c.bytes).sum();
    let db_bytes = crate::db::default_db_path()
        .metadata()
        .map(|m| m.len())
        .unwrap_or(0);
    Ok(StorageUsage {
        categories: cats,
        cache_bytes,
        database_bytes: db_bytes,
        cache_limit_bytes: setting("cache_limit_bytes", "2147483648")
            .parse()
            .unwrap_or(2147483648),
        high_water_percent: setting("cache_high_water_percent", "80")
            .parse()
            .unwrap_or(80),
        target_percent: setting("cache_target_percent", "60").parse().unwrap_or(60),
        last_cleanup_at: setting("cache_last_auto_cleanup_at", "").parse().ok(),
    })
}
