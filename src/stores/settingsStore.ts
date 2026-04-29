import { create } from "zustand";
import { persist } from "zustand/middleware";

export interface ProviderEntry {
  id: string;
  name: string;
  apiBase: string;
  models: string[];
  providerType: "openai-compat" | "anthropic";
}

interface SettingsStore {
  activeProviderId: string;
  activeModel: string;
  theme: "light" | "dark" | "system";
  fontSize: "small" | "medium" | "large";
  sendOnEnter: boolean;
  autoScroll: boolean;
  toolApprovalMode: "always" | "dangerous_only";
  showThinking: boolean | "auto";
  settingsOpen: boolean;
  feedbackOpen: boolean;
  // Phase 2: API Key 内存存储 (Phase 3 迁移到 OS Keychain)
  credentials: Record<string, { apiKey: string }>;

  setActiveProvider: (id: string) => void;
  setActiveModel: (model: string) => void;
  setTheme: (theme: "light" | "dark" | "system") => void;
  setFontSize: (size: "small" | "medium" | "large") => void;
  setSendOnEnter: (v: boolean) => void;
  setAutoScroll: (v: boolean) => void;
  setToolApprovalMode: (v: "always" | "dangerous_only") => void;
  setShowThinking: (v: boolean | "auto") => void;
  setSettingsOpen: (open: boolean) => void;
  setFeedbackOpen: (open: boolean) => void;
  setCredential: (providerId: string, apiKey: string) => void;
  removeCredential: (providerId: string) => void;
}

export const useSettingsStore = create<SettingsStore>()(
  persist(
    (set) => ({
      activeProviderId: "",
      activeModel: "",
      theme: "dark",
      fontSize: "medium",
      sendOnEnter: true,
      autoScroll: true,
      toolApprovalMode: "dangerous_only",
      showThinking: "auto" as const,
      settingsOpen: false,
      feedbackOpen: false,
      credentials: {},

      setActiveProvider: (id) => set({ activeProviderId: id }),
      setActiveModel: (model) => set({ activeModel: model }),
      setTheme: (theme) => set({ theme }),
      setFontSize: (fontSize) => set({ fontSize }),
      setSendOnEnter: (sendOnEnter) => set({ sendOnEnter }),
      setAutoScroll: (autoScroll) => set({ autoScroll }),
      setToolApprovalMode: (toolApprovalMode) => set({ toolApprovalMode }),
      setShowThinking: (showThinking) => set({ showThinking }),
      setSettingsOpen: (settingsOpen) => set({ settingsOpen }),
      setFeedbackOpen: (feedbackOpen) => set({ feedbackOpen }),
      setCredential: (providerId, apiKey) =>
        set((s) => ({
          credentials: { ...s.credentials, [providerId]: { apiKey } },
        })),
      removeCredential: (providerId) =>
        set((s) => {
          const { [providerId]: _, ...rest } = s.credentials;
          return { credentials: rest };
        }),
    }),
    {
      name: "crosschat-settings",
      // 不持久化临时状态
      partialize: (state) => ({
        activeProviderId: state.activeProviderId,
        activeModel: state.activeModel,
        theme: state.theme,
        fontSize: state.fontSize,
        sendOnEnter: state.sendOnEnter,
        autoScroll: state.autoScroll,
        toolApprovalMode: state.toolApprovalMode,
        showThinking: state.showThinking,
        credentials: state.credentials,
      }),
    }
  )
);
