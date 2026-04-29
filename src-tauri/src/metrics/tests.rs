#[cfg(test)]
mod tests {
    use crate::metrics::{MetricsManager, ToolMetrics};

    #[test]
    fn test_metrics_record_and_get() {
        let mgr = MetricsManager::new().unwrap();

        let metrics = ToolMetrics {
            tool_name: "test_tool".to_string(),
            execution_time_ms: 100,
            success: true,
            timestamp: 1234567890,
        };

        mgr.record(&metrics).unwrap();

        let stats = mgr.get_stats("test_tool").unwrap();
        assert!(stats.is_some());

        let stats = stats.unwrap();
        assert_eq!(stats.tool_name, "test_tool");
        assert!(stats.total_calls >= 1);
    }

    #[test]
    fn test_metrics_multiple_records() {
        let mgr = MetricsManager::new().unwrap();

        // 记录多次
        for i in 0..5 {
            let metrics = ToolMetrics {
                tool_name: "multi_tool".to_string(),
                execution_time_ms: 100 + i * 10,
                success: i % 2 == 0,
                timestamp: 1234567890 + i,
            };
            mgr.record(&metrics).unwrap();
        }

        let stats = mgr.get_stats("multi_tool").unwrap();
        assert!(stats.is_some());
    }

    #[test]
    fn test_get_all_stats() {
        let mgr = MetricsManager::new().unwrap();
        let all_stats = mgr.get_all_stats().unwrap();
        assert!(all_stats.len() >= 0);
    }

    #[test]
    fn test_cleanup() {
        let mgr = MetricsManager::new().unwrap();
        let deleted = mgr.cleanup(30).unwrap();
        assert!(deleted >= 0);
    }
}
