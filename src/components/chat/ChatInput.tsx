import { useState, useRef, useEffect, useMemo } from "react";
import { Send, Square, Maximize2, Minimize2, Paperclip, X, Image } from "lucide-react";
import { cn } from "../../lib/cn";
import { matchCommands, executeCommand, type SlashCommand } from "../../lib/slashCommands";

export interface FileAttachment {
  name: string;
  dataUrl: string;       // data:image/png;base64,... 或 data:text/plain;base64,...
  mimeType: string;
  size: number;
}

interface Props {
  onSend: (content: string, attachments: FileAttachment[]) => void;
  onCommandResult?: (text: string) => void;
  onStop?: () => void;
  isGenerating: boolean;
  disabled?: boolean;
}

export default function ChatInput({ onSend, onCommandResult, onStop, isGenerating, disabled }: Props) {
  const [input, setInput] = useState("");
  const [attachments, setAttachments] = useState<FileAttachment[]>([]);
  const [commandMenu, setCommandMenu] = useState<SlashCommand[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [expanded, setExpanded] = useState(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const isSlashMode = input.startsWith("/") && !input.includes(" ");
  const matchedCommands = useMemo(() => {
    if (!isSlashMode) return [];
    return matchCommands(input);
  }, [input, isSlashMode]);

  useEffect(() => { setCommandMenu(matchedCommands); setSelectedIndex(0); }, [matchedCommands]);

  // 处理 Ctrl+V 粘贴图片
  const handlePaste = (e: React.ClipboardEvent) => {
    const items = e.clipboardData?.items;
    if (!items) return;
    for (const item of Array.from(items)) {
      if (item.type.startsWith("image/")) {
        e.preventDefault();
        const file = item.getAsFile();
        if (file) processFiles([file]);
        break;
      }
    }
  };

  // 处理拖拽文件（由父组件传递，这里通过 onDrop）
  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    const files = e.dataTransfer?.files;
    if (files) processFiles(Array.from(files));
  };

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = "copy";
  };

  // 文件选择器
  const handleFileSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
    const files = e.target.files;
    if (files) processFiles(Array.from(files));
    if (fileInputRef.current) fileInputRef.current.value = "";
  };

  const processFiles = (files: File[]) => {
    const newAttachments: Promise<FileAttachment>[] = files.map((file) => {
      return new Promise<FileAttachment>((resolve, reject) => {
        const reader = new FileReader();
        reader.onload = () => {
          resolve({
            name: file.name,
            dataUrl: reader.result as string,
            mimeType: file.type || "application/octet-stream",
            size: file.size,
          });
        };
        reader.onerror = reject;
        reader.readAsDataURL(file);
      });
    });
    Promise.all(newAttachments).then((results) => {
      setAttachments((prev) => [...prev, ...results]);
    });
  };

  const removeAttachment = (index: number) => {
    setAttachments((prev) => prev.filter((_, i) => i !== index));
  };

  const handleSubmit = async () => {
    const trimmed = input.trim();
    if ((!trimmed && attachments.length === 0) || isGenerating || disabled) return;

    // 斜杠命令执行（无附件时）
    if (trimmed.startsWith("/") && attachments.length === 0) {
      const result = await executeCommand(trimmed);
      if (result !== undefined) {
        setInput("");
        if (result && onCommandResult) onCommandResult(result);
        return;
      }
    }

    onSend(trimmed || "(附件)", attachments);
    setInput("");
    setAttachments([]);
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (commandMenu.length > 0) {
      if (e.key === "ArrowDown") { e.preventDefault(); setSelectedIndex((i) => (i + 1) % commandMenu.length); return; }
      if (e.key === "ArrowUp") { e.preventDefault(); setSelectedIndex((i) => (i - 1 + commandMenu.length) % commandMenu.length); return; }
      if (e.key === "Tab" || e.key === "Enter") {
        e.preventDefault();
        const selected = commandMenu[selectedIndex];
        if (selected) { setInput("/" + selected.name + " "); setCommandMenu([]); textareaRef.current?.focus(); return; }
      }
      if (e.key === "Escape") { setCommandMenu([]); return; }
    }
    if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); handleSubmit(); }
  };

  return (
    <div
      className="border-t border-zinc-200 dark:border-zinc-700 bg-white dark:bg-zinc-900 p-4"
      onDrop={handleDrop}
      onDragOver={handleDragOver}
    >
      {commandMenu.length > 0 && (
        <div className="max-w-3xl mx-auto mb-1 border border-zinc-200 dark:border-zinc-700 rounded-xl bg-white dark:bg-zinc-800 shadow-lg overflow-hidden">
          {commandMenu.map((cmd, i) => (
            <button key={cmd.name}
              onClick={() => { setInput("/" + cmd.name + " "); setCommandMenu([]); textareaRef.current?.focus(); }}
              className={cn("w-full text-left px-4 py-2.5 flex items-center gap-3 transition-colors duration-100",
                i === selectedIndex ? "bg-slate-100 dark:bg-slate-800" : "hover:bg-zinc-50 dark:hover:bg-zinc-800/50")}
            >
              <span className="text-sm font-mono font-medium text-slate-600 dark:text-slate-400 w-24 flex-shrink-0">/{cmd.name}</span>
              <span className="text-xs text-zinc-500 dark:text-zinc-400">{cmd.description}</span>
            </button>
          ))}
        </div>
      )}

      {/* 附件预览 */}
      {attachments.length > 0 && (
        <div className="max-w-3xl mx-auto mb-2 flex flex-wrap gap-2">
          {attachments.map((att, i) => (
            <div key={i} className="relative group w-16 h-16 rounded-lg overflow-hidden border border-zinc-200 dark:border-zinc-700 bg-zinc-100 dark:bg-zinc-800">
              {att.mimeType.startsWith("image/") ? (
                <img src={att.dataUrl} alt={att.name} className="w-full h-full object-cover" />
              ) : (
                <div className="w-full h-full flex items-center justify-center">
                  <Image className="w-5 h-5 text-zinc-400" />
                </div>
              )}
              <button onClick={() => removeAttachment(i)}
                className="absolute top-0.5 right-0.5 p-0.5 rounded-full bg-black/50 text-white opacity-0 group-hover:opacity-100 transition-opacity">
                <X className="w-2.5 h-2.5" />
              </button>
              <span className="absolute bottom-0 left-0 right-0 text-[8px] text-white bg-black/50 px-1 truncate">{att.name}</span>
            </div>
          ))}
        </div>
      )}

      <div className="max-w-3xl mx-auto flex items-end gap-2">
        {/* 文件上传按钮 */}
        <input type="file" ref={fileInputRef} onChange={handleFileSelect} multiple className="hidden" accept="image/*,.pdf,.txt,.csv,.xlsx,.docx,.pptx" />
        <button onClick={() => fileInputRef.current?.click()}
          className="flex-shrink-0 p-2 rounded-lg text-zinc-400 hover:text-zinc-600 dark:hover:text-zinc-300 hover:bg-zinc-100 dark:hover:bg-zinc-800 mb-0.5"
          title="上传文件/图片">
          <Paperclip className="w-4 h-4" />
        </button>

        {expanded && (
          <button onClick={() => setExpanded(false)}
            className="flex-shrink-0 p-1.5 rounded-lg text-zinc-400 hover:text-zinc-600 dark:hover:text-zinc-300 hover:bg-zinc-100 dark:hover:bg-zinc-800 mb-1" title="收起">
            <Minimize2 className="w-3.5 h-3.5" />
          </button>
        )}
        <textarea ref={textareaRef} value={input}
          onChange={(e) => setInput(e.target.value)} onKeyDown={handleKeyDown} onPaste={handlePaste}
          placeholder="输入消息 / 斜杠命令... (Ctrl+V 粘贴图片) (Enter 发送, Shift+Enter 换行)"
          rows={expanded ? 15 : 1} disabled={disabled}
          className={cn(
            "flex-1 resize-none rounded-2xl border border-zinc-200 dark:border-zinc-700",
            "bg-zinc-50 dark:bg-zinc-800/50 px-4 py-3 text-sm",
            "text-zinc-800 dark:text-zinc-200",
            "placeholder:text-zinc-400 dark:placeholder:text-zinc-500",
            "focus:outline-none focus:ring-1 focus:ring-slate-400 dark:focus:ring-slate-500 focus:border-slate-300 dark:focus:border-slate-600",
            "disabled:opacity-50 disabled:cursor-not-allowed",
            "overflow-hidden transition-all duration-200"
          )}
          style={{ minHeight: "44px" }} />
        {input.length > 200 && !expanded && (
          <button onClick={() => setExpanded(true)}
            className="flex-shrink-0 p-1.5 rounded-lg text-zinc-400 hover:text-zinc-600 dark:hover:text-zinc-300 hover:bg-zinc-100 dark:hover:bg-zinc-800 mb-1" title="展开">
            <Maximize2 className="w-3.5 h-3.5" />
          </button>
        )}

        <button onClick={isGenerating ? onStop : handleSubmit}
          disabled={!isGenerating && (!input.trim() && attachments.length === 0) || disabled}
          className={cn("flex-shrink-0 w-10 h-10 rounded-xl flex items-center justify-center transition-all duration-200",
            isGenerating ? "bg-red-500 hover:bg-red-600 text-white" : "bg-slate-600 hover:bg-slate-700 text-white",
            "disabled:opacity-40 disabled:cursor-not-allowed")}>
          {isGenerating ? <Square className="w-4 h-4 fill-current" /> : <Send className="w-4 h-4" />}
        </button>
      </div>
    </div>
  );
}
