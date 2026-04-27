# OpenAiDesktop

跨模型供应商的桌面端 AI 助手。支持多家模型 API，一键切换，不绑定任何厂商。

## 特性

- **多模型支持** — OpenAI、Anthropic Claude、DeepSeek、通义千问、Groq、Ollama，自定义 API 接入
- **一键切换** — 设置中可同时录入多家 API，选择即用，无需手动换 Key
- **工具调用** — 文件读写、命令行执行、目录浏览，带安全确认机制
- **流式输出** — 实时流式响应，Markdown 渲染，代码高亮
- **思考链** — 支持 DeepSeek R1、Qwen QwQ 等推理模型的思考过程展示（可折叠）
- **MCP 插件** — 内置 8 款精选 MCP 插件，支持自定义接入
- **上下文管理** — 自动 token 统计，超阈值自动压缩摘要
- **AGENT.md** — 通过 `.md` 文件定义助手行为和规则约束
- **工作区** — 打开本地文件夹，AI 可直接操作工作目录内的文件
- **斜杠命令** — `/clear` `/new` `/compress` `/skills` `/help` 等

## 技术栈

| 层 | 技术 |
|---|------|
| 桌面框架 | Tauri v2 (Rust) |
| 前端 | React 19 + TypeScript + Tailwind CSS 4 |
| 状态管理 | Zustand |
| 动画 | Framer Motion |
| 构建 | Vite 7 + Cargo |

## 安装

从 [Releases](https://github.com/tian1419648701/OpenAiDesktop/releases) 页面下载对应平台的安装包：

- **Windows**: `.msi` 安装包
- **Linux**: `.AppImage` / `.deb` 包
- **macOS**: 暂未提供（需开发者签名）

## 开发

### 环境要求

- Node.js 22+
- Rust 1.94+
- 系统 WebView2（Windows 11 内置）

### 启动

```bash
npm install
npm run tauri dev
```

### 构建

```bash
npm run tauri build
```

构建产物在 `src-tauri/target/release/bundle/` 目录。

## 项目结构

```
src/                    # React 前端
├── components/
│   ├── chat/           # 聊天界面组件
│   └── settings/       # 设置面板（模型/MCP/通用）
├── hooks/              # 业务逻辑 hooks
├── shared/ui/          # 通用 UI 组件库
├── stores/             # Zustand 状态管理
├── lib/                # 桥接层、命令系统
└── styles/             # 全局样式

src-tauri/              # Rust 后端
├── src/
│   ├── commands/       # Tauri 命令（chat、文件、会话、MCP...）
│   ├── providers/      # LLM Provider 抽象层
│   ├── mcp/            # MCP 协议实现（JSON-RPC over stdio）
│   ├── tools/          # 内置工具（文件操作、命令执行）
│   ├── security/       # 安全策略（路径沙箱、危险命令检测）
│   └── streaming/      # SSE 流解析
└── Cargo.toml
```

## 使用说明

1. 启动后点击右上角齿轮进入**设置**
2. 在「模型」选项卡添加供应商，输入 API Key，点击测试
3. 选择模型后即可开始对话
4. 点击左侧面板按钮打开**工作区**，选择文件夹
5. 输入 `/` 查看可用命令

## 安全

- API Key Base64 编码存储，后续版本迁移至 OS Keychain
- 文件删除、命令执行需用户确认
- 危险命令自动检测拦截
- 文件操作限制在工作目录范围内
- LLM 请求在 Rust 后端完成，API Key 不进入前端进程

## 许可

MIT License
