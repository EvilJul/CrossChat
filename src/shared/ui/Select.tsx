import { cn } from "../../lib/cn";

type Props = React.SelectHTMLAttributes<HTMLSelectElement> & { label?: string };

export default function Select({ label, className, children, ...props }: Props) {
  return (
    <div className="w-full">
      {label && <label className="text-[10px] text-zinc-400 block mb-0.5">{label}</label>}
      <select
        className={cn(
          "w-full text-xs rounded-xl border border-zinc-300 dark:border-zinc-600",
          "bg-white dark:bg-zinc-800 px-3 py-1.5",
          "text-zinc-700 dark:text-zinc-300",
          "focus:outline-none focus:ring-1 focus:ring-slate-400",
          "dark:focus:ring-slate-500",
          className
        )}
        {...props}
      >
        {children}
      </select>
    </div>
  );
}
