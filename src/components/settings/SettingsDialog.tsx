import { useState } from "react";
import * as Dialog from "@radix-ui/react-dialog";
import * as Tabs from "@radix-ui/react-tabs";
import { X, Sparkles, Plug, Settings as SettingsIcon } from "lucide-react";
import { useSettingsStore } from "../../stores/settingsStore";
import ProviderTab from "./ProviderTab";
import McpSection from "./McpSection";
import GeneralTab from "./GeneralTab";

export default function SettingsDialog() {
  const settingsOpen = useSettingsStore((s) => s.settingsOpen);
  const setSettingsOpen = useSettingsStore((s) => s.setSettingsOpen);
  const [tab, setTab] = useState<string>("provider");

  return (
    <Dialog.Root open={settingsOpen} onOpenChange={setSettingsOpen}>
      <Dialog.Trigger asChild>
        <button 
          className="p-2 rounded-xl text-zinc-400 hover:text-zinc-600 dark:hover:text-zinc-300 hover:bg-zinc-100 dark:hover:bg-zinc-800 transition-all duration-200 group"
          title="设置"
        >
          <SettingsIcon className="w-4 h-4 group-hover:rotate-90 transition-transform duration-300" />
        </button>
      </Dialog.Trigger>

      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 bg-black/50 backdrop-blur-md z-50 animate-in fade-in duration-300" />
        <Dialog.Content className="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 bg-white dark:bg-zinc-900 rounded-3xl shadow-2xl w-[900px] max-h-[85vh] z-50 border border-zinc-200/50 dark:border-zinc-700/50 flex flex-col animate-in fade-in zoom-in-95 duration-300 overflow-hidden">
          
          {/* 渐变头部 */}
          <div className="relative px-8 py-6 border-b border-zinc-200/50 dark:border-zinc-700/50 bg-gradient-to-br from-purple-500/10 via-blue-500/10 to-indigo-500/10 dark:from-purple-500/20 dark:via-blue-500/20 dark:to-indigo-500/20 flex-shrink-0">
            <div className="absolute inset-0 bg-gradient-to-r from-purple-500/5 to-blue-500/5 dark:from-purple-500/10 dark:to-blue-500/10" />
            <div className="relative flex items-center justify-between">
              <Dialog.Title className="text-2xl font-bold bg-gradient-to-r from-purple-600 to-blue-600 bg-clip-text text-transparent truncate">
                Settings
              </Dialog.Title>
              <Dialog.Close className="p-2 rounded-xl text-zinc-400 hover:text-zinc-600 dark:hover:text-zinc-300 hover:bg-white/80 dark:hover:bg-zinc-800/80 transition-all duration-200 flex-shrink-0 ml-4">
                <X className="w-5 h-5" />
              </Dialog.Close>
            </div>
          </div>

          {/* 标签页导航 */}
          <Tabs.Root value={tab} onValueChange={setTab} className="flex-1 flex flex-col overflow-hidden min-h-0">
            <Tabs.List className="flex gap-2 px-8 py-4 border-b border-zinc-200/50 dark:border-zinc-700/50 bg-zinc-50/50 dark:bg-zinc-900/50 flex-shrink-0">
              <Tabs.Trigger
                value="provider"
                className="group flex items-center gap-2 px-4 py-2.5 rounded-xl text-sm font-medium transition-all duration-200 data-[state=active]:bg-gradient-to-r data-[state=active]:from-purple-500 data-[state=active]:to-blue-500 data-[state=active]:text-white data-[state=active]:shadow-lg data-[state=active]:shadow-purple-500/30 text-zinc-600 dark:text-zinc-400 hover:bg-white dark:hover:bg-zinc-800 hover:text-zinc-900 dark:hover:text-zinc-100 whitespace-nowrap"
              >
                <Sparkles className="w-4 h-4 flex-shrink-0" />
                <span className="truncate">AI 模型</span>
              </Tabs.Trigger>
              
              <Tabs.Trigger
                value="mcp"
                className="group flex items-center gap-2 px-4 py-2.5 rounded-xl text-sm font-medium transition-all duration-200 data-[state=active]:bg-gradient-to-r data-[state=active]:from-purple-500 data-[state=active]:to-blue-500 data-[state=active]:text-white data-[state=active]:shadow-lg data-[state=active]:shadow-purple-500/30 text-zinc-600 dark:text-zinc-400 hover:bg-white dark:hover:bg-zinc-800 hover:text-zinc-900 dark:hover:text-zinc-100 whitespace-nowrap"
              >
                <Plug className="w-4 h-4 flex-shrink-0" />
                <span className="truncate">MCP 工具</span>
              </Tabs.Trigger>
              
              <Tabs.Trigger
                value="general"
                className="group flex items-center gap-2 px-4 py-2.5 rounded-xl text-sm font-medium transition-all duration-200 data-[state=active]:bg-gradient-to-r data-[state=active]:from-purple-500 data-[state=active]:to-blue-500 data-[state=active]:text-white data-[state=active]:shadow-lg data-[state=active]:shadow-purple-500/30 text-zinc-600 dark:text-zinc-400 hover:bg-white dark:hover:bg-zinc-800 hover:text-zinc-900 dark:hover:text-zinc-100 whitespace-nowrap"
              >
                <SettingsIcon className="w-4 h-4 flex-shrink-0" />
                <span className="truncate">通用设置</span>
              </Tabs.Trigger>
            </Tabs.List>

            {/* 内容区域 - 禁止横向滚动 */}
            <div className="flex-1 overflow-y-auto overflow-x-hidden chat-scrollbar min-h-0">
              <Tabs.Content value="provider" className="p-8 focus:outline-none">
                <ProviderTab />
              </Tabs.Content>
              
              <Tabs.Content value="mcp" className="p-8 focus:outline-none">
                <McpSection />
              </Tabs.Content>
              
              <Tabs.Content value="general" className="p-8 focus:outline-none">
                <GeneralTab />
              </Tabs.Content>
            </div>
          </Tabs.Root>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
