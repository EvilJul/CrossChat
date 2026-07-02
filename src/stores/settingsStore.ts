import { create } from "zustand";
import { persist } from "zustand/middleware";
import { invoke } from "@tauri-apps/api/core";

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
  showToolCalls: boolean;
  settingsOpen: boolean;
  feedbackOpen: boolean;
  // Phase 3: API Key 存储在 OS Keychain（后端 command），
  // credentials 仅作为内存镜像供组件同步读取，不再明文持久化到 localStorage。
  credentials: Record<string, { apiKey: string }>;
  // 仅持久化 provider id 列表（不含 key 值），用于下次启动时从 keychain 加载。
  credentialIds: string[];

  setActiveProvider: (id: string) => void;
  setActiveModel: (model: string) => void;
  setTheme: (theme: "light" | "dark" | "system") => void;
  setFontSize: (size: "small" | "medium" | "large") => void;
  setSendOnEnter: (v: boolean) => void;
  setAutoScroll: (v: boolean) => void;
  setToolApprovalMode: (v: "always" | "dangerous_only") => void;
  setShowThinking: (v: boolean | "auto") => void;
  setShowToolCalls: (v: boolean) => void;
  setSettingsOpen: (open: boolean) => void;
  setFeedbackOpen: (open: boolean) => void;
  setCredential: (providerId: string, apiKey: string) => void;
  removeCredential: (providerId: string) => void;
  loadCredentials: () => Promise<void>;
}

// localStorage 中 zustand persist 使用的 key
const STORAGE_KEY = "crosschat-settings";

/**
 * 从 localStorage 里读取旧版本明文持久化的 credentials，并把明文清除。
 * 返回旧的 credentials（可能为空对象）。清除是幂等的：清除后再次调用返回空对象。
 */
function extractAndPurgeLegacyCredentials(): Record<string, { apiKey: string }> {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw);
    // zustand persist 结构为 { state: {...}, version: n }
    const legacy = parsed?.state?.credentials;
    if (!legacy || typeof legacy !== "object") return {};

    const result: Record<string, { apiKey: string }> = {};
    for (const [providerId, entry] of Object.entries(legacy)) {
      const apiKey = (entry as { apiKey?: unknown })?.apiKey;
      if (typeof apiKey === "string" && apiKey.length > 0) {
        result[providerId] = { apiKey };
      }
    }

    // 清除 localStorage 中的明文 credentials（重新写回不含 credentials 的对象），保证幂等。
    if (parsed?.state && "credentials" in parsed.state) {
      delete parsed.state.credentials;
      localStorage.setItem(STORAGE_KEY, JSON.stringify(parsed));
    }

    return result;
  } catch (e) {
    console.error("[settingsStore] 解析旧 credentials 失败", e);
    return {};
  }
}

export const useSettingsStore = create<SettingsStore>()(
  persist(
    (set, get) => ({
      activeProviderId: "",
      activeModel: "",
      theme: "dark",
      fontSize: "medium",
      sendOnEnter: true,
      autoScroll: true,
      toolApprovalMode: "dangerous_only",
      showThinking: "auto" as const,
      showToolCalls: true,
      settingsOpen: false,
      feedbackOpen: false,
      credentials: {},
      credentialIds: [],

      setActiveProvider: (id) => set({ activeProviderId: id }),
      setActiveModel: (model) => set({ activeModel: model }),
      setTheme: (theme) => set({ theme }),
      setFontSize: (fontSize) => set({ fontSize }),
      setSendOnEnter: (sendOnEnter) => set({ sendOnEnter }),
      setAutoScroll: (autoScroll) => set({ autoScroll }),
      setToolApprovalMode: (toolApprovalMode) => set({ toolApprovalMode }),
      setShowThinking: (showThinking) => set({ showThinking }),
      setShowToolCalls: (showToolCalls) => set({ showToolCalls }),
      setSettingsOpen: (settingsOpen) => set({ settingsOpen }),
      setFeedbackOpen: (feedbackOpen) => set({ feedbackOpen }),
      setCredential: (providerId, apiKey) => {
        // 1) 更新内存镜像 + 并入 credentialIds（去重）
        set((s) => ({
          credentials: { ...s.credentials, [providerId]: { apiKey } },
          credentialIds: s.credentialIds.includes(providerId)
            ? s.credentialIds
            : [...s.credentialIds, providerId],
        }));
        // 2) 写入 keychain（异步，失败不阻塞 UI）
        invoke("set_api_key", { providerId, key: apiKey }).catch((e) =>
          console.error(`[settingsStore] set_api_key 失败: ${providerId}`, e)
        );
      },
      removeCredential: (providerId) => {
        // 1) 从内存镜像与 credentialIds 移除
        set((s) => {
          const rest = { ...s.credentials };
          delete rest[providerId];
          return {
            credentials: rest,
            credentialIds: s.credentialIds.filter((id) => id !== providerId),
          };
        });
        // 2) 从 keychain 删除（异步，失败不阻塞 UI）
        invoke("delete_api_key", { providerId }).catch((e) =>
          console.error(`[settingsStore] delete_api_key 失败: ${providerId}`, e)
        );
      },
      loadCredentials: async () => {
        // 步骤 A：迁移旧版本 localStorage 明文 credentials 到 keychain（幂等）。
        // extractAndPurgeLegacyCredentials 读取后即清除明文，重复调用返回空对象。
        const legacy = extractAndPurgeLegacyCredentials();
        for (const [providerId, entry] of Object.entries(legacy)) {
          try {
            await invoke("set_api_key", { providerId, key: entry.apiKey });
            set((s) => ({
              credentials: { ...s.credentials, [providerId]: { apiKey: entry.apiKey } },
              credentialIds: s.credentialIds.includes(providerId)
                ? s.credentialIds
                : [...s.credentialIds, providerId],
            }));
          } catch (e) {
            console.error(`[settingsStore] 迁移旧 credential 失败: ${providerId}`, e);
          }
        }

        // 步骤 B：遍历 credentialIds，从 keychain 读取 key 填入内存镜像。
        const ids = get().credentialIds;
        for (const providerId of ids) {
          try {
            const key = await invoke<string | null>("get_api_key", { providerId });
            if (key) {
              set((s) => ({
                credentials: { ...s.credentials, [providerId]: { apiKey: key } },
              }));
            }
          } catch (e) {
            console.error(`[settingsStore] get_api_key 失败: ${providerId}`, e);
          }
        }
      },
    }),
    {
      name: STORAGE_KEY,
      // 不持久化临时状态；关键：不再持久化 credentials（明文 key），
      // 仅持久化 credentialIds（provider id 列表）用于下次启动从 keychain 加载。
      partialize: (state) => ({
        activeProviderId: state.activeProviderId,
        activeModel: state.activeModel,
        theme: state.theme,
        fontSize: state.fontSize,
        sendOnEnter: state.sendOnEnter,
        autoScroll: state.autoScroll,
        toolApprovalMode: state.toolApprovalMode,
        showThinking: state.showThinking,
        showToolCalls: state.showToolCalls,
        credentialIds: state.credentialIds,
      }),
    }
  )
);
