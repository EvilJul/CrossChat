use async_trait::async_trait;
use crate::core::models::ApprovalStatus;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct ApprovalRequest {
    pub id: String,
    pub tool_name: String,
    pub args: Value,
    pub rationale: String,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ApprovalPolicy {
    Always,
    Dangerous,
    Never,
}

#[async_trait]
pub trait ApprovalGate: Send + Sync {
    async fn request_approval(&self, request: ApprovalRequest) -> ApprovalStatus;
    fn get_policy(&self) -> ApprovalPolicy;
    fn set_policy(&mut self, policy: ApprovalPolicy);
}
