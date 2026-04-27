use serde::Serialize;

use crate::mcp::global_mcp;
use crate::tools;

#[derive(Debug, Serialize)]
pub struct SkillInfo {
    pub name: String,
    pub description: String,
    pub category: String,       // "file" | "command" | "mcp" | "system"
    pub enabled: bool,
}

/// 获取所有可用的 skills（内置工具 + MCP 插件工具 + 系统能力）
#[tauri::command]
pub async fn get_available_skills() -> Result<Vec<SkillInfo>, String> {
    let mut skills = Vec::new();

    // 内置工具
    let builtin = tools::get_all_tool_definitions();
    for tool in &builtin {
        let category = match tool.name.as_str() {
            "read_file" | "write_file" | "delete_file" | "list_directory" => "file",
            "run_command" => "command",
            _ => "builtin",
        };
        skills.push(SkillInfo {
            name: tool.name.clone(),
            description: tool.description.clone(),
            category: category.to_string(),
            enabled: true,
        });
    }

    // MCP 插件工具
    let mcp_servers = global_mcp().list_servers().await;
    for server in &mcp_servers {
        if server.enabled {
            // 尝试获取该服务器的工具列表
            let mcp_tools = crate::mcp::server::discover_tools(
                server.command.clone(),
                server.args.clone(),
            )
            .await
            .unwrap_or_default();

            for tool in &mcp_tools {
                // 去掉 mcp_ 前缀
                let name = tool.name.strip_prefix("mcp_").unwrap_or(&tool.name).to_string();
                skills.push(SkillInfo {
                    name,
                    description: tool.description.strip_prefix("[MCP] ").unwrap_or(&tool.description).to_string(),
                    category: format!("mcp/{}", server.name),
                    enabled: true,
                });
            }
        } else {
            // 已安装但未启用的插件
            skills.push(SkillInfo {
                name: server.name.clone(),
                description: format!("{} (已禁用)", server.command),
                category: "mcp".to_string(),
                enabled: false,
            });
        }
    }

    // 系统能力
    skills.push(SkillInfo {
        name: "thinking_chain".into(),
        description: "显示/隐藏模型的推理思考过程（支持 DeepSeek R1、Qwen QwQ 等）".into(),
        category: "system".into(),
        enabled: true,
    });
    skills.push(SkillInfo {
        name: "context_compression".into(),
        description: "对话超 80K tokens 时自动摘要压缩早期消息".into(),
        category: "system".into(),
        enabled: true,
    });
    skills.push(SkillInfo {
        name: "multi_provider".into(),
        description: "多模型供应商支持：OpenAI、Anthropic、DeepSeek、通义千问、Groq、Ollama".into(),
        category: "system".into(),
        enabled: true,
    });
    skills.push(SkillInfo {
        name: "workspace".into(),
        description: "工作区面板：打开本地文件夹，AI 可在工作目录内读写文件".into(),
        category: "system".into(),
        enabled: true,
    });

    Ok(skills)
}
