import { useMemo } from "react";
import { useChatStore } from "../stores/chatStore";
import { useSettingsStore } from "../stores/settingsStore";

function estimateTokens(text: string): number {
  let chinese = 0, other = 0;
  for (const ch of text) {
    if (ch > "\u{2FFF}") chinese++; else other++;
  }
  return Math.ceil(chinese / 1.5 + other / 3.5);
}

// 模型上下文长度映射表（单位：tokens）
const MODEL_CONTEXT_LIMITS: Record<string, number> = {
  // OpenAI
  "gpt-4o": 128000,
  "gpt-4o-mini": 128000,
  "gpt-4-turbo": 128000,
  "gpt-4": 8192,
  "gpt-3.5-turbo": 16385,
  "o3-mini": 200000,
  "o1": 200000,
  "o1-mini": 128000,
  
  // Anthropic Claude
  "claude-sonnet-4-6": 200000,
  "claude-opus-4-6": 200000,
  "claude-haiku-4-5": 200000,
  "claude-3-5-sonnet": 200000,
  "claude-3-opus": 200000,
  "claude-3-sonnet": 200000,
  "claude-3-haiku": 200000,
  
  // DeepSeek (根据官方文档：V3/V4 支持 128K 上下文)
  "deepseek-chat": 128000,
  "deepseek-reasoner": 128000,
  "deepseek-coder": 128000,
  "deepseek-v3": 128000,
  "deepseek-v4": 1000000,  // V4 支持 1M 上下文
  
  // 通义千问 (根据官方文档和 Qwen2.5 系列)
  "qwen-max": 32000,      // 官方文档：32K 上下文
  "qwen-plus": 128000,    // Qwen2.5 系列：128K 上下文
  "qwen-turbo": 1000000,  // Qwen2.5-Turbo：1M 上下文
  "qwen2.5": 128000,      // Qwen2.5 系列：128K 上下文
  "qwen2": 32000,
  "qwen-long": 1000000,   // Qwen-Long：1M 上下文
  
  // Groq
  "llama-3.3-70b-versatile": 8192,
  "llama-3.1-70b-versatile": 128000,
  "mixtral-8x7b-32768": 32768,
  
  // Ollama (本地模型，默认值)
  "llama3": 8192,
  "llama3.1": 128000,
  "llama3.2": 128000,
  "codellama": 16384,
};

// 根据模型名称获取上下文长度（支持模糊匹配）
function getModelContextLimit(modelName: string): number {
  // 精确匹配
  if (MODEL_CONTEXT_LIMITS[modelName]) {
    return MODEL_CONTEXT_LIMITS[modelName];
  }
  
  // 模糊匹配（处理版本号等变体）
  const lowerModel = modelName.toLowerCase();
  for (const [key, limit] of Object.entries(MODEL_CONTEXT_LIMITS)) {
    if (lowerModel.includes(key.toLowerCase()) || key.toLowerCase().includes(lowerModel)) {
      return limit;
    }
  }
  
  // 默认值：128K（现代模型的常见值）
  return 128000;
}

export function useContextUsage() {
  const messages = useChatStore((s) => s.messages);
  const activeModel = useSettingsStore((s) => s.activeModel);
  
  const usage = useMemo(() => {
    let total = 0;
    for (const msg of messages) {
      total += estimateTokens(msg.content);
      total += estimateTokens(msg.thinking || "");
      // 统计工具调用参数
      if (msg.toolCalls) {
        for (const tc of msg.toolCalls) {
          total += estimateTokens(tc.arguments || "");
          total += estimateTokens(tc.name || "");
        }
      }
    }
    
    // 根据当前选择的模型获取上下文窗口上限
    const maxTokens = getModelContextLimit(activeModel);
    
    return {
      tokens: total,
      maxTokens,
      percent: Math.min(Math.round((total / maxTokens) * 100), 100),
    };
  }, [messages, activeModel]);

  const { percent, tokens, maxTokens } = usage;
  const color = percent > 90 ? "bg-red-500" : percent > 60 ? "bg-amber-500" : "bg-emerald-500";
  const tooltip = `${tokens.toLocaleString()} / ${(maxTokens / 1000).toFixed(0)}K tokens (${activeModel || "未选择模型"})`;
  return { percent, color, tooltip };
}
