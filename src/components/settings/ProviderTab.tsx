import { useState } from "react";
import { Plus, Trash2, Check, Eye, EyeOff, Wifi, RefreshCw, Loader2 } from "lucide-react";
import { useSettingsStore } from "../../stores/settingsStore";
import { useProviderStore } from "../../stores/providerStore";
import { PRESET_PROVIDERS } from "../../lib/providers";
import { fetchModels } from "../../lib/tauri-bridge";
import { Button, Input, Select } from "../../shared/ui";

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
  const [customForm, setCustomForm] = useState<{ name: string; apiBase: string; providerType: "openai-compat" | "anthropic" }>({ name: "", apiBase: "https://api.openai.com/v1", providerType: "openai-compat" });

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
    if (!provider || !cred?.apiKey) { setTestResults((s) => ({ ...s, [providerId]: "请先输入 API Key" })); return; }
    setTesting((s) => ({ ...s, [providerId]: true }));
    setTestResults((s) => ({ ...s, [providerId]: null }));
    try {
      const models = await fetchModels(provider.apiBase, cred.apiKey, provider.providerType);
      setTestResults((s) => ({ ...s, [providerId]: `连接成功，${models.length} 个模型` }));
      if (models.length > 0) {
        updateProvider(providerId, { models });
        if (!activeModel || activeProviderId !== providerId) { setActiveProvider(providerId); setActiveModel(models[0]); }
      }
    } catch (e) { setTestResults((s) => ({ ...s, [providerId]: `失败: ${String(e).slice(0, 80)}` })); }
    finally { setTesting((s) => ({ ...s, [providerId]: false })); }
  };

  const saveModels = (providerId: string) => {
    const s = editingModels[providerId] || "";
    const models = s.split(",").map((m) => m.trim()).filter(Boolean);
    if (models.length > 0) { updateProvider(providerId, { models }); }
    setEditingModels((prev) => { const n = { ...prev }; delete n[providerId]; return n; });
  };

  return (
    <div className="space-y-5">
      {/* 预设 */}
      <div>
        <h3 className="text-xs font-medium text-zinc-400 uppercase tracking-wider mb-2">快速添加</h3>
        <div className="flex flex-wrap gap-1.5">
          {PRESET_PROVIDERS.map((preset) => {
            const id = preset.name.toLowerCase().replace(/\s+/g, "-");
            const added = providers.some((p) => p.id === id);
            return (
              <button key={id} onClick={() => !added && addPreset(preset)} disabled={added}
                className={`flex items-center gap-1 px-2.5 py-1 rounded-lg text-xs border transition-colors ${
                  added ? "border-green-300 dark:border-green-700 bg-green-50 dark:bg-green-900/20 text-green-700 dark:text-green-300 cursor-default"
                        : "border-zinc-200 dark:border-zinc-700 hover:border-slate-300 dark:hover:border-slate-600 bg-zinc-50 dark:bg-zinc-800 text-zinc-700 dark:text-zinc-300"
                }`}
              >{added ? <Check className="w-3 h-3" /> : <Plus className="w-3 h-3" />}{preset.name}</button>
            );
          })}
        </div>
      </div>

      {/* 自定义 */}
      <div>
        <h3 className="text-xs font-medium text-zinc-400 uppercase tracking-wider mb-2">自定义</h3>
        <div className="flex items-end gap-2">
          <Input label="名称" value={customForm.name} onChange={(e) => setCustomForm((f) => ({ ...f, name: e.target.value }))} placeholder="vLLM / Azure" />
          <Input label="API Base" value={customForm.apiBase} onChange={(e) => setCustomForm((f) => ({ ...f, apiBase: e.target.value }))} className="flex-[2]" />
          <Select label="类型" value={customForm.providerType}
            onChange={(e) => setCustomForm((f) => ({ ...f, providerType: e.target.value as "openai-compat" | "anthropic" }))}>
            <option value="openai-compat">OpenAI 兼容</option>
            <option value="anthropic">Anthropic</option>
          </Select>
          <Button onClick={addCustom} disabled={!customForm.name.trim()} size="sm"><Plus className="w-3 h-3" />添加</Button>
        </div>
      </div>

      {/* 已添加 */}
      {providers.length > 0 && (
        <div>
          <h3 className="text-xs font-medium text-zinc-400 uppercase tracking-wider mb-2">已添加 ({providers.length})</h3>
          <div className="space-y-3">
            {providers.map((provider) => {
              const cred = credentials[provider.id];
              const isActive = provider.id === activeProviderId;
              const isTesting = testing[provider.id];
              const testResult = testResults[provider.id];
              return (
                <div key={provider.id} className={`p-3.5 rounded-xl border transition-colors ${isActive ? "border-slate-400 dark:border-slate-600 bg-slate-50/50 dark:bg-slate-900/10" : "border-zinc-200 dark:border-zinc-700"}`}>
                  <div className="flex items-center justify-between mb-2">
                    <div className="flex items-center gap-2">
                      <span className="text-sm font-medium text-zinc-800 dark:text-zinc-200">{provider.name}</span>
                      <span className="text-[10px] text-zinc-400 bg-zinc-100 dark:bg-zinc-800 px-1.5 py-0.5 rounded">{provider.providerType}</span>
                    </div>
                    <div className="flex items-center gap-1">
                      {!isActive && <button onClick={() => { setActiveProvider(provider.id); if (provider.models[0]) setActiveModel(provider.models[0]); }} className="text-xs text-slate-600 dark:text-slate-400 hover:underline px-1">使用</button>}
                      {isActive && <span className="text-xs text-green-600 dark:text-green-400 font-medium">使用中</span>}
                      <button onClick={() => { removeProvider(provider.id); removeCredential(provider.id); }} className="p-1 rounded text-zinc-400 hover:text-red-500"><Trash2 className="w-3.5 h-3.5" /></button>
                    </div>
                  </div>

                  <Input value={provider.apiBase} onChange={(e) => updateProvider(provider.id, { apiBase: e.target.value })} className="font-mono mb-2" />

                  <div className="flex items-center gap-2 mb-2">
                    <div className="relative flex-1">
                      <input type={showKey[provider.id] ? "text" : "password"} value={cred?.apiKey || ""} onChange={(e) => setCredential(provider.id, e.target.value)}
                        className="w-full text-xs rounded-xl border border-zinc-300 dark:border-zinc-600 bg-white dark:bg-zinc-800 pl-3 pr-8 py-1.5 text-zinc-900 dark:text-zinc-100 placeholder:text-zinc-400 focus:outline-none focus:ring-1 focus:ring-slate-400" placeholder="API Key" />
                      <button onClick={() => setShowKey((s) => ({ ...s, [provider.id]: !s[provider.id] }))} className="absolute right-2 top-1/2 -translate-y-1/2 text-zinc-400">
                        {showKey[provider.id] ? <EyeOff className="w-3 h-3" /> : <Eye className="w-3 h-3" />}
                      </button>
                    </div>
                    <Button variant="secondary" size="sm" onClick={() => handleTest(provider.id)} disabled={isTesting || !cred?.apiKey}>
                      {isTesting ? <Loader2 className="w-3 h-3 animate-spin" /> : <Wifi className="w-3 h-3" />}测试
                    </Button>
                  </div>

                  {testResult && (
                    <div className={`mb-2 text-xs px-2 py-1 rounded-lg ${testResult.startsWith("连接成功") ? "bg-green-50 dark:bg-green-900/20 text-green-700 dark:text-green-300" : "bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-300"}`}>
                      {testResult}
                    </div>
                  )}

                  <div className="flex items-end gap-2">
                    <div className="flex-1">
                      <div className="flex items-center justify-between mb-0.5">
                        <span className="text-[10px] text-zinc-400">模型列表（逗号分隔）</span>
                        <button onClick={() => editingModels[provider.id] !== undefined ? saveModels(provider.id) : setEditingModels((s) => ({ ...s, [provider.id]: provider.models.join(", ")}))}
                          className="text-[10px] text-slate-500 hover:text-slate-600">
                          {editingModels[provider.id] !== undefined ? "保存" : "编辑"}
                        </button>
                      </div>
                      {editingModels[provider.id] !== undefined ? (
                        <input autoFocus value={editingModels[provider.id]} onChange={(e) => setEditingModels((s) => ({ ...s, [provider.id]: e.target.value }))}
                          onKeyDown={(e) => { if (e.key === "Enter") saveModels(provider.id); }}
                          className="w-full text-xs rounded-xl border border-slate-300 dark:border-slate-600 bg-white dark:bg-zinc-800 px-3 py-1.5 text-zinc-700 dark:text-zinc-300 focus:outline-none focus:ring-1 focus:ring-slate-400" />
                      ) : (
                        <div className="text-xs text-zinc-500 py-1.5 truncate">{provider.models.length > 0 ? provider.models.join(", ") : "尚未添加 — 点击测试连接自动拉取"}</div>
                      )}
                    </div>
                    <Button variant="secondary" size="sm" onClick={() => handleTest(provider.id)} disabled={isTesting || !cred?.apiKey}>
                      <RefreshCw className={`w-3 h-3 ${isTesting ? "animate-spin" : ""}`} />拉取
                    </Button>
                  </div>

                  {provider.models.length > 0 && (
                    <Select className="mt-2" value={provider.id === activeProviderId ? activeModel : ""} onChange={(e) => { setActiveProvider(provider.id); setActiveModel(e.target.value); }}>
                      <option value="" disabled>选择模型</option>
                      {provider.models.map((m) => (<option key={m} value={m}>{m}</option>))}
                    </Select>
                  )}
                </div>
              );
            })}
          </div>
        </div>
      )}

      {providers.length === 0 && (
        <div className="text-center py-8 text-zinc-400 dark:text-zinc-500 text-sm">点击预设供应商快速添加，或填写自定义表单接入自己的 API</div>
      )}
    </div>
  );
}
