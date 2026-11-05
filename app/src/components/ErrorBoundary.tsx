import { Component, type ErrorInfo, type ReactNode } from "react";
import { defaultLocale, formatMessage } from "../i18n/messages";
import { Button } from "../ui/Button";

type ErrorBoundaryState =
  | { status: "idle" }
  | { status: "error"; error: Error; errorInfo?: ErrorInfo };

type ErrorBoundaryProps = {
  children: ReactNode;
  fallback?: (props: { error: Error; retry: () => void }) => ReactNode;
};

function DefaultFallback({
  error,
  retry,
}: {
  error: Error;
  retry: () => void;
}) {
  return (
    <div className="error-boundary-fallback">
      <p>{formatMessage(defaultLocale, "error.unexpected", { message: error.message })}</p>
      <Button onClick={retry}>{formatMessage(defaultLocale, "error.retry")}</Button>
    </div>
  );
}

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
