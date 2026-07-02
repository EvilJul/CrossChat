pub mod dependency;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Skill 元数据
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
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub regex_patterns: Option<Vec<String>>,
    #[serde(default)]
    pub file_extensions: Option<Vec<String>>,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub allowed_tools: Option<Vec<String>>,
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

    /// 触发机制与激活评分算法：根据 prompt 和修改的文件扩展名，评分最高的前 3 个被激活并带 24KB 字节预算保护
    pub fn get_activated_skills_context(
        &self,
        prompt: &str,
        file_paths: &[String],
    ) -> (Option<String>, Option<Vec<String>>) {
        let skills = self.list_skills();
        let enabled: Vec<_> = skills.into_iter().filter(|s| s.enabled).collect();
        if enabled.is_empty() {
            return (None, None);
        }

        let mut scored_skills = Vec::new();
        let prompt_lower = prompt.to_lowercase();

        for skill in enabled {
            let mut score = 0;
            let mut matched = false;
            let skill_id = skill.id.clone().unwrap_or_else(|| skill.name.clone());

            // 1. 显式提及 (Explicit Mention)
            let mention_id = format!("@{}", skill_id.to_lowercase());
            let slash_id = format!("/skill:{}", skill_id.to_lowercase());
            if prompt_lower.contains(&mention_id) || prompt_lower.contains(&slash_id) {
                score = 1000 + skill.priority;
                matched = true;
            }

            // 2. 前缀命令触发 (Command Starts-with)
            if !matched {
                if let Some(cmd) = &skill.command {
                    let cmd_lower = cmd.to_lowercase();
                    if prompt_lower.starts_with(&cmd_lower) {
                        score = 900 + skill.priority;
                        matched = true;
                    }
                }
            }

            // 3. 正则表达式匹配 (Prompt Patterns Regex)
            if !matched {
                if let Some(regexes) = &skill.regex_patterns {
                    for pattern in regexes {
                        if let Ok(re) = regex::Regex::new(pattern) {
                            if re.is_match(prompt) {
                                score = 500 + skill.priority;
                                matched = true;
                                break;
                            }
                        }
                    }
                }
            }

            // 4. 文件类型/路径匹配 (File Types Extension)
            if !matched {
                if let Some(extensions) = &skill.file_extensions {
                    for path in file_paths {
                        let path_lower = path.to_lowercase();
                        for ext in extensions {
                            if path_lower.ends_with(&ext.to_lowercase()) {
                                score = 300 + skill.priority;
                                matched = true;
                                break;
                            }
                        }
                        if matched {
                            break;
                        }
                    }
                }
            }

            if matched && score > 0 {
                scored_skills.push((skill, score));
            }
        }

        if scored_skills.is_empty() {
            return (None, None);
        }

        // 按评分降序排序，取最高评分的前 3 个
        scored_skills.sort_by(|a, b| b.1.cmp(&a.1));
        let active_skills: Vec<_> = scored_skills.into_iter().take(3).map(|item| item.0).collect();

        // 拼接上下文并在 24KB 预算内截断
        let mut context = String::from("[已激活的 Skills — 已针对当前任务自动挂载扩展能力]\n\n");
        let mut budget_bytes = 24 * 1024; // 24KB
        let mut allowed_tools_union = Vec::new();
        let mut has_allowed_tools = false;

        for skill in &active_skills {
            if let Some(content) = self.read_skill_content(&skill.name) {
                let body = if content.starts_with("---") {
                    if let Some(end) = content[3..].find("---") {
                        content[3 + end + 3..].trim().to_string()
                    } else { content.clone() }
                } else { content.clone() };

                let skill_body = format!("## Active Skill: {}\n{}\n\n", skill.name, body);
                if skill_body.len() < budget_bytes {
                    budget_bytes -= skill_body.len();
                    context.push_str(&skill_body);
                } else {
                    context.push_str(&skill_body[..budget_bytes]);
                    break;
                }
            }

            // Allowed tools list union
            if let Some(tools) = &skill.allowed_tools {
                has_allowed_tools = true;
                for t in tools {
                    if !allowed_tools_union.contains(t) {
                        allowed_tools_union.push(t.clone());
                    }
                }
            }
        }

        let final_context = Some(context);
        let final_allowed = if has_allowed_tools { Some(allowed_tools_union) } else { None };

        (final_context, final_allowed)
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
            let skill_file = skill_dir.join("SKILL.md");

            // 检查是否需要更新：文件不存在或版本不匹配
            let needs_update = if !skill_file.exists() {
                true
            } else if let Ok(existing) = std::fs::read_to_string(&skill_file) {
                let existing_ver = extract_yaml_field(&existing, "version").unwrap_or_default();
                let new_ver = extract_yaml_field(content, "version").unwrap_or_default();
                existing_ver != new_ver
            } else {
                false
            };

            if needs_update {
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
        let dir_name = dir.file_name()?.to_string_lossy().to_string();
        let skill_json = dir.join("skill.json");

        let mut meta = if skill_json.exists() {
            if let Ok(json_content) = std::fs::read_to_string(&skill_json) {
                if let Ok(m) = serde_json::from_str::<SkillMeta>(&json_content) {
                    m
                } else {
                    self.parse_from_markdown(skill_md, &dir_name)?
                }
            } else {
                self.parse_from_markdown(skill_md, &dir_name)?
            }
        } else {
            self.parse_from_markdown(skill_md, &dir_name)?
        };

        let cfg_path = self.skills_dir.join("_config.json");
        let enabled = if cfg_path.exists() {
            if let Ok(cfg) = std::fs::read_to_string(&cfg_path) {
                if let Ok(map) = serde_json::from_str::<HashMap<String, bool>>(&cfg) {
                    map.get(&meta.name).copied().unwrap_or(true)
                } else { true }
            } else { true }
        } else { true };

        meta.enabled = enabled;
        Some(meta)
    }

    fn parse_from_markdown(&self, skill_md: &std::path::Path, dir_name: &str) -> Option<SkillMeta> {
        let content = std::fs::read_to_string(skill_md).ok()?;
        let mut name = dir_name.to_string();
        let mut description = "无描述".to_string();
        let mut version = "0.1.0".to_string();
        let mut builtin = false;
        let mut id = None;
        let mut command = None;
        let mut regex_patterns = None;
        let mut file_extensions = None;
        let mut priority = 0;
        let mut allowed_tools = None;

        if content.starts_with("---") {
            if let Some(end) = content[3..].find("---") {
                let fm = &content[3..3 + end];
                name = extract_yaml_field(fm, "name").unwrap_or_else(|| dir_name.to_string());
                description = extract_yaml_field(fm, "description").unwrap_or_else(|| "无描述".to_string());
                version = extract_yaml_field(fm, "version").unwrap_or_else(|| "0.1.0".to_string());
                builtin = extract_yaml_field(fm, "builtin").map(|v| v == "true").unwrap_or(false);
                id = extract_yaml_field(fm, "id");
                command = extract_yaml_field(fm, "command");
                
                if let Some(pat_str) = extract_yaml_field(fm, "regex_patterns").or(extract_yaml_field(fm, "regex")) {
                    regex_patterns = Some(pat_str.split(',').map(|s| s.trim().to_string()).collect());
                }
                if let Some(ext_str) = extract_yaml_field(fm, "file_extensions").or(extract_yaml_field(fm, "file_types")) {
                    file_extensions = Some(ext_str.split(',').map(|s| s.trim().to_string()).collect());
                }
                if let Some(pri_str) = extract_yaml_field(fm, "priority") {
                    priority = pri_str.parse().unwrap_or(0);
                }
                if let Some(tool_str) = extract_yaml_field(fm, "allowed_tools").or(extract_yaml_field(fm, "allowedTools")) {
                    allowed_tools = Some(tool_str.split(',').map(|s| s.trim().to_string()).collect());
                }
            }
        } else {
            let first_line = content.lines().next().unwrap_or(dir_name);
            description = first_line.trim_start_matches('#').trim().to_string();
        }

        Some(SkillMeta {
            name,
            description,
            version,
            enabled: true,
            builtin,
            id,
            command,
            regex_patterns,
            file_extensions,
            priority,
            allowed_tools,
        })
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
        id: None,
        command: None,
        regex_patterns: None,
        file_extensions: None,
        priority: 0,
        allowed_tools: None,
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
    vec![
        (
            "excel-automation",
            r#"---
name: excel-automation
description: 使用 Python 自动化处理 Excel 文件（.xlsx / .xls / .csv）
version: "1.0.2"
builtin: true
---

# Excel 自动化

你拥有 Excel 文件的读写和自动化处理能力。

## 重要：工具选择规则

- **普通文本文件**（.txt, .md, .json, .csv 纯文本, .py, .js 等）：必须使用 `read_file` / `write_file` 工具
- **Excel 二进制文件**（.xlsx, .xls）：使用 `run_command` 执行 Python 脚本
- **CSV 文件**：优先用 `read_file` 读取；仅当需要复杂数据处理（透视、合并、统计）时才用 Python

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
- **不要**对普通文本文件使用 Python 脚本，直接用 `read_file`/`write_file`
- **不要**用 `write_file` 保存 Python 脚本文件，直接用 `run_command` 执行内联 Python 代码
- 执行 Python 时使用 `python -c "..."` 格式，不要生成临时 .py 文件
"#,
        ),
        (
            "office-charts",
            r#"---
name: office-charts
description: 数据可视化图表自动生成技能 (Bar, Line, Pie, Doughnut)
version: "1.0.0"
builtin: true
---

# 数据可视化图表生成规范

当你需要展示数据对比、趋势、比例等分析结果时，除了文字描述，你必须在回答的末尾使用 ```chart-data 代码块输出结构化的 JSON 数据，以供前端渲染为交互式图表。

## 格式规范

```chart-data
{
  "type": "bar" | "line" | "pie" | "doughnut",
  "title": "图表标题",
  "labels": ["标签1", "标签2", "标签3", ...],
  "datasets": [
    {
      "label": "数据集名称",
      "data": [数值1, 数值2, 数值3, ...]
    }
  ]
}
```

## 注意事项
- datasets 中可以包含多个数据集（折线图、柱状图等多系列展示）。
- 数值必须为纯数字，不能包含单位（单位应写在标题或数据标签中）。
- 图表数据生成后应在文字中进行简单的数据解读。
"#,
        ),
        (
            "office-documents",
            r#"---
name: office-documents
description: 自动排版并生成 Word (.docx) 和 PPT (.pptx) 文档技能
version: "1.0.0"
builtin: true
---

# 智能文档与幻灯片生成规范

当你为用户梳理好文章大纲、报告大纲、或者PPT结构后，你可以通过输出 ```document-data 代码块以 JSON 格式提供结构化数据，前端会自动渲染“智能文档生成器”卡片，允许用户一键下载 Word 或 PPT。

## 格式规范

```document-data
{
  "document_type": "docx" | "pptx",
  "title": "文档或演示文稿标题",
  "sections": [
    {
      "heading": "章节/幻灯片标题",
      "content": "章节正文内容 / 幻灯片要点内容",
      "table": [
        ["表头1", "表头2", ...],
        ["单元格A1", "单元格A2", ...],
        ...
      ]
    }
  ]
}
```

## 章节分配规则
- 对于 docx：每个 section 会生成一级标题，正文及表格。
- 对于 pptx：每个 section 会生成一张幻灯片，其中 heading 为标题，content 为左侧要点文字，table 为右侧数据表。
"#,
        ),
        (
            "office-polishing",
            r#"---
name: office-polishing
description: 商务与政务公文排版与写作风格优化规范
version: "1.0.0"
builtin: true
---

# 公文写作与润色规范

在协助用户起草公文、信函、会议纪要等办公文书时，请严格遵守以下规范：

## 写作风格与语气
- **公文风格**：客观、严谨、得体、措辞严密。避免口语化表达和抒情色彩。
- **商务风格**：礼貌、专业、高效、重点突出。
- **纪要风格**：结构化、要点分明，必须包含“会议背景”、“议决事项”、“待办工作”。

## 层次排版原则
- 层次分明：建议使用“一、”、“（一）”、“1.”、“（1）”四级结构。
- 重点加粗：对于核心结论和待办人，进行加粗处理。
- 清爽简洁：正文段落之间保持适当空行，使用条目化（Bullet points）列出待办项。
"#,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skills_scoring_and_allowed_tools() {
        let dir = std::env::temp_dir().join(format!(
            "crosschat_test_skills_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let mgr = SkillManager { skills_dir: dir.clone() };

        let skill1_content = r#"---
name: test-python-skill
description: Python auto skill
version: "1.0.0"
priority: 10
command: "/run-py"
regex_patterns: "import .*"
file_extensions: ".py"
allowed_tools: "run_python, read_file"
---
# Test python skill content
"#;

        let skill2_content = r#"---
name: test-general-skill
description: General skill
version: "1.0.0"
priority: 5
file_extensions: ".txt"
allowed_tools: "read_file, write_file"
---
# Test general skill content
"#;

        mgr.install_builtin_skill("test-python-skill", skill1_content).unwrap();
        mgr.install_builtin_skill("test-general-skill", skill2_content).unwrap();

        // 1. Explicit mention trigger
        let (ctx, allowed) = mgr.get_activated_skills_context("please run @test-python-skill now", &[]);
        assert!(ctx.is_some());
        let ctx_str = ctx.unwrap();
        assert!(ctx_str.contains("test-python-skill"));
        let allowed_list = allowed.unwrap();
        assert!(allowed_list.contains(&"run_python".to_string()));
        assert!(allowed_list.contains(&"read_file".to_string()));
        assert_eq!(allowed_list.len(), 2);

        // 2. Command trigger
        let (ctx, _allowed) = mgr.get_activated_skills_context("/run-py hello", &[]);
        assert!(ctx.is_some());
        assert!(ctx.unwrap().contains("test-python-skill"));

        // 3. Regex pattern match trigger
        let (ctx, _allowed) = mgr.get_activated_skills_context("please write import math code", &[]);
        assert!(ctx.is_some());
        assert!(ctx.unwrap().contains("test-python-skill"));

        // 4. File extension match trigger
        let (ctx, _allowed) = mgr.get_activated_skills_context("work on this", &["main.py".to_string()]);
        assert!(ctx.is_some());
        assert!(ctx.unwrap().contains("test-python-skill"));

        // 5. Multiple skills and union of allowed_tools
        let (ctx, allowed) = mgr.get_activated_skills_context("please run @test-python-skill and @test-general-skill", &[]);
        assert!(ctx.is_some());
        let allowed_list = allowed.unwrap();
        assert!(allowed_list.contains(&"run_python".to_string()));
        assert!(allowed_list.contains(&"read_file".to_string()));
        assert!(allowed_list.contains(&"write_file".to_string()));
        assert_eq!(allowed_list.len(), 3);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
