# Error Boundary — React Crash Recovery

## Overview

`ErrorBoundary` catches JavaScript errors in child component trees and renders a fallback UI instead of crashing the entire app. `App` wraps the selected landing page or editor route in one shared boundary.

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

<ErrorBoundary>
  {showsLanding(window.location.search) ? <Landing /> : <EditorRoute />}
</ErrorBoundary>
```

## Files

| File | Role |
|------|------|
| `app/src/components/ErrorBoundary.tsx` | Component implementation |
| `app/src/App.tsx` | Integration — wraps the selected landing page or editor route |
