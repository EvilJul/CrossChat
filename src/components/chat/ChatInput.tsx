import { useState, useRef, useEffect, useMemo } from "react";
import { Send, Square } from "lucide-react";
import { cn } from "../../lib/cn";
import { matchCommands, executeCommand, type SlashCommand } from "../../lib/slashCommands";

interface Props {
  onSend: (content: string) => void;
  onCommandResult?: (text: string) => void;
  onStop?: () => void;
  isGenerating: boolean;
  disabled?: boolean;
}

export default function ChatInput({ onSend, onCommandResult, onStop, isGenerating, disabled }: Props) {
  const [input, setInput] = useState("");
  const [commandMenu, setCommandMenu] = useState<SlashCommand[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // 检测斜杠命令
  const isSlashMode = input.startsWith("/") && !input.includes(" ");
  const matchedCommands = useMemo(() => {
    if (!isSlashMode) return [];
    return matchCommands(input);
  }, [input, isSlashMode]);

  useEffect(() => {
    setCommandMenu(matchedCommands);
    setSelectedIndex(0);
  }, [matchedCommands]);

  const handleSubmit = async () => {
    const trimmed = input.trim();
    if (!trimmed || isGenerating || disabled) return;

    // 斜杠命令执行
    if (trimmed.startsWith("/")) {
      const result = await executeCommand(trimmed);
      // undefined = 命令不存在，当作普通消息发送
      if (result !== undefined) {
        setInput("");
        if (result && onCommandResult) {
          onCommandResult(result); // 仅在结果非空时显示
        }
        return;
      }
    }

    onSend(trimmed);
    setInput("");
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    // 斜杠命令菜单导航
    if (commandMenu.length > 0) {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setSelectedIndex((i) => (i + 1) % commandMenu.length);
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        setSelectedIndex((i) => (i - 1 + commandMenu.length) % commandMenu.length);
        return;
      }
      if (e.key === "Tab" || e.key === "Enter") {
        e.preventDefault();
        const selected = commandMenu[selectedIndex];
        if (selected) {
          setInput("/" + selected.name + " ");
          setCommandMenu([]);
          textareaRef.current?.focus();
          return;
        }
      }
      if (e.key === "Escape") {
        setCommandMenu([]);
        return;
      }
    }

    // 正常发送
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  };

  return (
    <div className="border-t border-zinc-200 dark:border-zinc-700 bg-white dark:bg-zinc-900 p-4">
      {commandMenu.length > 0 && (
        <div className="max-w-3xl mx-auto mb-1 border border-zinc-200 dark:border-zinc-700 rounded-xl bg-white dark:bg-zinc-800 shadow-lg overflow-hidden">
          {commandMenu.map((cmd, i) => (
            <button
              key={cmd.name}
              onClick={() => {
                setInput("/" + cmd.name + " ");
                setCommandMenu([]);
                textareaRef.current?.focus();
              }}
              className={cn(
                "w-full text-left px-4 py-2.5 flex items-center gap-3 transition-colors duration-100",
                i === selectedIndex
                  ? "bg-slate-100 dark:bg-slate-800"
                  : "hover:bg-zinc-50 dark:hover:bg-zinc-800/50"
              )}
            >
              <span className="text-sm font-mono font-medium text-slate-600 dark:text-slate-400 w-24 flex-shrink-0">
                /{cmd.name}
              </span>
              <span className="text-xs text-zinc-500 dark:text-zinc-400">{cmd.description}</span>
            </button>
          ))}
          {commandMenu.length === 0 && isSlashMode && (
            <div className="px-4 py-3 text-xs text-zinc-400">未匹配到命令</div>
          )}
        </div>
      )}

      <div className="max-w-3xl mx-auto flex items-end gap-3">
        <textarea
          ref={textareaRef}
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="输入消息 / 斜杠命令... (Enter 发送, Shift+Enter 换行, / 命令菜单)"
          rows={1}
          disabled={disabled}
          className={cn(
            "flex-1 resize-none rounded-2xl border border-zinc-200 dark:border-zinc-700",
            "bg-zinc-50 dark:bg-zinc-800/50 px-4 py-3 text-sm",
            "text-zinc-800 dark:text-zinc-200",
            "placeholder:text-zinc-400 dark:placeholder:text-zinc-500",
            "focus:outline-none focus:ring-1 focus:ring-slate-400 dark:focus:ring-slate-500 focus:border-slate-300 dark:focus:border-slate-600",
            "disabled:opacity-50 disabled:cursor-not-allowed",
            "max-h-32 overflow-y-auto transition-all duration-200"
          )}
          style={{ minHeight: "44px" }}
        />
        <button
          onClick={isGenerating ? onStop : handleSubmit}
          disabled={!isGenerating && (!input.trim() || disabled)}
          className={cn(
            "flex-shrink-0 w-10 h-10 rounded-xl flex items-center justify-center transition-all duration-200",
            isGenerating
              ? "bg-red-500 hover:bg-red-600 text-white"
              : "bg-slate-600 hover:bg-slate-700 text-white",
            "disabled:opacity-40 disabled:cursor-not-allowed"
          )}
        >
          {isGenerating ? (
            <Square className="w-4 h-4 fill-current" />
          ) : (
            <Send className="w-4 h-4" />
          )}
        </button>
      </div>
    </div>
  );
}
