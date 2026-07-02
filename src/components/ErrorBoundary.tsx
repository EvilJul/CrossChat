import { Component, type ReactNode } from "react";
import { AlertTriangle, RefreshCw } from "lucide-react";

interface Props {
  children: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

export default class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    console.error("[ErrorBoundary] Caught error:", error, errorInfo);
  }

  handleReset = () => {
    this.setState({ hasError: false, error: null });
  };

  render() {
    if (this.state.hasError) {
      return (
        <div className="flex h-screen items-center justify-center bg-ds-bg-main">
          <div className="max-w-md w-full mx-4 p-6 bg-ds-surface-card rounded-2xl border border-ds-border shadow-xl text-center space-y-4">
            <div className="flex justify-center">
              <div className="p-3 bg-ds-danger/10 rounded-xl">
                <AlertTriangle className="w-8 h-8 text-ds-danger" />
              </div>
            </div>
            <h2 className="text-lg font-semibold text-ds-text-primary">
              应用遇到了问题
            </h2>
            <p className="text-sm text-ds-muted">
              {this.state.error?.message || "发生了未知错误"}
            </p>
            <button
              onClick={this.handleReset}
              className="inline-flex items-center gap-2 px-4 py-2 bg-gradient-to-r from-brand-purple-600 to-brand-blue-600 text-white rounded-xl text-sm font-medium hover:opacity-90 transition-all cursor-pointer"
            >
              <RefreshCw className="w-4 h-4" />
              重新加载
            </button>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}
