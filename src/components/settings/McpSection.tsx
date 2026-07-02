import { Plug } from "lucide-react";

export default function McpSection() {
  return (
    <div className="flex flex-col items-center justify-center py-16 text-center">
      <div className="p-4 rounded-2xl bg-gradient-to-br from-brand-purple-600/10 to-brand-blue-600/10 mb-4">
        <Plug className="w-10 h-10 text-ds-muted" />
      </div>
      <h3 className="text-lg font-semibold text-ds-text-primary mb-2">
        MCP 工具
      </h3>
      <p className="text-sm text-ds-muted max-w-sm">
        即将推出 — 通过 Model Context Protocol 扩展 AI 能力，
        让助手可以读取文件、执行命令、访问数据库等。
      </p>
    </div>
  );
}
