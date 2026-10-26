import { ErrorBoundary } from "./components/ErrorBoundary";
import Editor from "./md_editor/components/Editor";
import { Landing } from "./landing/Landing";
import { showsLanding } from "./landing/routes";
import "./index.css";

export function App() {
  return (
    <div className="app">
      <ErrorBoundary>
        {showsLanding(window.location.search) ? <Landing /> : <Editor />}
      </ErrorBoundary>
    </div>
  );
}
