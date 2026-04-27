import { cn } from "../../lib/cn";
import { forwardRef } from "react";

type Props = React.InputHTMLAttributes<HTMLInputElement> & { label?: string };

const Input = forwardRef<HTMLInputElement, Props>(({ label, className, ...props }, ref) => (
  <div className="w-full">
    {label && <label className="text-[10px] text-zinc-400 block mb-0.5">{label}</label>}
    <input
      ref={ref}
      className={cn(
        "w-full text-xs rounded-xl border border-zinc-300 dark:border-zinc-600",
        "bg-white dark:bg-zinc-800 px-3 py-1.5",
        "text-zinc-900 dark:text-zinc-100 placeholder:text-zinc-400",
        "focus:outline-none focus:ring-1 focus:ring-slate-400 focus:border-slate-300",
        "dark:focus:ring-slate-500 dark:focus:border-slate-600",
        "disabled:opacity-50 disabled:cursor-not-allowed",
        className
      )}
      {...props}
    />
  </div>
));

Input.displayName = "Input";
export default Input;
