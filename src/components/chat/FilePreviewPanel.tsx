import { useEffect, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { X, Loader2 } from "lucide-react";
import { useWorkspaceStore } from "../../stores/workspaceStore";
import { invoke } from "@tauri-apps/api/core";

export default function FilePreviewPanel() {
  const selectedFile = useWorkspaceStore((s) => s.selectedFile);
  const previewOpen = useWorkspaceStore((s) => s.previewOpen);
  const setSelectedFile = useWorkspaceStore((s) => s.setSelectedFile);

  const [content, setContent] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!selectedFile) { setContent(null); setError(null); return; }
    setLoading(true);
    setError(null);
    invoke<string>("read_file_content", { path: selectedFile })
      .then((text) => { setContent(text); setError(null); })
      .catch((e) => { setContent(null); setError(String(e).slice(0, 200)); })
      .finally(() => setLoading(false));
  }, [selectedFile]);

  const fileName = selectedFile?.split(/[/\\]/).pop() || "";
  const isOpen = previewOpen && selectedFile !== null;

  return (
    <AnimatePresence>
      {isOpen && (
        <motion.div
          initial={{ x: 360 }}
          animate={{ x: 0 }}
          exit={{ x: 360 }}
          transition={{ type: "spring", stiffness: 350, damping: 30 }}
          className="fixed right-0 top-0 h-screen w-80 bg-white dark:bg-zinc-950 border-l border-zinc-200/70 dark:border-zinc-700/70 shadow-2xl z-40 flex flex-col"
        >
          {/* 标题栏 */}
          <div className="flex items-center justify-between px-3 py-2.5 border-b border-zinc-200/70 dark:border-zinc-700/70 flex-shrink-0">
            <span className="text-xs font-medium text-zinc-600 dark:text-zinc-400 truncate flex-1" title={selectedFile || ""}>
              {fileName}
            </span>
            <button
              onClick={() => setSelectedFile(null)}
              className="p-1 rounded text-zinc-400 hover:text-zinc-600 dark:hover:text-zinc-300 hover:bg-zinc-100 dark:hover:bg-zinc-800"
              title="关闭"
            >
              <X className="w-3.5 h-3.5" />
            </button>
          </div>

          {/* 内容区 */}
          <div className="flex-1 overflow-y-auto chat-scrollbar p-3">
            {loading ? (
              <div className="flex items-center justify-center py-8 text-zinc-400">
                <Loader2 className="w-4 h-4 animate-spin" />
              </div>
            ) : error ? (
              <div className="text-xs text-red-500 p-2">{error}</div>
            ) : content !== null ? (
              <pre className="text-xs text-zinc-700 dark:text-zinc-300 whitespace-pre-wrap break-all font-mono leading-relaxed">
                {content}
              </pre>
            ) : null}
          </div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
