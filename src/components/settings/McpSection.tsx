import { useState, useEffect } from "react";
import { Plus, Trash2, Power, PowerOff, Download } from "lucide-react";
import { addMcpServer, removeMcpServer, toggleMcpServer, listMcpServers, MCP_MARKETPLACE, type McpServerConfig } from "../../lib/tauri-bridge";

export default function McpSection() {
  const [servers, setServers] = useState<McpServerConfig[]>([]);
  const [showCustom, setShowCustom] = useState(false);
  const [customForm, setCustomForm] = useState({ name: "", command: "", args: "" });

  const refresh = async () => {
    try {
      const list = await listMcpServers();
      setServers(list);
    } catch { /* ignore */ }
  };

  useEffect(() => { refresh(); }, []);

  const handleInstall = async (plugin: typeof MCP_MARKETPLACE[number]) => {
    const id = "mcp-" + plugin.name.toLowerCase().replace(/\s+/g, "-") + "-" + Date.now();
    await addMcpServer({ id, name: plugin.name, command: plugin.command, args: plugin.args, enabled: false });
    refresh();
  };

  const handleToggle = async (id: string, enabled: boolean) => {
    await toggleMcpServer(id, enabled);
    refresh();
  };

  const handleDelete = async (id: string) => {
    await removeMcpServer(id);
    refresh();
  };

  const handleAdd = async () => {
    const name = customForm.name.trim();
    const command = customForm.command.trim();
    if (!name || !command) return;
    const id = "mcp-custom-" + name.toLowerCase().replace(/\s+/g, "-") + "-" + Date.now();
    const args = customForm.args.split(/\s+/).filter(Boolean);
    await addMcpServer({ id, name, command, args, enabled: false });
    setCustomForm({ name: "", command: "", args: "" });
    setShowCustom(false);
    refresh();
  };

  return (
    <section>
      <h3 className="text-xs font-medium text-zinc-400 uppercase tracking-wider mb-3">
        MCP 插件
      </h3>

      {/* 市场 */}
      <div className="mb-3">
        <p className="text-xs text-zinc-500 mb-2">一键安装精选 MCP 插件</p>
        <div className="grid grid-cols-2 gap-1.5 max-h-40 overflow-y-auto">
          {MCP_MARKETPLACE.map((plugin) => {
            const installed = servers.some((s) => s.name === plugin.name);
            return (
              <button
                key={plugin.name}
                onClick={() => !installed && handleInstall(plugin)}
                disabled={installed}
                className={`text-left px-2.5 py-1.5 rounded-lg text-xs border transition-colors ${
                  installed
                    ? "border-green-300 dark:border-green-700 bg-green-50 dark:bg-green-900/20 text-green-600 dark:text-green-400 cursor-default"
                    : "border-zinc-200 dark:border-zinc-700 hover:border-slate-300 dark:hover:border-slate-600"
                }`}
              >
                <div className="flex items-center gap-1">
                  {installed ? (
                    <Download className="w-2.5 h-2.5" />
                  ) : (
                    <Plus className="w-2.5 h-2.5" />
                  )}
                  <span className="font-medium text-zinc-700 dark:text-zinc-300">{plugin.name}</span>
                </div>
                <div className="text-[10px] text-zinc-400 truncate mt-0.5">{plugin.description}</div>
              </button>
            );
          })}
        </div>
      </div>

      {/* 已安装 */}
      {servers.length > 0 && (
        <div className="space-y-1.5 mb-3">
          {servers.map((s) => (
            <div key={s.id} className="flex items-center gap-2 px-3 py-2 rounded-lg border border-zinc-200 dark:border-zinc-700 text-xs">
              <span className="flex-1 font-medium text-zinc-700 dark:text-zinc-300 truncate">{s.name}</span>
              <span className="text-[10px] text-zinc-400">{s.command}</span>
              <button
                onClick={() => handleToggle(s.id, !s.enabled)}
                className={`p-1 rounded ${s.enabled ? "text-green-500" : "text-zinc-400"}`}
                title={s.enabled ? "禁用" : "启用"}
              >
                {s.enabled ? <Power className="w-3 h-3" /> : <PowerOff className="w-3 h-3" />}
              </button>
              <button onClick={() => handleDelete(s.id)} className="p-1 rounded text-zinc-400 hover:text-red-500">
                <Trash2 className="w-3 h-3" />
              </button>
            </div>
          ))}
        </div>
      )}

      {/* 自定义添加 */}
      {!showCustom ? (
        <button
          onClick={() => setShowCustom(true)}
          className="text-xs text-slate-500 hover:text-slate-600 flex items-center gap-1"
        >
          <Plus className="w-3 h-3" /> 自定义 MCP 服务器
        </button>
      ) : (
        <div className="space-y-1.5 p-3 rounded-xl border border-slate-200 dark:border-slate-800 bg-slate-50/50 dark:bg-slate-900/10">
          <input
            value={customForm.name}
            onChange={(e) => setCustomForm((f) => ({ ...f, name: e.target.value }))}
            placeholder="名称（如: 我的工具）"
            className="w-full text-xs rounded-lg border border-zinc-300 dark:border-zinc-600 bg-white dark:bg-zinc-800 px-2 py-1"
          />
          <input
            value={customForm.command}
            onChange={(e) => setCustomForm((f) => ({ ...f, command: e.target.value }))}
            placeholder="命令（如: node, python, npx）"
            className="w-full text-xs rounded-lg border border-zinc-300 dark:border-zinc-600 bg-white dark:bg-zinc-800 px-2 py-1"
          />
          <input
            value={customForm.args}
            onChange={(e) => setCustomForm((f) => ({ ...f, args: e.target.value }))}
            placeholder="参数（空格分隔）"
            className="w-full text-xs rounded-lg border border-zinc-300 dark:border-zinc-600 bg-white dark:bg-zinc-800 px-2 py-1"
          />
          <div className="flex gap-1.5">
            <button
              onClick={handleAdd}
              className="px-2.5 py-1 rounded-lg text-xs bg-slate-500 hover:bg-slate-600 text-white"
            >
              添加
            </button>
            <button
              onClick={() => setShowCustom(false)}
              className="px-2.5 py-1 rounded-lg text-xs border border-zinc-200 dark:border-zinc-700 text-zinc-500"
            >
              取消
            </button>
          </div>
        </div>
      )}
    </section>
  );
}
