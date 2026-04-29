/// ReAct (Reasoning + Acting) 模式实现
///
/// ReAct 循环：
/// 1. Thought（思考）：分析当前状态，决定下一步行动
/// 2. Action（行动）：选择并执行工具
/// 3. Observation（观察）：观察工具执行结果
/// 4. 重复直到问题解决或达到最大迭代次数

use crate::agent::{Agent, AgentConfig, AgentResult};
use crate::providers::types::{ContentBlock, MessageRole, StreamChunk, UnifiedMessage};
use crate::providers::LlmProvider;
use crate::commands::chat::StreamSender;

/// 使用 ReAct 模式运行 Agent（带流式输出）
pub async fn run_react_agent<T: StreamSender + Clone>(
    provider: &dyn LlmProvider,
    mut messages: Vec<UnifiedMessage>,
    model: &str,
    channel: T,
    work_dir: &str,
    tool_registry: crate::agent::tool_registry::ToolRegistry,
) -> Result<AgentResult, String> {
    // 注入 ReAct 系统提示
    let react_prompt = UnifiedMessage {
        role: MessageRole::User,
        content: vec![ContentBlock::Text {
            text: r#"[ReAct Agent 模式]

你是一个使用 ReAct（Reasoning + Acting）模式的自主智能体。

## 工作流程

每次迭代遵循以下步骤：

1. **Thought（思考）**：
   - 分析当前状态和已有信息
   - 思考下一步应该做什么
   - 判断是否已经可以回答用户问题

2. **Action（行动）**：
   - 如果需要更多信息，选择合适的工具执行
   - 如果已有足够信息，给出最终答案

3. **Observation（观察）**：
   - 仔细分析工具执行结果
   - 根据结果调整策略

## 核心原则

- **目标驱动**：始终记住用户的原始需求
- **逐步推进**：每次只做一件事，不要贪多
- **错误自愈**：工具失败时分析原因并尝试其他方案
- **主动探索**：不确定时用工具验证假设
- **及时终止**：完成任务后立即给出答案，不要过度执行

## 错误处理策略

- 文件不存在 → 用 list_directory 查找
- 命令失败 → 分析错误信息，调整参数重试
- 权限不足 → 换路径或方法
- 模块缺失 → Python 沙盒会自动安装

现在开始处理用户任务。
"#.to_string(),
        }],
        tool_calls: None,
        tool_call_id: None,
        reasoning_content: None,
    };
    messages.insert(0, react_prompt);

    // 创建 Agent
    let config = AgentConfig {
        max_iterations: 15,
        work_dir: work_dir.to_string(),
        enable_self_healing: true,
        enable_task_decomposition: true,
    };
    let mut agent = Agent::new(config, tool_registry);

    // 运行 Agent 并实时流式输出
    agent.run_with_stream(provider, messages, model, channel).await
}
