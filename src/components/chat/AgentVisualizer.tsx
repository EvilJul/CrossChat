import { Brain, Zap, CheckCircle2, XCircle, Clock } from "lucide-react";

interface AgentStep {
  iteration: number;
  thought: string;
  action?: {
    tool_name: string;
    result: string;
    success: boolean;
  };
  observation?: string;
}

interface AgentVisualizerProps {
  steps: AgentStep[];
  currentIteration?: number;
}

export function AgentVisualizer({ steps, currentIteration }: AgentVisualizerProps) {
  return (
    <div className="p-4 mb-4 rounded-lg border border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-900">
      <div className="flex items-center gap-2 mb-3">
        <Brain className="w-5 h-5 text-purple-500" />
        <h3 className="font-semibold text-sm">Agent 思考过程</h3>
      </div>

      <div className="space-y-3">
        {steps.map((step, idx) => (
          <div
            key={idx}
            className={`border-l-2 pl-3 ${
              idx === currentIteration
                ? "border-purple-500 bg-purple-50 dark:bg-purple-950/20"
                : "border-slate-300 dark:border-slate-700"
            }`}
          >
            <div className="flex items-start gap-2 mb-2">
              <Brain className="w-4 h-4 mt-0.5 text-purple-500 flex-shrink-0" />
              <div className="flex-1">
                <div className="text-xs text-slate-500 dark:text-slate-400">
                  步骤 {step.iteration + 1}
                </div>
                <div className="text-sm">{step.thought}</div>
              </div>
            </div>

            {step.action && (
              <div className="flex items-start gap-2 mb-2 ml-6">
                <Zap className="w-4 h-4 mt-0.5 text-amber-500 flex-shrink-0" />
                <div className="flex-1">
                  <div className="flex items-center gap-2">
                    <code className="text-xs bg-slate-200 dark:bg-slate-800 px-2 py-0.5 rounded">
                      {step.action.tool_name}
                    </code>
                    {step.action.success ? (
                      <CheckCircle2 className="w-3 h-3 text-green-500" />
                    ) : (
                      <XCircle className="w-3 h-3 text-red-500" />
                    )}
                  </div>
                  <div className="text-xs text-slate-600 dark:text-slate-400 mt-1 line-clamp-2">
                    {step.action.result}
                  </div>
                </div>
              </div>
            )}

            {idx === currentIteration && (
              <div className="flex items-center gap-2 ml-6 text-xs text-purple-600 dark:text-purple-400">
                <Clock className="w-3 h-3 animate-pulse" />
                <span>执行中...</span>
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
