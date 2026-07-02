import { cn } from "../../lib/cn";
import { forwardRef } from "react";

type Props = React.TextareaHTMLAttributes<HTMLTextAreaElement>;

const Textarea = forwardRef<HTMLTextAreaElement, Props>(({ className, ...props }, ref) => (
  <textarea
    ref={ref}
    className={cn(
      "w-full resize-none rounded-xl border border-ds-border",
      "bg-zinc-50 dark:bg-ds-surface-elevated px-4 py-2.5 text-sm",
      "text-zinc-800 dark:text-zinc-200 placeholder:text-ds-muted",
      "focus:outline-none focus:ring-1 focus:ring-ds-accent/50 focus:border-ds-accent",
      "disabled:opacity-50 disabled:cursor-not-allowed",
      className
    )}
    {...props}
  />
));

Textarea.displayName = "Textarea";
export default Textarea;
