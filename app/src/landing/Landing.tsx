import { useEffect, useState } from "react";
import type { EditorScope } from "../manuscripts/contract";
import { createManuscriptClient } from "../manuscripts/client";
import { createLoreClient } from "../manuscripts/lore_client";
import { ManuscriptTree } from "../md_editor/components/ManuscriptTree";
import { Button } from "../ui/Button";
import { Heading } from "../ui/Heading";
import { editorUrl, LAST_EDITED_KEY, parseLastEdited, type LastEdited } from "./routes";
import { resolveResume, type ResumeClient, type ResumeState } from "./resume";
import "./landing.css";

function readLastEdited(): LastEdited | null {
  try {
    return parseLastEdited(window.localStorage.getItem(LAST_EDITED_KEY));
  } catch {
    return null;
  }
}

function initialResume(): ResumeState {
  const stored = readLastEdited();
  return stored ? { status: "checking", stored } : { status: "empty" };
}

function createResumeClient(): ResumeClient {
  return { ...createManuscriptClient(), seriesUniverse: createLoreClient().seriesUniverse };
}

function open(scope: EditorScope) {
  window.location.assign(editorUrl(window.location.href, scope));
}

function ResumeCard({ resume, onForget }: { resume: ResumeState; onForget: () => void }) {
  if (resume.status === "empty") {
    return <p className="m-0 text-ui text-graphite">Nothing edited in this browser yet. Open or create a piece below.</p>;
  }
  if (resume.status === "missing") {
    return <div className="flex items-center justify-between gap-4">
      <div className="flex flex-col gap-1">
        <p className="m-0 text-title font-semibold text-ink">{resume.stored.title} is no longer in the library.</p>
        <p className="m-0 text-ui text-graphite">It may have been deleted. Pick another piece below.</p>
      </div>
      <Button onClick={onForget}>Forget</Button>
    </div>;
  }
  const title = resume.status === "ready" ? resume.title : resume.stored.title;
  const detail = resume.status === "ready" ? resume.path.join(" · ")
    : resume.status === "checking" ? "Checking…" : `Could not check it: ${resume.message}`;
  return <div className="flex items-center justify-between gap-4">
    <div className="flex min-w-0 flex-col gap-1">
      <p className="m-0 text-title font-semibold text-ink">{title}</p>
      <p className="m-0 text-ui text-graphite" role={resume.status === "checking" ? "status" : undefined}>{detail}</p>
    </div>
    <Button variant="primary" disabled={resume.status === "checking"} onClick={() => open(resume.stored.scope)}>Open</Button>
  </div>;
}

export function Landing() {
  const [resume, setResume] = useState(initialResume);
  const [client] = useState(createResumeClient);

  useEffect(() => {
    if (resume.status !== "checking") return;
    let active = true;
    void resolveResume(resume.stored, client).then(next => { if (active) setResume(next); });
    return () => { active = false; };
  }, [resume, client]);

  function forget() {
    try {
      window.localStorage.removeItem(LAST_EDITED_KEY);
    } catch {}
    setResume({ status: "empty" });
  }

  return <main className="mx-auto flex max-w-7xl flex-col gap-6 px-6 py-6">
    <header className="flex items-baseline gap-3">
      <Heading level="app">QuillAI</Heading>
      <p className="m-0 text-ui text-graphite">Your library</p>
    </header>
    <section className="flex flex-col gap-3 rounded-card bg-paper p-5 shadow-sheet" aria-labelledby="landing-resume">
      <Heading level="section" id="landing-resume">Resume</Heading>
      <ResumeCard resume={resume} onForget={forget} />
    </section>
    <section className="landing-library flex flex-col gap-2" role="region" aria-labelledby="landing-library">
      <Heading level="section" id="landing-library">Library</Heading>
      <ManuscriptTree selectedChapter={null} disabled={false} collapsedExcept={resume.status === "ready" ? resume.universeId : null}
        onSelect={open} />
    </section>
  </main>;
}
