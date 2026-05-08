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
      <h3 className="text-xs font-medium bg-gradient-to-r from-purple-600 to-blue-600 bg-clip-text text-transparent uppercase tracking-wider">通用设置</h3>

      <div className="space-y-3">
        <div className="flex items-center justify-between p-3.5 rounded-xl border border-zinc-200/70 dark:border-zinc-700/70 bg-gradient-to-br from-white to-zinc-50/50 dark:from-zinc-900 dark:to-zinc-800/50 hover:border-purple-200 dark:hover:border-purple-800/50 transition-all duration-200 group">
          <div>
            <span className="text-sm text-zinc-800 dark:text-zinc-200 font-medium">思考链</span>
            <p className="text-xs text-zinc-400 mt-0.5">显示推理过程（DeepSeek R1 等支持）</p>
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

        <div className="flex items-center justify-between p-3.5 rounded-xl border border-zinc-200/70 dark:border-zinc-700/70 bg-gradient-to-br from-white to-zinc-50/50 dark:from-zinc-900 dark:to-zinc-800/50 hover:border-purple-200 dark:hover:border-purple-800/50 transition-all duration-200 group">
          <div>
            <span className="text-sm text-zinc-800 dark:text-zinc-200 font-medium">主题</span>
            <p className="text-xs text-zinc-400 mt-0.5">深色/浅色/自动</p>
          </div>
          <Select value={theme} onChange={(e) => setTheme(e.target.value as "light" | "dark" | "system")}>
            <option value="dark">深色</option>
            <option value="light">浅色</option>
            <option value="system">跟随系统</option>
          </Select>
        </div>

        <div className="flex items-center justify-between p-3.5 rounded-xl border border-zinc-200/70 dark:border-zinc-700/70 bg-gradient-to-br from-white to-zinc-50/50 dark:from-zinc-900 dark:to-zinc-800/50 hover:border-purple-200 dark:hover:border-purple-800/50 transition-all duration-200 group">
          <div>
            <span className="text-sm text-zinc-800 dark:text-zinc-200 font-medium">Enter 发送</span>
            <p className="text-xs text-zinc-400 mt-0.5">Enter 直接发送消息</p>
          </div>
          <Switch.Root
            checked={sendOnEnter}
            onCheckedChange={setSendOnEnter}
            className="w-10 h-5 rounded-full bg-zinc-300 dark:bg-zinc-600 data-[state=checked]:bg-gradient-to-r data-[state=checked]:from-purple-500 data-[state=checked]:to-blue-500 relative transition-all duration-200 shadow-sm data-[state=checked]:shadow-purple-500/30"
          >
            <Switch.Thumb className="block w-4 h-4 bg-white rounded-full shadow-md transition-transform translate-x-0.5 data-[state=checked]:translate-x-[22px]" />
          </Switch.Root>
        </div>
      </div>
    </div>
  );
}
