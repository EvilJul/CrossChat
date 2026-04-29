#[cfg(test)]
mod tests {
    use crate::memory::{Memory, MemoryManager};

    #[test]
    fn test_memory_save_and_search() {
        let mgr = MemoryManager::new().unwrap();

        let memory = Memory {
            id: None,
            task: "测试任务".to_string(),
            solution: "测试解决方案".to_string(),
            tools_used: r#"["read_file"]"#.to_string(),
            success: true,
            timestamp: 1234567890,
            failure_reason: None,
            fix_applied: None,
        };

        let id = mgr.save(&memory).unwrap();
        assert!(id > 0);

        let results = mgr.search("测试", 10).unwrap();
        assert!(results.len() >= 1);
    }

    #[test]
    fn test_memory_cleanup() {
        let mgr = MemoryManager::new().unwrap();
        let deleted = mgr.cleanup(5).unwrap();
        assert!(deleted >= 0);
    }

    #[test]
    fn test_get_recent() {
        let mgr = MemoryManager::new().unwrap();
        let recent = mgr.get_recent(5).unwrap();
        assert!(recent.len() <= 5);
    }
}
