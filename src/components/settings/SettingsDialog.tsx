import { useState } from "react";
import * as Dialog from "@radix-ui/react-dialog";
import * as Tabs from "@radix-ui/react-tabs";
import { X, Settings } from "lucide-react";
import { useSettingsStore } from "../../stores/settingsStore";
import { cn } from "../../lib/cn";
import ProviderTab from "./ProviderTab";
import McpSection from "./McpSection";
import GeneralTab from "./GeneralTab";

const TABS = [
  { value: "provider", label: "AI 模型" },
  { value: "mcp", label: "MCP 工具" },
  { value: "general", label: "通用" },
];

export default function SettingsDialog() {
  const settingsOpen = useSettingsStore((s) => s.settingsOpen);
  const setSettingsOpen = useSettingsStore((s) => s.setSettingsOpen);
  const [tab, setTab] = useState("provider");

  return (
    <Dialog.Root open={settingsOpen} onOpenChange={setSettingsOpen}>
      <Dialog.Trigger asChild>
        <button className="p-1.5 rounded-lg text-ds-muted hover:text-ds-text-primary hover:bg-ds-hover transition-colors" title="设置">
          <Settings className="w-4 h-4" />
        </button>
      </Dialog.Trigger>

      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 bg-black/40 z-50 data-[state=open]:animate-in data-[state=open]:fade-in duration-200" />
        <Dialog.Content className="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[680px] max-h-[80vh] z-50 bg-ds-surface-card backdrop-blur-xl rounded-2xl border border-ds-border shadow-2xl flex flex-col data-[state=open]:animate-in data-[state=open]:fade-in data-[state=open]:zoom-in-95 duration-200 overflow-hidden">
          <div className="flex items-center justify-between px-6 py-4 border-b border-ds-border flex-shrink-0">
            <Dialog.Title className="text-base font-semibold text-ds-text-primary">
              Settings
            </Dialog.Title>
            <Dialog.Close className="p-1 rounded-md text-ds-muted hover:text-ds-text-primary hover:bg-ds-hover transition-colors">
              <X className="w-4 h-4" />
            </Dialog.Close>
          </div>

          <Tabs.Root value={tab} onValueChange={setTab} className="flex-1 flex flex-col overflow-hidden">
            <Tabs.List className="flex gap-0 px-6 border-b border-ds-border flex-shrink-0" role="tablist">
              {TABS.map((t) => (
                <Tabs.Trigger
                  key={t.value}
                  value={t.value}
                  className={cn(
                      "relative px-4 py-2.5 text-sm font-medium transition-colors",
                      "text-ds-muted hover:text-ds-text-primary",
                      "data-[state=active]:text-ds-accent",
                      "after:absolute after:bottom-0 after:left-4 after:right-4 after:h-0.5 after:rounded-full",
                      "after:bg-ds-accent after:scale-x-0 after:transition-transform after:duration-200",
                      "data-[state=active]:after:scale-x-100"
                    )}>
                  {t.label}
                </Tabs.Trigger>
              ))}
            </Tabs.List>

            <div className="flex-1 overflow-y-auto min-h-0 p-6">
              <Tabs.Content value="provider">
                <ProviderTab />
              </Tabs.Content>
              <Tabs.Content value="mcp">
                <McpSection />
              </Tabs.Content>
              <Tabs.Content value="general">
                <GeneralTab />
              </Tabs.Content>
            </div>
          </Tabs.Root>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
