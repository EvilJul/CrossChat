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
    running: { label: "执行中", color: "text-amber-500", bg: "bg-amber-100 dark:bg-amber-900/20", pulse: true },
    executing: { label: "运行中", color: "text-blue-500", bg: "bg-blue-100 dark:bg-blue-900/20", pulse: true },
    completed: { label: "完成", color: "text-emerald-600 dark:text-emerald-400", bg: "bg-emerald-100 dark:bg-emerald-900/20", pulse: false },
    failed: { label: "失败", color: "text-red-500", bg: "bg-red-100 dark:bg-red-900/20", pulse: false },
    pending: { label: "等待中", color: "text-zinc-400", bg: "bg-zinc-100 dark:bg-zinc-800", pulse: false },
  };
  const config = statusConfig[toolCall.status] || statusConfig.pending;

  return (
    <div className="text-xs">
      <button
        onClick={() => setExpanded(!expanded)}
        className={cn(
          "w-full px-2.5 py-1 rounded-xl flex items-center gap-2 transition-colors",
          "bg-white/60 dark:bg-zinc-700/60 hover:bg-white/80 dark:hover:bg-zinc-700/80",
          config.pulse && "animate-pulse"
        )}
      >
        <Wrench className="w-3 h-3 text-zinc-400 flex-shrink-0" />
        <span className="font-medium text-zinc-600 dark:text-zinc-300 flex-1 text-left truncate">
          {toolCall.name}
        </span>
        <span className={cn("text-[10px] px-1.5 py-0.5 rounded-full font-medium", config.bg, config.color)}>
          {config.label}
        </span>
        <ChevronDown className={cn("w-3 h-3 text-zinc-400 transition-transform", expanded && "rotate-180")} />
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
            <div className="mt-1 ml-2 pl-3 border-l-2 border-zinc-200 dark:border-zinc-700 space-y-2 py-1">
              {toolCall.arguments && (
                <div>
                  <span className="text-[10px] text-zinc-400 font-medium uppercase tracking-wider">参数</span>
                  <pre className="text-xs text-zinc-600 dark:text-zinc-400 mt-1 bg-zinc-50 dark:bg-zinc-800/50 rounded-lg px-2 py-1.5 overflow-x-auto whitespace-pre-wrap break-all font-mono max-h-32 overflow-y-auto">
                    {(() => {
                      try { return JSON.stringify(JSON.parse(toolCall.arguments), null, 2); }
                      catch { return toolCall.arguments; }
                    })()}
                  </pre>
                </div>
              )}
              {toolCall.result && (
                <div>
                  <span className="text-[10px] text-zinc-400 font-medium uppercase tracking-wider">结果</span>
                  <div className="text-xs text-zinc-600 dark:text-zinc-400 mt-1 bg-zinc-50 dark:bg-zinc-800/50 rounded-lg px-2 py-1.5 max-h-40 overflow-y-auto whitespace-pre-wrap break-all font-mono">
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
