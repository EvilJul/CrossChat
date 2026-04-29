import { useState, useMemo } from "react";
import { motion, AnimatePresence } from "framer-motion";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { type ChatMessage, type ToolCallState } from "../../stores/chatStore";
import { useSettingsStore } from "../../stores/settingsStore";
import { cn } from "../../lib/cn";
import { Avatar } from "../../shared/ui";
import StreamingText from "./StreamingText";
import ThinkingBubble from "./ThinkingBubble";
import ToolCallBadge from "./ToolCallBadge";
import { Check, Copy, Activity, ChevronDown, Wrench } from "lucide-react";

interface Props {
  message: ChatMessage;
}

/** 状态/进度信息折叠展示组件 */
function StatusMessagesBar({ messages }: { messages: string[] }) {
  const [expanded, setExpanded] = useState(false);
  return (
    <div className="mt-1.5">
      <button
        onClick={() => setExpanded(!expanded)}
        className="flex items-center gap-1 text-[11px] text-zinc-400 hover:text-zinc-600 dark:hover:text-zinc-300 transition-colors"
      >
        <Activity className="w-3 h-3" />
        <span>{messages.length} 条状态更新</span>
        <motion.div
          animate={{ rotate: expanded ? 180 : 0 }}
          transition={{ type: "spring", stiffness: 300, damping: 22 }}
        >
          <ChevronDown className="w-2.5 h-2.5" />
        </motion.div>
      </button>
      {expanded && (
        <motion.div
          initial={{ opacity: 0, height: 0 }}
          animate={{ opacity: 1, height: "auto" }}
          className="mt-1 space-y-0.5 pl-3 border-l-2 border-zinc-200 dark:border-zinc-700"
        >
          {messages.map((msg, i) => (
            <div key={i} className="text-[11px] text-zinc-400 dark:text-zinc-500">{msg}</div>
          ))}
        </motion.div>
      )}
    </div>
  );
}

/** 工具调用折叠展示组件 */
function ToolCallsCollapse({ toolCalls }: { toolCalls: ToolCallState[] }) {
  const [expanded, setExpanded] = useState(false);
  const completedCount = toolCalls.filter(tc => tc.status === "completed" || tc.status === "failed").length;
  const runningCount = toolCalls.filter(tc => tc.status === "running" || tc.status === "executing").length;

  return (
    <div className="mt-2">
      <button
        onClick={() => setExpanded(!expanded)}
        className={cn(
          "flex items-center gap-1.5 text-xs px-2.5 py-1 rounded-xl transition-all w-full text-left",
          "bg-blue-50/80 dark:bg-blue-900/10 hover:bg-blue-100 dark:hover:bg-blue-900/20",
          "border border-blue-200/40 dark:border-blue-800/20 text-blue-700 dark:text-blue-300",
          runningCount > 0 && "animate-pulse"
        )}
      >
        <Wrench className="w-3 h-3 opacity-60" />
        <span className="flex-1 font-medium">
          {runningCount > 0
            ? `工具调用 (${completedCount}/${toolCalls.length}) 执行中...`
            : `工具调用 (${toolCalls.length})`}
        </span>
        <motion.div
          animate={{ rotate: expanded ? 180 : 0 }}
          transition={{ type: "spring", stiffness: 300, damping: 22 }}
        >
          <ChevronDown className="w-3 h-3 opacity-50" />
        </motion.div>
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
            <div className="mt-1 space-y-1">
              {toolCalls.map(tc => (
                <ToolCallBadge key={tc.id} toolCall={tc} />
              ))}
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}

export default function MessageBubble({ message }: Props) {
  const [copied, setCopied] = useState(false);
  const isUser = message.role === "user";
  const isTool = message.role === "tool";
  const showThinking = useSettingsStore((s) => s.showThinking);
  const hasThinking = message.thinking || message.isThinking;

  // 当 showThinking 为 false 时，过滤掉内联的 <think>...</think> XML 标签
  const filteredContent = useMemo(() => {
    if (showThinking !== false) return message.content;
    return message.content.replace(/<\s*think\s*>[\s\S]*?<\s*\/\s*think\s*>/gi, "").trim();
  }, [message.content, showThinking]);

  const handleCopy = async () => {
    await navigator.clipboard.writeText(filteredContent);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <motion.div
      initial={{ opacity: 0, y: 6 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ type: "spring", stiffness: 400, damping: 28 }}
      className={cn(
        "flex gap-2.5 px-4 py-2.5 group",
        isUser ? "justify-end" : "justify-start"
      )}
    >
      {!isUser && <Avatar role={isTool ? "tool" : "assistant"} />}
      {isUser && <Avatar role="user" />}

      <div className={cn("max-w-[72%]", isUser ? "order-first" : "")}>
        {!isUser && showThinking !== false && hasThinking && (
          <ThinkingBubble
            content={message.thinking || ""}
            isStreaming={message.isThinking || false}
            defaultExpanded={showThinking === true}
          />
        )}

        <div
          className={cn(
            "rounded-2xl px-4 py-2.5 text-sm leading-relaxed break-words overflow-hidden",
            isUser
              ? "bg-slate-600 dark:bg-slate-500 text-white rounded-br-lg"
              : "bg-zinc-100 dark:bg-zinc-800 text-zinc-800 dark:text-zinc-200 rounded-bl-lg border border-zinc-200/60 dark:border-zinc-700/60"
          )}
        >
          {isUser ? (
            <div>
              {message.attachments && message.attachments.length > 0 && (
                <div className="flex flex-wrap gap-1.5 mb-2">
                  {message.attachments.map((att, i) => (
                    att.mimeType.startsWith("image/") ? (
                      <img key={i} src={att.dataUrl} alt={att.name}
                        className="max-w-[200px] max-h-[200px] rounded-lg object-cover border border-white/20" />
                    ) : (
                      <div key={i} className="text-xs bg-white/20 rounded-lg px-2 py-1 truncate max-w-[200px]">
                        📎 {att.name}
                      </div>
                    )
                  ))}
                </div>
              )}
              <p className="whitespace-pre-wrap">{message.content}</p>
            </div>
          ) : message.isStreaming ? (
            <StreamingText text={filteredContent} isStreaming={message.isStreaming} />
          ) : (
            <div className="prose prose-sm dark:prose-invert max-w-none [&_pre]:overflow-x-auto [&_code]:break-all [&_a]:break-all">
              <ReactMarkdown remarkPlugins={[remarkGfm]}>{filteredContent}</ReactMarkdown>
            </div>
          )}

          {message.toolCalls && message.toolCalls.length > 0 && (
            <ToolCallsCollapse toolCalls={message.toolCalls} />
          )}

          {message.statusMessages && message.statusMessages.length > 0 && (
            <StatusMessagesBar messages={message.statusMessages} />
          )}
        </div>
      </div>

      {!isUser && message.content && !message.isStreaming && !isTool && (
        <button
          onClick={handleCopy}
          className="flex-shrink-0 self-start mt-1.5 opacity-0 group-hover:opacity-100 transition-opacity duration-150 text-zinc-300 hover:text-zinc-500 dark:text-zinc-600 dark:hover:text-zinc-300"
        >
          {copied ? <Check className="w-3.5 h-3.5" /> : <Copy className="w-3.5 h-3.5" />}
        </button>
      )}
    </motion.div>
  );
}
