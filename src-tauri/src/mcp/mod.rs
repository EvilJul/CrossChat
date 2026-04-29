pub mod health;
pub mod server;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::providers::types::ToolDefinition;

/// MCP 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub id: String,
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub enabled: bool,
    #[serde(default)]
    pub version: String,
}

/// MCP 工具缓存条目
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedTools {
    tools: Vec<ToolDefinition>,
    version: String,
    timestamp: i64,
}

/// MCP 管理器
pub struct McpManager {
    configs: Arc<Mutex<HashMap<String, McpServerConfig>>>,
    tools_cache: Arc<Mutex<HashMap<String, Vec<ToolDefinition>>>>,
    config_path: std::path::PathBuf,
    cache_db_path: std::path::PathBuf,
}

impl McpManager {
    pub fn new() -> Self {
        let home = std::env::var("APPDATA")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".into());
        let dir = std::path::PathBuf::from(home).join(".crosschat");
        std::fs::create_dir_all(&dir).ok();
        let config_path = dir.join("mcp_servers.json");
        let cache_db_path = dir.join("mcp_cache.json");

        let configs = Self::load_configs(&config_path);
        let initial_cache = Self::load_cache_from_disk(&cache_db_path);

        Self {
            configs: Arc::new(Mutex::new(configs)),
            tools_cache: Arc::new(Mutex::new(initial_cache)),
            config_path,
            cache_db_path,
        }
    }

    fn load_configs(path: &std::path::Path) -> HashMap<String, McpServerConfig> {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(configs) = serde_json::from_str::<Vec<McpServerConfig>>(&content) {
                return configs.into_iter().map(|c| (c.id.clone(), c)).collect();
            }
        }
        HashMap::new()
    }

    async fn save_configs(&self) {
        let configs: Vec<McpServerConfig> = self.configs.lock().await.values().cloned().collect();
        if let Ok(json) = serde_json::to_string_pretty(&configs) {
            std::fs::write(&self.config_path, json).ok();
        }
    }

    /// 从磁盘加载持久化缓存（纯同步，不需要锁 async mutex）
    fn load_cache_from_disk(cache_db_path: &std::path::Path) -> HashMap<String, Vec<ToolDefinition>> {
        let mut result = HashMap::new();
        if let Ok(content) = std::fs::read_to_string(cache_db_path) {
            if let Ok(cache) = serde_json::from_str::<HashMap<String, CachedTools>>(&content) {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64;
                for (id, cached) in cache {
                    if now - cached.timestamp < 86400 {
                        result.insert(id, cached.tools);
                    }
                }
            }
        }
        result
    }

    /// 保存持久化缓存
    async fn save_cache(&self) {
        let tools_cache = self.tools_cache.lock().await;
        let mut cache_map = HashMap::new();
        let configs = self.configs.lock().await;

        for (id, tools) in tools_cache.iter() {
            if let Some(cfg) = configs.get(id) {
                cache_map.insert(id.clone(), CachedTools {
                    tools: tools.clone(),
                    version: cfg.version.clone(),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs() as i64,
                });
            }
        }

        if let Ok(json) = serde_json::to_string_pretty(&cache_map) {
            std::fs::write(&self.cache_db_path, json).ok();
        }
    }

    pub async fn add_server(&self, config: McpServerConfig) {
        let id = config.id.clone();
        self.configs.lock().await.insert(id.clone(), config);
        self.tools_cache.lock().await.remove(&id);
        self.save_configs().await;
        self.save_cache().await;
    }

    pub async fn remove_server(&self, id: &str) {
        self.configs.lock().await.remove(id);
        self.tools_cache.lock().await.remove(id);
        self.save_configs().await;
        self.save_cache().await;
    }

    pub async fn set_enabled(&self, id: &str, enabled: bool) {
        if let Some(cfg) = self.configs.lock().await.get_mut(id) {
            cfg.enabled = enabled;
            if !enabled {
                self.tools_cache.lock().await.remove(id);
            }
        }
        self.save_configs().await;
    }

    pub async fn list_servers(&self) -> Vec<McpServerConfig> {
        self.configs.lock().await.values().cloned().collect()
    }

    /// 获取所有已启用 MCP 服务器的工具定义
    pub async fn get_all_tools(&self) -> Vec<ToolDefinition> {
        let configs = self.configs.lock().await.clone();
        let mut all_tools = Vec::new();

        for (id, cfg) in configs.iter() {
            if !cfg.enabled {
                continue;
            }

            // 检查缓存
            let has_cache = self.tools_cache.lock().await.contains_key(id);
            if !has_cache {
                match server::discover_tools(cfg.command.clone(), cfg.args.clone()).await {
                    Ok(tools) => {
                        self.tools_cache.lock().await.insert(id.clone(), tools);
                        self.save_cache().await;
                    }
                    Err(e) => {
                        eprintln!("MCP {} 工具发现失败: {}", cfg.name, e);
                    }
                }
            }

            if let Some(tools) = self.tools_cache.lock().await.get(id) {
                all_tools.extend(tools.clone());
            }
        }

        all_tools
    }

    /// 执行 MCP 工具调用
    pub async fn execute_mcp_tool(
        &self,
        server_id: &str,
        tool_name: &str,
        arguments: &serde_json::Value,
    ) -> Result<String, String> {
        let configs = self.configs.lock().await;
        let cfg = configs.get(server_id).ok_or("MCP 服务器未找到")?;
        server::call_tool(cfg.command.clone(), cfg.args.clone(), tool_name, arguments).await
    }
}

/// 全局 MCP 管理器实例
static MCP_MANAGER: std::sync::LazyLock<McpManager> = std::sync::LazyLock::new(McpManager::new);

pub fn global_mcp() -> &'static McpManager {
    &MCP_MANAGER
}
