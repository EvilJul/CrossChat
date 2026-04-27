import { useMemo } from "react";
import { useChatStore } from "../stores/chatStore";

function estimateTokens(text: string): number {
  let chinese = 0, other = 0;
  for (const ch of text) {
    if (ch > "\u{2FFF}") chinese++; else other++;
  }
  return Math.ceil(chinese / 1.5 + other / 3.5);
}

export function useContextUsage() {
  const messages = useChatStore((s) => s.messages);
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
    // 根据模型调整上下文窗口上限
    const maxTokens = 200000; // Claude 200K / GPT-4o 128K，取较大值
    return {
      tokens: total,
      maxTokens,
      percent: Math.min(Math.round((total / maxTokens) * 100), 100),
    };
  }, [messages]);

  const { percent, tokens, maxTokens } = usage;
  const color = percent > 90 ? "bg-red-500" : percent > 60 ? "bg-amber-500" : "bg-emerald-500";
  const tooltip = `${tokens.toLocaleString()} / ${(maxTokens / 1000).toFixed(0)}K tokens`;
  return { percent, color, tooltip };
}
