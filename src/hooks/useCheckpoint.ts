import { useCallback } from "react";
import { useChatStore } from "../stores/chatStore";
import { saveCheckpoint, loadCheckpoint, clearCheckpoint } from "../lib/tauri-bridge";

interface UseCheckpointOptions {
  activeProviderId: string;
  activeModel: string;
  workDir: string;
}

export function useCheckpoint({ activeProviderId, activeModel, workDir }: UseCheckpointOptions) {
  const messages = useChatStore((s) => s.messages);
  const clearMessages = useChatStore((s) => s.clearMessages);
  const addMessage = useChatStore((s) => s.addMessage);

  const save = useCallback(async () => {
    await saveCheckpoint({
      messages: messages.map((m) => ({
        role: m.role, content: m.content, thinking: m.thinking || null,
      })),
      provider_id: activeProviderId, model: activeModel,
      work_dir: workDir, saved_at: Date.now(),
    });
  }, [messages, activeProviderId, activeModel, workDir]);

  const restore = useCallback(async () => {
    const cp = await loadCheckpoint();
    if (!cp || cp.messages.length === 0) return "没有可恢复的对话记录。";
    clearMessages();
    for (const msg of cp.messages) {
      addMessage({
        id: `cp-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
        role: msg.role as "user" | "assistant",
        content: msg.content,
        thinking: msg.thinking || undefined,
        timestamp: Date.now(),
      });
    }
    await clearCheckpoint();
    return `已恢复 ${cp.messages.length} 条消息，可继续对话。`;
  }, [clearMessages, addMessage]);

  return { save, restore };
}
