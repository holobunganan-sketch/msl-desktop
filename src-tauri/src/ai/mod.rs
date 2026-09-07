//! AI 能力（指南 §8）：Provider 适配 + Morning Brief。
//! 默认不常驻——只在用户明确触发时按需调用。

pub mod analysis;
pub mod analysis_snapshot;
pub mod apply;
pub mod brief;
pub mod catalog;
pub mod efficiency;
pub mod knowledge_contract;
pub mod prompts;
pub mod provider;
pub mod receipts;
pub mod report_contract;
pub mod reports;
pub mod router;
pub mod schema;
pub mod translation;
