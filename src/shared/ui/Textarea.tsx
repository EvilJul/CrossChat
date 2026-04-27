import { cn } from "../../lib/cn";
import { forwardRef } from "react";

type Props = React.TextareaHTMLAttributes<HTMLTextAreaElement>;

const Textarea = forwardRef<HTMLTextAreaElement, Props>(({ className, ...props }, ref) => (
  <textarea
    ref={ref}
    className={cn(
      "w-full resize-none rounded-xl border border-zinc-200 dark:border-zinc-700",
      "bg-zinc-50 dark:bg-zinc-800/50 px-4 py-2.5 text-sm",
      "text-zinc-800 dark:text-zinc-200 placeholder:text-zinc-400",
      "focus:outline-none focus:ring-1 focus:ring-slate-400 dark:focus:ring-slate-500",
      "disabled:opacity-50 disabled:cursor-not-allowed",
      className
    )}
    {...props}
  />
));

Textarea.displayName = "Textarea";
export default Textarea;
