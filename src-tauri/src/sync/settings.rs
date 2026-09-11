use super::{SyncError, SyncResult};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct SyncConfig {
    pub directory: String,
    pub enabled: bool,
    pub interval_minutes: u32,
    pub device_id: String,
    pub dataset_id: String,
    pub generation: String,
    pub ai_primary: bool,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            directory: String::new(),
            enabled: true,
            interval_minutes: 180,
            device_id: uuid::Uuid::new_v4().to_string(),
            dataset_id: String::new(),
            generation: String::new(),
            ai_primary: true,
        }
    }
}

impl SyncConfig {
    pub fn validate(&self) -> SyncResult<()> {
        if !(5..=10_080).contains(&self.interval_minutes) {
            return Err(SyncError::new(
                "SYNC_INVALID_INTERVAL",
                "同步间隔需要在 5–10080 分钟之间",
            ));
        }
        if !super::protocol::valid_component(&self.device_id)
            || (!self.dataset_id.is_empty() && !super::protocol::valid_component(&self.dataset_id))
            || (!self.generation.is_empty() && !super::protocol::valid_component(&self.generation))
        {
            return Err(SyncError::new("SYNC_INVALID_DEVICE", "本机同步身份无效"));
        }
        Ok(())
    }
}
