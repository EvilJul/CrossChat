import { create } from "zustand";

export type MessageRole = "user" | "assistant" | "tool";

export interface ToolCallState {
  id: string;
  name: string;
  arguments: string;
  status: "pending" | "running" | "executing" | "completed" | "failed";
  result?: string;
}

export interface FileAttachment {
  name: string;
  dataUrl: string;
  mimeType: string;
  size: number;
}

export interface ChatMessage {
  id: string;
  role: MessageRole;
  content: string;
  thinking?: string;
  timestamp: number;
  toolCalls?: ToolCallState[];
  attachments?: FileAttachment[];
  isStreaming?: boolean;
  isThinking?: boolean;
}

interface ChatStore {
  messages: ChatMessage[];
  isGenerating: boolean;
  addMessage: (msg: ChatMessage) => void;
  appendContent: (id: string, delta: string) => void;
  appendThinking: (id: string, delta: string) => void;
  setThinkingDone: (id: string) => void;
  setStreaming: (id: string, isStreaming: boolean) => void;
  addToolCall: (msgId: string, toolCall: ToolCallState) => void;
  updateToolCall: (
    msgId: string,
    toolCallId: string,
    update: Partial<ToolCallState>
  ) => void;
  clearMessages: () => void;
  setIsGenerating: (v: boolean) => void;
}

/// 从原始文本中提取完整的 <think>...</think> 块
/// 返回 { clean: 不含think标签的文本, thinking: 提取出的思考内容 }
function extractThinkBlocks(raw: string): { clean: string; thinking: string } {
  const thinkRegex = /<\s*think\s*>([\s\S]*?)<\s*\/\s*think\s*>/gi;
  const parts: string[] = [];
  let match;
  while ((match = thinkRegex.exec(raw)) !== null) {
    parts.push(match[1].trim());
  }
  const clean = raw.replace(thinkRegex, "").trim();
  return { clean, thinking: parts.join("\n\n") };
}

export const useChatStore = create<ChatStore>((set) => ({
  messages: [],
  isGenerating: false,

  addMessage: (msg) =>
    set((s) => ({
      messages: [...s.messages, msg],
    })),

  appendContent: (id, delta) =>
    set((s) => ({
      messages: s.messages.map((m) => {
        if (m.id !== id) return m;
        const rawContent = m.content + delta;
        const { clean, thinking } = extractThinkBlocks(rawContent);
        return {
          ...m,
          content: clean,
          // 只有提取到新 think 内容时才更新 thinking 字段
          thinking: thinking ? ((m.thinking || "") + "\n" + thinking).trim() : m.thinking,
          isThinking: thinking.length > 0 || m.isThinking,
        };
      }),
    })),

  appendThinking: (id, delta) =>
    set((s) => ({
      messages: s.messages.map((m) =>
        m.id === id
          ? { ...m, thinking: (m.thinking || "") + delta, isThinking: true }
          : m
      ),
    })),

  setThinkingDone: (id) =>
    set((s) => ({
      messages: s.messages.map((m) =>
        m.id === id ? { ...m, isThinking: false } : m
      ),
    })),

  setStreaming: (id, isStreaming) =>
    set((s) => ({
      messages: s.messages.map((m) =>
        m.id === id ? { ...m, isStreaming } : m
      ),
    })),

  addToolCall: (msgId, toolCall) =>
    set((s) => ({
      messages: s.messages.map((m) =>
        m.id === msgId
          ? { ...m, toolCalls: [...(m.toolCalls || []), toolCall] }
          : m
      ),
    })),

  updateToolCall: (msgId, toolCallId, update) =>
    set((s) => ({
      messages: s.messages.map((m) =>
        m.id === msgId
          ? {
              ...m,
              toolCalls: m.toolCalls?.map((tc) =>
                tc.id === toolCallId ? { ...tc, ...update } : tc
              ),
            }
          : m
      ),
    })),

  clearMessages: () => set({ messages: [] }),

  setIsGenerating: (v) => set({ isGenerating: v }),
}));
