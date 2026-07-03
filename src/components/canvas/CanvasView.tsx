import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import * as Dialog from "@radix-ui/react-dialog";
import { X, Search, Pin, Plus, MoreHorizontal } from "lucide-react";
import { useSettingsStore } from "../../stores/settingsStore";
import { useProviderStore } from "../../stores/providerStore";
import SettingsDialog from "../settings/SettingsDialog";
import { cn } from "../../lib/cn";

interface SessionMeta {
  id: string;
  title: string;
  created_at: number;
  updated_at: number;
  message_count: number;
  status: string;
  pinned: boolean;
}

interface SessionMessage {
  role: string;
  content: string;
  timestamp: number;
  thinking?: string | null;
  tool_name?: string | null;
}

interface Session {
  meta: SessionMeta;
  messages: SessionMessage[];
  summary?: string | null;
}

function ThinkingBlock({ content }: { content: string }) {
  const [open, setOpen] = useState(false);
  return (
    <div className="mb-2 rounded-lg border border-ds-border/60 bg-ds-bg-main/50 overflow-hidden">
      <button
        onClick={() => setOpen(!open)}
        className="flex items-center gap-2 w-full px-3 py-1.5 text-xs text-ds-muted hover:text-ds-text-primary transition-colors"
      >
        <span className="text-[10px] font-mono">{open ? "▼" : "▶"}</span>
        <span>推理过程</span>
      </button>
      {open && (
        <div className="px-3 pb-2 text-xs text-ds-text-secondary leading-relaxed whitespace-pre-wrap border-t border-ds-border/40 pt-2">
          {content}
        </div>
      )}
    </div>
  );
}

function ToolCallBlock({ content, toolName }: { content: string; toolName: string }) {
  const [open, setOpen] = useState(false);
  return (
    <div className="mb-2 rounded-lg border border-ds-border/60 bg-amber-500/5 overflow-hidden">
      <button
        onClick={() => setOpen(!open)}
        className="flex items-center gap-2 w-full px-3 py-1.5 text-xs text-ds-muted hover:text-ds-text-primary transition-colors"
      >
        <span className="text-[10px] font-mono">{open ? "▼" : "▶"}</span>
        <span className="text-ds-accent font-medium">{toolName}</span>
      </button>
      {open && (
        <div className="px-3 pb-2 text-xs text-ds-text-secondary leading-relaxed whitespace-pre-wrap border-t border-ds-border/40 pt-2 font-mono">
          {content}
        </div>
      )}
    </div>
  );
}

/**
 * 通用「输入名称」居中弹窗，用于「新建画布」与「重命名」两处复用。
 * 由外部通过 open / onOpenChange 完全受控；回车确认、Esc 由 Radix 自动关闭。
 */
function NameDialog({
  open,
  title,
  placeholder,
  confirmText,
  initialValue,
  onConfirm,
  onOpenChange,
}: {
  open: boolean;
  title: string;
  placeholder: string;
  confirmText: string;
  initialValue: string;
  onConfirm: (value: string) => void;
  onOpenChange: (open: boolean) => void;
}) {
  const [value, setValue] = useState(initialValue);
  const inputRef = useRef<HTMLInputElement>(null);

  // 打开时同步初始值（重命名场景带入原标题）
  useEffect(() => {
    if (open) setValue(initialValue);
  }, [open, initialValue]);

  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 bg-black/40 z-50 data-[state=open]:animate-in data-[state=open]:fade-in duration-200" />
        <Dialog.Content
          onOpenAutoFocus={(e) => { e.preventDefault(); inputRef.current?.focus(); inputRef.current?.select(); }}
          onKeyDown={(e) => {
            if (e.key === "Enter") { e.preventDefault(); onConfirm(value); }
          }}
          className="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[360px] z-50 bg-ds-surface-card backdrop-blur-xl rounded-2xl border border-ds-border shadow-2xl flex flex-col data-[state=open]:animate-in data-[state=open]:fade-in data-[state=open]:zoom-in-95 duration-200 overflow-hidden"
        >
          <div className="flex items-center justify-between px-5 py-4 border-b border-ds-border">
            <Dialog.Title className="text-sm font-semibold text-ds-text-primary">{title}</Dialog.Title>
            <Dialog.Close className="p-1 rounded-md text-ds-muted hover:text-ds-text-primary hover:bg-ds-hover transition-colors">
              <X className="w-4 h-4" />
            </Dialog.Close>
          </div>
          <div className="p-5 space-y-4">
            <input
              ref={inputRef}
              value={value}
              onChange={(e) => setValue(e.target.value)}
              placeholder={placeholder}
              className="w-full text-sm rounded-lg border border-ds-border bg-ds-bg-main px-3 py-2 text-ds-text-primary placeholder:text-ds-muted focus:outline-none focus:ring-2 focus:ring-ds-accent/30 focus:border-ds-accent transition-all"
            />
            <div className="flex justify-end gap-2">
              <button
                onClick={() => onOpenChange(false)}
                className="px-3 py-1.5 text-sm rounded-lg border border-ds-border text-ds-muted hover:text-ds-text-primary transition-colors"
              >
                取消
              </button>
              <button
                onClick={() => onConfirm(value)}
                className="px-4 py-1.5 text-sm rounded-lg bg-ds-accent text-white hover:opacity-90 active:scale-[0.98] font-medium transition-all"
              >
                {confirmText}
              </button>
            </div>
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}

export default function CanvasView() {
  const [threads, setThreads] = useState<SessionMeta[]>([]);
  const [activeId, setActiveId] = useState<string | null>(null);
  const [messages, setMessages] = useState<SessionMessage[]>([]);
  const [input, setInput] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [menuThreadId, setMenuThreadId] = useState<string | null>(null);
  const [showNewDialog, setShowNewDialog] = useState(false);
  const [renameTarget, setRenameTarget] = useState<SessionMeta | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [archivedOpen, setArchivedOpen] = useState(false);
  const canvasRef = useRef<HTMLDivElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  const activeProviderId = useSettingsStore(s => s.activeProviderId);
  const activeModel = useSettingsStore(s => s.activeModel);
  const getCredential = useSettingsStore(s => s.credentials);
  const providers = useProviderStore(s => s.providers);
  const showThinking = useSettingsStore(s => s.showThinking);
  const showToolCalls = useSettingsStore(s => s.showToolCalls);
  const sendOnEnter = useSettingsStore(s => s.sendOnEnter);

  const loadThreads = useCallback(async () => {
    try {
      const list = await invoke<SessionMeta[]>("list_sessions");
      setThreads(list);
    } catch (e) { console.error(e); }
  }, []);

  useEffect(() => { loadThreads(); }, [loadThreads]);

  useEffect(() => {
    if (activeId) {
      invoke<Session>("get_session", { id: activeId })
        .then(s => setMessages(s.messages || []))
        .catch(console.error);
    }
  }, [activeId]);

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setMenuThreadId(null);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, []);

  const createThread = async (name?: string) => {
    try {
      const title = name?.trim() || `画布 ${new Date().toLocaleDateString()}`;
      const meta = await invoke<SessionMeta>("create_session", { title });
      await loadThreads();
      setActiveId(meta.id);
      setShowNewDialog(false);
    } catch (e) { console.error(e); }
  };

  const renameThread = async (id: string, title: string) => {
    const t = title.trim();
    if (!t) return;
    try {
      await invoke("rename_session", { id, title: t });
      await loadThreads();
      setRenameTarget(null);
    } catch (e) { console.error(e); }
  };

  const toggleArchive = async (t: SessionMeta) => {
    try {
      const status = t.status === "archived" ? "active" : "archived";
      await invoke("set_session_status", { id: t.id, status });
      await loadThreads();
      setMenuThreadId(null);
    } catch (e) { console.error(e); }
  };

  const togglePin = async (t: SessionMeta) => {
    try {
      await invoke("set_session_pinned", { id: t.id, pinned: !t.pinned });
      await loadThreads();
      setMenuThreadId(null);
    } catch (e) { console.error(e); }
  };

  const openRename = (t: SessionMeta) => {
    setRenameTarget(t);
    setMenuThreadId(null);
  };

  const deleteThread = async (id: string) => {
    try {
      await invoke("delete_session", { id });
      setThreads(prev => prev.filter(t => t.id !== id));
      if (activeId === id) setActiveId(null);
      setMenuThreadId(null);
    } catch (e) { console.error(e); }
  };

  const exportThread = async (id: string) => {
    try {
      const session = await invoke<Session>("get_session", { id });
      const blob = new Blob([JSON.stringify(session, null, 2)], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `${session.meta.title.replace(/[/\\?%*:|"<>]/g, "_")}.json`;
      a.click();
      URL.revokeObjectURL(url);
      setMenuThreadId(null);
    } catch (e) { console.error(e); }
  };

  const scrollToBottom = () => {
    requestAnimationFrame(() => {
      if (canvasRef.current) {
        canvasRef.current.scrollTop = canvasRef.current.scrollHeight;
      }
    });
  };

  const sendMessage = async () => {
    if (!input.trim() || !activeId || loading) return;

    const provider = providers.find(p => p.id === activeProviderId);
    if (!provider) {
      setError("请先在设置中配置 API Provider");
      return;
    }
    const apiKey = getCredential[activeProviderId]?.apiKey;
    if (!apiKey) {
      setError("请先在设置中配置 API Key");
      return;
    }
    const model = activeModel || "gpt-4o";
    const apiBase = provider.apiBase || "https://api.openai.com/v1";

    const userMsg: SessionMessage = {
      role: "user",
      content: input,
      timestamp: Math.floor(Date.now() / 1000),
    };
    const optimistic = [...messages, userMsg];
    setMessages(optimistic);
    setInput("");
    setLoading(true);
    setError(null);

    try {
      await invoke<string>("send_chat_message", {
        sessionId: activeId,
        userText: input,
        apiKey,
        model,
        apiBase,
      });

      await loadThreads();
      const session = await invoke<Session>("get_session", { id: activeId });
      setMessages(session.messages || []);
      scrollToBottom();
    } catch (e) {
      setError(typeof e === "string" ? e : "请求失败");
      invoke<Session>("get_session", { id: activeId })
        .then(s => setMessages(s.messages || []))
        .catch(console.error);
    } finally {
      setLoading(false);
    }
  };

  const shouldShowThinking = (thinking?: string | null) => {
    if (!thinking) return false;
    if (showThinking === true) return true;
    if (showThinking === false) return false;
    return thinking.length < 500;
  };

  // Extract <think>...</think> tags from message content into thinking field
  const extractedMessages = messages.map(msg => {
    if (!msg.content || !msg.content.includes('<think>')) return msg;
    const match = msg.content.match(/<think>([\s\S]*?)<\/think>/);
    if (!match) return msg;
    const thinkContent = match[1].trim();
    const cleanContent = msg.content.replace(/<think>[\s\S]*?<\/think>/g, '').trim();
    return {
      ...msg,
      content: cleanContent,
      thinking: msg.thinking || thinkContent,
    };
  });

  // Merge consecutive AssistantReasoning into the next AssistantText
  const mergedMessages: SessionMessage[] = [];
  for (let i = 0; i < extractedMessages.length; i++) {
    const msg = extractedMessages[i];
    if (msg.thinking && !msg.content && i + 1 < extractedMessages.length && !extractedMessages[i + 1].thinking) {
      mergedMessages.push({ ...extractedMessages[i + 1], thinking: msg.thinking });
      i++;
    } else {
      mergedMessages.push(msg);
    }
  }

  const visibleMessages = mergedMessages.filter(msg => {
    if (msg.role === "tool_call" || msg.role === "tool_result") {
      return showToolCalls;
    }
    if (showThinking === false && msg.thinking && !msg.content) {
      return false;
    }
    return true;
  });

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      if (sendOnEnter) sendMessage();
    }
    if (e.key === "Enter" && e.shiftKey && !sendOnEnter) {
      e.preventDefault();
      sendMessage();
    }
  };

  // 搜索过滤（纯前端，按 title 实时匹配）；活跃与归档都参与匹配。排序沿用后端返回顺序。
  const query = searchQuery.trim().toLowerCase();
  const filtered = query
    ? threads.filter(t => t.title.toLowerCase().includes(query))
    : threads;
  const activeThreads = filtered.filter(t => t.status !== "archived");
  const archivedThreads = filtered.filter(t => t.status === "archived");

  // 单个会话列表项渲染（活跃/归档分区共用）
  const renderThreadItem = (t: SessionMeta) => (
    <div key={t.id} className="relative group">
      <button
        onClick={() => setActiveId(t.id)}
        className={cn(
          "w-full text-left pl-3 pr-8 py-2 text-sm border-b border-ds-border/50 transition-colors",
          t.id === activeId
            ? "bg-ds-selected font-medium"
            : "hover:bg-ds-hover"
        )}
      >
        <div className="flex items-center gap-1.5 min-w-0">
          {t.pinned && <Pin className="w-3 h-3 text-ds-accent flex-shrink-0" fill="currentColor" />}
          <div className="truncate text-ds-text-primary text-xs leading-snug">{t.title}</div>
        </div>
        <div className="text-[10px] text-ds-muted mt-0.5">{t.message_count} 条</div>
      </button>
      <button
        onClick={() => setMenuThreadId(menuThreadId === t.id ? null : t.id)}
        className="absolute right-1 top-1/2 -translate-y-1/2 p-1 rounded text-ds-muted opacity-70 hover:opacity-100 hover:text-ds-text-primary hover:bg-ds-hover transition-all"
      >
        <MoreHorizontal className="w-4 h-4" />
      </button>
      {menuThreadId === t.id && (
        <div ref={menuRef} className="absolute right-1 top-8 z-50 w-32 rounded-lg border border-ds-border bg-ds-surface-card shadow-lg py-1">
          <button
            onClick={() => openRename(t)}
            className="w-full text-left px-3 py-1.5 text-xs text-ds-text-primary hover:bg-ds-hover transition-colors"
          >
            重命名
          </button>
          <button
            onClick={() => togglePin(t)}
            className="w-full text-left px-3 py-1.5 text-xs text-ds-text-primary hover:bg-ds-hover transition-colors"
          >
            {t.pinned ? "取消置顶" : "置顶"}
          </button>
          <button
            onClick={() => toggleArchive(t)}
            className="w-full text-left px-3 py-1.5 text-xs text-ds-text-primary hover:bg-ds-hover transition-colors"
          >
            {t.status === "archived" ? "取消归档" : "归档"}
          </button>
          <button
            onClick={() => exportThread(t.id)}
            className="w-full text-left px-3 py-1.5 text-xs text-ds-text-primary hover:bg-ds-hover transition-colors"
          >
            导出
          </button>
          <button
            onClick={() => {
              // 后端 delete_session 是软删除（设 status='deleted'），前端从列表移除即可
              if (confirm("确认移除此画布？\n\n移除后将从列表中消失（数据仍保留在本地文件中，可通过数据目录恢复）。")) {
                deleteThread(t.id);
              }
            }}
            className="w-full text-left px-3 py-1.5 text-xs text-ds-danger hover:bg-ds-danger/10 transition-colors"
          >
            移除
          </button>
        </div>
      )}
    </div>
  );

  return (
    <div className="flex h-screen bg-ds-bg-canvas text-ds-text-primary">
      <div className="w-44 flex-shrink-0 border-r border-ds-border bg-ds-surface-card flex flex-col">
        <div className="p-3 border-b border-ds-border">
          <div className="flex items-center gap-2">
            <div className="relative flex-1 min-w-0">
              <Search className="absolute left-2 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-ds-muted pointer-events-none" />
              <input
                value={searchQuery}
                onChange={e => setSearchQuery(e.target.value)}
                placeholder="搜索会话"
                className="w-full text-xs rounded-md border border-ds-border bg-ds-bg-main pl-7 pr-2 py-1.5 text-ds-text-primary placeholder:text-ds-muted focus:outline-none focus:ring-1 focus:ring-ds-accent/40"
              />
            </div>
            <button
              onClick={() => setShowNewDialog(true)}
              title="新建画布"
              className="flex-shrink-0 grid place-items-center w-8 h-8 rounded-md bg-ds-accent text-white hover:opacity-90 active:scale-95 transition-all"
            >
              <Plus className="w-4 h-4" />
            </button>
          </div>
        </div>
        <div className="flex-1 overflow-y-auto chat-scrollbar">
          {activeThreads.map(renderThreadItem)}

          {archivedThreads.length > 0 && (
            <div className="border-t border-ds-border/50">
              <button
                onClick={() => setArchivedOpen(o => !o)}
                className="flex items-center gap-1.5 w-full px-3 py-2 text-[11px] text-ds-muted hover:text-ds-text-primary transition-colors"
              >
                <span className="text-[9px] font-mono">{archivedOpen ? "▼" : "▶"}</span>
                <span>已归档 ({archivedThreads.length})</span>
              </button>
              {archivedOpen && archivedThreads.map(renderThreadItem)}
            </div>
          )}

          {filtered.length === 0 && (
            <p className="px-3 py-4 text-[11px] text-ds-muted text-center">
              {query ? "无匹配会话" : "暂无会话"}
            </p>
          )}
        </div>
      </div>

      <div className="flex-1 flex flex-col min-w-0">
        <div className="h-12 flex-shrink-0 border-b border-ds-border px-5 flex items-center bg-ds-surface-card/60 backdrop-blur-sm">
          <h1 className="text-sm font-semibold text-ds-muted">
            {threads.find(t => t.id === activeId)?.title || "CrossChat"}
          </h1>
          {error && (
            <span className="ml-4 text-xs text-ds-danger truncate">{error}</span>
          )}
          <div className="ml-auto">
            <SettingsDialog />
          </div>
        </div>

        <div
          ref={canvasRef}
          className="flex-1 overflow-y-auto bg-gradient-to-br from-transparent via-ds-bg-main/20 to-ds-bg-main/30"
        >
          <div className="max-w-4xl mx-auto py-8 px-6">
            {visibleMessages.length === 0 && activeId && !loading && (
              <p className="text-center text-ds-muted text-sm mt-20">在画布上开始对话</p>
            )}
            {!activeId && (
              <p className="text-center text-ds-muted text-sm mt-20">选择或创建一个画布</p>
            )}
            <div className="space-y-5">
              {visibleMessages.map((msg, i) => {
                if (msg.role === "tool_call" || msg.role === "tool_result") {
                  return (
                    <div key={i} className="flex justify-center">
                      <div className="w-full max-w-xl">
                        <ToolCallBlock
                          content={msg.content}
                          toolName={msg.tool_name || (msg.role === "tool_result" ? "结果" : "工具")}
                        />
                      </div>
                    </div>
                  );
                }

                const isUser = msg.role === "user";
                const isSystem = msg.role === "system";
                const showThinkingBlock = shouldShowThinking(msg.thinking);
                const isThinkingOnly = msg.thinking && !msg.content;

                return (
                  <div key={i} className={cn("flex", isUser ? "justify-end" : "justify-start")}>
                    <div className={cn(
                      isUser ? "max-w-[70%] min-w-0" : "max-w-[75%] min-w-0"
                    )}>
                      {showThinkingBlock && msg.thinking && (
                        <ThinkingBlock content={msg.thinking} />
                      )}
                      {!isThinkingOnly && (
                        <div className={cn(
                          "rounded-xl px-4 py-3 shadow-sm border break-words",
                          isUser
                            ? "bg-ds-accent text-white border-transparent"
                            : isSystem
                            ? "bg-ds-selected border-ds-border text-ds-text-secondary"
                            : "bg-ds-surface-card border-ds-border"
                        )}>
                          <div className="whitespace-pre-wrap text-sm leading-relaxed">{msg.content}</div>
                          <div className={cn(
                            "text-[10px] mt-1.5",
                            isUser ? "text-white/60" : "text-ds-muted"
                          )}>
                            {isUser ? "你" : isSystem ? "系统" : "AI"} · {new Date(msg.timestamp * 1000).toLocaleTimeString()}
                          </div>
                        </div>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
            {loading && (
              <div className="flex justify-start mt-5">
                <div className="bg-ds-surface-card border border-ds-border rounded-xl px-4 py-3 shadow-sm">
                  <div className="flex items-center gap-2 text-ds-muted text-sm">
                    <span className="w-2 h-2 rounded-full bg-ds-accent animate-bounce" style={{ animationDelay: "0ms" }} />
                    <span className="w-2 h-2 rounded-full bg-ds-accent animate-bounce" style={{ animationDelay: "150ms" }} />
                    <span className="w-2 h-2 rounded-full bg-ds-accent animate-bounce" style={{ animationDelay: "300ms" }} />
                    <span className="ml-1">AI 思考中...</span>
                  </div>
                </div>
              </div>
            )}
          </div>
        </div>

        <div className="flex-shrink-0 border-t border-ds-border bg-ds-surface-card/80 backdrop-blur-sm p-4">
          <div className="max-w-4xl mx-auto flex gap-2">
            <input
              value={input}
              onChange={e => setInput(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder={activeId ? "在画布上输入消息..." : "先创建或选择一个画布"}
              disabled={!activeId || loading}
              className="flex-1 px-4 py-2.5 rounded-xl border border-ds-border bg-ds-bg-main text-sm focus:outline-none focus:ring-2 focus:ring-ds-accent/30 focus:border-ds-accent disabled:opacity-40 transition-all text-ds-text-primary placeholder:text-ds-muted"
            />
            <button
              onClick={sendMessage}
              disabled={!input.trim() || !activeId || loading}
              className="px-5 py-2.5 bg-ds-accent text-white text-sm rounded-xl hover:opacity-90 disabled:opacity-40 active:scale-[0.97] font-medium transition-all"
            >
              {loading ? "..." : "发送"}
            </button>
          </div>
        </div>
      </div>

      {/* 新建画布弹窗（居中 Modal） */}
      <NameDialog
        open={showNewDialog}
        title="新建画布"
        placeholder="画布名称"
        confirmText="创建"
        initialValue=""
        onConfirm={(name) => createThread(name)}
        onOpenChange={(o) => setShowNewDialog(o)}
      />

      {/* 重命名弹窗（复用同款 Dialog） */}
      <NameDialog
        open={renameTarget !== null}
        title="重命名画布"
        placeholder="新名称"
        confirmText="保存"
        initialValue={renameTarget?.title ?? ""}
        onConfirm={(name) => { if (renameTarget) renameThread(renameTarget.id, name); }}
        onOpenChange={(o) => { if (!o) setRenameTarget(null); }}
      />
    </div>
  );
}
