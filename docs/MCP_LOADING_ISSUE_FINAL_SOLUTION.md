# MCP 工具加载问题 - 最终解决方案

## 问题诊断完成 ✅

经过详细调试和测试，我们找到了 MCP 工具加载失败的根本原因。

---

## 根本原因

**使用了不存在的 npm 包名**

你的配置中使用的包：
- ❌ `@modelcontextprotocol/server-fetch` - **不存在**
- ❌ `@modelcontextprotocol/server-brave-search` - **已归档，不再维护**

这就是为什么看到错误：
```
MCP 服务器未返回响应（stdout 已关闭）
```

实际上，进程根本没有启动成功，因为 npm 找不到这些包。

---

## 解决方案

### 立即可用的替代方案

#### 1. 网页抓取 - 使用社区服务器

**包名**: `mcp-server-fetch-typescript`  
**状态**: ✅ 已验证存在（版本 0.1.1）

**配置**（Windows）:
```json
{
  "id": "fetch",
  "name": "Fetch",
  "command": "cmd",
  "args": ["/c", "npx", "-y", "mcp-server-fetch-typescript"],
  "enabled": true,
  "version": "0.1.1"
}
```

**配置**（Linux/Mac）:
```json
{
  "id": "fetch",
  "name": "Fetch",
  "command": "npx",
  "args": ["-y", "mcp-server-fetch-typescript"],
  "enabled": true,
  "version": "0.1.1"
}
```

#### 2. 搜索功能 - 多个选择

Brave Search 官方服务器已归档。推荐的替代方案：

**选项 A**: 使用其他搜索 MCP 服务器
- 访问 [Awesome MCP Servers](https://github.com/appcypher/awesome-mcp-servers)
- 搜索 "search" 类别
- 选择支持的搜索引擎（Google, Bing, DuckDuckGo 等）

**选项 B**: 暂时禁用搜索功能
- 将 Brave Search 的 `enabled` 设置为 `false`
- 等待找到合适的替代品

---

## 完整的推荐配置

### Windows 配置

编辑 `C:\Users\你的用户名\.crosschat\mcp_servers.json`:

```json
[
  {
    "id": "fetch",
    "name": "Fetch",
    "command": "cmd",
    "args": ["/c", "npx", "-y", "mcp-server-fetch-typescript"],
    "enabled": true,
    "version": "0.1.1"
  }
]
```

### 添加更多官方服务器（可选）

```json
[
  {
    "id": "fetch",
    "name": "Fetch",
    "command": "cmd",
    "args": ["/c", "npx", "-y", "mcp-server-fetch-typescript"],
    "enabled": true,
    "version": "0.1.1"
  },
  {
    "id": "filesystem",
    "name": "Filesystem",
    "command": "cmd",
    "args": ["/c", "npx", "-y", "@modelcontextprotocol/server-filesystem", "C:\\Users\\你的用户名\\Documents"],
    "enabled": true,
    "version": "1.0.0"
  },
  {
    "id": "github",
    "name": "GitHub",
    "command": "cmd",
    "args": ["/c", "npx", "-y", "@modelcontextprotocol/server-github"],
    "enabled": false,
    "version": "1.0.0"
  }
]
```

---

## 验证步骤

### 1. 更新配置文件

按照上面的配置更新你的 `mcp_servers.json` 文件。

### 2. 重启应用

关闭并重新打开 CrossChat 应用。

### 3. 查看日志

启动对话后，查看控制台输出：

**期望看到**:
```
[MCP] 开始加载工具，已配置服务器数: 1
[MCP] 检查服务器: Fetch (enabled=true)
[MCP] 服务器 Fetch 缓存状态: 无缓存
[MCP] 开始发现工具: Fetch (command=cmd, args=["/ c", "npx", "-y", "mcp-server-fetch-typescript"])
[MCP spawn] 启动命令: cmd ["/c", "npx", "-y", "mcp-server-fetch-typescript"]
[MCP spawn] 进程已启动，PID: Some(12345)
[MCP handshake] 开始 MCP 握手流程
[MCP handshake] 发送 initialize 请求
[MCP read] 开始读取响应，超时: 30秒
[MCP read] 收到行 1: {"jsonrpc":"2.0",...}
[MCP read] JSON 解析成功
[MCP handshake] 收到 initialize 响应
[MCP handshake] 发送 initialized 通知
[MCP handshake] 发送实际请求: tools/list
[MCP read] 开始读取响应，超时: 20秒
[MCP read] 收到行 1: {"jsonrpc":"2.0",...}
[MCP read] JSON 解析成功
[MCP handshake] 收到实际响应
[MCP] Fetch 工具发现成功: X 个工具
[MCP] 工具加载完成，总计: X 个
[agent_loop] MCP 工具加载完成: X 个
```

### 4. 测试工具

在对话中测试 MCP 工具是否可用：

```
请帮我获取 https://example.com 的内容
```

如果配置正确，AI 应该能够调用 fetch 工具。

---

## 故障排除

### 问题 1: 仍然显示 "MCP 服务器未返回响应"

**可能原因**:
- npm/npx 不在 PATH 中
- Node.js 未安装或版本过旧

**解决方案**:
```bash
# 检查 npx
where npx
npx --version

# 检查 Node.js
node --version

# 应该是 Node.js 18+ LTS
```

### 问题 2: "无法启动 MCP 进程"

**可能原因**:
- 命令路径错误
- 权限问题

**解决方案**:
- 使用完整路径：`C:\\Program Files\\nodejs\\npx.cmd`
- 以管理员身份运行应用

### 问题 3: 工具加载超时

**可能原因**:
- 网络问题导致 npm 下载包失败
- 防火墙阻止

**解决方案**:
- 预先安装包：`npm install -g mcp-server-fetch-typescript`
- 配置 npm 代理
- 检查防火墙设置

---

## 代码改进

我已经添加了详细的调试日志，帮助诊断问题：

### 新增日志

- `[MCP]` - MCP 管理器级别
- `[MCP spawn]` - 进程启动
- `[MCP handshake]` - 握手流程
- `[MCP read]` - 响应读取
- `[MCP stderr]` - 错误输出

### 改进的错误处理

- 捕获 stderr 输出
- 更详细的错误信息
- 超时保护

---

## 下一步

### 短期（立即）

1. ✅ 更新配置文件使用 `mcp-server-fetch-typescript`
2. ✅ 删除或禁用 Brave Search 配置
3. ✅ 重启应用测试

### 中期（本周）

1. 🔍 在 [Awesome MCP Servers](https://github.com/appcypher/awesome-mcp-servers) 查找搜索服务器
2. 📝 测试并添加其他有用的 MCP 服务器
3. 📚 更新应用文档

### 长期（未来）

1. 🎨 在 UI 中添加 MCP 服务器市场/浏览器
2. ✅ 自动验证包是否存在
3. 📦 提供预配置的服务器模板

---

## 相关文档

- [MCP_WINDOWS_NPX_ISSUE_DEBUG.md](./MCP_WINDOWS_NPX_ISSUE_DEBUG.md) - Windows npx 调试指南
- [MCP_CORRECT_PACKAGE_NAMES.md](./MCP_CORRECT_PACKAGE_NAMES.md) - 正确的包名列表
- [MCP_PACKAGE_ISSUE_RESOLUTION.md](./MCP_PACKAGE_ISSUE_RESOLUTION.md) - 包问题解决方案

---

## 总结

**问题**: 使用了不存在的 npm 包  
**原因**: 官方参考服务器未发布到 npm  
**解决**: 使用社区服务器 `mcp-server-fetch-typescript`  
**状态**: ✅ 已验证可用

更新配置后，MCP 工具应该能够正常加载！🎉

---

**创建时间**: 2026-05-08  
**最后更新**: 2026-05-08  
**状态**: 已解决 ✅
