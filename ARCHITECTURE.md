# CrossChat 代码架构文档

## 📊 项目概览

**技术栈：** Tauri + React + Rust  
**总代码量：** ~4,806 行 Rust + 前端代码  
**架构模式：** 分层架构 + Agent 系统

---

## 🏗️ 整体架构

```
┌─────────────────────────────────────────────────────┐
│                   前端层 (React)                      │
│  - UI 组件 (Chat, Settings, Workspace)              │
│  - 状态管理 (Zustand)                                │
│  - Tauri Bridge                                      │
└─────────────────────────────────────────────────────┘
                         ↕ IPC
┌─────────────────────────────────────────────────────┐
│                 Tauri 命令层 (Rust)                   │
│  - stream_chat, test_provider_connection            │
│  - file_ops, session_cmd, mcp_cmd, memory_cmd       │
└─────────────────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────┐
│                  Agent 核心层                         │
│  ┌─────────────┬──────────────┬──────────────┐      │
│  │ ReAct 引擎  │ 任务分解器    │ 工具注册表    │      │
│  │ (循环推理)  │ (并行执行)    │ (工具管理)    │      │
│  └─────────────┴──────────────┴──────────────┘      │
│  ┌─────────────┬──────────────┬──────────────┐      │
│  │ 记忆系统    │ 上下文管理    │ 自愈机制      │      │
│  │ (SQLite)    │ (压缩/检索)   │ (错误修复)    │      │
│  └─────────────┴──────────────┴──────────────┘      │
└─────────────────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────┐
│                  能力扩展层                           │
│  ┌──────────┬──────────┬──────────┬──────────┐      │
│  │ Providers│  Tools   │  Skills  │   MCP    │      │
│  │ (LLM)    │ (工具)   │ (技能)   │ (协议)   │      │
│  └──────────┴──────────┴──────────┴──────────┘      │
└─────────────────────────────────────────────────────┘
```

---

## 📁 目录结构

### 后端 (Rust)

```
src-tauri/src/
├── agent/                    # Agent 核心系统 (488 行)
│   ├── mod.rs               # Agent 主逻辑、ReAct 循环
│   ├── react.rs             # ReAct 引擎入口
│   ├── task_decomposer.rs   # 任务分解器 (116 行)
│   └── tool_registry.rs     # 工具注册表
│
├── commands/                 # Tauri 命令层
│   ├── chat.rs              # 聊天命令 (402 行)
│   ├── memory_cmd.rs        # 记忆管理命令
│   ├── mcp_cmd.rs           # MCP 服务器管理
│   ├── skills_cmd.rs        # Skill 管理 (125 行)
│   ├── session_cmd.rs       # 会话管理 (142 行)
│   ├── file_ops.rs          # 文件操作 (97 行)
│   ├── provider_cmd.rs      # Provider 测试 (203 行)
│   ├── agent_cmd.rs         # Agent 配置
│   └── checkpoint_cmd.rs    # 检查点管理
│
├── providers/               # LLM Provider 层
│   ├── mod.rs              # Provider 接口 (89 行)
│   ├── types.rs            # 统一类型定义 (109 行)
│   ├── openai_compat.rs    # OpenAI 兼容 (450 行)
│   └── anthropic.rs        # Anthropic (514 行)
│
├── tools/                   # 工具系统
│   ├── mod.rs              # 工具定义 (141 行)
│   ├── file_tools.rs       # 文件工具 (131 行)
│   ├── shell_tool.rs       # Shell 工具 (255 行)
│   └── python_sandbox.rs   # Python 沙盒 (169 行)
│
├── memory/                  # 记忆系统
│   └── mod.rs              # SQLite 记忆管理 (152 行)
│
├── mcp/                     # MCP 协议支持
│   ├── mod.rs              # MCP 管理器 (136 行)
│   └── server.rs           # MCP 服务器通信 (209 行)
│
├── skills/                  # Skill 系统
│   └── mod.rs              # Skill 管理器 (325 行)
│
├── security/                # 安全模块
│   ├── mod.rs
│   └── sandbox.rs          # 沙盒安全策略
│
├── streaming/               # 流式处理
│   ├── mod.rs
│   └── sse_parser.rs       # SSE 解析器
│
├── lib.rs                   # 库入口
└── main.rs                  # 主程序入口
```

### 前端 (React + TypeScript)

```
src/
├── components/
│   ├── chat/               # 聊天组件
│   │   ├── ChatView.tsx
│   │   ├── ChatInput.tsx
│   │   ├── MessageBubble.tsx
│   │   ├── ThinkingBubble.tsx
│   │   ├── WorkspaceSidebar.tsx
│   │   ├── FilePreviewPanel.tsx
│   │   └── ToolCallBadge.tsx
│   └── settings/           # 设置组件
│       ├── SettingsDialog.tsx
│       └── ProviderTab.tsx
│
├── stores/                 # 状态管理 (Zustand)
│   ├── chatStore.ts
│   └── workspaceStore.ts
│
├── hooks/                  # React Hooks
│   └── useChat.ts
│
└── lib/                    # 工具库
    ├── tauri-bridge.ts     # Tauri 桥接
    ├── providers.ts        # Provider 配置
    └── builtinCommands.ts  # 内置命令
```

---

## 🔑 核心模块详解

### 1. Agent 系统 (488 行)

**职责：** 智能任务执行引擎

**核心组件：**
- `Agent` - 主控制器
- `AgentConfig` - 配置管理
- `AgentResult` - 执行结果
- `AgentStep` - 执行步骤记录

**关键方法：**
```rust
run_with_stream()           // 带流式输出的执行
run_standard()              // 标准执行流程
run_with_decomposed_tasks() // 任务分解执行
```

**特性：**
- ✅ ReAct 循环（Thought → Action → Observation）
- ✅ 记忆检索和保存
- ✅ 任务分解和并行执行
- ✅ 实时流式输出

---

### 2. 任务分解器 (116 行)

**职责：** 将复杂任务拆分为子任务

**核心结构：**
```rust
Task {
    id: String,
    description: String,
    status: TaskStatus,
    dependencies: Vec<String>,
    result: Option<String>,
}
```

**关键方法：**
```rust
decompose()        // LLM 驱动的任务分解
get_ready_tasks()  // 获取可执行任务
```

---

### 3. 记忆系统 (152 行)

**职责：** 持久化存储和检索历史任务

**数据库：** SQLite (`~/.crosschat/memory.db`)

**核心方法：**
```rust
save()        // 保存记忆
search()      // 搜索相似任务
get_recent()  // 获取最近记忆
cleanup()     // 清理旧记忆
```

**表结构：**
```sql
memories (
    id INTEGER PRIMARY KEY,
    task TEXT,
    solution TEXT,
    tools_used TEXT,
    success INTEGER,
    timestamp INTEGER
)
```

---

### 4. Provider 层 (1,062 行)

**职责：** 统一 LLM 接口

**支持的 Provider：**
- OpenAI Compatible (450 行)
- Anthropic (514 行)

**统一接口：**
```rust
trait LlmProvider {
    async fn stream_chat()
    async fn chat_sync()
    async fn chat_sync_with_tools()
}
```

**类型系统：**
- `UnifiedMessage` - 统一消息格式
- `ContentBlock` - 内容块（文本/图片）
- `ToolCall` - 工具调用
- `StreamChunk` - 流式响应块

---

### 5. 工具系统 (697 行)

**内置工具：**
1. `read_file` - 读取文件
2. `write_file` - 写入文件
3. `delete_file` - 删除文件
4. `list_directory` - 列出目录
5. `run_command` - 执行命令
6. `install_skill` - 安装技能

**特殊工具：**
- **Python 沙盒** (169 行) - 隔离的 Python 环境
- **Shell 工具** (255 行) - 安全的命令执行

**安全机制：**
- 危险命令检测
- 路径沙盒限制
- 超时控制
- 自动模块安装

---

### 6. MCP 系统 (345 行)

**职责：** Model Context Protocol 支持

**核心功能：**
- 服务器管理（添加/删除/启用）
- 工具发现（tools/list）
- 工具调用（tools/call）
- 工具缓存

**通信协议：**
- JSON-RPC 2.0
- stdio 通信
- 自动握手和初始化

---

### 7. Skill 系统 (325 行)

**职责：** 可扩展能力系统

**Skill 格式：**
```markdown
---
name: skill-name
description: 描述
version: 1.0.0
---

# Skill 内容
...
```

**功能：**
- GitHub 自动安装
- 启用/禁用管理
- 内容注入到 AI 上下文
- 内置 excel-automation skill

---

## 📈 代码质量指标

### 模块化程度
- ✅ **高内聚**：每个模块职责单一
- ✅ **低耦合**：通过接口通信
- ✅ **可扩展**：易于添加新功能

### 代码复杂度
| 模块 | 行数 | 复杂度 |
|------|------|--------|
| Agent 核心 | 488 | 中等 |
| Anthropic Provider | 514 | 中等 |
| OpenAI Provider | 450 | 中等 |
| Chat 命令 | 402 | 较高 |
| Skill 管理 | 325 | 中等 |

### 测试覆盖
- ⚠️ **单元测试**：缺失
- ⚠️ **集成测试**：缺失
- ✅ **手动测试**：通过

---

## 🔄 数据流

### 聊天流程
```
用户输入
  ↓
前端 ChatInput
  ↓ invoke('stream_chat')
Tauri 命令层
  ↓
run_agent_loop()
  ↓
Agent.run_with_stream()
  ↓ [检测复杂度]
任务分解？
  ├─ 是 → 分解 → 并行执行子任务
  └─ 否 → 标准执行
       ↓
    记忆检索
       ↓
    ReAct 循环
       ↓
    工具执行
       ↓
    保存记忆
       ↓
StreamChunk → 前端实时显示
```

### 工具调用流程
```
LLM 返回 tool_calls
  ↓
ToolRegistry.execute()
  ↓
内置工具？
  ├─ 是 → tools::execute_tool()
  └─ 否 → execute_mcp_tool()
       ↓
    MCP 服务器
       ↓
ToolResult → 添加到上下文
```

---

## 🎯 架构优势

### 1. 分层清晰
- 前端层、命令层、业务层、能力层分离
- 每层职责明确，易于维护

### 2. 可扩展性强
- Provider 接口：易于添加新 LLM
- 工具系统：易于添加新工具
- Skill 系统：用户可自定义能力
- MCP 协议：标准化扩展

### 3. 智能化
- ReAct 循环：自主推理
- 记忆系统：从历史学习
- 任务分解：并行执行
- 自愈机制：错误恢复

### 4. 安全性
- Python 沙盒：隔离执行
- 路径限制：防止越界
- 危险命令检测：防止破坏
- 超时控制：防止卡死

---

## ⚠️ 待改进点

### 1. 测试覆盖
- 缺少单元测试
- 缺少集成测试
- 建议：添加 `#[cfg(test)]` 模块

### 2. 错误处理
- 部分错误处理较简单
- 建议：使用 `thiserror` 统一错误类型

### 3. 性能优化
- 记忆检索使用简单文本匹配
- 建议：引入向量检索（embedding）

### 4. 文档
- 代码注释较少
- 建议：添加 rustdoc 注释

---

## 📊 总结

**架构评分：** ⭐⭐⭐⭐☆ (4/5)

**优点：**
- ✅ 模块化设计优秀
- ✅ Agent 系统先进
- ✅ 扩展性强
- ✅ 功能完整

**改进方向：**
- 添加测试覆盖
- 优化错误处理
- 引入向量检索
- 完善文档

**总体评价：** 这是一个设计良好、功能完整的 AI Agent 应用，具备学习、规划、自愈等高级能力。代码结构清晰，易于维护和扩展。
