use crate::metrics::{global_metrics, ToolMetrics};
use crate::providers::types::ToolDefinition;
use crate::tools::{self, ToolResult};
use std::collections::HashMap;

/// 工具注册表
pub struct ToolRegistry {
    tools: HashMap<String, ToolDefinition>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            tools: HashMap::new(),
        };

        // 注册内置工具
        for tool in tools::get_all_tool_definitions() {
            registry.register(tool);
        }

        registry
    }

    /// 注册工具
    pub fn register(&mut self, tool: ToolDefinition) {
        self.tools.insert(tool.name.clone(), tool);
    }

    /// 批量注册工具
    pub fn register_batch(&mut self, tools: Vec<ToolDefinition>) {
        for tool in tools {
            self.register(tool);
        }
    }

    /// 获取所有工具定义
    pub fn get_all_definitions(&self) -> Vec<ToolDefinition> {
        self.tools.values().cloned().collect()
    }

    /// 执行工具（带性能监控）
    pub async fn execute(
        &self,
        name: &str,
        arguments: &serde_json::Value,
        work_dir: &str,
    ) -> ToolResult {
        if name.starts_with("mcp_") {
            return ToolResult {
                success: false,
                content: "MCP 工具需要由 Agent 层面执行".into(),
            };
        }

        let start = std::time::Instant::now();
        let result = tools::execute_tool(name, arguments, work_dir).await;
        let duration = start.elapsed().as_millis() as i64;

        // 记录性能指标
        let _ = global_metrics().record(&ToolMetrics {
            tool_name: name.to_string(),
            execution_time_ms: duration,
            success: result.success,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        });

        result
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
