use crate::providers::types::{UnifiedMessage, ContentBlock, MessageRole};
use sha2::{Sha256, Digest};
use std::fmt::Write;

pub struct ContextCompactor;

impl ContextCompactor {
    /// 估算当前消息队列的 token 数 (1 token 约等 3.5 字节)
    pub fn estimate_tokens(messages: &[UnifiedMessage]) -> usize {
        let mut total_chars = 0;
        for msg in messages {
            for block in &msg.content {
                if let ContentBlock::Text { text } = block {
                    total_chars += text.len();
                }
            }
            if let Some(reasoning) = &msg.reasoning_content {
                total_chars += reasoning.len();
            }
            if let Some(tool_calls) = &msg.tool_calls {
                for call in tool_calls {
                    total_chars += call.name.len();
                    total_chars += call.arguments.to_string().len();
                }
            }
        }
        (total_chars as f64 / 3.5) as usize + messages.len() * 20
    }

    /// 上下文无损/折叠压缩
    pub fn compact(
        messages: Vec<UnifiedMessage>,
        model: &str,
    ) -> Vec<UnifiedMessage> {
        let model_lower = model.to_lowercase();
        let is_deepseek = model_lower.contains("deepseek");

        // 阈值匹配
        let (soft_threshold, hard_threshold) = if is_deepseek {
            (980_000, 990_000)
        } else {
            (16_000, 24_000)
        };

        let total_tokens = Self::estimate_tokens(&messages);
        if total_tokens < soft_threshold || messages.len() < 8 {
            return messages; // 未达到阈值或消息数量过少，不触发压缩
        }

        // 决定压缩模式与滑动保留窗口
        let (mode, keep_recent) = if total_tokens >= hard_threshold {
            ("force", 1)
        } else if total_tokens >= soft_threshold + (0.6 * (hard_threshold - soft_threshold) as f64) as usize {
            ("aggressive", 2)
        } else {
            ("normal", 4)
        };

        eprintln!("[Compactor] Triggered compaction mode: {}, keep_recent: {}, tokens: {}", mode, keep_recent, total_tokens);

        // 区分冻结消息（如系统指令、工作区说明、已启用的技能）与动态会话历史
        let mut frozen_messages = Vec::new();
        let mut history_messages = Vec::new();

        for msg in messages {
            let mut is_frozen = false;
            if let MessageRole::User = msg.role {
                if let Some(ContentBlock::Text { text }) = msg.content.first() {
                    if text.starts_with("[系统指令") 
                        || text.starts_with("[当前工作目录") 
                        || text.starts_with("[已安装的 Skills") 
                        || text.contains("Active Skill:") 
                        || text.contains("Skill Pin:")
                        || text.contains("Pinned Skill:")
                    {
                        is_frozen = true;
                    }
                }
            }
            if is_frozen {
                frozen_messages.push(msg);
            } else {
                history_messages.push(msg);
            }
        }

        if history_messages.len() <= keep_recent {
            let mut result = frozen_messages;
            result.extend(history_messages);
            return result;
        }

        // 历史会话拆分：需要折叠的消息 vs 最新保留的消息
        let split_idx = history_messages.len() - keep_recent;
        let (fold_messages, recent_messages) = history_messages.split_at(split_idx);
        let fold_messages = fold_messages.to_vec();
        let recent_messages = recent_messages.to_vec();

        // 编译折叠摘要与约束提取
        let mut pinned_constraints = String::new();
        let mut active_skills_pins = String::new();
        let mut folded_log = String::new();

        for msg in &fold_messages {
            for block in &msg.content {
                if let ContentBlock::Text { text } = block {
                    // 保留硬性系统约束
                    if text.contains("本地优先") || text.contains("必须") || text.contains("禁止") {
                        pinned_constraints.push_str(text);
                        pinned_constraints.push('\n');
                    }
                    // 保留活跃技能特征
                    for line in text.lines() {
                        let trimmed = line.trim();
                        if trimmed.starts_with("Active Skill:") 
                            || trimmed.starts_with("Skill Pin:")
                            || trimmed.starts_with("Pinned Skill:")
                        {
                            active_skills_pins.push_str(trimmed);
                            active_skills_pins.push('\n');
                        }
                    }
                }
            }

            // 单行缩影编译 (User, Assistant, Tool call)
            let role_str = match msg.role {
                MessageRole::User => "User",
                MessageRole::Assistant => "Assistant",
                MessageRole::Tool => "Tool result",
            };

            let mut body = String::new();
            for block in &msg.content {
                if let ContentBlock::Text { text } = block {
                    body.push_str(text);
                }
            }

            let summary_line = if body.len() > 80 {
                format!("{}...", &body[..80].replace('\n', " "))
            } else {
                body.replace('\n', " ")
            };

            if let Some(tool_calls) = &msg.tool_calls {
                for tc in tool_calls {
                    let _ = writeln!(folded_log, "- Assistant Tool call: {} with args {}", tc.name, tc.arguments);
                }
            } else {
                let _ = writeln!(folded_log, "- {}: {}", role_str, summary_line.trim());
            }
        }

        // 限制字符长度预算（最少 1200，最多 12000 字符）
        let max_chars = std::cmp::min(12000, std::cmp::max(1200, total_tokens / 10));
        let trimmed_log = if folded_log.len() > max_chars {
            format!("... (folded historical logs) ...\n{}", &folded_log[folded_log.len() - max_chars..])
        } else {
            folded_log
        };

        // 计算被折叠历史链的 SHA256 完整性指纹
        let mut hasher = Sha256::new();
        for msg in &fold_messages {
            hasher.update(format!("{:?}", msg).as_bytes());
        }
        let hash_result = hasher.finalize();
        let digest_marker = format!("{:x}", hash_result);

        // 构建合并的 compaction 消息项
        let mut compaction_body = String::from("[上下文折叠摘要 — 历史消息被压缩]\n");
        if !pinned_constraints.is_empty() {
            compaction_body.push_str("【系统级指令约束】\n");
            compaction_body.push_str(&pinned_constraints);
            compaction_body.push('\n');
        }
        if !active_skills_pins.is_empty() {
            compaction_body.push_str("【活跃技能指令特征】\n");
            compaction_body.push_str(&active_skills_pins);
            compaction_body.push('\n');
        }
        compaction_body.push_str("【折叠历史事件线摘要】\n");
        compaction_body.push_str(&trimmed_log);
        compaction_body.push_str(&format!("\nCompaction digest marker: [mcp_digest_{}]\n", &digest_marker[..16]));

        let compaction_message = UnifiedMessage {
            role: MessageRole::User, // 用 user 角色传递以保证最大 API 兼容性
            content: vec![ContentBlock::Text { text: compaction_body }],
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        };

        // 重组消息：[冻结前置指令, 折叠的压缩消息项, 最新历史对话]
        let mut result = frozen_messages;
        result.push(compaction_message);
        result.extend(recent_messages);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_compactor_estimate_and_compact() {
        let mut messages = vec![
            UnifiedMessage {
                role: MessageRole::User,
                content: vec![ContentBlock::Text { text: "[系统指令 — 遵循以下规则] Always be helpful.".to_string() }],
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            },
            UnifiedMessage {
                role: MessageRole::User,
                content: vec![ContentBlock::Text { text: "[当前工作目录] /Volumes/DATA".to_string() }],
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            },
        ];

        // Add 10 historical conversation messages with large text to exceed 16k tokens
        for i in 0..10 {
            messages.push(UnifiedMessage {
                role: MessageRole::User,
                content: vec![ContentBlock::Text { text: format!("Dialogue message {} with very large text payload: {}", i, "a".repeat(6000)) }],
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            });
            messages.push(UnifiedMessage {
                role: MessageRole::Assistant,
                content: vec![ContentBlock::Text { text: format!("Response {} with very large text payload: {}", i, "b".repeat(6000)) }],
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            });
        }

        let estimated = ContextCompactor::estimate_tokens(&messages);
        assert!(estimated > 16000, "Estimated tokens should exceed 16000, got {}", estimated);

        let compacted = ContextCompactor::compact(messages, "gpt-4");

        // Verify that the frozen system instruction & work directory messages are preserved
        assert!(matches!(compacted[0].role, MessageRole::User));
        if let ContentBlock::Text { text } = &compacted[0].content[0] {
            assert!(text.contains("[系统指令"));
        } else {
            panic!("First message should be text");
        }

        assert!(matches!(compacted[1].role, MessageRole::User));
        if let ContentBlock::Text { text } = &compacted[1].content[0] {
            assert!(text.contains("[当前工作目录"));
        } else {
            panic!("Second message should be text");
        }

        // Verify compaction message contains the digest marker
        let has_compaction_summary = compacted.iter().any(|msg| {
            if !msg.content.is_empty() {
                if let ContentBlock::Text { text } = &msg.content[0] {
                    text.contains("[上下文折叠摘要 — 历史消息被压缩]") && text.contains("mcp_digest_")
                } else {
                    false
                }
            } else {
                false
            }
        });
        assert!(has_compaction_summary, "Should contain compaction summary with digest");

        // Verify recent messages are kept (compacted list length should be much smaller than original)
        assert!(compacted.len() < 10, "Compacted message list length should be reduced, got {}", compacted.len());
    }
}
