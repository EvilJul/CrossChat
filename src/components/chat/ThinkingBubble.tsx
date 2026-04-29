import { useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { ChevronDown } from "lucide-react";
import { cn } from "../../lib/cn";

interface Props {
  content: string;
  isStreaming: boolean;
  defaultExpanded?: boolean;
}

export default function ThinkingBubble({ content, isStreaming, defaultExpanded = false }: Props) {
  const [expanded, setExpanded] = useState(defaultExpanded);

  if (!content && !isStreaming) return null;

  return (
    <div className="mb-2">
      <button
        onClick={() => setExpanded(!expanded)}
        className={cn(
          "flex items-center gap-1.5 text-xs px-2.5 py-1 rounded-xl transition-all duration-200 w-full text-left",
          "bg-amber-50/80 dark:bg-amber-900/10 hover:bg-amber-100 dark:hover:bg-amber-900/20 border border-amber-200/40 dark:border-amber-800/20",
          "text-amber-800 dark:text-amber-300"
        )}
      >
        <span className="text-[10px] opacity-60">●</span>
        <span className="flex-1 font-medium">Thinking...</span>
        {isStreaming && (
          <span className="text-[10px] opacity-50 animate-pulse">...</span>
        )}
        <motion.div
          animate={{ rotate: expanded ? 180 : 0 }}
          transition={{ type: "spring", stiffness: 300, damping: 22 }}
        >
          <ChevronDown className="w-3 h-3 opacity-50" />
        </motion.div>
      </button>

      <AnimatePresence initial={false}>
        {expanded && (
          <motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ type: "spring", stiffness: 260, damping: 24 }}
            className="overflow-hidden"
          >
            <div
              className={cn(
                "mt-1.5 px-3 py-2 rounded-xl text-xs leading-relaxed",
                "bg-amber-50/40 dark:bg-amber-900/5 border border-amber-200/30 dark:border-amber-800/20",
                "text-amber-800/80 dark:text-amber-300/80",
                isStreaming && "streaming-cursor"
              )}
            >
              {content || (isStreaming ? "Thinking..." : "")}
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
