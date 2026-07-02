# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

<!-- CODEGRAPH_START -->
## CodeGraph

This repository is indexed by CodeGraph (`.codegraph/` exists at the repo root). **Reach for it BEFORE grep/find or reading files when you need to understand or locate code:**

- **MCP tool** (when available): `codegraph_explore` answers most code questions in one call — the relevant symbols' verbatim source plus the call paths between them, including dynamic-dispatch hops grep can't follow. Name a file or symbol in the query to read its current line-numbered source. If it's listed but deferred, load it by name via tool search.
- **Shell** (always works): `codegraph explore "<symbol names or question>"` prints the same output.

If there is no `.codegraph/` directory, skip CodeGraph entirely — indexing is the user's decision.
<!-- CODEGRAPH_END -->

## 项目一句话

CrossChat 是基于 **Tauri 2 (Rust) + React 19 + TypeScript** 的跨 LLM 桌面端 AI 助手。**已完成向单一「六边形架构」的重写**：旧的双栈共存（Tauri command 全家桶 / 内嵌 HTTP 服务）**已从编译链移除**，当前只有一条激活路径，六边形的多数适配器目前是「已搭好但未接线」的脚手架。

> ⚠️ **版本号与文档超前警告**：`Cargo.toml` / `package.json` / `tauri.conf.json` 均为 `0.2.3`，但 `CHANGELOG.md`（标 v0.3.0）、`QUICKSTART.md`、`docs/REFACTOR_*.md` 描述的 **Axum HTTP Server / SSE / 21434 端口 / sqlx / 10-50x 流式**，在当前代码里**并不存在**（`Cargo.toml` 无 `axum`、无 `sqlx`，`lib.rs` 无 `http_server` 模块）。**以代码为准，别信这些文档**。

## 开发命令

环境要求：Node.js 22+、Rust（`dtolnay/rust-toolchain@stable`，未锁版本）、Python 3.11（仅构建期打包 office 依赖用）。

```bash
# 启动开发（前端固定端口 5174 + Tauri 主进程）
npm install
npm run tauri dev

# 生产构建（产物在 src-tauri/target/release/bundle/）
npm run tauri build

# 仅前端
npm run dev
npm run build           # tsc 类型检查 + vite build
npm run preview

# Rust 检查
cd src-tauri
cargo check
cargo build

# 重新生成 Tauri 图标
npm run icon design/icon-1024.png   # 或 npx @tauri-apps/cli icon design/icon-1024.png
```

**没有前端测试框架、没有 lint 脚本**。改动 TS 至少跑 `npm run build`（含 `tsc`）；改动 Rust 跑 `cargo check`。
注意：仓库里 `*_tests.rs`（`agent/`、`mcp/`、`memory/`、`metrics/`、`skills/`）**大多位于已停用的死代码模块**，不参与编译，`cargo test` 跑不到它们。

## 架构现状（最关键认知）

**只有一条激活路径**，且非常薄：

```
CanvasView.tsx  ──invoke()──▶  commands/{chat_cmd,session_cmd,file_ops}.rs
                                        │
                                        ├─ SqliteThreadStore (rusqlite)  ← ~/.../.crosschat/threads.db
                                        └─ reqwest 直连 /chat/completions（stream:false，一次性返回）
```

- **发消息**：`chat_cmd::send_chat_message` 把历史 turns 拼成 OpenAI messages → **非流式** POST `/chat/completions` → 落库一个新 Turn → 返回完整文本。前端发完后重新 `get_session` 拉全量刷新。
- **当前激活链里没有**：ReAct 循环、工具调用、MCP、流式/SSE、上下文压缩、skill、向量记忆。这些要么是死代码，要么是未接线脚手架（见下）。
- **`lib.rs` 只装配 6 个模块**：`core / ports / adapters / application / migration / commands`。
- **`invoke_handler` 只注册 14 个 command**：`send_chat_message` `fetch_models` `list_directory` `get_home_dir` `read_file_content` `get_file_preview_info` `read_file_bytes` `write_file_bytes` `delete_file_or_dir` `create_session` `list_sessions` `get_session` `save_messages` `delete_session` `migrate_data`。

## 后端地图（`src-tauri/src/`）

### 激活代码（真正编译、真正跑）

```
core/models/       # 纯数据：thread.rs / turn.rs / tool.rs / message.rs（无 I/O）
                   # Turn 用 #[serde(tag="type")] 的 TurnItem 枚举：UserMessage/AssistantText/
                   # AssistantReasoning/ToolCall/ToolResult/Compaction/Approval/Error
ports/             # 5 个 trait：ModelClient / ToolHost / ThreadStore / EventBus / ApprovalGate
adapters/store/    # sqlite_store.rs = SqliteThreadStore（rusqlite，唯一被激活的 adapter）
                   # 三张表：threads / turns(data 存 Turn 的 JSON) / todos
commands/          # mod.rs 只声明 chat_cmd / file_ops / session_cmd
                   #   chat_cmd.rs   → send_chat_message（中枢）、fetch_models
                   #   session_cmd.rs→ create/list/get/save/delete session（读写 SqliteThreadStore）
                   #   file_ops.rs   → 目录/文件读写删（⚠️ 无沙箱，见「安全」）
migration.rs       # 旧 ~/.crosschat/sessions/*.json → 新 threads.db
                   # 首次启动后台 spawn，成功写 .migrated 标记，原文件备份到 sessions_backup/
lib.rs             # setup：建 data_dir、初始化 SqliteThreadStore、spawn 迁移；注册 14 command
```

### 未接线脚手架（编译，但激活链没调用）

六边形其余部分**已实现但没接进 command 层**，改这些**不会影响运行行为**：

```
ports/             # ModelClient/ToolHost/EventBus/ApprovalGate 这 4 个 trait 无激活实现方使用
adapters/model/    # openai_client / anthropic_client / deepseek_client（SSE 流式实现，未被调用）
adapters/tool/     # local_tool_host / mcp_persistent / sandbox（未被调用）
adapters/event/    # memory_bus（tokio::broadcast，未被调用）
application/        # agent_loop.rs（新 ReAct，未被 command 调用）
```

### 磁盘死代码（**不在 `lib.rs` 编译链**，别在里面改 bug）

以下目录/文件**仍在磁盘上但没有 `mod` 声明**，grep 会撞到它们，但改了**完全无效**：

```
agent/  providers/  tools/  mcp/  streaming/  skills/  memory/  metrics/  security/
http_server/  python_env.rs
commands/ 下：agent_cmd / chat.rs / checkpoint_cmd / mcp_cmd / mcp_health_cmd /
              memory_cmd / metrics_cmd / provider_cmd / python_cmd / skills_cmd / stream_cmd
```

**要改后端行为，先确认目标文件在「激活代码」里；若功能只存在于死代码/脚手架，说明它当前根本没通电，需要先接线（改 `commands/mod.rs` + `lib.rs`）——这属于架构决策，应先与用户确认。**

## 前端地图（`src/`）

```
App.tsx                       # 只渲染 <CanvasView/> + <WelcomeDialog/>（无新旧 UI 切换）
main.tsx                      # 挂载 <ErrorBoundary><App/>；主题切换（dark/light/system）写 localStorage
components/
  canvas/CanvasView.tsx       # 唯一主界面：左侧会话列表 + 右侧消息 + 输入框 + 设置入口
                              # 内含 ThinkingBlock / ToolCallBlock；解析 <think> 标签、合并推理消息
  WelcomeDialog.tsx           # 新用户引导
  ErrorBoundary.tsx           # 顶层错误边界
  settings/                   # SettingsDialog / ProviderTab / McpSection / GeneralTab / FeedbackDialog
lib/
  tauri-bridge.ts             # ⚠️ 含大量后端已不注册的死 invoke（MCP/skills/checkpoint/streamChat/
                              #    readAgentConfig 等）。有效的只有 file/session/chat/fetchModels
  officeParser.ts             # 纯前端解析 office 预览（xlsx/mammoth/pptxgenjs/docx，不走后端 Python）
  providers.ts / slashCommands.ts / builtinCommands.ts / cn.ts
stores/                       # Zustand（persist 到 localStorage）：
                              #   settingsStore(crosschat-settings) / providerStore(crosschat-providers)
                              #   / workspaceStore  ——— 旧的 chatStore 已删
shared/ui/                    # Button/Input/Select/Textarea/Avatar
styles/globals.css            # Tailwind 4 入口，紫蓝渐变主题，ds-* 语义色
```

> `components/chat/*`（旧 ChatView 全家桶 13 个组件）、`hooks/*`（useChat/useAgent/useCheckpoint/useContextUsage）、`stores/chatStore.ts`、`lib/http-client.ts` **均已删除**。若旧文档或代码引用它们，视为过时。

## 数据存储（`~/.crosschat` 系）

- **统一走 `dirs::data_dir().join(".crosschat")`** → `threads.db`（rusqlite）。
  - macOS: `~/Library/Application Support/.crosschat/`
  - Linux: `~/.local/share/.crosschat/`
  - Windows: `%APPDATA%\.crosschat\`
- **迁移的跨平台坑**：`migration.rs` **读旧数据**用的是 `dirs::home_dir().join(".crosschat/sessions")`（即 `~/.crosschat/sessions`），**写新库**用 `data_dir()`。在 macOS/Linux 上这**不是同一个目录**——旧数据在 `~/.crosschat`，新库在 data_dir。调试迁移时注意这个不对称。
- 改 schema 必须同步 `migration.rs` 与 `SqliteThreadStore::new` 里的建表 SQL，否则老用户 `threads.db` 读不到。

## 安全 / 配置点（含已知退化）

- `tauri.conf.json`：`csp: null`、`withGlobalTauri: true`、`assetProtocol.scope: ["**"]`——**CSP 未启用，P0 未修**。
- ⚠️ **`file_ops.rs` 目前没有任何路径沙箱/白名单**：`list_directory` / `read_file_content` / `read_file_bytes` / `write_file_bytes` / `delete_file_or_dir` 都是裸 `std::fs`，可读写删任意路径（旧 `security/sandbox.rs`、新 `adapters/tool/sandbox.rs` 都未接线）。新增文件类 command 时务必自己加校验。
- ⚠️ **API Key 明文存 localStorage**（`settingsStore` 的 `credentials`，persist key `crosschat-settings`），未编码、未进 Keychain。
- `capabilities/default.json`：Tauri 权限清单。**新增 command 若需新权限，Rust 端 `invoke_handler!` + 此文件两处都要加**。当前含 `shell:allow-execute`、`dialog`、`global-shortcut`、`clipboard-manager` 等。

## Python 打包（构建期，与运行时解耦）

`tauri.conf.json` 仍把 `resources/python` 打进包（`setup_python.py` 下载 embed Python + `install_office_deps.py` 装 openpyxl/python-docx/pptx 等）。**但后端 `python_env.rs` / `tools/python_sandbox.rs` 已是死代码**——运行时不再用内嵌 Python；office 预览改由前端 `officeParser.ts` 纯 JS 完成。即：**目前是「打包了但激活链不消费」的状态**，动它前先想清楚是否还需要。

CI 缓存 key（改 Python 依赖时 bump）：Windows `python-embed-windows-3.11.9`、Linux `python-embed-linux-3.11.7`。改 setup 脚本注意 Windows 编码问题（见 `fix: resolve Windows CI encoding issue` 提交）。

## CI

`.github/workflows/build.yml`：push 到 `master` 或 tag `v*` / `Beta*` 触发，构建 Windows (msi/nsis) + Linux (deb/rpm)。**macOS 未配置**（无签名包）。

## 改代码前必读

- **先分清三层**：激活代码 / 未接线脚手架 / 磁盘死代码（见「后端地图」）。改错层 = 白改。
- 改「发消息」逻辑 → `commands/chat_cmd.rs`（不是死代码 `commands/chat.rs`、也不是脚手架 `application/agent_loop.rs`）。
- 改「会话增删改查」→ `commands/session_cmd.rs` + `adapters/store/sqlite_store.rs`。
- 改前端界面 → 只有 `components/canvas/CanvasView.tsx` 一个主界面。
- 想恢复流式 / 工具调用 / MCP → 这些代码在脚手架或死代码里，需要**接线 + 架构决策**，先与用户确认，别默默启用。
- 改存储 schema → 同步 `migration.rs` 与建表 SQL。
- 新增 Tauri command → `lib.rs` 的 `invoke_handler!` + `commands/mod.rs` + 必要时 `capabilities/default.json`。
- **不要参照** `CHANGELOG.md` / `QUICKSTART.md` / `docs/REFACTOR_*.md` 里的 HTTP/SSE/端口 21434/sqlx 描述——它们与当前代码不符。
