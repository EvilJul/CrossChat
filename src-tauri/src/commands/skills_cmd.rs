use serde::Serialize;

use crate::mcp::global_mcp;
use crate::skills::{global_skills, SkillMeta};
use crate::tools;

#[derive(Debug, Serialize)]
pub struct SkillInfo {
    pub name: String,
    pub description: String,
    pub category: String, // "file" | "command" | "mcp" | "system" | "skill"
    pub enabled: bool,
}

/// 列出已安装的 Skills（兼容 Claude Code 格式，~/.crosschat/skills/*/SKILL.md）
#[tauri::command]
pub fn list_skills() -> Result<Vec<SkillMeta>, String> {
    Ok(global_skills().list_skills())
}

/// 启用/禁用某个 Skill
#[tauri::command]
pub fn toggle_skill(name: String, enabled: bool) -> Result<(), String> {
    global_skills().set_enabled(&name, enabled)
}

/// 删除非内置 Skill
#[tauri::command]
pub fn remove_skill(name: String) -> Result<(), String> {
    global_skills().remove_skill(&name)
}

/// 获取所有可用的 skills（内置工具 + MCP 插件 + 系统能力 + 已安装 Skills）
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
            let mcp_tools = crate::mcp::server::discover_tools(
                server.command.clone(),
                server.args.clone(),
            )
            .await
            .unwrap_or_default();

            for tool in &mcp_tools {
                let name = tool.name.strip_prefix("mcp_").unwrap_or(&tool.name).to_string();
                skills.push(SkillInfo {
                    name,
                    description: tool
                        .description
                        .strip_prefix("[MCP] ")
                        .unwrap_or(&tool.description)
                        .to_string(),
                    category: format!("mcp/{}", server.name),
                    enabled: true,
                });
            }
        } else {
            skills.push(SkillInfo {
                name: server.name.clone(),
                description: format!("{} (已禁用)", server.command),
                category: "mcp".to_string(),
                enabled: false,
            });
        }
    }

    // 已安装的 Skills（兼容 Claude Code 生态）
    for sk in global_skills().list_skills() {
        skills.push(SkillInfo {
            name: sk.name.clone(),
            description: sk.description.clone(),
            category: "skill".to_string(),
            enabled: sk.enabled,
        });
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
        description: "对话超 100K tokens 时自动摘要压缩早期消息".into(),
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
