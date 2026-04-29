#[cfg(test)]
mod tests {
    use crate::skills::dependency::{SkillDependency, SkillDependencyManager};

    #[test]
    fn test_parse_dependencies() {
        let content = r#"---
name: test-skill
dependencies:
  - basic-skill@1.0.0
  - another-skill
---
# Content
"#;
        let deps = SkillDependencyManager::parse_dependencies(content);
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].name, "basic-skill");
        assert_eq!(deps[0].version, "1.0.0");
        assert_eq!(deps[1].name, "another-skill");
        assert_eq!(deps[1].version, "*");
    }

    #[test]
    fn test_resolve_install_order() {
        let skills = vec![
            ("A".to_string(), vec![SkillDependency { name: "B".to_string(), version: "*".to_string() }]),
            ("B".to_string(), vec![SkillDependency { name: "C".to_string(), version: "*".to_string() }]),
            ("C".to_string(), vec![]),
        ];

        let order = SkillDependencyManager::resolve_install_order(&skills).unwrap();

        // C 应该在 B 之前，B 应该在 A 之前
        let c_idx = order.iter().position(|s| s == "C").unwrap();
        let b_idx = order.iter().position(|s| s == "B").unwrap();
        let a_idx = order.iter().position(|s| s == "A").unwrap();

        assert!(c_idx < b_idx);
        assert!(b_idx < a_idx);
    }

    #[test]
    fn test_circular_dependency() {
        let skills = vec![
            ("A".to_string(), vec![SkillDependency { name: "B".to_string(), version: "*".to_string() }]),
            ("B".to_string(), vec![SkillDependency { name: "A".to_string(), version: "*".to_string() }]),
        ];

        let result = SkillDependencyManager::resolve_install_order(&skills);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("循环依赖"));
    }
}
