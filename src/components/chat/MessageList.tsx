import { useEffect, useRef } from "react";
import { useChatStore } from "../../stores/chatStore";
import { useSettingsStore } from "../../stores/settingsStore";
import MessageBubble from "./MessageBubble";

export default function MessageList() {
  const messages = useChatStore((s) => s.messages);
  const autoScroll = useSettingsStore((s) => s.autoScroll);
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (autoScroll) {
      bottomRef.current?.scrollIntoView({ behavior: "smooth" });
    }
  }, [messages, autoScroll]);

  return (
    <div className="flex-1 overflow-y-auto chat-scrollbar">
      {messages.length === 0 ? (
        <div className="flex flex-col items-center justify-center h-full text-zinc-400 dark:text-zinc-500 gap-3">
          <div className="w-16 h-16 rounded-2xl bg-slate-200 dark:bg-slate-800 flex items-center justify-center">
            <svg className="w-8 h-8 text-slate-400 dark:text-slate-500" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round">
              <rect x="3" y="3" width="18" height="14" rx="2" />
              <path d="M7 21h10M12 17v4" />
            </svg>
          </div>
          <p className="text-sm">开始一段新对话</p>
        </div>
      ) : (
        <div className="py-4">
          {messages.map((msg) => (
            <MessageBubble key={msg.id} message={msg} />
          ))}
          <div ref={bottomRef} />
        </div>
      )}
    </div>
  );
}
