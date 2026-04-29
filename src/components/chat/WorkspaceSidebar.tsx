import { useState, useCallback, useEffect } from "react";
import { Folder, File, Home, ChevronRight, ChevronDown, X, FolderInput, ArrowUp, Trash2 } from "lucide-react";
import { useWorkspaceStore, type FileEntry } from "../../stores/workspaceStore";
import { listDirectory, getHomeDir } from "../../lib/tauri-bridge";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { cn } from "../../lib/cn";

export default function WorkspaceSidebar() {
  const currentDir = useWorkspaceStore((s) => s.currentDir);
  const files = useWorkspaceStore((s) => s.files);
  const isSidebarOpen = useWorkspaceStore((s) => s.isSidebarOpen);
  const selectedFile = useWorkspaceStore((s) => s.selectedFile);
  const setCurrentDir = useWorkspaceStore((s) => s.setCurrentDir);
  const setFiles = useWorkspaceStore((s) => s.setFiles);
  const setSidebarOpen = useWorkspaceStore((s) => s.setSidebarOpen);
  const setSelectedFile = useWorkspaceStore((s) => s.setSelectedFile);
  const refreshTrigger = useWorkspaceStore((s) => s.refreshTrigger);

  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [ctxMenu, setCtxMenu] = useState<{ x: number; y: number; entry: FileEntry } | null>(null);
  const [loading, setLoading] = useState(false);
  const [dirCache, setDirCache] = useState<Record<string, FileEntry[]>>({});

  const refreshDir = useCallback(async (dir: string) => {
    setLoading(true);
    try {
      const entries = await listDirectory(dir);
      setFiles(entries);
      setDirCache((c) => ({ ...c, [dir]: entries }));
    } catch (e) { console.error(e); }
    setLoading(false);
  }, [setFiles]);

  // AI 工具修改文件后自动刷新当前目录
  useEffect(() => {
    if (refreshTrigger > 0 && currentDir) {
      refreshDir(currentDir);
    }
  }, [refreshTrigger]); // eslint-disable-line react-hooks/exhaustive-deps

  const handleOpenFolder = async () => {
    const selected = await open({ directory: true, multiple: false });
    if (selected) { setCurrentDir(selected as string); refreshDir(selected as string); }
  };

  const handleOpenHome = async () => {
    const home = await getHomeDir();
    setCurrentDir(home);
    refreshDir(home);
  };

  // 单击文件夹：展开/折叠
  const toggleExpand = async (entry: FileEntry) => {
    if (!entry.is_dir) return;
    const next = new Set(expanded);
    if (next.has(entry.path)) {
      next.delete(entry.path);
    } else {
      next.add(entry.path);
      if (!dirCache[entry.path]) {
        try {
          const entries = await listDirectory(entry.path);
          setDirCache((c) => ({ ...c, [entry.path]: entries }));
        } catch { /* ignore */ }
      }
    }
    setExpanded(next);
  };

  // 双击文件夹：进入该目录
  const enterDir = (entry: FileEntry) => {
    if (!entry.is_dir) return;
    setCurrentDir(entry.path);
    refreshDir(entry.path);
  };

  // 返回上级目录
  const handleContextMenu = (e: React.MouseEvent, entry: FileEntry) => {
    e.preventDefault();
    setCtxMenu({ x: e.clientX, y: e.clientY, entry });
  };

  const handleDeleteFile = async () => {
    if (!ctxMenu) return;
    try {
      await invoke("delete_file_or_dir", { path: ctxMenu.entry.path });
      if (currentDir) refreshDir(currentDir);
      if (selectedFile === ctxMenu.entry.path) setSelectedFile(null);
    } catch (e) { console.error(e); }
    setCtxMenu(null);
  };

  // 关闭右键菜单
  useEffect(() => {
    const handler = () => setCtxMenu(null);
    document.addEventListener("click", handler);
    return () => document.removeEventListener("click", handler);
  }, []);

  const goUp = () => {
    if (!currentDir) return;
    const parent = currentDir.replace(/[/\\][^/\\]+$/, "") || currentDir.replace(/^([A-Z]:)\\/, "$1\\");
    if (parent && parent !== currentDir) {
      setCurrentDir(parent);
      refreshDir(parent);
    }
  };

  const renderTree = (entries: FileEntry[], depth = 0) => {
    return entries.map((entry) => {
      const isExpanded = expanded.has(entry.path);
      const children = entry.is_dir ? dirCache[entry.path] : undefined;
      return (
        <div key={entry.path}>
          <div
            className={cn(
              "flex items-center gap-1 px-2 py-0.5 cursor-pointer hover:bg-zinc-100 dark:hover:bg-zinc-800 rounded text-xs text-zinc-600 dark:text-zinc-400",
              "select-none",
              selectedFile === entry.path && "bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300"
            )}
            style={{ paddingLeft: `${8 + depth * 12}px` }}
            onClick={() => {
              if (entry.is_dir) { toggleExpand(entry); }
              else { setSelectedFile(entry.path); }
            }}
            onDoubleClick={() => { if (entry.is_dir) enterDir(entry); }}
            onContextMenu={(e) => handleContextMenu(e, entry)}
            title={entry.is_dir ? "单击展开 | 双击进入 | 右键菜单" : `点击预览 | 右键删除: ${entry.name}`}
          >
            {entry.is_dir ? (
              isExpanded ? <ChevronDown className="w-3 h-3 flex-shrink-0" /> : <ChevronRight className="w-3 h-3 flex-shrink-0" />
            ) : (
              <span className="w-3 flex-shrink-0" />
            )}
            {entry.is_dir ? (
              <Folder className="w-3.5 h-3.5 text-amber-500 flex-shrink-0" />
            ) : (
              <File className="w-3.5 h-3.5 text-zinc-400 flex-shrink-0" />
            )}
            <span className="truncate">{entry.name}</span>
          </div>
          {entry.is_dir && isExpanded && children && renderTree(children, depth + 1)}
        </div>
      );
    });
  };

  if (!isSidebarOpen) return null;

  return (
    <div className="flex flex-col w-56 border-r border-zinc-200/70 dark:border-zinc-700/70 bg-zinc-50 dark:bg-zinc-900 h-full">
      <div className="p-3 border-b border-zinc-200/70 dark:border-zinc-700/70 space-y-2">
        <div className="flex items-center justify-between">
          <span className="text-xs font-medium text-zinc-400">工作区</span>
          <button onClick={() => setSidebarOpen(false)} className="p-0.5 rounded text-zinc-400 hover:text-zinc-600 dark:hover:text-zinc-300">
            <X className="w-3.5 h-3.5" />
          </button>
        </div>
        <div className="flex gap-1.5">
          <button onClick={handleOpenFolder}
            className="flex-1 flex items-center justify-center gap-1 px-2 py-1.5 rounded-xl text-xs bg-white dark:bg-zinc-800 border border-zinc-200/70 dark:border-zinc-700/70 hover:border-slate-300 dark:hover:border-slate-600 text-zinc-600 dark:text-zinc-400 transition-all duration-200">
            <FolderInput className="w-3 h-3" />打开
          </button>
          <button onClick={handleOpenHome}
            className="flex-1 flex items-center justify-center gap-1 px-2 py-1.5 rounded-xl text-xs bg-white dark:bg-zinc-800 border border-zinc-200/70 dark:border-zinc-700/70 hover:border-slate-300 dark:hover:border-slate-600 text-zinc-600 dark:text-zinc-400 transition-all duration-200">
            <Home className="w-3 h-3" />主目录
          </button>
        </div>
        {currentDir && (
          <div className="flex items-center gap-1">
            <button onClick={goUp} className="p-0.5 rounded text-zinc-400 hover:text-zinc-600 dark:hover:text-zinc-300 flex-shrink-0" title="返回上级">
              <ArrowUp className="w-3 h-3" />
            </button>
            <div className="text-[10px] text-zinc-400 truncate bg-white dark:bg-zinc-800 rounded px-2 py-1 border border-zinc-200/70 dark:border-zinc-700/70 flex-1" title={currentDir}>
              {currentDir}
            </div>
          </div>
        )}
      </div>

      <div className="flex-1 overflow-y-auto chat-scrollbar p-1.5">
        {!currentDir ? (
          <div className="text-center text-xs text-zinc-400 py-8">点击「打开」选择工作目录</div>
        ) : loading ? (
          <div className="text-center text-xs text-zinc-400 py-4">加载中...</div>
        ) : files.length === 0 ? (
          <div className="text-center text-xs text-zinc-400 py-4">空目录</div>
        ) : (
          renderTree(files)
        )}
      </div>

      {/* 右键菜单 */}
      {ctxMenu && (
        <div
          className="fixed z-50 bg-white dark:bg-zinc-800 border border-zinc-200 dark:border-zinc-700 rounded-xl shadow-xl py-1 min-w-[140px]"
          style={{ left: ctxMenu.x, top: ctxMenu.y }}
        >
          <button
            onClick={handleDeleteFile}
            className="w-full flex items-center gap-2 px-3 py-1.5 text-xs text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors"
          >
            <Trash2 className="w-3 h-3" />删除 {ctxMenu.entry.name.length > 20 ? ctxMenu.entry.name.slice(0, 20) + "..." : ctxMenu.entry.name}
          </button>
        </div>
      )}
    </div>
  );
}
