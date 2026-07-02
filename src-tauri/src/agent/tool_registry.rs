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

    /// 获取所有工具的哈希映射引用
    pub fn get_tools(&self) -> &HashMap<String, ToolDefinition> {
        &self.tools
    }

    /// 获取所有工具定义（支持多工具动态目录检索折叠）
    pub fn get_all_definitions(&self) -> Vec<ToolDefinition> {
        let mcp_tool_count = self.tools.keys().filter(|k| k.starts_with("mcp_")).count();
        if mcp_tool_count > 24 {
            let mut result = Vec::new();
            for (name, tool) in &self.tools {
                if !name.starts_with("mcp_") {
                    result.push(tool.clone());
                }
            }
            // 注入 4 个元管理工具定义
            result.push(ToolDefinition {
                name: "mcp_search".into(),
                description: "在本地 MCP 工具库中模糊搜索相关工具。当可用工具过多时，使用此工具查找所需的能力。".into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {"type": "string", "description": "搜索关键词（如同义词、操作名称）"}
                    },
                    "required": ["query"]
                }),
            });
            result.push(ToolDefinition {
                name: "mcp_describe".into(),
                description: "获取特定 MCP 工具的详细 JSON Schema 说明和输入参数定义。".into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "tool_name": {"type": "string", "description": "工具名称（包含 mcp_ 前缀）"}
                    },
                    "required": ["tool_name"]
                }),
            });
            result.push(ToolDefinition {
                name: "mcp_call".into(),
                description: "调用指定的 MCP 工具。传入工具名称和其所需的 JSON arguments 参数字典。".into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "tool_name": {"type": "string", "description": "要调用的工具名称（如 mcp_xxx）"},
                        "arguments": {"type": "object", "description": "要传入工具的 JSON 参数字典"}
                    },
                    "required": ["tool_name", "arguments"]
                }),
            });
            result.push(ToolDefinition {
                name: "mcp_refresh_catalog".into(),
                description: "刷新本地 MCP 服务器缓存，重新构建索引目录。".into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            });
            result
        } else {
            self.tools.values().cloned().collect()
        }
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
