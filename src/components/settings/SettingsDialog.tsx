import { useState } from "react";
import * as Dialog from "@radix-ui/react-dialog";
import { X, Settings2, PlugZap, SlidersHorizontal } from "lucide-react";
import { useSettingsStore } from "../../stores/settingsStore";
import ProviderTab from "./ProviderTab";
import McpSection from "./McpSection";
import GeneralTab from "./GeneralTab";
import { cn } from "../../lib/cn";

const TABS = [
  { key: "provider", label: "模型", icon: PlugZap },
  { key: "mcp", label: "MCP", icon: Settings2 },
  { key: "general", label: "通用", icon: SlidersHorizontal },
] as const;

export default function SettingsDialog() {
  const settingsOpen = useSettingsStore((s) => s.settingsOpen);
  const setSettingsOpen = useSettingsStore((s) => s.setSettingsOpen);
  const [tab, setTab] = useState<string>("provider");

  return (
    <Dialog.Root open={settingsOpen} onOpenChange={setSettingsOpen}>
      <Dialog.Trigger asChild>
        <button className="p-1.5 rounded-xl text-zinc-400 hover:text-zinc-600 dark:hover:text-zinc-300 hover:bg-zinc-100 dark:hover:bg-zinc-800 transition-all duration-200"
          title="设置">
          <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" />
            <circle cx="12" cy="12" r="3" />
          </svg>
        </button>
      </Dialog.Trigger>

      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 bg-black/40 backdrop-blur-sm z-50" />
        <Dialog.Content className="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 bg-white dark:bg-zinc-900 rounded-2xl shadow-2xl w-[620px] max-h-[82vh] z-50 border border-zinc-200 dark:border-zinc-700 flex flex-col">
          {/* Header */}
          <div className="flex items-center justify-between px-5 py-3.5 border-b border-zinc-200 dark:border-zinc-700 flex-shrink-0">
            <Dialog.Title className="text-sm font-semibold text-zinc-900 dark:text-zinc-100">设置</Dialog.Title>
            <Dialog.Close className="p-1 rounded-lg text-zinc-400 hover:text-zinc-600 dark:hover:text-zinc-300 hover:bg-zinc-100 dark:hover:bg-zinc-800">
              <X className="w-4 h-4" />
            </Dialog.Close>
          </div>

          {/* Tabs */}
          <div className="flex border-b border-zinc-200 dark:border-zinc-700 flex-shrink-0 px-5">
            {TABS.map(({ key, label, icon: Icon }) => (
              <button
                key={key}
                onClick={() => setTab(key)}
                className={cn(
                  "flex items-center gap-1.5 px-4 py-2.5 text-xs font-medium border-b-2 transition-colors -mb-px",
                  tab === key
                    ? "border-slate-500 text-slate-700 dark:text-slate-300"
                    : "border-transparent text-zinc-400 hover:text-zinc-600 dark:hover:text-zinc-300"
                )}
              >
                <Icon className="w-3.5 h-3.5" />
                {label}
              </button>
            ))}
          </div>

          {/* Content */}
          <div className="flex-1 overflow-y-auto overflow-x-hidden chat-scrollbar p-5">
            {tab === "provider" && <ProviderTab />}
            {tab === "mcp" && <McpSection />}
            {tab === "general" && <GeneralTab />}
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
