import { useEffect, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { X, Loader2, File, Settings, Code, Image, Music, Video, Archive, FileText } from "lucide-react";
import { useWorkspaceStore } from "../../stores/workspaceStore";
import { invoke } from "@tauri-apps/api/core";

interface FilePreviewInfo {
  name: string;
  path: string;
  size: number;
  is_executable: boolean;
  file_type: string;
  preview_content: string | null;
}

function getFileIcon(isExecutable: boolean, fileType: string) {
  if (isExecutable) {
    return <Settings className="w-4 h-4 text-blue-500" />;
  }

  if (fileType.includes("图像")) {
    return <Image className="w-4 h-4 text-green-500" />;
  }
  if (fileType.includes("音频")) {
    return <Music className="w-4 h-4 text-purple-500" />;
  }
  if (fileType.includes("视频")) {
    return <Video className="w-4 h-4 text-red-500" />;
  }
  if (fileType.includes("压缩")) {
    return <Archive className="w-4 h-4 text-yellow-500" />;
  }
  if (fileType.includes("文档") || fileType.includes("表格") || fileType.includes("演示")) {
    return <FileText className="w-4 h-4 text-blue-500" />;
  }
  if (fileType.includes("源代码") || fileType.includes("脚本")) {
    return <Code className="w-4 h-4 text-green-500" />;
  }

  return <File className="w-4 h-4 text-zinc-500" />;
}

function formatFileSize(bytes: number): string {
  if (bytes < 1024) return bytes + " B";
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + " KB";
  if (bytes < 1024 * 1024 * 1024) return (bytes / (1024 * 1024)).toFixed(1) + " MB";
  return (bytes / (1024 * 1024 * 1024)).toFixed(1) + " GB";
}

export default function FilePreviewPanel() {
  const selectedFile = useWorkspaceStore((s) => s.selectedFile);
  const previewOpen = useWorkspaceStore((s) => s.previewOpen);
  const setSelectedFile = useWorkspaceStore((s) => s.setSelectedFile);

  const [previewInfo, setPreviewInfo] = useState<FilePreviewInfo | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!selectedFile) {
      setPreviewInfo(null);
      setError(null);
      return;
    }

    setLoading(true);
    setError(null);

    invoke<FilePreviewInfo>("get_file_preview_info", { path: selectedFile })
      .then((info) => {
        setPreviewInfo(info);
        setError(null);
      })
      .catch((e) => {
        setPreviewInfo(null);
        setError(String(e).slice(0, 200));
      })
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
            <div className="flex items-center gap-2 flex-1 min-w-0">
              {previewInfo && getFileIcon(previewInfo.is_executable, previewInfo.file_type)}
              <span className="text-xs font-medium text-zinc-600 dark:text-zinc-400 truncate" title={selectedFile || ""}>
                {fileName}
              </span>
            </div>
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
            ) : previewInfo ? (
              <div className="space-y-3">
                {/* 文件信息 */}
                <div className="text-xs text-zinc-500 dark:text-zinc-400 space-y-1">
                  <div className="flex justify-between">
                    <span>类型:</span>
                    <span className="text-zinc-700 dark:text-zinc-300">{previewInfo.file_type}</span>
                  </div>
                  <div className="flex justify-between">
                    <span>大小:</span>
                    <span className="text-zinc-700 dark:text-zinc-300">{formatFileSize(previewInfo.size)}</span>
                  </div>
                  {previewInfo.is_executable && (
                    <div className="flex justify-between">
                      <span>可执行:</span>
                      <span className="text-blue-500">是</span>
                    </div>
                  )}
                </div>

                {/* 预览内容 */}
                {previewInfo.preview_content && (
                  <div className="mt-3">
                    <div className="text-xs font-medium text-zinc-600 dark:text-zinc-400 mb-2">
                      预览内容
                    </div>
                    <pre className="text-xs text-zinc-700 dark:text-zinc-300 whitespace-pre-wrap break-all font-mono leading-relaxed bg-zinc-50 dark:bg-zinc-900 p-2 rounded">
                      {previewInfo.preview_content}
                    </pre>
                  </div>
                )}

                {/* 可执行文件警告 */}
                {previewInfo.is_executable && (
                  <div className="mt-3 p-2 bg-yellow-50 dark:bg-yellow-900/20 rounded text-xs text-yellow-700 dark:text-yellow-300">
                    注意：可执行文件可能存在安全风险，请谨慎运行。
                  </div>
                )}
              </div>
            ) : null}
          </div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
