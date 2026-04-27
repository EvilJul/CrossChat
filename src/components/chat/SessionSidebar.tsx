import { useState, useEffect, useCallback } from "react";
import { Plus, MessageSquare, Trash2, Loader2 } from "lucide-react";
import { motion } from "framer-motion";
import { useChatStore } from "../../stores/chatStore";
import { createSession, listSessions, deleteSession as deleteSessionApi, saveMessages, type SessionMeta } from "../../lib/tauri-bridge";

interface Props {
  activeSessionId: string;
  onSelectSession: (id: string) => void;
  onNewSession: () => void;
}

export default function SessionSidebar({ activeSessionId, onSelectSession, onNewSession }: Props) {
  const [sessions, setSessions] = useState<SessionMeta[]>([]);
  const [loading, setLoading] = useState(false);
  const messages = useChatStore((s) => s.messages);

  const refreshSessions = useCallback(async () => {
    try {
      const list = await listSessions();
      setSessions(list);
    } catch {
      // No sessions yet
    }
  }, []);

  useEffect(() => {
    refreshSessions();
  }, [refreshSessions]);

  // Auto-save current session
  useEffect(() => {
    if (!activeSessionId || messages.length === 0) return;
    const timer = setTimeout(async () => {
      try {
        await saveMessages(
          activeSessionId,
          messages.map((m) => ({
            role: m.role,
            content: m.content,
            timestamp: m.timestamp,
          })),
          null
        );
      } catch {
        // ignore save errors
      }
    }, 1000); // 防抖 1 秒
    return () => clearTimeout(timer);
  }, [messages, activeSessionId]);

  const handleNew = async () => {
    setLoading(true);
    try {
      await createSession("新对话");
      onNewSession();
      await refreshSessions();
    } catch (e) {
      console.error(e);
    }
    setLoading(false);
  };

  const handleDelete = async (id: string, e: React.MouseEvent) => {
    e.stopPropagation();
    await deleteSessionApi(id);
    if (id === activeSessionId) {
      onNewSession();
    }
    await refreshSessions();
  };

  return (
    <div className="flex flex-col w-48 border-r border-zinc-200 dark:border-zinc-700 bg-zinc-50 dark:bg-zinc-900 h-full">
      <div className="p-2 border-b border-zinc-200 dark:border-zinc-700">
        <button
          onClick={handleNew}
          disabled={loading}
          className="w-full flex items-center justify-center gap-1.5 px-3 py-1.5 rounded-lg text-xs bg-white dark:bg-zinc-800 border border-zinc-200 dark:border-zinc-700 hover:border-purple-300 dark:hover:border-purple-600 text-zinc-600 dark:text-zinc-400 transition-colors"
        >
          {loading ? <Loader2 className="w-3 h-3 animate-spin" /> : <Plus className="w-3 h-3" />}
          新对话
        </button>
      </div>

      <div className="flex-1 overflow-y-auto chat-scrollbar p-1.5">
        {sessions.length === 0 ? (
          <div className="text-center text-xs text-zinc-400 py-6">
            暂无对话历史
          </div>
        ) : (
          sessions.map((session) => (
            <motion.button
              key={session.id}
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              onClick={() => onSelectSession(session.id)}
              className={`w-full text-left px-2.5 py-2 rounded-lg mb-0.5 group transition-colors ${
                session.id === activeSessionId
                  ? "bg-purple-50 dark:bg-purple-900/10 border border-purple-200 dark:border-purple-800"
                  : "hover:bg-zinc-100 dark:hover:bg-zinc-800 border border-transparent"
              }`}
            >
              <div className="flex items-center gap-1.5">
                <MessageSquare className="w-3 h-3 text-zinc-400 flex-shrink-0" />
                <span className="text-xs text-zinc-700 dark:text-zinc-300 truncate flex-1">
                  {session.title || "新对话"}
                </span>
                <button
                  onClick={(e) => handleDelete(session.id, e)}
                  className="opacity-0 group-hover:opacity-100 p-0.5 rounded text-zinc-400 hover:text-red-500 transition-all"
                >
                  <Trash2 className="w-3 h-3" />
                </button>
              </div>
              <div className="text-[10px] text-zinc-400 mt-0.5 ml-[18px]">
                {session.message_count} 条消息
              </div>
            </motion.button>
          ))
        )}
      </div>
    </div>
  );
}
