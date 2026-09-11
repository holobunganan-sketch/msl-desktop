pub mod merge;
pub mod protocol;
#[cfg(test)]
mod regression_tests;
pub mod rows;
pub mod service;
pub mod settings;

pub type SyncResult<T> = Result<T, SyncError>;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct SyncError {
    pub code: String,
    pub message: String,
}

impl SyncError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for SyncError {}
pub mod blobs;
