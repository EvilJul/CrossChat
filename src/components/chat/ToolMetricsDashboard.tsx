import { BarChart3, TrendingUp, Clock, CheckCircle2 } from "lucide-react";
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface ToolStat {
  tool_name: string;
  total_calls: number;
  success_calls: number;
  avg_time_ms: number;
  max_time_ms: number;
  min_time_ms: number;
}

export function ToolMetricsDashboard() {
  const [stats, setStats] = useState<ToolStat[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    loadStats();
  }, []);

  const loadStats = async () => {
    try {
      const result = await invoke<ToolStat[]>("get_tool_stats", { toolName: null });
      setStats(result);
    } catch (error) {
      console.error("加载工具统计失败:", error);
    } finally {
      setLoading(false);
    }
  };

  if (loading) {
    return (
      <div className="p-4 rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-950">
        <div className="text-sm text-slate-500">加载中...</div>
      </div>
    );
  }

  const topTools = stats.slice(0, 5);

  return (
    <div className="p-4 rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-950">
      <div className="flex items-center gap-2 mb-4">
        <BarChart3 className="w-5 h-5 text-blue-500" />
        <h3 className="font-semibold text-sm">工具性能统计</h3>
      </div>

      <div className="space-y-3">
        {topTools.map((stat) => {
          const successRate = (stat.success_calls / stat.total_calls) * 100;
          const isSlow = stat.avg_time_ms > 1000;

          return (
            <div key={stat.tool_name} className="border-b pb-3 last:border-0">
              <div className="flex items-center justify-between mb-2">
                <code className="text-xs font-mono bg-slate-100 dark:bg-slate-800 px-2 py-1 rounded">
                  {stat.tool_name}
                </code>
                <div className="flex items-center gap-3 text-xs">
                  <span className="flex items-center gap-1">
                    <TrendingUp className="w-3 h-3" />
                    {stat.total_calls}
                  </span>
                  <span className="flex items-center gap-1">
                    <CheckCircle2 className="w-3 h-3 text-green-500" />
                    {successRate.toFixed(0)}%
                  </span>
                </div>
              </div>

              <div className="flex items-center gap-2 text-xs text-slate-600 dark:text-slate-400">
                <Clock className={`w-3 h-3 ${isSlow ? "text-red-500" : ""}`} />
                <span className={isSlow ? "text-red-500 font-semibold" : ""}>
                  平均 {stat.avg_time_ms.toFixed(0)}ms
                </span>
                <span className="text-slate-400">
                  (最快 {stat.min_time_ms}ms, 最慢 {stat.max_time_ms}ms)
                </span>
              </div>

              <div className="mt-2 h-1.5 bg-slate-200 dark:bg-slate-700 rounded-full overflow-hidden">
                <div
                  className="h-full bg-blue-500 transition-all"
                  style={{ width: `${successRate}%` }}
                />
              </div>
            </div>
          );
        })}
      </div>

      {stats.length === 0 && (
        <div className="text-sm text-slate-500 text-center py-4">
          暂无工具使用数据
        </div>
      )}
    </div>
  );
}
