import { Component, type ErrorInfo, type ReactNode } from "react";

// --- Functional Core: Types ---

type ErrorBoundaryState =
  | { status: "idle" }
  | { status: "error"; error: Error; errorInfo?: ErrorInfo };

type ErrorBoundaryProps = {
  children: ReactNode;
  fallback?: (props: { error: Error; retry: () => void }) => ReactNode;
};

// --- Functional Core: Pure render function ---

function DefaultFallback({
  error,
  retry,
}: {
  error: Error;
  retry: () => void;
}) {
  return (
    <div className="error-boundary-fallback">
      <p>Something went wrong: {error.message}</p>
      <button onClick={retry}>Try again</button>
    </div>
  );
}

// --- Imperative Shell: React class component (required for error boundaries) ---

export class ErrorBoundary extends Component<
  ErrorBoundaryProps,
  ErrorBoundaryState
> {
  constructor(props: ErrorBoundaryProps) {
    super(props);
    this.state = { status: "idle" };
  }

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { status: "error", error };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo): void {
    console.error("[ErrorBoundary]", error, errorInfo.componentStack);
    this.setState({ status: "error", error, errorInfo });
  }

  resetErrorBoundary = (): void => {
    this.setState({ status: "idle" });
  };

  render(): ReactNode {
    switch (this.state.status) {
      case "idle":
        return this.props.children;
      case "error": {
        const renderFallback = this.props.fallback ?? DefaultFallback;
        return renderFallback({
          error: this.state.error,
          retry: this.resetErrorBoundary,
        });
      }
    }
  }
}
