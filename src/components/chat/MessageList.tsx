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
    <div className="flex-1 overflow-y-auto chat-scrollbar bg-zinc-50 dark:bg-zinc-950">
      {messages.length === 0 ? (
        <div className="flex flex-col items-center justify-center h-full text-zinc-400 dark:text-zinc-500 gap-6 px-6">
          {/* 空状态图标 */}
          <div className="w-20 h-20 rounded-2xl bg-gradient-to-br from-purple-500 to-blue-500 flex items-center justify-center shadow-2xl shadow-purple-500/30">
            <svg className="w-10 h-10 text-white" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round">
              <rect x="3" y="3" width="18" height="14" rx="2" />
              <path d="M7 21h10M12 17v4" />
            </svg>
          </div>
          {/* 欢迎文字 */}
          <div className="text-center space-y-2">
            <h2 className="text-xl font-semibold text-zinc-700 dark:text-zinc-300">
              开始新对话
            </h2>
            <p className="text-sm text-zinc-500 dark:text-zinc-400 max-w-md">
              选择模型，输入消息开始聊天。支持文件上传、工作区操作和斜杠命令。
            </p>
          </div>
        </div>
      ) : (
        <div className="py-6 max-w-5xl mx-auto">
          {messages.map((msg) => (
            <MessageBubble key={msg.id} message={msg} />
          ))}
          <div ref={bottomRef} />
        </div>
      )}
    </div>
  );
}
