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
            // 先尝试 Anthropic 原生认证
            let url = format!("{}/models", base);
            let response = client
                .get(&url)
                .header("x-api-key", &api_key)
                .header("anthropic-version", "2023-06-01")
                .send()
                .await
                .map_err(|e| format!("连接失败: {}", e))?;

            if response.status().is_success() {
                let data: serde_json::Value = response
                    .json()
                    .await
                    .map_err(|e| format!("解析响应失败: {}", e))?;
                let models = parse_model_list(&data);
                if !models.is_empty() {
                    return Ok(models);
                }
                // 格式不标准，继续往下尝试 OpenAI 兼容认证
            }

            // Anthropic 认证失败或模型列表为空，自动降级尝试 OpenAI 兼容认证
            // 注意：这里不直接用上个 response 的 status，而是重新发起请求
            let url2 = format!("{}/models", base);
            let response2 = client
                .get(&url2)
                .header("Authorization", format!("Bearer {}", api_key))
                .send()
                .await
                .map_err(|e| format!("连接失败 (OpenAI 兼容降级): {}", e))?;

            if !response2.status().is_success() {
                let status = response2.status();
                let body = response2.text().await.unwrap_or_default();
                return Err(format!(
                    "HTTP {}: {}。\n\n已尝试 Anthropic 原生和 OpenAI 兼容两种认证方式均失败。请检查 Base URL 和 API Key 是否正确。",
                    status, body
                ));
            }

            let data: serde_json::Value = response2
                .json()
                .await
                .map_err(|e| format!("解析响应失败: {}", e))?;

            let mut models = parse_model_list(&data);
            models.retain(|id| {
                !id.contains("whisper")
                    && !id.contains("tts")
                    && !id.contains("dall-e")
                    && !id.contains("embedding")
                    && !id.contains("moderation")
            });

            if models.is_empty() {
                let preview = data.to_string();
                let preview = if preview.len() > 200 { &preview[..200] } else { &preview };
                return Err(format!(
                    "未找到可用模型，API 返回格式可能不标准。\n\n返回内容: {}\n\n请手动在模型列表中输入模型名称，或检查 Base URL 是否正确。",
                    preview
                ));
            }

            Ok(models)
        }
        _ => Err(format!("不支持的 Provider 类型: {}", provider_type)),
    }
}
