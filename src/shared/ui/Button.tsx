import { cn } from "../../lib/cn";

type Props = React.ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: "primary" | "secondary" | "ghost" | "danger";
  size?: "sm" | "md";
};

const VARIANT: Record<string, string> = {
  primary: "bg-ds-accent hover:opacity-90 text-white active:scale-[0.98]",
  secondary: "bg-ds-surface-elevated border border-ds-border hover:border-ds-accent/30 text-ds-text-primary active:scale-[0.98]",
  ghost: "hover:bg-ds-hover text-ds-muted hover:text-ds-text-primary",
  danger: "bg-ds-danger hover:opacity-90 text-white active:scale-[0.98]",
};

export default function Button({ variant = "primary", size = "md", className, children, disabled, ...props }: Props) {
  return (
    <button
      className={cn(
        "inline-flex items-center gap-1.5 rounded-lg font-medium transition-all duration-200 disabled:opacity-40 disabled:cursor-not-allowed",
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
