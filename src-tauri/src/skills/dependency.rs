use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Skill 依赖信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDependency {
    pub name: String,
    #[serde(default)]
    pub version: String,
}

/// 扩展的 Skill 元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetaExtended {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub builtin: bool,
    #[serde(default)]
    pub dependencies: Vec<SkillDependency>,
}

/// Skill 依赖管理器
pub struct SkillDependencyManager;

impl SkillDependencyManager {
    /// 解析依赖（从 YAML frontmatter）
    pub fn parse_dependencies(content: &str) -> Vec<SkillDependency> {
        if !content.starts_with("---") {
            return vec![];
        }

        let end = content[3..].find("---").unwrap_or(0);
        let fm = &content[3..3 + end];

        let mut deps = Vec::new();
        let mut in_deps = false;

        for line in fm.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("dependencies:") {
                in_deps = true;
                continue;
            }
            if in_deps {
                if trimmed.starts_with("- ") {
                    let dep_str = trimmed.trim_start_matches("- ").trim();
                    if let Some((name, version)) = dep_str.split_once('@') {
                        deps.push(SkillDependency {
                            name: name.to_string(),
                            version: version.to_string(),
                        });
                    } else {
                        deps.push(SkillDependency {
                            name: dep_str.to_string(),
                            version: "*".to_string(),
                        });
                    }
                } else if !trimmed.is_empty() && !trimmed.starts_with('#') {
                    in_deps = false;
                }
            }
        }

        deps
    }

    /// 检查依赖是否满足
    pub fn check_dependencies(
        skill_name: &str,
        dependencies: &[SkillDependency],
        installed_skills: &[crate::skills::SkillMeta],
    ) -> Result<(), String> {
        for dep in dependencies {
            let found = installed_skills.iter().find(|s| s.name == dep.name);
            match found {
                None => {
                    return Err(format!(
                        "Skill '{}' 依赖 '{}' 未安装",
                        skill_name, dep.name
                    ));
                }
                Some(skill) if !skill.enabled => {
                    return Err(format!(
                        "Skill '{}' 依赖 '{}' 未启用",
                        skill_name, dep.name
                    ));
                }
                Some(_) => {
                    // 版本检查（简化版，只检查是否存在）
                    // 完整版本检查需要 semver 库
                }
            }
        }
        Ok(())
    }

    /// 获取安装顺序（拓扑排序）
    pub fn resolve_install_order(
        skills: &[(String, Vec<SkillDependency>)],
    ) -> Result<Vec<String>, String> {
        let mut order = Vec::new();
        let mut visited = HashMap::new();
        let mut visiting = HashMap::new();

        fn visit(
            name: &str,
            skills: &HashMap<String, Vec<SkillDependency>>,
            visited: &mut HashMap<String, bool>,
            visiting: &mut HashMap<String, bool>,
            order: &mut Vec<String>,
        ) -> Result<(), String> {
            if visited.get(name).copied().unwrap_or(false) {
                return Ok(());
            }
            if visiting.get(name).copied().unwrap_or(false) {
                return Err(format!("检测到循环依赖: {}", name));
            }

            visiting.insert(name.to_string(), true);

            if let Some(deps) = skills.get(name) {
                for dep in deps {
                    visit(&dep.name, skills, visited, visiting, order)?;
                }
            }

            visiting.insert(name.to_string(), false);
            visited.insert(name.to_string(), true);
            order.push(name.to_string());

            Ok(())
        }

        let skill_map: HashMap<String, Vec<SkillDependency>> =
            skills.iter().cloned().collect();

        for (name, _) in skills {
            visit(name, &skill_map, &mut visited, &mut visiting, &mut order)?;
        }

        Ok(order)
    }
}

#[cfg(test)]
#[path = "dependency_tests.rs"]
mod tests;

