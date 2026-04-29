import { ListTree, CheckCircle2, Clock, AlertCircle } from "lucide-react";

interface Task {
  id: string;
  description: string;
  status: "pending" | "running" | "completed" | "failed";
  dependencies: string[];
}

interface TaskDecompositionViewProps {
  tasks: Task[];
}

export function TaskDecompositionView({ tasks }: TaskDecompositionViewProps) {
  if (tasks.length === 0) return null;

  const getStatusIcon = (status: Task["status"]) => {
    switch (status) {
      case "completed":
        return <CheckCircle2 className="w-4 h-4 text-green-500" />;
      case "running":
        return <Clock className="w-4 h-4 text-blue-500 animate-pulse" />;
      case "failed":
        return <AlertCircle className="w-4 h-4 text-red-500" />;
      default:
        return <div className="w-4 h-4 rounded-full border-2 border-slate-300" />;
    }
  };

  const getStatusColor = (status: Task["status"]) => {
    switch (status) {
      case "completed":
        return "bg-green-50 dark:bg-green-950/20 border-green-200 dark:border-green-800";
      case "running":
        return "bg-blue-50 dark:bg-blue-950/20 border-blue-200 dark:border-blue-800";
      case "failed":
        return "bg-red-50 dark:bg-red-950/20 border-red-200 dark:border-red-800";
      default:
        return "bg-slate-50 dark:bg-slate-900 border-slate-200 dark:border-slate-700";
    }
  };

  return (
    <div className="p-4 mb-4 rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-950">
      <div className="flex items-center gap-2 mb-3">
        <ListTree className="w-5 h-5 text-indigo-500" />
        <h3 className="font-semibold text-sm">任务分解</h3>
        <span className="text-xs text-slate-500">
          {tasks.filter((t) => t.status === "completed").length} / {tasks.length} 完成
        </span>
      </div>

      <div className="space-y-2">
        {tasks.map((task) => (
          <div
            key={task.id}
            className={`p-3 rounded-lg border ${getStatusColor(task.status)}`}
          >
            <div className="flex items-start gap-3">
              {getStatusIcon(task.status)}
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2 mb-1">
                  <span className="text-xs font-mono text-slate-500">#{task.id}</span>
                  {task.dependencies.length > 0 && (
                    <span className="text-xs text-slate-400">
                      依赖: {task.dependencies.join(", ")}
                    </span>
                  )}
                </div>
                <div className="text-sm">{task.description}</div>
              </div>
            </div>
          </div>
        ))}
      </div>

      <div className="mt-4">
        <div className="h-2 bg-slate-200 dark:bg-slate-700 rounded-full overflow-hidden">
          <div
            className="h-full bg-indigo-500 transition-all duration-500"
            style={{
              width: `${(tasks.filter((t) => t.status === "completed").length / tasks.length) * 100}%`,
            }}
          />
        </div>
      </div>
    </div>
  );
}
