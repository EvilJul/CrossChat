import { useSettingsStore } from "../../stores/settingsStore";
import { Select } from "../../shared/ui";
import * as Switch from "@radix-ui/react-switch";

export default function GeneralTab() {
  const showThinking = useSettingsStore((s) => s.showThinking);
  const setShowThinking = useSettingsStore((s) => s.setShowThinking);
  const theme = useSettingsStore((s) => s.theme);
  const setTheme = useSettingsStore((s) => s.setTheme);
  const sendOnEnter = useSettingsStore((s) => s.sendOnEnter);
  const setSendOnEnter = useSettingsStore((s) => s.setSendOnEnter);

  return (
    <div className="space-y-4">
      <h3 className="text-xs font-medium text-zinc-400 uppercase tracking-wider">通用设置</h3>

      <div className="space-y-3">
        <div className="flex items-center justify-between p-3 rounded-xl border border-zinc-200 dark:border-zinc-700">
          <div>
            <span className="text-sm text-zinc-800 dark:text-zinc-200">思考链</span>
            <p className="text-xs text-zinc-400">显示推理过程（DeepSeek R1 等支持）</p>
          </div>
          <Select value={String(showThinking)} onChange={(e) => {
            const v = e.target.value;
            setShowThinking(v === "auto" ? "auto" : v === "true");
          }}>
            <option value="auto">自动</option>
            <option value="true">始终展开</option>
            <option value="false">始终隐藏</option>
          </Select>
        </div>

        <div className="flex items-center justify-between p-3 rounded-xl border border-zinc-200 dark:border-zinc-700">
          <div>
            <span className="text-sm text-zinc-800 dark:text-zinc-200">主题</span>
            <p className="text-xs text-zinc-400">深色/浅色/自动</p>
          </div>
          <Select value={theme} onChange={(e) => setTheme(e.target.value as "light" | "dark" | "system")}>
            <option value="dark">深色</option>
            <option value="light">浅色</option>
            <option value="system">跟随系统</option>
          </Select>
        </div>

        <div className="flex items-center justify-between p-3 rounded-xl border border-zinc-200 dark:border-zinc-700">
          <div>
            <span className="text-sm text-zinc-800 dark:text-zinc-200">Enter 发送</span>
            <p className="text-xs text-zinc-400">Enter 直接发送消息</p>
          </div>
          <Switch.Root
            checked={sendOnEnter}
            onCheckedChange={setSendOnEnter}
            className="w-9 h-5 rounded-full bg-zinc-300 dark:bg-zinc-600 data-[state=checked]:bg-slate-500 relative transition-colors"
          >
            <Switch.Thumb className="block w-4 h-4 bg-white rounded-full shadow transition-transform translate-x-0.5 data-[state=checked]:translate-x-4" />
          </Switch.Root>
        </div>
      </div>
    </div>
  );
}
