import { useState, useMemo, useRef, useEffect } from "react";
import { Plus, Check, Trash2, Eye, EyeOff, Loader2, AlertCircle, Search } from "lucide-react";
import { cn } from "../../lib/cn";
import { useSettingsStore } from "../../stores/settingsStore";
import { useProviderStore } from "../../stores/providerStore";
import { fetchModels } from "../../lib/tauri-bridge";

const PRESET_PROVIDERS = [
  { name: "OpenAI", apiBase: "https://api.openai.com/v1", models: ["gpt-4o", "gpt-4o-mini", "gpt-4-turbo", "o1-mini"], providerType: "openai-compat" as const },
  { name: "Anthropic", apiBase: "https://api.anthropic.com/v1", models: ["claude-sonnet-4-20250514", "claude-haiku-3-5-sonnet-20241022"], providerType: "anthropic" as const },
  { name: "DeepSeek", apiBase: "https://api.deepseek.com/v1", models: ["deepseek-chat", "deepseek-reasoner"], providerType: "openai-compat" as const },
];

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
        <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 w-3 h-3 text-ds-muted" />
        <input
          value={query}
          onChange={(e) => { setQuery(e.target.value); setOpen(true); setHighlight(0); }}
          onFocus={() => setOpen(true)}
          onKeyDown={handleKey}
          placeholder="搜索或输入模型名..."
          className="w-full text-xs rounded-lg border border-ds-border bg-ds-surface-elevated pl-7 pr-3 py-1.5 text-ds-text-primary placeholder:text-ds-muted focus:outline-none focus:ring-1 focus:ring-ds-accent/40 focus:border-ds-accent transition-colors"
        />
      </div>
      {open && filtered.length > 0 && (
        <div className="absolute z-50 mt-1 w-full max-h-40 overflow-y-auto rounded-lg border border-ds-border bg-ds-surface-card shadow-lg backdrop-blur-xl">
          {filtered.map((model, i) => (
            <button
              key={model}
              onClick={() => selectModel(model)}
              className={cn(
                "w-full text-left px-3 py-1.5 text-xs transition-colors",
                "hover:bg-ds-hover",
                i === highlight && "bg-ds-selected",
                model === activeModel && "text-ds-accent font-medium"
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
  const [showCustom, setShowCustom] = useState(false);
  const [customForm, setCustomForm] = useState<{ name: string; apiBase: string; providerType: "openai-compat" | "anthropic" }>({ name: "", apiBase: "https://api.openai.com/v1", providerType: "openai-compat" });

  const addPreset = (preset: (typeof PRESET_PROVIDERS)[number]) => {
    const id = preset.name.toLowerCase().replace(/\s+/g, "-");
    if (providers.find((p) => p.id === id)) return;
    addProvider({ ...preset, id });
    setActiveProvider(id);
    setActiveModel(preset.models[0]);
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
      setTestResults((s) => ({ ...s, [providerId]: `✗ ${String(e).slice(0, 80)}` }));
    } finally {
      setTesting((s) => ({ ...s, [providerId]: false }));
    }
  };

  return (
    <div className="space-y-6">
      <div>
        <h3 className="text-xs font-medium text-ds-muted uppercase tracking-wider mb-3">快速添加</h3>
        <div className="flex flex-wrap gap-2">
          {PRESET_PROVIDERS.map((preset) => {
            const id = preset.name.toLowerCase().replace(/\s+/g, "-");
            const added = providers.some((p) => p.id === id);
            return (
              <button
                key={id}
                onClick={() => !added && addPreset(preset)}
                disabled={added}
                className={cn(
                  "inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium border transition-colors",
                  added
                    ? "border-ds-success/30 bg-ds-success/10 text-ds-success"
                    : "border-ds-border bg-ds-surface-elevated text-ds-text-primary hover:border-ds-accent/30 hover:bg-ds-hover"
                )}
              >
                {added ? <Check className="w-3 h-3" /> : <Plus className="w-3 h-3" />}
                {preset.name}
              </button>
            );
          })}
          <button
            onClick={() => setShowCustom(!showCustom)}
            className={cn(
              "inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium border transition-colors",
              showCustom
                ? "border-ds-accent/40 bg-ds-accent/10 text-ds-accent"
                : "border-ds-border bg-ds-surface-elevated text-ds-muted hover:border-ds-accent/30 hover:text-ds-text-primary"
            )}
          >
            <Plus className="w-3 h-3" />
            自定义
          </button>
        </div>
      </div>

      {showCustom && (
        <div className="p-3 rounded-lg border border-ds-border bg-ds-surface-elevated space-y-2">
          <div className="flex gap-2">
            <input
              value={customForm.name}
              onChange={(e) => setCustomForm((f) => ({ ...f, name: e.target.value }))}
              placeholder="名称"
              className="flex-1 text-xs rounded-lg border border-ds-border bg-ds-bg-main px-2.5 py-1.5 text-ds-text-primary placeholder:text-ds-muted focus:outline-none focus:ring-1 focus:ring-ds-accent/40 focus:border-ds-accent transition-colors"
            />
            <input
              value={customForm.apiBase}
              onChange={(e) => setCustomForm((f) => ({ ...f, apiBase: e.target.value }))}
              placeholder="API Base URL"
              className="flex-[2] text-xs rounded-lg border border-ds-border bg-ds-bg-main px-2.5 py-1.5 text-ds-text-primary placeholder:text-ds-muted focus:outline-none focus:ring-1 focus:ring-ds-accent/40 focus:border-ds-accent transition-colors font-mono"
            />
            <select
              value={customForm.providerType}
              onChange={(e) => setCustomForm((f) => ({ ...f, providerType: e.target.value as "openai-compat" | "anthropic" }))}
              className="text-xs rounded-lg border border-ds-border bg-ds-bg-main px-2 py-1.5 text-ds-text-primary focus:outline-none focus:ring-1 focus:ring-ds-accent/40 focus:border-ds-accent"
            >
              <option value="openai-compat">OpenAI</option>
              <option value="anthropic">Anthropic</option>
            </select>
          </div>
          <div className="flex gap-2">
            <button
              onClick={() => {
                const name = customForm.name.trim();
                if (!name) return;
                const id = "custom-" + name.toLowerCase().replace(/\s+/g, "-") + "-" + Date.now();
                addProvider({ id, name, apiBase: customForm.apiBase, models: [], providerType: customForm.providerType });
                setCustomForm({ name: "", apiBase: "https://api.openai.com/v1", providerType: "openai-compat" });
                setShowCustom(false);
              }}
              disabled={!customForm.name.trim()}
              className="px-3 py-1.5 rounded-lg text-xs font-medium bg-ds-accent text-white hover:opacity-90 disabled:opacity-40 transition-all"
            >
              添加
            </button>
            <button
              onClick={() => { setShowCustom(false); setCustomForm({ name: "", apiBase: "https://api.openai.com/v1", providerType: "openai-compat" }); }}
              className="px-3 py-1.5 rounded-lg text-xs font-medium border border-ds-border text-ds-muted hover:text-ds-text-primary hover:bg-ds-hover transition-colors"
            >
              取消
            </button>
          </div>
        </div>
      )}

      {providers.length === 0 && (
        <div className="text-center py-10">
          <p className="text-sm text-ds-muted">点击上方按钮添加一个供应商开始使用</p>
        </div>
      )}

      <div className="space-y-2">
        {providers.map((provider) => {
          const cred = credentials[provider.id];
          const isActive = provider.id === activeProviderId;
          const isTesting = testing[provider.id];
          const testResult = testResults[provider.id];
          return (
            <div
              key={provider.id}
              className={cn(
                "rounded-lg border transition-colors",
                isActive
                  ? "border-ds-accent/30 bg-ds-surface-elevated"
                  : "border-ds-border bg-ds-surface-elevated/50 hover:border-ds-accent/20"
              )}
            >
              <div className="flex items-center justify-between px-4 py-2.5">
                <div className="flex items-center gap-2 min-w-0 flex-1">
                  <span className="text-sm font-medium text-ds-text-primary truncate">{provider.name}</span>
                  <span className="text-[10px] text-ds-muted bg-ds-bg-main px-1.5 py-0.5 rounded">
                    {provider.providerType === "anthropic" ? "Anthropic" : "OpenAI"}
                  </span>
                  {isActive && (
                    <span className="text-[10px] text-ds-accent font-medium">使用中</span>
                  )}
                </div>
                <div className="flex items-center gap-1">
                  {!isActive && (
                    <button
                      onClick={() => { setActiveProvider(provider.id); if (provider.models[0]) setActiveModel(provider.models[0]); }}
                      className="px-2 py-1 text-xs text-ds-accent hover:bg-ds-hover rounded-md transition-colors font-medium"
                    >
                      使用
                    </button>
                  )}
                  <button
                    onClick={() => { removeProvider(provider.id); removeCredential(provider.id); }}
                    className="p-1 rounded text-ds-muted hover:text-ds-danger hover:bg-ds-danger/10 transition-colors"
                    title="删除"
                  >
                    <Trash2 className="w-3.5 h-3.5" />
                  </button>
                </div>
              </div>

              <div className="px-4 pb-3.5 space-y-2.5">
                <div>
                  <label className="text-[10px] text-ds-muted block mb-0.5">API Base</label>
                  <input
                    value={provider.apiBase}
                    onChange={(e) => updateProvider(provider.id, { apiBase: e.target.value })}
                    className="w-full text-xs rounded-md border border-ds-border bg-ds-bg-main px-2.5 py-1.5 text-ds-text-primary focus:outline-none focus:ring-1 focus:ring-ds-accent/40 focus:border-ds-accent transition-colors font-mono"
                  />
                </div>
                <div className="flex gap-3 items-end">
                  <div className="flex-1">
                    <label className="text-[10px] text-ds-muted block mb-0.5">API Key</label>
                    <div className="relative">
                      <input
                        type={showKey[provider.id] ? "text" : "password"}
                        value={cred?.apiKey || ""}
                        onChange={(e) => setCredential(provider.id, e.target.value)}
                        className="w-full text-xs rounded-md border border-ds-border bg-ds-bg-main px-2.5 py-1.5 pr-7 text-ds-text-primary placeholder:text-ds-muted focus:outline-none focus:ring-1 focus:ring-ds-accent/40 focus:border-ds-accent transition-colors"
                        placeholder="sk-..."
                      />
                      <button
                        onClick={() => setShowKey((s) => ({ ...s, [provider.id]: !s[provider.id] }))}
                        className="absolute right-1.5 top-1/2 -translate-y-1/2 text-ds-muted hover:text-ds-text-primary"
                      >
                        {showKey[provider.id] ? <EyeOff className="w-3 h-3" /> : <Eye className="w-3 h-3" />}
                      </button>
                    </div>
                  </div>
                  <div className="w-44">
                    <label className="text-[10px] text-ds-muted block mb-0.5">模型</label>
                    <SearchableModelSelect
                      models={provider.models}
                      activeModel={isActive ? activeModel : (provider.models[0] || "")}
                      onSelect={(m) => { setActiveProvider(provider.id); setActiveModel(m); }}
                    />
                  </div>
                  <button
                    onClick={() => handleTest(provider.id)}
                    disabled={isTesting || !cred?.apiKey}
                    className={cn(
                      "px-3.5 py-1.5 rounded-lg text-xs font-medium border transition-colors flex-shrink-0",
                      "border-ds-border text-ds-muted hover:text-ds-text-primary hover:bg-ds-hover",
                      "disabled:opacity-40 disabled:cursor-not-allowed"
                    )}
                  >
                    {isTesting ? <Loader2 className="w-3 h-3 animate-spin" /> : "测试"}
                  </button>
                </div>

                {testResult && (
                  <div className={cn(
                    "text-xs px-2.5 py-1.5 rounded-md flex items-start gap-1.5",
                    testResult.startsWith("✓")
                      ? "bg-ds-success/10 text-ds-success"
                      : "bg-ds-danger/10 text-ds-danger"
                  )}>
                    <AlertCircle className="w-3 h-3 flex-shrink-0 mt-0.5" />
                    <span className="break-words">{testResult}</span>
                  </div>
                )}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
