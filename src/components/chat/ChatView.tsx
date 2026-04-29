import { useEffect } from "react";
import { useChatStore } from "../../stores/chatStore";
import { useSettingsStore } from "../../stores/settingsStore";
import { useProviderStore } from "../../stores/providerStore";
import { useWorkspaceStore } from "../../stores/workspaceStore";
import { setupBuiltinCommands } from "../../lib/builtinCommands";
import { useChat } from "../../hooks/useChat";
import { useCheckpoint } from "../../hooks/useCheckpoint";
import { useContextUsage } from "../../hooks/useContextUsage";
import ChatInput from "./ChatInput";
import MessageList from "./MessageList";
import SettingsDialog from "../settings/SettingsDialog";
import FeedbackDialog from "../settings/FeedbackDialog";
import WorkspaceSidebar from "./WorkspaceSidebar";
import FilePreviewPanel from "./FilePreviewPanel";
import { PanelLeftOpen } from "lucide-react";

export default function ChatView() {
  const addMessage = useChatStore((s) => s.addMessage);

  const activeProviderId = useSettingsStore((s) => s.activeProviderId);
  const activeModel = useSettingsStore((s) => s.activeModel);

  const isSidebarOpen = useWorkspaceStore((s) => s.isSidebarOpen);
  const setSidebarOpen = useWorkspaceStore((s) => s.setSidebarOpen);
  const currentDir = useWorkspaceStore((s) => s.currentDir);

  // Hooks: 业务逻辑
  const { isGenerating, send, stop } = useChat({ workDir: currentDir });
  const { save: saveCheckpoint, restore: restoreCheckpoint } = useCheckpoint({ activeProviderId, activeModel, workDir: currentDir });
  const { percent, color, tooltip } = useContextUsage();

  // 停止时自动保存检查点
  const handleStop = () => {
    const msgs = stop();
    if (msgs.length > 0) saveCheckpoint();
  };

  // 注册斜杠命令
  useEffect(() => {
    setupBuiltinCommands({
      clearMessages: () => useChatStore.getState().clearMessages(),
      onNewSession: () => { useChatStore.getState().clearMessages(); return Promise.resolve(); },
      onContinue: restoreCheckpoint,
      openWorkspace: () => setSidebarOpen(true),
      openSettings: () => useSettingsStore.getState().setSettingsOpen(true),
      openFeedback: () => useSettingsStore.getState().setFeedbackOpen(true),
      getCurrentProvider: () => {
        const pid = useSettingsStore.getState().activeProviderId;
        return useProviderStore.getState().providers.find((p) => p.id === pid)?.name || "";
      },
      getCurrentModel: () => useSettingsStore.getState().activeModel,
    });
  }, [setSidebarOpen, restoreCheckpoint]);

  return (
    <div className="flex h-screen bg-white dark:bg-zinc-950">
      <WorkspaceSidebar />

      <div className="flex-1 flex flex-col min-w-0">
        <header className="border-b border-zinc-200/70 dark:border-zinc-800/70 px-5 py-3 flex items-center justify-between bg-white/80 dark:bg-zinc-950/80 backdrop-blur-sm flex-shrink-0">
          <div className="flex items-center gap-2">
            {!isSidebarOpen && (
              <button onClick={() => setSidebarOpen(true)} className="p-1 rounded-lg text-zinc-400 hover:text-zinc-600 dark:hover:text-zinc-300 hover:bg-zinc-100 dark:hover:bg-zinc-800 transition-colors">
                <PanelLeftOpen className="w-4 h-4" />
              </button>
            )}
            <div className="w-7 h-7 rounded-lg bg-slate-700 dark:bg-slate-600 flex items-center justify-center">
              <svg className="w-3.5 h-3.5 text-white" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round">
                <rect x="3" y="3" width="18" height="14" rx="2" />
                <path d="M7 21h10M12 17v4" />
              </svg>
            </div>
            <h1 className="text-sm font-semibold text-zinc-800 dark:text-zinc-200">CrossChat</h1>
            <span className="text-xs text-zinc-400 dark:text-zinc-500 bg-zinc-100 dark:bg-zinc-800 px-2 py-0.5 rounded-full">v0.2.1</span>
            {currentDir && <span className="text-[10px] text-zinc-400 truncate max-w-[200px]" title={currentDir}>{currentDir}</span>}
          </div>

          <div className="flex items-center gap-3">
            <div className="flex items-center gap-1.5" title={tooltip}>
              <div className="w-14 h-1.5 rounded-full bg-zinc-200 dark:bg-zinc-700 overflow-hidden">
                <div className={`h-full rounded-full transition-all duration-500 ${color}`} style={{ width: `${Math.max(percent, 2)}%` }} />
              </div>
              <span className={`text-[10px] font-mono ${percent > 90 ? "text-red-500" : percent > 60 ? "text-amber-500" : "text-zinc-400"}`}>{percent}%</span>
            </div>
            <FeedbackDialog />
            <SettingsDialog />
          </div>
        </header>

        <div className="flex-1 flex flex-col overflow-hidden">
          <MessageList />
        </div>

        <ChatInput
          onSend={send}
          onStop={handleStop}
          onCommandResult={(text) => addMessage({ id: `cmd-${Date.now()}`, role: "assistant", content: text, timestamp: Date.now() })}
          isGenerating={isGenerating}
        />
      </div>

      <FilePreviewPanel />
    </div>
  );
}
