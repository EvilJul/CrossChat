#[cfg(test)]
mod tests {
    use crate::mcp::health::{HealthStatus, McpHealth, McpHealthManager};

    #[test]
    fn test_health_record_and_get() {
        let mgr = McpHealthManager::new().unwrap();

        let health = McpHealth {
            server_id: "test-server".to_string(),
            status: HealthStatus::Healthy,
            last_check: 1234567890,
            response_time_ms: 150,
            error_message: None,
        };

        mgr.record(&health).unwrap();

        let result = mgr.get_health("test-server").unwrap();
        assert!(result.is_some());

        let result = result.unwrap();
        assert_eq!(result.server_id, "test-server");
        assert_eq!(result.status, HealthStatus::Healthy);
        assert_eq!(result.response_time_ms, 150);
    }

    #[test]
    fn test_health_status_update() {
        let mgr = McpHealthManager::new().unwrap();

        // 首次记录为健康
        let health1 = McpHealth {
            server_id: "test-server-2".to_string(),
            status: HealthStatus::Healthy,
            last_check: 1234567890,
            response_time_ms: 100,
            error_message: None,
        };
        mgr.record(&health1).unwrap();

        // 更新为不可用
        let health2 = McpHealth {
            server_id: "test-server-2".to_string(),
            status: HealthStatus::Down,
            last_check: 1234567900,
            response_time_ms: 5000,
            error_message: Some("超时".to_string()),
        };
        mgr.record(&health2).unwrap();

        let result = mgr.get_health("test-server-2").unwrap().unwrap();
        assert_eq!(result.status, HealthStatus::Down);
        assert_eq!(result.error_message, Some("超时".to_string()));
    }
}
