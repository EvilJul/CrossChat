import { create } from "zustand";

export type MessageRole = "user" | "assistant" | "tool";

export interface ToolCallState {
  id: string;
  name: string;
  arguments: string;
  status: "pending" | "running" | "completed" | "failed";
  result?: string;
}

export interface ChatMessage {
  id: string;
  role: MessageRole;
  content: string;
  thinking?: string;
  timestamp: number;
  toolCalls?: ToolCallState[];
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

export const useChatStore = create<ChatStore>((set) => ({
  messages: [],
  isGenerating: false,

  addMessage: (msg) =>
    set((s) => ({
      messages: [...s.messages, msg],
    })),

  appendContent: (id, delta) =>
    set((s) => ({
      messages: s.messages.map((m) =>
        m.id === id ? { ...m, content: m.content + delta } : m
      ),
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
