import { cn } from "../../lib/cn";

type Props = React.SelectHTMLAttributes<HTMLSelectElement> & { label?: string };

export default function Select({ label, className, children, ...props }: Props) {
  return (
    <div className="w-full">
      {label && <label className="text-[10px] text-ds-muted block mb-0.5">{label}</label>}
      <select
        className={cn(
          "w-full text-xs rounded-lg border border-ds-border",
          "bg-ds-bg-main px-3 py-1.5",
          "text-ds-text-primary",
          "focus:outline-none focus:ring-1 focus:ring-ds-accent/40 focus:border-ds-accent",
          "transition-colors",
          className
        )}
        {...props}
      >
        {children}
      </select>
    </div>
  );
}
