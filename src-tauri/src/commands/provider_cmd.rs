// (serde types not needed directly in this file)

/// 测试 Provider 连接，拉取可用模型列表
#[tauri::command]
pub async fn test_provider_connection(
    api_base: String,
    api_key: String,
    provider_type: String,
) -> Result<Vec<String>, String> {
    let client = reqwest::Client::new();
    let base = api_base.trim_end_matches('/').to_string();

    // 根据 Provider 类型选择不同的模型列表端点
    match provider_type.as_str() {
        "openai-compat" => {
            let url = format!("{}/models", base);
            let response = client
                .get(&url)
                .header("Authorization", format!("Bearer {}", api_key))
                .send()
                .await
                .map_err(|e| format!("连接失败: {}", e))?;

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(format!("HTTP {}: {}", status, body));
            }

            let data: serde_json::Value = response
                .json()
                .await
                .map_err(|e| format!("解析响应失败: {}", e))?;

            // OpenAI 格式: { data: [{ id: "gpt-4o" }, ...] }
            let models: Vec<String> = data["data"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|m| m["id"].as_str().map(|s| s.to_string()))
                .filter(|id| {
                    // 过滤掉非聊天模型（如 embedding、whisper、davinci 等）
                    !id.contains("whisper")
                        && !id.contains("tts")
                        && !id.contains("dall-e")
                        && !id.contains("embedding")
                        && !id.contains("moderation")
                })
                .collect();

            Ok(models)
        }
        "anthropic" => {
            // Anthropic 没有公开的模型列表端点，返回预置列表
            Ok(vec![
                "claude-opus-4-6".into(),
                "claude-sonnet-4-6".into(),
                "claude-haiku-4-5".into(),
            ])
        }
        _ => Err(format!("不支持的 Provider 类型: {}", provider_type)),
    }
}
