import { useState, useMemo, useRef, useEffect } from "react";
import { Plus, Trash2, Check, Eye, EyeOff, Wifi, RefreshCw, Loader2, Search, AlertCircle, Sparkles } from "lucide-react";
import { useSettingsStore } from "../../stores/settingsStore";
import { useProviderStore } from "../../stores/providerStore";
import { PRESET_PROVIDERS } from "../../lib/providers";
import { fetchModels } from "../../lib/tauri-bridge";
import { Button, Input, Select } from "../../shared/ui";
import { cn } from "../../lib/cn";

/// 可搜索的模型选择组件
function SearchableModelSelect({
  models, activeModel, onSelect
}: {
  models: string[]; activeModel: string; onSelect: (model: string) => void;
}) {
  const [query, setQuery] = useState(activeModel);
  const [open, setOpen] = useState(false);
  const [highlight, setHighlight] = useState(0);
  const containerRef = useRef<HTMLDivElement>(null);

  const filtered = useMemo(() => {
    if (!query.trim()) return models;
    const q = query.toLowerCase();
    const exact = models.filter((m) => m.toLowerCase() === q);
    const fuzzy = models.filter((m) => m.toLowerCase().includes(q) && m.toLowerCase() !== q);
    return [...exact, ...fuzzy];
  }, [models, query]);

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, []);

  const selectModel = (model: string) => {
    setQuery(model);
    onSelect(model);
    setOpen(false);
  };

  const handleKey = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") {
      e.preventDefault();
      if (open && filtered.length > 0) {
        selectModel(filtered[highlight] || filtered[0]);
      } else if (query.trim()) {
        onSelect(query.trim());
        setOpen(false);
      }
    }
    if (e.key === "ArrowDown") { e.preventDefault(); setHighlight((h) => Math.min(h + 1, filtered.length - 1)); }
    if (e.key === "ArrowUp") { e.preventDefault(); setHighlight((h) => Math.max(h - 1, 0)); }
  };

  return (
    <div ref={containerRef} className="relative">
      <div className="relative">
        <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 w-3 h-3 text-zinc-400 flex-shrink-0" />
        <input
          value={query}
          onChange={(e) => { setQuery(e.target.value); setOpen(true); setHighlight(0); }}
          onFocus={() => setOpen(true)}
          onKeyDown={handleKey}
          placeholder="搜索或输入模型名..."
          className="w-full text-xs rounded-xl border border-zinc-300 dark:border-zinc-600 bg-white dark:bg-zinc-800 pl-7 pr-3 py-1.5 text-zinc-700 dark:text-zinc-300 focus:outline-none focus:ring-1 focus:ring-purple-400 truncate"
        />
      </div>
      {open && filtered.length > 0 && (
        <div className="absolute z-50 mt-1 w-full max-h-40 overflow-y-auto overflow-x-hidden chat-scrollbar rounded-xl border border-zinc-200 dark:border-zinc-700 bg-white dark:bg-zinc-800 shadow-lg">
          {filtered.map((model, i) => (
            <button
              key={model}
              onClick={() => selectModel(model)}
              className={cn(
                "w-full text-left px-3 py-1.5 text-xs transition-colors truncate",
                "hover:bg-zinc-100 dark:hover:bg-zinc-700",
                i === highlight && "bg-purple-100 dark:bg-purple-900/30",
                model === activeModel && "text-purple-700 dark:text-purple-300 font-medium"
              )}
              title={model}
            >
              {model}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

export default function ProviderTab() {
  const activeProviderId = useSettingsStore((s) => s.activeProviderId);
  const activeModel = useSettingsStore((s) => s.activeModel);
  const credentials = useSettingsStore((s) => s.credentials);
  const setActiveProvider = useSettingsStore((s) => s.setActiveProvider);
  const setActiveModel = useSettingsStore((s) => s.setActiveModel);
  const setCredential = useSettingsStore((s) => s.setCredential);
  const removeCredential = useSettingsStore((s) => s.removeCredential);
  const providers = useProviderStore((s) => s.providers);
  const addProvider = useProviderStore((s) => s.addProvider);
  const removeProvider = useProviderStore((s) => s.removeProvider);
  const updateProvider = useProviderStore((s) => s.updateProvider);

  const [showKey, setShowKey] = useState<Record<string, boolean>>({});
  const [testing, setTesting] = useState<Record<string, boolean>>({});
  const [testResults, setTestResults] = useState<Record<string, string | null>>({});
  const [editingModels, setEditingModels] = useState<Record<string, string>>({});
  const [customForm, setCustomForm] = useState<{ name: string; apiBase: string; providerType: "openai-compat" | "anthropic" }>({ 
    name: "", 
    apiBase: "https://api.openai.com/v1", 
    providerType: "openai-compat" 
  });

  const addPreset = (preset: (typeof PRESET_PROVIDERS)[number]) => {
    const id = preset.name.toLowerCase().replace(/\s+/g, "-");
    if (providers.find((p) => p.id === id)) return;
    addProvider({ ...preset, id });
    setActiveProvider(id);
    setActiveModel(preset.models[0]);
  };

  const addCustom = () => {
    const name = customForm.name.trim();
    if (!name) return;
    const id = "custom-" + name.toLowerCase().replace(/\s+/g, "-") + "-" + Date.now();
    addProvider({ id, name, apiBase: customForm.apiBase, models: [], providerType: customForm.providerType });
    setCustomForm({ name: "", apiBase: "https://api.openai.com/v1", providerType: "openai-compat" });
  };

  const handleTest = async (providerId: string) => {
    const provider = providers.find((p) => p.id === providerId);
    const cred = credentials[providerId];
    if (!provider || !cred?.apiKey) { 
      setTestResults((s) => ({ ...s, [providerId]: "请先输入 API Key" })); 
      return; 
    }
    setTesting((s) => ({ ...s, [providerId]: true }));
    setTestResults((s) => ({ ...s, [providerId]: null }));
    try {
      const models = await fetchModels(provider.apiBase, cred.apiKey, provider.providerType);
      setTestResults((s) => ({ ...s, [providerId]: `✓ 连接成功，${models.length} 个模型` }));
      if (models.length > 0) {
        updateProvider(providerId, { models });
        if (!activeModel || activeProviderId !== providerId) { 
          setActiveProvider(providerId); 
          setActiveModel(models[0]); 
        }
      }
    } catch (e) { 
      setTestResults((s) => ({ ...s, [providerId]: `✗ ${String(e).slice(0, 60)}...` })); 
    }
    finally { setTesting((s) => ({ ...s, [providerId]: false })); }
  };

  const saveModels = (providerId: string) => {
    const s = editingModels[providerId] || "";
    const models = s.split(",").map((m) => m.trim()).filter(Boolean);
    updateProvider(providerId, { models });
    setEditingModels((prev) => { const n = { ...prev }; delete n[providerId]; return n; });
  };

  return (
    <div className="space-y-6 overflow-x-hidden">
      {/* 预设供应商 */}
      <div>
        <h3 className="text-sm font-semibold bg-gradient-to-r from-purple-600 to-blue-600 bg-clip-text text-transparent mb-3 flex items-center gap-2">
          <Sparkles className="w-4 h-4 text-purple-500" />
          <span className="truncate">快速添加供应商</span>
        </h3>
        <div className="grid grid-cols-3 gap-2">
          {PRESET_PROVIDERS.map((preset) => {
            const id = preset.name.toLowerCase().replace(/\s+/g, "-");
            const added = providers.some((p) => p.id === id);
            return (
              <button 
                key={id} 
                onClick={() => !added && addPreset(preset)} 
                disabled={added}
                className={cn(
                  "flex items-center justify-center gap-1.5 px-3 py-2 rounded-xl text-xs font-medium border transition-all duration-200 whitespace-nowrap",
                  added 
                    ? "border-green-300 dark:border-green-700 bg-gradient-to-r from-green-50 to-emerald-50 dark:from-green-900/20 dark:to-emerald-900/20 text-green-700 dark:text-green-300 cursor-default shadow-sm" 
                    : "border-zinc-200/70 dark:border-zinc-700/70 hover:border-purple-300 dark:hover:border-purple-600 bg-white dark:bg-zinc-800 text-zinc-700 dark:text-zinc-300 hover:shadow-md hover:shadow-purple-500/10 hover:-translate-y-0.5"
                )}
                title={preset.name}
              >
                {added ? <Check className="w-3 h-3 flex-shrink-0" /> : <Plus className="w-3 h-3 flex-shrink-0" />}
                <span className="truncate">{preset.name}</span>
              </button>
            );
          })}
        </div>
      </div>

      {/* 自定义供应商 */}
      <div>
        <h3 className="text-sm font-semibold bg-gradient-to-r from-purple-600 to-blue-600 bg-clip-text text-transparent mb-3 flex items-center gap-2">
          <Plus className="w-4 h-4 text-purple-500" />
          <span className="truncate">自定义供应商</span>
        </h3>
        <div className="grid grid-cols-12 gap-2">
          <div className="col-span-3">
            <Input 
              label="名称" 
              value={customForm.name} 
              onChange={(e) => setCustomForm((f) => ({ ...f, name: e.target.value }))} 
              placeholder="vLLM" 
              className="truncate"
            />
          </div>
          <div className="col-span-5">
            <Input 
              label="API Base" 
              value={customForm.apiBase} 
              onChange={(e) => setCustomForm((f) => ({ ...f, apiBase: e.target.value }))} 
              className="truncate font-mono text-xs"
            />
          </div>
          <div className="col-span-2">
            <Select 
              label="类型" 
              value={customForm.providerType}
              onChange={(e) => setCustomForm((f) => ({ ...f, providerType: e.target.value as "openai-compat" | "anthropic" }))}
            >
              <option value="openai-compat">OpenAI</option>
              <option value="anthropic">Anthropic</option>
            </Select>
          </div>
          <div className="col-span-2 flex items-end">
            <Button 
              onClick={addCustom} 
              disabled={!customForm.name.trim()} 
              size="sm" 
              className="w-full whitespace-nowrap"
            >
              <Plus className="w-3 h-3" />
              添加
            </Button>
          </div>
        </div>
      </div>

      {/* 已添加的供应商 */}
      {providers.length > 0 && (
        <div>
          <h3 className="text-sm font-semibold bg-gradient-to-r from-purple-600 to-blue-600 bg-clip-text text-transparent mb-3 flex items-center gap-2">
            <Wifi className="w-4 h-4 text-purple-500" />
            <span className="truncate">已配置供应商 ({providers.length})</span>
          </h3>
          <div className="space-y-3">
            {providers.map((provider) => {
              const cred = credentials[provider.id];
              const isActive = provider.id === activeProviderId;
              const isTesting = testing[provider.id];
              const testResult = testResults[provider.id];
              return (
                <div 
                  key={provider.id} 
                  className={cn(
                    "p-4 rounded-xl border transition-all duration-200 overflow-hidden",
                    isActive 
                      ? "border-purple-300 dark:border-purple-700/50 bg-gradient-to-br from-purple-50/50 to-blue-50/50 dark:from-purple-950/20 dark:to-blue-950/20 shadow-md shadow-purple-500/10" 
                      : "border-zinc-200/70 dark:border-zinc-700/70 hover:border-purple-200 dark:hover:border-purple-800/50"
                  )}
                >
                  {/* 头部 */}
                  <div className="flex items-center justify-between mb-3">
                    <div className="flex items-center gap-2 min-w-0 flex-1">
                      <span className="text-sm font-semibold text-zinc-800 dark:text-zinc-200 truncate">{provider.name}</span>
                      <span className="text-[10px] text-zinc-400 bg-zinc-100 dark:bg-zinc-800 px-1.5 py-0.5 rounded whitespace-nowrap flex-shrink-0">
                        {provider.providerType === "anthropic" ? "Anthropic" : "OpenAI"}
                      </span>
                    </div>
                    <div className="flex items-center gap-2 flex-shrink-0">
                      {!isActive && (
                        <button 
                          onClick={() => { 
                            setActiveProvider(provider.id); 
                            if (provider.models[0]) setActiveModel(provider.models[0]); 
                          }} 
                          className="text-xs text-purple-600 dark:text-purple-400 hover:underline px-2 py-1 font-medium whitespace-nowrap"
                        >
                          使用
                        </button>
                      )}
                      {isActive && (
                        <span className="text-xs text-green-600 dark:text-green-400 font-medium flex items-center gap-1 whitespace-nowrap">
                          <Check className="w-3 h-3" />使用中
                        </span>
                      )}
                      <button 
                        onClick={() => { removeProvider(provider.id); removeCredential(provider.id); }} 
                        className="p-1.5 rounded text-zinc-400 hover:text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors"
                        title="删除"
                      >
                        <Trash2 className="w-3.5 h-3.5" />
                      </button>
                    </div>
                  </div>

                  {/* API Base */}
                  <Input 
                    value={provider.apiBase} 
                    onChange={(e) => updateProvider(provider.id, { apiBase: e.target.value })} 
                    className="font-mono text-xs mb-3 truncate" 
                    placeholder="API Base URL"
                  />

                  {/* API Key */}
                  <div className="flex items-center gap-2 mb-3">
                    <div className="relative flex-1 min-w-0">
                      <input 
                        type={showKey[provider.id] ? "text" : "password"} 
                        value={cred?.apiKey || ""} 
                        onChange={(e) => setCredential(provider.id, e.target.value)}
                        className="w-full text-xs rounded-xl border border-zinc-300 dark:border-zinc-600 bg-white dark:bg-zinc-800 pl-3 pr-8 py-2 text-zinc-900 dark:text-zinc-100 placeholder:text-zinc-400 focus:outline-none focus:ring-1 focus:ring-purple-400 truncate" 
                        placeholder="API Key" 
                      />
                      <button 
                        onClick={() => setShowKey((s) => ({ ...s, [provider.id]: !s[provider.id] }))} 
                        className="absolute right-2 top-1/2 -translate-y-1/2 text-zinc-400 hover:text-zinc-600"
                      >
                        {showKey[provider.id] ? <EyeOff className="w-3 h-3" /> : <Eye className="w-3 h-3" />}
                      </button>
                    </div>
                    <Button 
                      variant="secondary" 
                      size="sm" 
                      onClick={() => handleTest(provider.id)} 
                      disabled={isTesting || !cred?.apiKey}
                      className="whitespace-nowrap flex-shrink-0"
                    >
                      {isTesting ? <Loader2 className="w-3 h-3 animate-spin" /> : <Wifi className="w-3 h-3" />}
                      测试
                    </Button>
                  </div>

                  {/* 测试结果 */}
                  {testResult && (
                    <div className={cn(
                      "mb-3 text-xs px-3 py-2 rounded-lg flex items-start gap-2",
                      testResult.startsWith("✓") 
                        ? "bg-green-50 dark:bg-green-900/20 text-green-700 dark:text-green-300" 
                        : "bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-300"
                    )}>
                      <AlertCircle className="w-3 h-3 flex-shrink-0 mt-0.5" />
                      <span className="flex-1 min-w-0 break-words">{testResult}</span>
                    </div>
                  )}

                  {/* 模型选择 */}
                  <div className="flex items-end gap-2">
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center justify-between mb-1">
                        <span className="text-[10px] text-zinc-400 truncate">模型</span>
                        <button 
                          onClick={() => editingModels[provider.id] !== undefined 
                            ? saveModels(provider.id) 
                            : setEditingModels((s) => ({ ...s, [provider.id]: provider.models.join(", ")}))}
                          className="text-[10px] text-purple-500 hover:text-purple-600 whitespace-nowrap"
                        >
                          {editingModels[provider.id] !== undefined ? "保存" : "编辑"}
                        </button>
                      </div>
                      {editingModels[provider.id] !== undefined ? (
                        <input 
                          autoFocus 
                          value={editingModels[provider.id]} 
                          onChange={(e) => setEditingModels((s) => ({ ...s, [provider.id]: e.target.value }))}
                          onKeyDown={(e) => { if (e.key === "Enter") saveModels(provider.id); }}
                          placeholder="模型名称，逗号分隔"
                          className="w-full text-xs rounded-xl border border-purple-300 dark:border-purple-600 bg-white dark:bg-zinc-800 px-3 py-1.5 text-zinc-700 dark:text-zinc-300 focus:outline-none focus:ring-1 focus:ring-purple-400 truncate" 
                        />
                      ) : (
                        <SearchableModelSelect
                          models={provider.models}
                          activeModel={provider.id === activeProviderId ? activeModel : ""}
                          onSelect={(model) => { setActiveProvider(provider.id); setActiveModel(model); }}
                        />
                      )}
                    </div>
                    <Button 
                      variant="secondary" 
                      size="sm" 
                      onClick={() => handleTest(provider.id)} 
                      disabled={isTesting || !cred?.apiKey}
                      className="whitespace-nowrap flex-shrink-0"
                    >
                      <RefreshCw className={cn("w-3 h-3", isTesting && "animate-spin")} />
                      拉取
                    </Button>
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {providers.length === 0 && (
        <div className="text-center py-12 text-zinc-400 dark:text-zinc-500 text-sm">
          <Sparkles className="w-8 h-8 mx-auto mb-2 opacity-50" />
          <p className="truncate">点击上方快速添加供应商，或自定义配置</p>
        </div>
      )}
    </div>
  );
}
