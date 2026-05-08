import { useState, useEffect } from "react";
import { Plus, Trash2, Power, PowerOff, Download, Loader2, CheckCircle, XCircle, AlertCircle } from "lucide-react";
import { 
  addMcpServer, 
  removeMcpServer, 
  toggleMcpServer, 
  listMcpServers, 
  validateMcpCommand,
  testMcpServer,
  MCP_MARKETPLACE, 
  type McpServerConfig,
  type ValidationResult 
} from "../../lib/tauri-bridge";

export default function McpSection() {
  const [servers, setServers] = useState<McpServerConfig[]>([]);
  const [showCustom, setShowCustom] = useState(false);
  const [customForm, setCustomForm] = useState({ name: "", command: "", args: "" });
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<ValidationResult | null>(null);
  const [commandStatus, setCommandStatus] = useState<{
    valid: boolean;
    message: string;
  } | null>(null);

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
    // 检查是否已测试
    if (!testResult?.success) {
      alert("请先测试连接，确保服务器可用");
      return;
    }

    const name = customForm.name.trim();
    const command = customForm.command.trim();
    if (!name || !command) return;
    
    const id = "mcp-custom-" + name.toLowerCase().replace(/\s+/g, "-") + "-" + Date.now();
    const args = customForm.args.split(/\s+/).filter(Boolean);
    
    try {
      await addMcpServer({ id, name, command, args, enabled: false });
      setCustomForm({ name: "", command: "", args: "" });
      setTestResult(null);
      setCommandStatus(null);
      setShowCustom(false);
      refresh();
    } catch (error) {
      alert(`添加失败: ${error}`);
    }
  };

  // 验证命令（失焦时）
  const handleCommandBlur = async () => {
    const command = customForm.command.trim();
    if (!command) {
      setCommandStatus(null);
      return;
    }

    try {
      const version = await validateMcpCommand(command);
      setCommandStatus({
        valid: true,
        message: version
      });
    } catch (error) {
      setCommandStatus({
        valid: false,
        message: String(error)
      });
    }
  };

  // 测试 MCP 服务器连接
  const handleTestConnection = async () => {
    const command = customForm.command.trim();
    const args = customForm.args.split(/\s+/).filter(Boolean);
    
    if (!command) {
      alert("请输入命令");
      return;
    }

    setTesting(true);
    setTestResult(null);

    try {
      const result = await testMcpServer(command, args);
      setTestResult(result);
    } catch (error) {
      setTestResult({
        success: false,
        message: String(error),
        details: null
      });
    } finally {
      setTesting(false);
    }
  };

  return (
    <section className="overflow-x-hidden">
      <h3 className="text-xs font-medium text-zinc-400 uppercase tracking-wider mb-3 truncate">
        MCP 插件
      </h3>

      {/* 市场 */}
      <div className="mb-3 overflow-x-hidden">
        <p className="text-xs text-zinc-500 mb-2 truncate">一键安装精选 MCP 插件</p>
        <div className="grid grid-cols-2 gap-1.5 max-h-40 overflow-y-auto overflow-x-hidden chat-scrollbar">
          {MCP_MARKETPLACE.map((plugin) => {
            const installed = servers.some((s) => s.name === plugin.name);
            return (
              <button
                key={plugin.name}
                onClick={() => !installed && handleInstall(plugin)}
                disabled={installed}
                className={`text-left px-2.5 py-1.5 rounded-lg text-xs border transition-all duration-200 overflow-hidden ${
                  installed
                    ? "border-green-300 dark:border-green-700 bg-green-50 dark:bg-green-900/20 text-green-600 dark:text-green-400 cursor-default"
                    : "border-zinc-200/70 dark:border-zinc-700 hover:border-purple-300 dark:hover:border-purple-600 hover:shadow-md hover:shadow-purple-500/10"
                }`}
              >
                <div className="flex items-center gap-1 whitespace-nowrap overflow-hidden">
                  {installed ? (
                    <Download className="w-2.5 h-2.5 flex-shrink-0" />
                  ) : (
                    <Plus className="w-2.5 h-2.5 flex-shrink-0" />
                  )}
                  <span className="font-medium text-zinc-700 dark:text-zinc-300 truncate">{plugin.name}</span>
                </div>
                <div className="text-[10px] text-zinc-400 truncate mt-0.5">{plugin.description}</div>
              </button>
            );
          })}
        </div>
      </div>

      {/* 已安装 */}
      {servers.length > 0 && (
        <div className="space-y-1.5 mb-3 overflow-x-hidden">
          {servers.map((s) => (
            <div key={s.id} className="flex items-center gap-2 px-3 py-2 rounded-lg border border-zinc-200/70 dark:border-zinc-700 text-xs overflow-hidden">
              <span className="flex-1 font-medium text-zinc-700 dark:text-zinc-300 truncate">{s.name}</span>
              <span className="text-[10px] text-zinc-400 truncate whitespace-nowrap">{s.command}</span>
              <button
                onClick={() => handleToggle(s.id, !s.enabled)}
                className={`p-1 rounded transition-colors duration-200 flex-shrink-0 ${s.enabled ? "text-green-500 hover:text-green-600" : "text-zinc-400 hover:text-zinc-500"}`}
                title={s.enabled ? "禁用" : "启用"}
              >
                {s.enabled ? <Power className="w-3 h-3" /> : <PowerOff className="w-3 h-3" />}
              </button>
              <button 
                onClick={() => handleDelete(s.id)} 
                className="p-1 rounded text-zinc-400 hover:text-red-500 transition-colors duration-200 flex-shrink-0"
              >
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
          className="text-xs text-slate-500 hover:text-purple-600 dark:hover:text-purple-400 flex items-center gap-1 transition-colors duration-200 whitespace-nowrap"
        >
          <Plus className="w-3 h-3" /> 自定义 MCP 服务器
        </button>
      ) : (
        <div className="space-y-2 p-3 rounded-xl border border-slate-200/70 dark:border-slate-800 bg-gradient-to-br from-slate-50/50 to-purple-50/30 dark:from-slate-900/10 dark:to-purple-900/5 overflow-x-hidden">
          {/* 名称输入 */}
          <div className="overflow-x-hidden">
            <label className="text-[10px] text-zinc-500 dark:text-zinc-400 mb-1 block truncate">服务器名称 *</label>
            <input
              value={customForm.name}
              onChange={(e) => setCustomForm((f) => ({ ...f, name: e.target.value }))}
              placeholder="例如: 我的搜索工具"
              className="w-full text-xs rounded-lg border border-zinc-300 dark:border-zinc-600 bg-white dark:bg-zinc-800 px-2 py-1.5 focus:outline-none focus:ring-2 focus:ring-purple-500/50 transition-all duration-200 truncate"
            />
          </div>

          {/* 命令输入 + 实时验证 */}
          <div className="overflow-x-hidden">
            <label className="text-[10px] text-zinc-500 dark:text-zinc-400 mb-1 block truncate">命令 *</label>
            <input
              value={customForm.command}
              onChange={(e) => {
                setCustomForm((f) => ({ ...f, command: e.target.value }));
                setCommandStatus(null);
                setTestResult(null);
              }}
              onBlur={handleCommandBlur}
              placeholder="例如: uvx, npx, node"
              className="w-full text-xs rounded-lg border border-zinc-300 dark:border-zinc-600 bg-white dark:bg-zinc-800 px-2 py-1.5 focus:outline-none focus:ring-2 focus:ring-purple-500/50 transition-all duration-200 truncate"
            />
            {/* 命令验证状态 */}
            {commandStatus && (
              <div className={`flex items-start gap-1 text-[10px] mt-1 overflow-x-hidden ${
                commandStatus.valid 
                  ? 'text-green-600 dark:text-green-400' 
                  : 'text-red-600 dark:text-red-400'
              }`}>
                {commandStatus.valid ? (
                  <CheckCircle className="w-3 h-3 flex-shrink-0 mt-0.5" />
                ) : (
                  <XCircle className="w-3 h-3 flex-shrink-0 mt-0.5" />
                )}
                <span className="line-clamp-2 break-words">{commandStatus.message}</span>
              </div>
            )}
          </div>

          {/* 参数输入 */}
          <div className="overflow-x-hidden">
            <label className="text-[10px] text-zinc-500 dark:text-zinc-400 mb-1 block truncate">参数（空格分隔）</label>
            <input
              value={customForm.args}
              onChange={(e) => {
                setCustomForm((f) => ({ ...f, args: e.target.value }));
                setTestResult(null);
              }}
              placeholder="例如: mcp-server-fetch-typescript"
              className="w-full text-xs rounded-lg border border-zinc-300 dark:border-zinc-600 bg-white dark:bg-zinc-800 px-2 py-1.5 focus:outline-none focus:ring-2 focus:ring-purple-500/50 transition-all duration-200 truncate"
            />
          </div>

          {/* 测试连接按钮 */}
          <button
            onClick={handleTestConnection}
            disabled={testing || !customForm.command.trim()}
            className="w-full px-3 py-2 rounded-lg text-xs font-medium bg-gradient-to-r from-blue-500 to-blue-600 hover:from-blue-600 hover:to-blue-700 disabled:from-gray-400 disabled:to-gray-400 text-white transition-all duration-200 shadow-md hover:shadow-lg hover:shadow-blue-500/30 disabled:cursor-not-allowed flex items-center justify-center gap-2 whitespace-nowrap"
          >
            {testing ? (
              <>
                <Loader2 className="w-3 h-3 animate-spin flex-shrink-0" />
                <span className="truncate">测试中...</span>
              </>
            ) : (
              <>
                <AlertCircle className="w-3 h-3 flex-shrink-0" />
                <span className="truncate">测试连接</span>
              </>
            )}
          </button>

          {/* 测试结果显示 */}
          {testResult && (
            <div className={`p-3 rounded-lg text-[10px] border overflow-x-hidden ${
              testResult.success 
                ? 'bg-green-50 dark:bg-green-900/20 border-green-200 dark:border-green-800' 
                : 'bg-red-50 dark:bg-red-900/20 border-red-200 dark:border-red-800'
            }`}>
              <div className={`flex items-start gap-2 overflow-x-hidden ${
                testResult.success 
                  ? 'text-green-700 dark:text-green-300' 
                  : 'text-red-700 dark:text-red-300'
              }`}>
                {testResult.success ? (
                  <CheckCircle className="w-4 h-4 flex-shrink-0 mt-0.5" />
                ) : (
                  <XCircle className="w-4 h-4 flex-shrink-0 mt-0.5" />
                )}
                <pre className="whitespace-pre-wrap flex-1 font-mono break-words overflow-x-hidden">
                  {testResult.message}
                </pre>
              </div>
              
              {/* 显示发现的工具 */}
              {testResult.success && testResult.details?.tools_discovered && testResult.details.tools_discovered.length > 0 && (
                <div className="mt-2 pt-2 border-t border-green-200 dark:border-green-800 overflow-x-hidden">
                  <div className="font-medium text-green-700 dark:text-green-300 mb-1.5 flex items-center gap-1 whitespace-nowrap">
                    <CheckCircle className="w-3 h-3 flex-shrink-0" />
                    <span className="truncate">发现 {testResult.details.tools_discovered.length} 个工具：</span>
                  </div>
                  <div className="space-y-1 max-h-32 overflow-y-auto overflow-x-hidden chat-scrollbar">
                    {testResult.details.tools_discovered.map((tool) => (
                      <div key={tool} className="text-green-600 dark:text-green-400 pl-2 truncate">
                        • {tool}
                      </div>
                    ))}
                  </div>
                  <div className="mt-2 text-green-600 dark:text-green-400 truncate">
                    响应时间: {testResult.details.response_time_ms} ms
                  </div>
                </div>
              )}
            </div>
          )}

          {/* 操作按钮 */}
          <div className="flex gap-2 pt-1 overflow-x-hidden">
            <button
              onClick={handleAdd}
              disabled={!testResult?.success}
              className="flex-1 px-3 py-1.5 rounded-lg text-xs font-medium bg-gradient-to-r from-purple-500 to-blue-500 hover:from-purple-600 hover:to-blue-600 disabled:from-gray-400 disabled:to-gray-400 text-white transition-all duration-200 shadow-md hover:shadow-lg hover:shadow-purple-500/30 disabled:cursor-not-allowed whitespace-nowrap truncate"
            >
              添加服务器
            </button>
            <button
              onClick={() => {
                setShowCustom(false);
                setTestResult(null);
                setCommandStatus(null);
                setCustomForm({ name: "", command: "", args: "" });
              }}
              className="px-3 py-1.5 rounded-lg text-xs font-medium border border-zinc-200/70 dark:border-zinc-700 text-zinc-600 dark:text-zinc-400 hover:bg-zinc-100 dark:hover:bg-zinc-800 transition-all duration-200 whitespace-nowrap truncate"
            >
              取消
            </button>
          </div>
        </div>
      )}
    </section>
  );
}
