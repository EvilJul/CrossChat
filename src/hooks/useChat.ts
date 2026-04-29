import { useCallback } from "react";
import { useChatStore } from "../stores/chatStore";
import { useSettingsStore } from "../stores/settingsStore";
import { useProviderStore } from "../stores/providerStore";
import { useWorkspaceStore } from "../stores/workspaceStore";
import { streamChat } from "../lib/tauri-bridge";
import type { StreamChunk } from "../lib/providers";
import type { FileAttachment } from "../components/chat/ChatInput";

const MAX_CONTEXT_MESSAGES = 40;

/// 不支持图片的模型（名称包含这些词的提示用户）
const MODELS_WITHOUT_IMAGE = ["deepseek", "qwen", "llama", "mixtral", "codellama", "deepseek-chat", "deepseek-reasoner"];

interface UseChatOptions {
  workDir: string;
}

export function useChat({ workDir }: UseChatOptions) {
  const isGenerating = useChatStore((s) => s.isGenerating);
  const addMessage = useChatStore((s) => s.addMessage);
  const appendContent = useChatStore((s) => s.appendContent);
  const appendThinking = useChatStore((s) => s.appendThinking);
  const setThinkingDone = useChatStore((s) => s.setThinkingDone);
  const setStreaming = useChatStore((s) => s.setStreaming);
  const addToolCall = useChatStore((s) => s.addToolCall);
  const updateToolCall = useChatStore((s) => s.updateToolCall);
  const setIsGenerating = useChatStore((s) => s.setIsGenerating);

  const activeProviderId = useSettingsStore((s) => s.activeProviderId);
  const activeModel = useSettingsStore((s) => s.activeModel);
  const credentials = useSettingsStore((s) => s.credentials);
  const providers = useProviderStore((s) => s.providers);

  const send = useCallback(async (content: string, attachments: FileAttachment[] = []) => {
    // 模型不支持图片时提示
    if (attachments.some((a) => a.mimeType.startsWith("image/")) && activeModel) {
      const modelLower = activeModel.toLowerCase();
      if (MODELS_WITHOUT_IMAGE.some((kw) => modelLower.includes(kw))) {
        appendContent("", ""); // no-op
        addMessage({
          id: `warn-${Date.now()}`,
          role: "assistant" as const,
          content: `⚠️ 当前模型 **${activeModel}** 可能不支持图片识别。建议切换到支持 Vision 的模型（如 gpt-4o、claude-sonnet-4-6、gpt-4-turbo）。`,
          timestamp: Date.now(),
        });
        return;
      }
    }

    const userMsg = { id: `msg-${Date.now()}`, role: "user" as const, content, timestamp: Date.now(), attachments };
    addMessage(userMsg);

    const assistantId = `msg-${Date.now() + 1}`;
    addMessage({ id: assistantId, role: "assistant" as const, content: "", timestamp: Date.now(), isStreaming: true });
    setIsGenerating(true);

    let providerConfig = null;
    if (activeProviderId) {
      const p = providers.find((pp) => pp.id === activeProviderId);
      const cred = credentials[activeProviderId];
      if (p && cred) providerConfig = { apiBase: p.apiBase, apiKey: cred.apiKey, providerType: p.providerType };
    }

    const currentMessages = useChatStore.getState().messages
      .filter((m) => m.id !== assistantId && !m.id.startsWith("context-note") && !m.id.startsWith("warn-"))
      .slice(-MAX_CONTEXT_MESSAGES)
      .filter((m) => m.content)
      .map((m) => ({ role: m.role, content: m.content, attachments: (m as any).attachments }));

    await streamChat(
      activeProviderId || "demo", activeModel || "demo-model", currentMessages,
      providerConfig, workDir,
      (chunk: StreamChunk) => {
        const store = useChatStore.getState();
        switch (chunk.type) {
          case "ThinkingDelta": appendThinking(assistantId, chunk.delta); break;
          case "ThinkingDone": setThinkingDone(assistantId); break;
          case "TextDelta": appendContent(assistantId, chunk.delta); break;
          case "ToolCallStart": addToolCall(assistantId, { id: chunk.id, name: chunk.name, arguments: "", status: "running" }); break;
          case "ToolCallDelta": {
            const msg = store.messages.find((m) => m.id === assistantId);
            const tc = msg?.toolCalls?.find((t) => t.id === chunk.id);
            try {
              updateToolCall(assistantId, chunk.id, { arguments: JSON.stringify(JSON.parse(chunk.arguments_delta)) });
            } catch {
              updateToolCall(assistantId, chunk.id, { arguments: (tc?.arguments || "") + chunk.arguments_delta });
            }
            break;
          }
          case "ToolCallEnd": updateToolCall(assistantId, chunk.id, { status: "executing" }); break;
          case "ToolResult":
            updateToolCall(assistantId, chunk.call_id, {
              status: chunk.success ? "completed" : "failed",
              result: chunk.content,
            });
            // 文件修改操作后自动刷新侧边栏
            if (chunk.success && ["write_file", "delete_file"].includes(chunk.name)) {
              useWorkspaceStore.getState().triggerRefresh();
            }
            break;
          case "Error": appendContent(assistantId, `\n\n> **错误**: ${chunk.message}`); break;
        }
      },
      (error) => { appendContent(assistantId, `\n\n> 错误: ${error}`); setStreaming(assistantId, false); setIsGenerating(false); },
      () => { setStreaming(assistantId, false); setIsGenerating(false); }
    );
  }, [activeProviderId, activeModel, credentials, providers, workDir, addMessage, appendContent, appendThinking, setThinkingDone, setStreaming, addToolCall, updateToolCall, setIsGenerating]);

  const stop = useCallback(() => {
    setIsGenerating(false);
    const store = useChatStore.getState();
    const streaming = store.messages.find((m) => m.isStreaming);
    if (streaming) store.setStreaming(streaming.id, false);
    return store.messages;
  }, [setIsGenerating]);

  return { isGenerating, send, stop };
}
