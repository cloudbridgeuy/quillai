# Error Boundary — React Crash Recovery

## Overview

`ErrorBoundary` catches JavaScript errors in child component trees and renders a fallback UI instead of crashing the entire app. Each major section (`Editor`, `APITester`) is wrapped in its own boundary so crashes are isolated.

## Architecture (FC/IS + ADTs)

**State ADT** — discriminated union makes impossible states unrepresentable:

```ts
type ErrorBoundaryState =
  | { status: "idle" }
  | { status: "error"; error: Error; errorInfo?: ErrorInfo };
```

No `{ hasError: true, error: null }` is possible.

**Functional Core**: `DefaultFallback` — pure component rendering error message + retry button.

**Imperative Shell**: `ErrorBoundary` class — React requires class components for error boundaries. Uses `getDerivedStateFromError` (render phase) and `componentDidCatch` (commit phase, logs to console).

## Usage

```tsx
import { ErrorBoundary } from "./components/ErrorBoundary";

// Default fallback
<ErrorBoundary>
  <Editor />
</ErrorBoundary>

// Custom fallback
<ErrorBoundary fallback={({ error, retry }) => <p>{error.message} <button onClick={retry}>Retry</button></p>}>
  <Editor />
</ErrorBoundary>
```

## Files

| File | Role |
|------|------|
| `app/src/components/ErrorBoundary.tsx` | Component implementation |
| `app/src/App.tsx` | Integration — wraps Editor + APITester |
