import { cn } from "../../lib/cn";

type Props = React.ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: "primary" | "secondary" | "ghost" | "danger";
  size?: "sm" | "md";
};

const VARIANT: Record<string, string> = {
  primary: "bg-slate-600 hover:bg-slate-700 text-white",
  secondary: "bg-white dark:bg-zinc-800 border border-zinc-200/70 dark:border-zinc-700/70 hover:border-slate-300 dark:hover:border-slate-600 text-zinc-600 dark:text-zinc-400",
  ghost: "hover:bg-zinc-100 dark:hover:bg-zinc-800 text-zinc-500 dark:text-zinc-400",
  danger: "bg-red-500 hover:bg-red-600 text-white",
};

export default function Button({ variant = "primary", size = "md", className, children, disabled, ...props }: Props) {
  return (
    <button
      className={cn(
        "inline-flex items-center gap-1.5 rounded-xl font-medium transition-all duration-200 disabled:opacity-40 disabled:cursor-not-allowed",
        size === "sm" ? "px-2.5 py-1 text-xs" : "px-3.5 py-2 text-sm",
        VARIANT[variant],
        className
      )}
      disabled={disabled}
      {...props}
    >
      {children}
    </button>
  );
}
