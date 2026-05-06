import { registerCommand, getCommands } from "./slashCommands";

export function setupBuiltinCommands(deps: {
  clearMessages: () => void;
  onNewSession: () => Promise<void>;
  onContinue: () => Promise<string | undefined>;
  openWorkspace: () => void;
  openSettings: () => void;
  openFeedback: () => void;
  getCurrentProvider: () => string;
  getCurrentModel: () => string;
}) {
  // /clear — 静默执行
  registerCommand({
    name: "clear",
    description: "清空当前对话消息",
    clearInput: true,
    handler: () => {
      deps.clearMessages();
    },
  });

  // /continue — 恢复上次中断的任务
  registerCommand({
    name: "continue",
    description: "恢复上次中断的对话",
    clearInput: true,
    handler: async () => {
      const result = await deps.onContinue();
      return result;
    },
  });

  // /compress — 手动压缩上下文
  registerCommand({
    name: "compress",
    description: "立即压缩对话上下文（摘要早期消息）",
    clearInput: true,
    handler: () => {
      // 上下文压缩在下一次发送消息时自动触发（80K tokens 阈值）
      // 这里立即触发一次消息来强制压缩
      return "上下文压缩将在下一条消息发送时自动执行。\n\n" +
        "**压缩机制**:\n" +
        "- 超过 80K tokens 自动触发\n" +
        "- LLM 自动摘要前 60% 的消息\n" +
        "- 摘要注入上下文，保留关键决策和进度\n" +
        "- 最新 40% 消息完整保留\n\n" +
        "发送任意消息即可触发压缩。";
    },
  });

  // /new — 静默执行
  registerCommand({
    name: "new",
    description: "创建新对话",
    clearInput: true,
    handler: async () => {
      await deps.onNewSession();
    },
  });

  // /workspace — 静默执行
  registerCommand({
    name: "workspace",
    description: "打开/展开工作区面板",
    clearInput: true,
    handler: () => {
      deps.openWorkspace();
    },
  });

  // /settings — 静默执行
  registerCommand({
    name: "settings",
    description: "打开设置面板",
    clearInput: true,
    handler: () => {
      deps.openSettings();
    },
  });

  // /feedback — 静默执行
  registerCommand({
    name: "feedback",
    description: "打开反馈面板",
    clearInput: true,
    handler: () => {
      deps.openFeedback();
    },
  });

  // /status — 显示状态
  registerCommand({
    name: "status",
    description: "显示当前模型状态",
    handler: () => {
      const provider = deps.getCurrentProvider();
      const model = deps.getCurrentModel();
      if (!provider) return "当前未配置模型 — 请先在设置中添加 Provider 并输入 API Key";
      if (!model) return `当前 Provider: ${provider}，但未选择模型`;
      return `当前模型: **${provider}** / **${model}**`;
    },
  });

  // /skills — 快速列出已安装的 Skills
  registerCommand({
    name: "skills",
    description: "列出已安装的 Skills 扩展",
    handler: async () => {
      try {
        const { listSkills } = await import("./tauri-bridge");
        const skills = await listSkills();

        if (skills.length === 0) {
          return "暂无已安装的 Skills。用户可输入 `install_skill <GitHub URL>` 安装，或手动放入 `~/.crosschat/skills/` 目录。";
        }

        const enabled = skills.filter((s) => s.enabled);
        const disabled = skills.filter((s) => !s.enabled);

        const lines: string[] = [];
        lines.push(`**Skills** (${enabled.length} 启用${disabled.length > 0 ? `, ${disabled.length} 禁用` : ""})`);
        lines.push("");

        for (const s of enabled) {
          lines.push(`- **${s.name}** v${s.version}`);
        }

        if (disabled.length > 0) {
          for (const s of disabled) {
            lines.push(`- ~~${s.name}~~ v${s.version} (已禁用)`);
          }
        }

        return lines.join("\n");
      } catch {
        return "获取 Skills 失败。";
      }
    },
  });

  // /help — 帮助信息
  registerCommand({
    name: "help",
    description: "显示帮助和所有命令",
    handler: () => {
      const cmds = getCommands();
      return (
        "**CrossChat 帮助**\n\n" +
        "**斜杠命令**:\n" +
        cmds.map((c) => `- **/${c.name}** — ${c.description}`).join("\n") +
        "\n\n**使用提示**:\n" +
        "- Enter 发送消息，Shift+Enter 换行\n" +
        "- 输入 / 弹出命令菜单，上下箭头选择\n" +
        "- 在设置中添加 API Key 后即可使用真实模型\n" +
        "- 打开工作区面板后 AI 可操作该目录下的文件\n" +
        "- 顶部上下文用量达红色时自动压缩历史对话\n" +
        "- 使用 /skills 查看 AI 的全部能力"
      );
    },
  });
}
