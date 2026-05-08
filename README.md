# CrossChat

跨模型供应商的桌面端 AI 助手。支持多家模型 API，一键切换，不绑定任何厂商。

## ✨ 特性

- **多模型支持** — OpenAI、Anthropic Claude、DeepSeek、通义千问、Groq、Ollama，自定义 API 接入
- **一键切换** — 设置中可同时录入多家 API，选择即用，无需手动换 Key
- **工具调用** — 文件读写、命令行执行、目录浏览，带安全确认机制
- **流式输出** — 实时流式响应，Markdown 渲染，代码高亮
- **思考链** — 支持 DeepSeek R1、Qwen QwQ 等推理模型的思考过程展示（可折叠）
- **MCP 插件** — 内置精选 MCP 插件，支持自定义接入和验证
- **上下文管理** — 自动 token 统计，支持 30+ 模型的上下文长度检测
- **AGENT.md** — 通过 `.md` 文件定义助手行为和规则约束
- **工作区** — 打开本地文件夹，AI 可直接操作工作目录内的文件
- **斜杠命令** — `/clear` `/new` `/compress` `/skills` `/help` 等
- **现代 UI** — iMessage 风格聊天气泡，紫蓝渐变主题，流畅动画

## 🎨 界面预览

- **聊天界面**: iMessage 风格气泡，支持思考过程展示
- **设置面板**: 简洁的 Settings 界面，支持多供应商管理
- **MCP 工具**: 一键安装精选插件，实时验证连接
- **应用图标**: 紫蓝渐变 + AI 星标，现代科技感

## 🛠️ 技术栈

| 层 | 技术 |
|---|------|
| 桌面框架 | Tauri v2 (Rust) |
| 前端 | React 19 + TypeScript + Tailwind CSS 4 |
| UI 组件 | Radix UI + Lucide Icons |
| 状态管理 | Zustand |
| 动画 | Framer Motion |
| 构建 | Vite 7 + Cargo |

## 📦 安装

从 [Releases](https://github.com/EvilJul/CrossChat/releases) 页面下载对应平台的安装包：

- **Windows**: `.msi` 安装包
- **Linux**: `.AppImage` / `.deb` 包
- **macOS**: 暂未提供（需开发者签名）

## 🚀 开发

### 环境要求

- Node.js 22+
- Rust 1.94+
- 系统 WebView2（Windows 11 内置）

### 启动开发服务器

```bash
npm install
npm run tauri dev
```

### 构建生产版本

```bash
npm run tauri build
```

构建产物在 `src-tauri/target/release/bundle/` 目录。

### 生成应用图标

```bash
# 1. 转换 SVG 为 PNG (使用 https://svgtopng.com/)
#    上传: design/app-icon-flat.svg
#    尺寸: 1024x1024
#    保存: design/icon-1024.png

# 2. 生成所有尺寸
npm run generate-icons

# 或直接使用 npx
npx @tauri-apps/cli icon design/icon-1024.png
```

## 📁 项目结构

```
src/                    # React 前端
├── components/
│   ├── chat/           # 聊天界面组件（气泡、输入、侧边栏）
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

design/                 # 设计资源
├── app-icon.svg        # 应用图标（详细版）
├── app-icon-flat.svg   # 应用图标（简化版）
└── ICON_DESIGN_GUIDE.md # 图标设计指南

docs/                   # 项目文档
├── README.md           # 文档索引
├── FEATURES.md         # 功能特性说明
├── UI_OPTIMIZATION_COMPLETE.md # UI 优化总结
└── MCP_LOADING_ISSUE_FINAL_SOLUTION.md # MCP 配置指南
```

## 📖 使用说明

1. 启动后点击右上角齿轮进入**设置**
2. 在「AI 模型」选项卡添加供应商，输入 API Key，点击测试
3. 选择模型后即可开始对话
4. 点击左侧面板按钮打开**工作区**，选择文件夹
5. 输入 `/` 查看可用命令

### MCP 插件配置

1. 进入设置 → MCP 工具
2. 从精选插件中选择需要的工具（Filesystem、GitHub、Fetch 等）
3. 点击安装并启用
4. 使用"测试连接"验证插件是否正常工作

详细配置指南: [docs/MCP_LOADING_ISSUE_FINAL_SOLUTION.md](docs/MCP_LOADING_ISSUE_FINAL_SOLUTION.md)

## 🔒 安全

- API Key Base64 编码存储，后续版本迁移至 OS Keychain
- 文件删除、命令执行需用户确认
- 危险命令自动检测拦截
- 文件操作限制在工作目录范围内
- LLM 请求在 Rust 后端完成，API Key 不进入前端进程
- MCP 工具支持命令验证和连接测试

## 📝 更新日志

### v0.2.3 (2026-05-08)

- ✨ 重新设计应用图标（紫蓝渐变 + AI 星标）
- 🎨 简化设置页面标题为 "Settings"
- 🐛 修复 MCP 工具加载问题（更新包名）
- 🐛 修复 Anthropic 供应商测试失败问题
- 📝 清理冗余文档，优化文档结构
- 🔧 优化 McpSection UI，严格遵守设计规范

### v0.2.2

- 🎨 UI 全面优化：iMessage 风格聊天气泡
- ✨ 动态模型上下文长度检测（支持 30+ 模型）
- 🔧 MCP 验证功能：命令检测和连接测试
- 📚 完善文档和使用指南

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

## 📄 许可

MIT License

---

**开发者**: [EvilJul](https://github.com/EvilJul)  
**项目地址**: [CrossChat](https://github.com/EvilJul/CrossChat)
