import { ErrorBoundary } from "./components/ErrorBoundary";
import Editor from "./md_editor/components/Editor";
import "./index.css";

export function App() {
  return (
    <div className="app">
      <ErrorBoundary>
        <Editor />
      </ErrorBoundary>
    </div>
  );
}
