/// 解析各种 Provider 的模型列表响应
/// 支持格式: OpenAI/Anthropic { data: [{ id }] }, { models: [...] }, [ { id } ], [ "str" ]
fn parse_model_list(value: &serde_json::Value) -> Vec<String> {
    // 格式 1: OpenAI/Anthropic 标准 — { data: [{ id: "gpt-4o" }, ...] }
    if let Some(arr) = value["data"].as_array() {
        let models: Vec<String> = arr
            .iter()
            .filter_map(|m| m["id"].as_str().map(|s| s.to_string()))
            .collect();
        if !models.is_empty() {
            return models;
        }
    }

    // 格式 2: { models: [{ id: "..." }, ...] }
    if let Some(arr) = value["models"].as_array() {
        // 子格式 2a: [{ id: "..." }, ...]
        let obj_models: Vec<String> = arr
            .iter()
            .filter_map(|m| m["id"].as_str().map(|s| s.to_string()))
            .collect();
        if !obj_models.is_empty() {
            return obj_models;
        }
        // 子格式 2b: ["model1", "model2", ...]
        let str_models: Vec<String> = arr
            .iter()
            .filter_map(|m| m.as_str().map(|s| s.to_string()))
            .collect();
        if !str_models.is_empty() {
            return str_models;
        }
    }

    // 格式 3: 根数组 [{ id: "..." }, ...]
    if let Some(arr) = value.as_array() {
        // 子格式 3a: [{ id: "..." }, ...]
        let obj_models: Vec<String> = arr
            .iter()
            .filter_map(|m| m["id"].as_str().map(|s| s.to_string()))
            .collect();
        if !obj_models.is_empty() {
            return obj_models;
        }
        // 子格式 3b: ["model1", "model2", ...]
        let str_models: Vec<String> = arr
            .iter()
            .filter_map(|m| m.as_str().map(|s| s.to_string()))
            .collect();
        if !str_models.is_empty() {
            return str_models;
        }
    }

    // 格式 4: { "result": [...] } 或 { "body": { "data": [...] } } — 阿里云 DashScope 等
    if let Some(arr) = value["result"].as_array() {
        let models: Vec<String> = arr
            .iter()
            .filter_map(|m| m["id"].as_str().map(|s| s.to_string()))
            .collect();
        if !models.is_empty() {
            return models;
        }
    }

    // 格式 5: { "object": "list", "data": [...] } — 某些兼容格式
    if let Some(arr) = value["body"]["data"].as_array() {
        let models: Vec<String> = arr
            .iter()
            .filter_map(|m| m["id"].as_str().map(|s| s.to_string()))
            .collect();
        if !models.is_empty() {
            return models;
        }
    }

    Vec::new()
}

/// 测试 Provider 连接，拉取可用模型列表
#[tauri::command]
pub async fn test_provider_connection(
    api_base: String,
    api_key: String,
    provider_type: String,
) -> Result<Vec<String>, String> {
    let client = reqwest::Client::new();
    let base = api_base.trim_end_matches('/').to_string();

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

            let mut models = parse_model_list(&data);

            // 过滤非聊天模型
            models.retain(|id| {
                !id.contains("whisper")
                    && !id.contains("tts")
                    && !id.contains("dall-e")
                    && !id.contains("embedding")
                    && !id.contains("moderation")
            });

            if models.is_empty() {
                // 返回原始响应摘要，方便用户调试
                let preview = data.to_string();
                let preview = if preview.len() > 200 { &preview[..200] } else { &preview };
                return Err(format!(
                    "未找到可用模型，API 返回格式可能不标准。\n\n返回内容: {}\n\n请手动在模型列表中输入模型名称，或检查 Base URL 是否正确。",
                    preview
                ));
            }

            Ok(models)
        }
        "anthropic" => {
            // Anthropic API 不提供 /models 端点
            // 直接返回预设的模型列表
            eprintln!("[provider_cmd] Anthropic 不支持模型列表 API，返回预设模型");
            
            // 验证 API Key 是否有效（通过发送一个简单的请求）
            let url = format!("{}/messages", base);
            let test_request = serde_json::json!({
                "model": "claude-sonnet-4-6",
                "max_tokens": 1,
                "messages": [{
                    "role": "user",
                    "content": "test"
                }]
            });
            
            let response = client
                .post(&url)
                .header("x-api-key", &api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&test_request)
                .send()
                .await
                .map_err(|e| format!("连接失败: {}", e))?;

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                
                // 检查是否是认证错误
                if status == 401 {
                    return Err("API Key 无效。请检查您的 Anthropic API Key 是否正确。".to_string());
                }
                
                return Err(format!(
                    "HTTP {}: {}。请检查 API Key 和网络连接。",
                    status, body
                ));
            }

            // API Key 有效，返回预设模型列表
            Ok(vec![
                "claude-sonnet-4-6".to_string(),
                "claude-opus-4-6".to_string(),
                "claude-haiku-4-5".to_string(),
                "claude-3-5-sonnet-20241022".to_string(),
                "claude-3-5-haiku-20241022".to_string(),
                "claude-3-opus-20240229".to_string(),
            ])
        }
        _ => Err(format!("不支持的 Provider 类型: {}", provider_type)),
    }
}
