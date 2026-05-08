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

  // 点击外部关闭预览
  const handleBackdropClick = (e: React.MouseEvent) => {
    // 只有点击遮罩层本身时才关闭，点击面板内部不关闭
    if (e.target === e.currentTarget) {
      setSelectedFile(null);
    }
  };

  return (
    <AnimatePresence>
      {isOpen && (
        <>
          {/* 遮罩层 */}
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.2 }}
            className="fixed inset-0 bg-black/20 dark:bg-black/40 backdrop-blur-sm z-30"
            onClick={handleBackdropClick}
          />
          
          {/* 预览面板 */}
          <motion.div
            initial={{ x: 360 }}
            animate={{ x: 0 }}
            exit={{ x: 360 }}
            transition={{ type: "spring", stiffness: 350, damping: 30 }}
            className="fixed right-0 top-0 h-screen w-80 bg-white dark:bg-zinc-900 border-l border-zinc-200/50 dark:border-zinc-800/50 shadow-2xl z-40 flex flex-col"
          >
            {/* 标题栏 - 优化样式 */}
            <div className="flex items-center justify-between px-4 py-3 border-b border-zinc-200/50 dark:border-zinc-800/50 bg-zinc-50/50 dark:bg-zinc-900/50 backdrop-blur-sm flex-shrink-0">
              <div className="flex items-center gap-2 flex-1 min-w-0">
                {previewInfo && getFileIcon(previewInfo.is_executable, previewInfo.file_type)}
                <span className="text-sm font-medium text-zinc-700 dark:text-zinc-300 truncate" title={selectedFile || ""}>
                  {fileName}
                </span>
              </div>
              <button
                onClick={() => setSelectedFile(null)}
                className="p-1.5 rounded-lg text-zinc-400 hover:text-zinc-600 dark:hover:text-zinc-300 hover:bg-zinc-200 dark:hover:bg-zinc-800 transition-all duration-200"
                title="关闭"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            {/* 内容区 - 优化样式 */}
            <div className="flex-1 overflow-y-auto chat-scrollbar p-4">
              {loading ? (
                <div className="flex items-center justify-center py-12 text-zinc-400">
                  <Loader2 className="w-5 h-5 animate-spin" />
                </div>
              ) : error ? (
                <div className="text-sm text-red-500 bg-red-50 dark:bg-red-900/20 p-3 rounded-xl border border-red-200 dark:border-red-800">
                  {error}
                </div>
              ) : previewInfo ? (
                <div className="space-y-4">
                  {/* 文件信息卡片 */}
                  <div className="bg-zinc-50 dark:bg-zinc-800/50 rounded-xl p-3 space-y-2">
                    <div className="text-xs font-medium text-zinc-500 dark:text-zinc-400 mb-2">
                      文件信息
                    </div>
                    <div className="text-sm text-zinc-600 dark:text-zinc-400 space-y-1.5">
                      <div className="flex justify-between items-center">
                        <span className="text-zinc-500 dark:text-zinc-500">类型</span>
                        <span className="text-zinc-700 dark:text-zinc-300 font-medium">{previewInfo.file_type}</span>
                      </div>
                      <div className="flex justify-between items-center">
                        <span className="text-zinc-500 dark:text-zinc-500">大小</span>
                        <span className="text-zinc-700 dark:text-zinc-300 font-medium">{formatFileSize(previewInfo.size)}</span>
                      </div>
                      {previewInfo.is_executable && (
                        <div className="flex justify-between items-center">
                          <span className="text-zinc-500 dark:text-zinc-500">可执行</span>
                          <span className="text-blue-500 font-medium">是</span>
                        </div>
                      )}
                    </div>
                  </div>

                  {/* 预览内容 */}
                  {previewInfo.preview_content && (
                    <div>
                      <div className="text-xs font-medium text-zinc-500 dark:text-zinc-400 mb-2">
                        预览内容
                      </div>
                      <pre className="text-xs text-zinc-700 dark:text-zinc-300 whitespace-pre-wrap break-all font-mono leading-relaxed bg-zinc-50 dark:bg-zinc-900 p-3 rounded-xl border border-zinc-200 dark:border-zinc-800">
                        {previewInfo.preview_content}
                      </pre>
                    </div>
                  )}

                  {/* 可执行文件警告 */}
                  {previewInfo.is_executable && (
                    <div className="p-3 bg-amber-50 dark:bg-amber-900/20 rounded-xl text-sm text-amber-700 dark:text-amber-300 border border-amber-200 dark:border-amber-800">
                      ⚠️ 注意：可执行文件可能存在安全风险，请谨慎运行。
                    </div>
                  )}
                </div>
              ) : null}
            </div>
          </motion.div>
        </>
      )}
    </AnimatePresence>
  );
}
