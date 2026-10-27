import { useEffect, useState } from "react";
import { ErrorBoundary } from "./components/ErrorBoundary";
import Editor from "./md_editor/components/Editor";
import { Landing } from "./landing/Landing";
import { scriptGroupId, showsLanding } from "./landing/routes";
import { createManuscriptClient } from "./manuscripts/client";
import "./index.css";

function EditorRoute() {
  const [route, setRoute] = useState<{ scriptGroupId: number | undefined } | null>(null);
  useEffect(() => {
    const pieceId = Number(new URLSearchParams(window.location.search).get("pieceId"));
    if (!Number.isSafeInteger(pieceId) || pieceId <= 0) {
      setRoute({ scriptGroupId: undefined });
      return;
    }
    createManuscriptClient().getPiece({ pieceId }).then(
      (piece) => setRoute({ scriptGroupId: scriptGroupId(piece) }),
      () => setRoute({ scriptGroupId: undefined }),
    );
  }, []);
  return route === null ? <p role="status">Loading…</p> : <Editor scriptGroupId={route.scriptGroupId} />;
}

export function App() {
  return (
    <div className="app">
      <ErrorBoundary>
        {showsLanding(window.location.search) ? <Landing /> : <EditorRoute />}
      </ErrorBoundary>
    </div>
  );
}
