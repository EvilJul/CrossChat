import { useState } from "react";
import { motion } from "framer-motion";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { type ChatMessage } from "../../stores/chatStore";
import { useSettingsStore } from "../../stores/settingsStore";
import { cn } from "../../lib/cn";
import { Avatar } from "../../shared/ui";
import StreamingText from "./StreamingText";
import ThinkingBubble from "./ThinkingBubble";
import { Check, Copy } from "lucide-react";

interface Props {
  message: ChatMessage;
}

export default function MessageBubble({ message }: Props) {
  const [copied, setCopied] = useState(false);
  const isUser = message.role === "user";
  const isTool = message.role === "tool";
  const showThinking = useSettingsStore((s) => s.showThinking);
  const hasThinking = message.thinking || message.isThinking;

  const handleCopy = async () => {
    await navigator.clipboard.writeText(message.content);
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
            <p className="whitespace-pre-wrap">{message.content}</p>
          ) : message.isStreaming ? (
            <StreamingText text={message.content} isStreaming={message.isStreaming} />
          ) : (
            <div className="prose prose-sm dark:prose-invert max-w-none [&_pre]:overflow-x-auto [&_code]:break-all [&_a]:break-all">
              <ReactMarkdown remarkPlugins={[remarkGfm]}>{message.content}</ReactMarkdown>
            </div>
          )}

          {message.toolCalls && message.toolCalls.length > 0 && (
            <div className="mt-2 space-y-1">
              {message.toolCalls.map((tc) => (
                <div
                  key={tc.id}
                  className={cn(
                    "text-xs px-2.5 py-1 rounded-xl flex items-center gap-2",
                    "bg-white/60 dark:bg-zinc-700/60",
                    tc.status === "running" && "animate-pulse"
                  )}
                >
                  <span className="font-medium text-zinc-600 dark:text-zinc-300">{tc.name}</span>
                  {tc.status === "running" && <span className="text-zinc-400">执行中...</span>}
                  {tc.status === "completed" && <span className="text-emerald-600 dark:text-emerald-400">完成</span>}
                  {tc.status === "failed" && <span className="text-red-500">失败</span>}
                </div>
              ))}
            </div>
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
