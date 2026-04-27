use std::path::PathBuf;

/// 读取 AGENT.md 约束文件
/// 优先级: 工作目录 > 全局目录
#[tauri::command]
pub fn read_agent_config(work_dir: Option<String>) -> Result<AgentConfig, String> {
    let mut global_content = String::new();
    let mut workspace_content = String::new();
    let mut global_path = String::new();
    let mut workspace_path = String::new();

    // 全局 AGENT.md: ~/.openai-desktop/AGENT.md
    let home = std::env::var("APPDATA")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".into());
    let global_file = PathBuf::from(&home).join(".openai-desktop").join("AGENT.md");
    if global_file.exists() {
        global_path = global_file.to_string_lossy().to_string();
        global_content = std::fs::read_to_string(&global_file).unwrap_or_default();
    }

    // 工作目录 AGENT.md
    if let Some(dir) = work_dir {
        let ws_file = PathBuf::from(&dir).join("AGENT.md");
        if ws_file.exists() {
            workspace_path = ws_file.to_string_lossy().to_string();
            workspace_content = std::fs::read_to_string(&ws_file).unwrap_or_default();
        }
    }

    let found = !global_content.is_empty() || !workspace_content.is_empty();
    let merged = if workspace_content.is_empty() {
        global_content.clone()
    } else if global_content.is_empty() {
        workspace_content.clone()
    } else {
        format!("{}\n\n---\n\n{}", global_content, workspace_content)
    };

    Ok(AgentConfig {
        found,
        global_content,
        global_path,
        workspace_content,
        workspace_path,
        merged,
    })
}

/// 写入全局 AGENT.md
#[tauri::command]
pub fn write_global_agent_config(content: String) -> Result<(), String> {
    let home = std::env::var("APPDATA")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".into());
    let dir = PathBuf::from(&home).join(".openai-desktop");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let file = dir.join("AGENT.md");
    std::fs::write(&file, content).map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Debug, Default, serde::Serialize)]
pub struct AgentConfig {
    pub found: bool,
    pub global_content: String,
    pub global_path: String,
    pub workspace_content: String,
    pub workspace_path: String,
    pub merged: String,
}
