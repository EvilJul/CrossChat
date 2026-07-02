import { useSettingsStore } from "../../stores/settingsStore";
import { cn } from "../../lib/cn";

function SettingRow({ label, description, children }: { label: string; description?: string; children: React.ReactNode }) {
  return (
    <div className="flex items-center justify-between px-3.5 py-3 rounded-lg border border-ds-border bg-ds-surface-elevated/50">
      <div className="min-w-0">
        <span className="text-sm text-ds-text-primary">{label}</span>
        {description && <p className="text-xs text-ds-muted mt-0.5">{description}</p>}
      </div>
      <div className="flex-shrink-0 ml-4 min-w-0">
        {children}
      </div>
    </div>
  );
}

function Toggle({ checked, onChange }: { checked: boolean; onChange: (v: boolean) => void }) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      onClick={() => onChange(!checked)}
      className={cn(
        "w-9 h-5 rounded-full relative transition-colors flex-shrink-0 cursor-pointer",
        checked ? "bg-ds-accent" : "bg-zinc-300 dark:bg-zinc-600"
      )}
    >
      <span className={cn(
        "block w-3.5 h-3.5 bg-white rounded-full shadow-sm transition-transform absolute top-0.5",
        checked ? "translate-x-[18px]" : "translate-x-0.5"
      )} />
    </button>
  );
}

export default function GeneralTab() {
  const showThinking = useSettingsStore((s) => s.showThinking);
  const setShowThinking = useSettingsStore((s) => s.setShowThinking);
  const showToolCalls = useSettingsStore((s) => s.showToolCalls);
  const setShowToolCalls = useSettingsStore((s) => s.setShowToolCalls);
  const theme = useSettingsStore((s) => s.theme);
  const setTheme = useSettingsStore((s) => s.setTheme);
  const sendOnEnter = useSettingsStore((s) => s.sendOnEnter);
  const setSendOnEnter = useSettingsStore((s) => s.setSendOnEnter);

  return (
    <div className="space-y-2">
      <SettingRow label="主题" description="深色/浅色/跟随系统">
        <select
          value={theme}
          onChange={(e) => setTheme(e.target.value as "light" | "dark" | "system")}
          className="text-xs rounded-md border border-ds-border bg-ds-bg-main px-2 py-1.5 text-ds-text-primary focus:outline-none focus:ring-1 focus:ring-ds-accent/40 focus:border-ds-accent transition-colors"
        >
          <option value="dark">深色</option>
          <option value="light">浅色</option>
          <option value="system">跟随系统</option>
        </select>
      </SettingRow>

      <SettingRow label="思考链" description="显示推理过程（DeepSeek R1 等支持）">
        <select
          value={String(showThinking)}
          onChange={(e) => {
            const v = e.target.value;
            setShowThinking(v === "auto" ? "auto" : v === "true");
          }}
          className="text-xs rounded-md border border-ds-border bg-ds-bg-main px-2 py-1.5 text-ds-text-primary focus:outline-none focus:ring-1 focus:ring-ds-accent/40 focus:border-ds-accent transition-colors"
        >
          <option value="auto">自动</option>
          <option value="true">始终展开</option>
          <option value="false">始终隐藏</option>
        </select>
      </SettingRow>

      <SettingRow label="工具调用" description="显示 AI 使用的工具及结果">
        <Toggle checked={showToolCalls} onChange={setShowToolCalls} />
      </SettingRow>

      <SettingRow label="Enter 发送" description="Enter 直接发送消息">
        <Toggle checked={sendOnEnter} onChange={setSendOnEnter} />
      </SettingRow>
    </div>
  );
}
