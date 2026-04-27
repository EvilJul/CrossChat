import { cn } from "../../lib/cn";

type Role = "user" | "assistant" | "tool";

const STYLE: Record<Role, string> = {
  user: "bg-slate-600 dark:bg-slate-500 text-white",
  assistant: "bg-slate-400 dark:bg-slate-600 text-white",
  tool: "bg-amber-500 dark:bg-amber-600 text-white",
};

const LETTER: Record<Role, string> = { user: "U", assistant: "A", tool: "T" };

export default function Avatar({ role, size = "md" }: { role: Role; size?: "sm" | "md" }) {
  const cls = size === "sm" ? "w-6 h-6 text-[10px]" : "w-7 h-7 text-[11px]";
  return (
    <div className={cn("flex-shrink-0 rounded-full flex items-center justify-center font-medium tracking-tight", cls, STYLE[role])}>
      {LETTER[role]}
    </div>
  );
}
