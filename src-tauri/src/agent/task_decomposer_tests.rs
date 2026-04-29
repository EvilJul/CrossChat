#[cfg(test)]
mod tests {
    use crate::agent::task_decomposer::{Task, TaskDecomposer, TaskStatus};

    #[test]
    fn test_is_complex_task() {
        use crate::agent::is_complex_task;

        assert!(is_complex_task("读取文件并且处理数据"));
        assert!(is_complex_task("首先做A，然后做B"));
        assert!(!is_complex_task("读取文件"));
    }

    #[test]
    fn test_get_ready_tasks() {
        let tasks = vec![
            Task {
                id: "1".to_string(),
                description: "任务1".to_string(),
                status: TaskStatus::Pending,
                dependencies: vec![],
                result: None,
            },
            Task {
                id: "2".to_string(),
                description: "任务2".to_string(),
                status: TaskStatus::Pending,
                dependencies: vec!["1".to_string()],
                result: None,
            },
        ];

        let ready = TaskDecomposer::get_ready_tasks(&tasks);
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0], "1");
    }

    #[test]
    fn test_get_ready_tasks_after_completion() {
        let tasks = vec![
            Task {
                id: "1".to_string(),
                description: "任务1".to_string(),
                status: TaskStatus::Completed,
                dependencies: vec![],
                result: Some("完成".to_string()),
            },
            Task {
                id: "2".to_string(),
                description: "任务2".to_string(),
                status: TaskStatus::Pending,
                dependencies: vec!["1".to_string()],
                result: None,
            },
        ];

        let ready = TaskDecomposer::get_ready_tasks(&tasks);
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0], "2");
    }
}
