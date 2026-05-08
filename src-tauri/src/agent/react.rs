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

## 🌐 重要：你的能力

**你拥有强大的工具调用能力！** 当需要执行操作时，使用提供的工具函数，而不是编写代码。

你拥有以下能力：
- ✅ **网络搜索**：使用 brave_search、fetch 等 MCP 工具进行实时网络搜索
- ✅ **文件操作**：使用 read_file、write_file 等工具读取、写入、修改文件
- ✅ **命令执行**：使用 run_command 工具运行 shell 命令
- ✅ **代码执行**：使用 run_python 工具运行 Python 代码
- ✅ **API 调用**：通过 MCP 工具调用各种 API

**重要提示**：
- 当你需要执行操作时，直接调用相应的工具函数
- 不要输出工具调用的文本格式（如 [TOOL_CALL]{...}）
- 不要编写 Python 代码来执行网络请求，使用 MCP 搜索工具
- 系统会自动处理工具调用并返回结果

## 工作流程

每次迭代遵循以下步骤：

1. **Thought（思考）**：
   - 分析当前状态和已有信息
   - 思考下一步应该做什么
   - 判断是否已经可以回答用户问题
   - **如果需要网络信息，选择合适的 MCP 搜索工具（不要用 run_python 写爬虫）**

2. **Action（行动）**：
   - 如果需要更多信息，调用合适的工具
   - **网络搜索：优先使用 brave_search、fetch 等 MCP 工具**
   - **不要用 run_python 执行 requests 库来搜索，使用专门的搜索工具**
   - 如果已有足够信息，给出最终答案

3. **Observation（观察）**：
   - 仔细分析工具执行结果
   - 根据结果调整策略

## 核心原则

- **目标驱动**：始终记住用户的原始需求
- **正确使用工具**：使用专门的工具而不是编写代码来实现功能
- **逐步推进**：每次只做一件事，不要贪多
- **错误自愈**：工具失败时分析原因并尝试其他方案
- **主动探索**：不确定时用工具验证假设
- **及时终止**：完成任务后立即给出答案，不要过度执行

## 工具使用指南

### 网络搜索
- ✅ 使用 `brave_search` 或 `fetch` 等 MCP 工具
- ❌ 不要使用 `run_python` 执行 `requests.get()`

### 文件操作
- ✅ 使用 `read_file`、`write_file` 等工具
- ❌ 不要使用 `run_python` 执行 `open()` 和 `write()`

### 命令执行
- ✅ 使用 `run_command` 工具
- ❌ 不要使用 `run_python` 执行 `os.system()`

## 错误处理策略

- 文件不存在 → 用 list_directory 查找
- 命令失败 → 分析错误信息，调整参数重试
- 权限不足 → 换路径或方法
- 模块缺失 → Python 沙盒会自动安装
- **工具不可用 → 尝试其他类似工具或告知用户具体原因**

现在开始处理用户任务。记住：直接调用工具函数，不要输出文本格式的工具调用！
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
