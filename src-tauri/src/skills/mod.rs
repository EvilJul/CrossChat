pub mod dependency;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Skill 元数据（从 SKILL.md 的 YAML frontmatter 解析）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMeta {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub builtin: bool,
}

/// Skill 管理器
pub struct SkillManager {
    skills_dir: PathBuf,
}

impl SkillManager {
    pub fn new() -> Self {
        let home = std::env::var("APPDATA")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".into());
        let dir = PathBuf::from(&home).join(".crosschat").join("skills");
        std::fs::create_dir_all(&dir).ok();
        Self { skills_dir: dir }
    }

    pub fn skills_dir(&self) -> &std::path::Path {
        &self.skills_dir
    }

    /// 列出所有已安装的 skill
    pub fn list_skills(&self) -> Vec<SkillMeta> {
        let mut skills = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.skills_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() { continue; }
                let skill_md = path.join("SKILL.md");
                if skill_md.exists() {
                    if let Some(meta) = self.parse_skill_meta(&skill_md, &path) {
                        skills.push(meta);
                    }
                }
            }
        }
        skills.sort_by(|a, b| a.name.cmp(&b.name));
        skills
    }

    /// 读取 skill 的 SKILL.md 内容
    pub fn read_skill_content(&self, name: &str) -> Option<String> {
        let skill_md = self.skills_dir.join(name).join("SKILL.md");
        if skill_md.exists() { std::fs::read_to_string(&skill_md).ok() } else { None }
    }

    /// 获取所有已启用 skill 的合并内容（注入到 AI 上下文）
    pub fn get_enabled_skill_context(&self) -> Option<String> {
        let skills = self.list_skills();
        let enabled: Vec<_> = skills.into_iter().filter(|s| s.enabled).collect();
        if enabled.is_empty() { return None; }

        let mut context = String::from(
            "[已安装的 Skills — 你拥有以下扩展能力，可根据需要使用]\n\n",
        );
        for skill in &enabled {
            if let Some(content) = self.read_skill_content(&skill.name) {
                let body = if content.starts_with("---") {
                    if let Some(end) = content[3..].find("---") {
                        content[3 + end + 3..].trim().to_string()
                    } else { content.clone() }
                } else { content.clone() };
                context.push_str(&format!("## Skill: {}\n{}\n\n---\n\n", skill.name, body));
            }
        }
        Some(context)
    }

    pub fn install_builtin_skill(&self, name: &str, content: &str) -> Result<(), String> {
        let skill_dir = self.skills_dir.join(name);
        std::fs::create_dir_all(&skill_dir).map_err(|e| e.to_string())?;
        std::fs::write(skill_dir.join("SKILL.md"), content).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn ensure_builtin_skills(&self) {
        for (name, content) in builtin_skills() {
            let skill_dir = self.skills_dir.join(name);
            if !skill_dir.join("SKILL.md").exists() {
                let _ = self.install_builtin_skill(name, content);
            }
        }
    }

    pub fn remove_skill(&self, name: &str) -> Result<(), String> {
        if let Some(meta) = self.list_skills().into_iter().find(|s| s.name == name) {
            if meta.builtin {
                return Err(format!("内置 Skill '{}' 不可删除，但可以禁用", name));
            }
        }
        let skill_dir = self.skills_dir.join(name);
        if skill_dir.exists() { std::fs::remove_dir_all(&skill_dir).map_err(|e| e.to_string())?; }
        Ok(())
    }

    pub fn set_enabled(&self, name: &str, enabled: bool) -> Result<(), String> {
        let config_path = self.skills_dir.join("_config.json");
        let mut config: HashMap<String, bool> = if config_path.exists() {
            serde_json::from_str(&std::fs::read_to_string(&config_path).unwrap_or_default()).unwrap_or_default()
        } else { HashMap::new() };
        config.insert(name.to_string(), enabled);
        std::fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap_or_default())
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn parse_skill_meta(&self, skill_md: &std::path::Path, dir: &std::path::Path) -> Option<SkillMeta> {
        let content = std::fs::read_to_string(skill_md).ok()?;
        let dir_name = dir.file_name()?.to_string_lossy().to_string();

        let (name, description, version, builtin) = if content.starts_with("---") {
            let end = content[3..].find("---")?;
            let fm = &content[3..3 + end];
            (
                extract_yaml_field(fm, "name").unwrap_or_else(|| dir_name.clone()),
                extract_yaml_field(fm, "description").unwrap_or_else(|| "无描述".into()),
                extract_yaml_field(fm, "version").unwrap_or_else(|| "0.1.0".into()),
                extract_yaml_field(fm, "builtin").map(|v| v == "true").unwrap_or(false),
            )
        } else {
            let first_line = content.lines().next().unwrap_or(&dir_name);
            (dir_name.clone(), first_line.trim_start_matches('#').trim().into(), "0.1.0".into(), false)
        };

        let cfg_path = self.skills_dir.join("_config.json");
        let enabled = if cfg_path.exists() {
            if let Ok(cfg) = std::fs::read_to_string(&cfg_path) {
                if let Ok(map) = serde_json::from_str::<HashMap<String, bool>>(&cfg) {
                    map.get(&name).copied().unwrap_or(true)
                } else { true }
            } else { true }
        } else { true };

        Some(SkillMeta { name, description, version, enabled, builtin })
    }
}

/// 提取 YAML 字段值
fn extract_yaml_field(yaml: &str, key: &str) -> Option<String> {
    for line in yaml.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(&format!("{}:", key)) {
            let v = rest.trim().trim_matches('"').trim_matches('\'');
            if !v.is_empty() { return Some(v.to_string()); }
        }
    }
    None
}

/// 解析 GitHub URL → (owner, repo, subpath)
fn parse_github_url(url: &str) -> Result<(String, String, String), String> {
    let url = url.trim().trim_end_matches('/');
    if let Some(rest) = url.strip_prefix("https://github.com/") {
        let parts: Vec<&str> = rest.split('/').collect();
        if parts.len() < 2 { return Err("URL 格式无效，需要至少 user/repo".into()); }
        let owner = parts[0].to_string();
        let repo = parts[1].to_string();
        let subpath = if parts.len() > 4 && parts[2] == "tree" {
            parts[4..].join("/")
        } else if parts.len() > 2 {
            parts[2..].join("/")
        } else { String::new() };
        return Ok((owner, repo, subpath));
    }
    let parts: Vec<&str> = url.split('/').collect();
    if parts.len() >= 2 && !parts[0].contains('.') {
        return Ok((parts[0].to_string(), parts[1].to_string(), String::new()));
    }
    Err(format!("无法解析 URL: {}。支持格式: https://github.com/user/repo 或 user/repo", url))
}

/// 推断 Skill 名称
fn infer_skill_name(content: &str, subpath: &str) -> String {
    if let Some(name) = extract_yaml_field(content, "name") {
        return name.to_lowercase().replace(' ', "-");
    }
    if !subpath.is_empty() {
        if let Some(last) = subpath.split('/').last() { return last.to_string(); }
    }
    "installed-skill".into()
}

/// 从 GitHub URL 下载并安装 Skill
pub async fn install_skill_from_url(url: &str) -> Result<SkillMeta, String> {
    let (owner, repo, subpath) = parse_github_url(url)?;

    let api_url = if subpath.is_empty() {
        format!("https://api.github.com/repos/{}/{}/contents/", owner, repo)
    } else {
        format!("https://api.github.com/repos/{}/{}/contents/{}", owner, repo, subpath)
    };

    let client = reqwest::Client::new();
    let response = client.get(&api_url)
        .header("User-Agent", "crosschat-skill-installer")
        .header("Accept", "application/vnd.github.v3+json")
        .send().await.map_err(|e| format!("网络请求失败: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("GitHub API 返回 {} — 仓库或路径可能不存在", response.status()));
    }

    let items: Vec<serde_json::Value> = response.json().await
        .map_err(|e| format!("解析 GitHub 响应: {}", e))?;

    let skill_md = items.iter().find(|item| {
        item["name"].as_str().map(|n| n.eq_ignore_ascii_case("SKILL.md")).unwrap_or(false)
    }).ok_or_else(|| {
        let names: Vec<_> = items.iter().filter_map(|i| i["name"].as_str()).collect();
        format!("未找到 SKILL.md。目录包含:\n{}",
            names.iter().map(|n| format!("  - {}", n)).collect::<Vec<_>>().join("\n"))
    })?;

    let download_url = skill_md["download_url"].as_str()
        .ok_or("无法获取 SKILL.md 下载链接")?;

    let content = client.get(download_url)
        .header("User-Agent", "crosschat-skill-installer")
        .send().await.map_err(|e| format!("下载失败: {}", e))?
        .text().await.map_err(|e| format!("读取失败: {}", e))?;

    let skill_name = infer_skill_name(&content, &subpath);

    let mgr = global_skills();
    let skill_dir = mgr.skills_dir().join(&skill_name);
    std::fs::create_dir_all(&skill_dir).map_err(|e| e.to_string())?;
    std::fs::write(skill_dir.join("SKILL.md"), &content).map_err(|e| e.to_string())?;

    let desc = extract_yaml_field(&content, "description")
        .unwrap_or_else(|| format!("从 {}/{} 安装", owner, repo));
    Ok(SkillMeta {
        name: skill_name,
        description: desc,
        version: extract_yaml_field(&content, "version").unwrap_or_else(|| "0.1.0".into()),
        enabled: true,
        builtin: false,
    })
}

/// 全局 Skill 管理器
static SKILL_MANAGER: std::sync::LazyLock<SkillManager> =
    std::sync::LazyLock::new(|| {
        let mgr = SkillManager::new();
        mgr.ensure_builtin_skills();
        mgr
    });

pub fn global_skills() -> &'static SkillManager { &SKILL_MANAGER }

/// 内置 Skill 定义
fn builtin_skills() -> Vec<(&'static str, &'static str)> {
    vec![(
        "excel-automation",
        r#"---
name: excel-automation
description: 使用 Python 自动化处理 Excel 文件（.xlsx / .xls / .csv）
version: "1.0.0"
builtin: true
---

# Excel 自动化

你拥有 Excel 文件的读写和自动化处理能力。当你需要处理 Excel 文件时，使用 `run_command` 工具执行 Python 脚本。

## 依赖检查

在首次使用前，检查并安装必要的 Python 库：

```bash
pip install openpyxl xlsxwriter pandas xlrd
```

## 读取 Excel 文件

```python
import openpyxl
wb = openpyxl.load_workbook('文件路径.xlsx')
ws = wb.active
for row in ws.iter_rows(values_only=True):
    print(row)
```

## 创建新的 Excel 文件

```python
import xlsxwriter
wb = xlsxwriter.Workbook('输出文件.xlsx')
ws = wb.add_worksheet()
ws.write(0, 0, '标题')
wb.close()
```

## 使用 pandas 处理数据

```python
import pandas as pd
df = pd.read_excel('文件.xlsx')
# 数据处理...
df.to_excel('输出.xlsx', index=False)
```

## 注意事项

- 处理敏感 Excel 数据前先询问用户确认
- 大文件（>10MB）提示用户可能需要较长时间
- 始终保留原始文件备份
- 使用 `write_file` 工具保存生成的 Python 脚本供用户审查
"#,
    )]
}
