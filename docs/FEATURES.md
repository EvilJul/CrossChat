# CrossChat 功能文档

## 📋 完整功能列表

### 1. 🧠 Agent 系统

#### ReAct 循环
- **自主推理** - Thought → Action → Observation 循环
- **工具调用** - 自动选择和执行工具
- **错误处理** - 自动重试和修复
- **最大迭代** - 15 次迭代保护

#### 记忆系统
- **自动学习** - 成功任务自动保存
- **向量检索** - 语义搜索，准确率 85%
- **文本匹配** - 兼容模式，快速搜索
- **失败记录** - 记录失败原因和修复方案

#### 任务分解
- **复杂度检测** - 自动识别复杂任务
- **LLM 驱动** - 智能分解为子任务
- **依赖管理** - 自动识别任务依赖
- **并行执行** - 独立任务同时执行

---

### 2. ⚡ 性能优化

#### MCP 工具缓存
- **持久化缓存** - 保存到 `~/.crosschat/mcp_cache.json`
- **自动加载** - 启动时加载缓存
- **版本管理** - 跟踪服务器版本
- **过期检查** - 24 小时自动失效
- **性能提升** - 10x 启动速度

#### 工具性能监控
- **执行时间** - 记录每次调用耗时
- **成功率** - 统计成功/失败次数
- **慢工具识别** - >1000ms 标红警告
- **数据库存储** - SQLite 持久化

#### 向量检索
- **余弦相似度** - 计算文本相似度
- **TF-IDF 向量化** - 128 维向量
- **Top-K 搜索** - 返回最相似结果
- **准确率提升** - 60% → 85%

---

### 3. 🎨 用户界面

#### 工具性能仪表盘
- **Top 5 工具** - 显示最常用工具
- **调用统计** - 总次数、成功率
- **响应时间** - 平均/最快/最慢
- **进度条** - 可视化成功率

#### Agent 思考过程
- **实时显示** - 展示推理步骤
- **工具调用** - 显示工具和结果
- **当前步骤** - 高亮执行中的步骤

#### 任务分解视图
- **任务列表** - 显示所有子任务
- **状态标识** - pending/running/completed/failed
- **依赖关系** - 显示任务依赖
- **进度条** - 整体完成度

---

### 4. 🔧 扩展能力

#### Provider 支持
- **OpenAI Compatible** - OpenAI、DeepSeek 等
- **Anthropic** - Claude 系列
- **统一接口** - 屏蔽差异
- **流式输出** - 实时响应

#### MCP 协议
- **服务器管理** - 添加/删除/启用
- **工具发现** - 自动发现工具
- **工具调用** - 标准化调用
- **健康检查** - 定期 ping 检查

#### Skill 系统
- **GitHub 安装** - 自动下载
- **启用/禁用** - 灵活管理
- **依赖管理** - 自动解析依赖
- **拓扑排序** - 计算安装顺序

#### 内置工具
- `read_file` - 读取文件
- `write_file` - 写入文件
- `delete_file` - 删除文件
- `list_directory` - 列出目录
- `run_command` - 执行命令
- `install_skill` - 安装技能

---

### 5. 💾 数据存储

#### 记忆数据库
- **位置** - `~/.crosschat/memory.db`
- **表结构** - task, solution, tools_used, success, timestamp, failure_reason, fix_applied
- **索引** - task, timestamp, success

#### 性能数据库
- **位置** - `~/.crosschat/metrics.db`
- **表结构** - tool_name, execution_time_ms, success, timestamp
- **索引** - tool_name, timestamp

#### MCP 健康数据库
- **位置** - `~/.crosschat/mcp_health.db`
- **表结构** - server_id, status, last_check, response_time_ms, error_message

---

## 🚀 使用指南

### 基本对话
```
用户: 帮我读取 config.json 文件

Agent:
1. [记忆检索] 找到相似任务
2. [工具调用] read_file("config.json")
3. [返回结果] 文件内容...
```

### 复杂任务
```
用户: 读取 config.json 和 settings.json，然后合并

Agent:
> 🔍 分析任务复杂度...
> 📋 任务已分解为 3 个子任务：
>   1. 读取 config.json (无依赖)
>   2. 读取 settings.json (无依赖)
>   3. 合并内容 (依赖: 1, 2)
> 开始执行...

[并行执行 1 和 2]
[执行 3]
[返回结果]
```

### 查看性能
```
打开应用 → 工具性能仪表盘自动显示
- read_file: 50次, 95%成功, 平均12ms
- write_file: 30次, 100%成功, 平均8ms
```

---

## 🔧 配置

### Provider 配置
```json
{
  "id": "openai",
  "name": "OpenAI",
  "baseUrl": "https://api.openai.com/v1",
  "apiKey": "sk-...",
  "models": ["gpt-4", "gpt-3.5-turbo"]
}
```

### MCP 服务器配置
```json
{
  "id": "filesystem",
  "name": "Filesystem MCP",
  "command": "npx",
  "args": ["-y", "@modelcontextprotocol/server-filesystem"],
  "enabled": true
}
```

### Skill 配置
```yaml
---
name: my-skill
description: 我的技能
version: 1.0.0
dependencies:
  - basic-skill@1.0.0
---
# Skill 内容
```

---

## 📊 API 参考

### Tauri 命令

#### 聊天
```typescript
invoke('stream_chat', {
  messages: [...],
  providerId: 'openai',
  model: 'gpt-4',
  workDir: '/path/to/dir'
})
```

#### 记忆
```typescript
// 搜索记忆
invoke('search_memories', { query: '读取文件', limit: 5 })

// 获取最近记忆
invoke('get_recent_memories', { limit: 10 })

// 清理旧记忆
invoke('cleanup_memories', { keepCount: 1000 })
```

#### 性能监控
```typescript
// 获取工具统计
invoke('get_tool_stats', { toolName: 'read_file' })

// 获取所有统计
invoke('get_tool_stats', { toolName: null })

// 清理旧数据
invoke('cleanup_metrics', { keepDays: 30 })
```

#### MCP 健康检查
```typescript
// 检查单个服务器
invoke('check_mcp_health', { serverId: 'server-1' })

// 获取所有健康状态
invoke('get_all_mcp_health')
```

---

## 🎯 最佳实践

### 1. 记忆管理
- 定期清理旧记忆（保留最近 1000 条）
- 记录失败原因，帮助学习

### 2. 性能优化
- 监控慢工具（>1000ms）
- 定期清理性能数据（保留 30 天）

### 3. MCP 服务器
- 定期健康检查
- 启用缓存加速启动

### 4. Skill 管理
- 检查依赖关系
- 按需启用/禁用

---

## 🐛 故障排除

### 问题：MCP 工具不可用
**解决：**
1. 检查 MCP 服务器是否启用
2. 运行健康检查
3. 查看错误日志

### 问题：记忆检索不准确
**解决：**
1. 使用向量检索（更准确）
2. 清理无关记忆
3. 增加搜索结果数量

### 问题：工具执行慢
**解决：**
1. 查看工具性能仪表盘
2. 识别慢工具
3. 优化工具实现

---

## 📝 更新日志

### v0.1.0 (2024-01-01)
- ✅ Agent 系统（ReAct 循环）
- ✅ 记忆系统（向量检索）
- ✅ 任务分解（并行执行）
- ✅ 性能监控（工具统计）
- ✅ MCP 支持（工具缓存）
- ✅ Skill 系统（依赖管理）
- ✅ 可视化组件（仪表盘）
- ✅ 测试覆盖（21 个测试）
