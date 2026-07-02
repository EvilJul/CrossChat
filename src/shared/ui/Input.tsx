import { cn } from "../../lib/cn";
import { forwardRef } from "react";

type Props = React.InputHTMLAttributes<HTMLInputElement> & { label?: string };

const Input = forwardRef<HTMLInputElement, Props>(({ label, className, ...props }, ref) => (
  <div className="w-full">
    {label && <label className="text-[10px] text-ds-muted block mb-0.5">{label}</label>}
    <input
      ref={ref}
      className={cn(
        "w-full text-xs rounded-lg border border-ds-border",
        "bg-ds-bg-main px-3 py-1.5",
        "text-ds-text-primary placeholder:text-ds-muted",
        "focus:outline-none focus:ring-1 focus:ring-ds-accent/40 focus:border-ds-accent",
        "disabled:opacity-50 disabled:cursor-not-allowed transition-colors",
        className
      )}
      {...props}
    />
  </div>
));

Input.displayName = "Input";
export default Input;
