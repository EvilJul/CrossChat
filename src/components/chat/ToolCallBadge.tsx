import { useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { Wrench, ChevronDown } from "lucide-react";
import type { ToolCallState } from "../../stores/chatStore";
import { cn } from "../../lib/cn";

interface Props {
  toolCall: ToolCallState;
}

export default function ToolCallBadge({ toolCall }: Props) {
  const [expanded, setExpanded] = useState(false);

  const statusConfig = {
    running: { label: "执行中", color: "text-amber-600 dark:text-amber-400", bg: "bg-amber-50 dark:bg-amber-900/20", border: "border-amber-200 dark:border-amber-800", pulse: true },
    executing: { label: "运行中", color: "text-blue-600 dark:text-blue-400", bg: "bg-blue-50 dark:bg-blue-900/20", border: "border-blue-200 dark:border-blue-800", pulse: true },
    completed: { label: "完成", color: "text-emerald-600 dark:text-emerald-400", bg: "bg-emerald-50 dark:bg-emerald-900/20", border: "border-emerald-200 dark:border-emerald-800", pulse: false },
    failed: { label: "失败", color: "text-red-600 dark:text-red-400", bg: "bg-red-50 dark:bg-red-900/20", border: "border-red-200 dark:border-red-800", pulse: false },
    pending: { label: "等待中", color: "text-zinc-500 dark:text-zinc-400", bg: "bg-zinc-50 dark:bg-zinc-800", border: "border-zinc-200 dark:border-zinc-700", pulse: false },
  };
  const config = statusConfig[toolCall.status] || statusConfig.pending;

  return (
    <div className="text-xs">
      <button
        onClick={() => setExpanded(!expanded)}
        className={cn(
          "w-full px-3 py-2 rounded-xl flex items-center gap-2 transition-all duration-200 border",
          "bg-white dark:bg-zinc-800/50 hover:bg-zinc-50 dark:hover:bg-zinc-800",
          "border-zinc-200 dark:border-zinc-700",
          config.pulse && "animate-pulse-slow"
        )}
      >
        <Wrench className="w-3.5 h-3.5 text-zinc-400 flex-shrink-0" />
        <span className="font-medium text-zinc-700 dark:text-zinc-300 flex-1 text-left truncate">
          {toolCall.name}
        </span>
        <span className={cn("text-[10px] px-2 py-0.5 rounded-full font-medium border", config.bg, config.color, config.border)}>
          {config.label}
        </span>
        <ChevronDown className={cn("w-3 h-3 text-zinc-400 transition-transform duration-200", expanded && "rotate-180")} />
      </button>

      <AnimatePresence>
        {expanded && (
          <motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ type: "spring", stiffness: 350, damping: 28 }}
            className="overflow-hidden"
          >
            <div className="mt-2 ml-3 pl-3 border-l-2 border-zinc-200 dark:border-zinc-700 space-y-2 py-2">
              {toolCall.arguments && (
                <div>
                  <span className="text-[10px] text-zinc-500 dark:text-zinc-400 font-semibold uppercase tracking-wider">参数</span>
                  <pre className="text-xs text-zinc-600 dark:text-zinc-400 mt-1.5 bg-zinc-50 dark:bg-zinc-900/50 rounded-lg px-3 py-2 overflow-x-auto whitespace-pre-wrap break-all font-mono max-h-32 overflow-y-auto border border-zinc-200 dark:border-zinc-800">
                    {(() => {
                      try { return JSON.stringify(JSON.parse(toolCall.arguments), null, 2); }
                      catch { return toolCall.arguments; }
                    })()}
                  </pre>
                </div>
              )}
              {toolCall.result && (
                <div>
                  <span className="text-[10px] text-zinc-500 dark:text-zinc-400 font-semibold uppercase tracking-wider">结果</span>
                  <div className="text-xs text-zinc-600 dark:text-zinc-400 mt-1.5 bg-zinc-50 dark:bg-zinc-900/50 rounded-lg px-3 py-2 max-h-40 overflow-y-auto whitespace-pre-wrap break-all font-mono border border-zinc-200 dark:border-zinc-800">
                    {toolCall.result}
                  </div>
                </div>
              )}
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
