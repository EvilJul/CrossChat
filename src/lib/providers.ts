// 前端 Provider 类型定义 + 默认配置
import type { ProviderEntry } from "../stores/settingsStore";

// 流式响应块类型 (与 Rust StreamChunk 对应)
export type StreamChunk =
  | { type: "TextDelta"; delta: string }
  | { type: "ThinkingDelta"; delta: string }
  | { type: "ThinkingDone" }
  | { type: "ToolCallStart"; id: string; name: string }
  | { type: "ToolCallDelta"; id: string; arguments_delta: string }
  | { type: "ToolCallEnd"; id: string }
  | { type: "Done"; finish_reason?: string }
  | { type: "Error"; message: string };

export const PRESET_PROVIDERS: Omit<ProviderEntry, "id">[] = [
  {
    name: "OpenAI",
    apiBase: "https://api.openai.com/v1",
    models: ["gpt-4o", "gpt-4o-mini", "gpt-4-turbo", "o3-mini"],
    providerType: "openai-compat",
  },
  {
    name: "DeepSeek",
    apiBase: "https://api.deepseek.com/v1",
    models: ["deepseek-chat", "deepseek-reasoner"],
    providerType: "openai-compat",
  },
  {
    name: "Anthropic",
    apiBase: "https://api.anthropic.com/v1",
    models: [
      "claude-sonnet-4-6",
      "claude-opus-4-6",
      "claude-haiku-4-5",
    ],
    providerType: "anthropic",
  },
  {
    name: "通义千问 (DashScope)",
    apiBase: "https://dashscope.aliyuncs.com/compatible-mode/v1",
    models: ["qwen-max", "qwen-plus", "qwen-turbo"],
    providerType: "openai-compat",
  },
  {
    name: "Groq",
    apiBase: "https://api.groq.com/openai/v1",
    models: ["llama-3.3-70b-versatile", "mixtral-8x7b-32768"],
    providerType: "openai-compat",
  },
  {
    name: "Ollama (本地)",
    apiBase: "http://localhost:11434/v1",
    models: ["llama3", "codellama", "qwen2.5"],
    providerType: "openai-compat",
  },
];
