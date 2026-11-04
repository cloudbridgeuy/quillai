import { useEffect, useState } from "react";
import type { EditorScope } from "../manuscripts/contract";
import { createManuscriptClient } from "../manuscripts/client";
import { createLoreClient } from "../manuscripts/lore_client";
import { ManuscriptTree } from "../md_editor/components/ManuscriptTree";
import { Button } from "../ui/Button";
import { Heading } from "../ui/Heading";
import { editorUrl } from "./routes";
import { forgetRecent, readRecent, saveRecent } from "./recent";
import { resolveResume, type ResumeClient, type ResumeState } from "./resume";
import { RecentlyDeleted } from "./RecentlyDeleted";
import { ActivityList } from "./ActivityList";
import { Tips } from "./LandingTips";
import "./landing.css";

function initialResume(): ResumeState[] {
  return readRecent().map(stored => ({ status: "checking", stored }));
}

function createResumeClient(): ResumeClient {
  return { ...createManuscriptClient(), seriesUniverse: createLoreClient().seriesUniverse };
}

function open(scope: EditorScope) {
  window.location.assign(editorUrl(window.location.href, scope));
}

function ResumeCard({ resume, onForget }: { resume: ResumeState; onForget: () => void }) {
  if (resume.status === "empty" || resume.status === "missing") return null;
  const title = resume.status === "ready" ? resume.title : resume.stored.title;
  const detail = resume.status === "ready" ? resume.path.join(" · ")
    : resume.status === "checking" ? "Checking…" : `Could not check it: ${resume.message}`;
  return <div className="flex items-center justify-between gap-4">
    <div className="flex min-w-0 flex-col gap-1">
      <p className="m-0 text-title font-semibold text-ink">{title}</p>
      <p className="m-0 text-ui text-graphite" role={resume.status === "checking" ? "status" : undefined}>{detail}</p>
    </div>
    <div className="flex gap-2">
      <Button variant="primary" disabled={resume.status === "checking"} onClick={() => open(resume.stored.scope)}>Open</Button>
      <Button onClick={onForget}>Forget</Button>
    </div>
  </div>;
}

export function ResumeList({ resume, onForget }: { resume: ResumeState[]; onForget: (scope: EditorScope) => void }) {
  return resume.length === 0 ? <p className="m-0 text-ui text-graphite">Nothing edited in this browser yet. Open or create a piece below.</p>
    : resume.map((item, index) => item.status === "empty" ? null : <ResumeCard key={index} resume={item} onForget={() => onForget(item.stored.scope)} />);
}

export function Landing() {
  const [resume, setResume] = useState(initialResume);
  const [client] = useState(createResumeClient);
  const [libraryRevision, setLibraryRevision] = useState(0);
  const [trashRevision, setTrashRevision] = useState(0);

  useEffect(() => {
    let active = true;
    const stored = readRecent();
    setResume(stored.map(item => ({ status: "checking", stored: item })));
    void Promise.all(stored.map(item => resolveResume(item, client))).then(results => {
      if (!active) return;
      const missing = results.filter(result => result.status === "missing");
      setResume(current => {
        const remaining = current.flatMap(item => item.status === "empty" ? [] : [item.stored]);
        const kept = results.filter(result => result.status !== "empty" && result.status !== "missing"
          && forgetRecent(remaining, result.stored.scope).length < remaining.length);
        return kept;
      });
      if (missing.length) {
        const remaining = missing.reduce((items, result) => result.status === "missing" ? forgetRecent(items, result.stored.scope) : items, readRecent());
        saveRecent(remaining);
      }
    });
    return () => { active = false; };
  }, [client, libraryRevision, trashRevision]);

  function forget(scope: EditorScope) {
    saveRecent(forgetRecent(readRecent(), scope));
    setResume(current => current.filter(item => item.status !== "empty" && forgetRecent([item.stored], scope).length > 0));
  }

  return <main className="mx-auto flex max-w-7xl flex-col gap-6 px-6 py-6">
    <header className="flex items-baseline gap-3">
      <Heading level="app">QuillAI</Heading>
      <p className="m-0 text-ui text-graphite">Your library</p>
    </header>
    <section className="flex flex-col gap-3 rounded-card bg-paper p-5 shadow-sheet" aria-labelledby="landing-resume">
      <Heading level="section" id="landing-resume">Resume</Heading>
      <ResumeList resume={resume} onForget={forget} />
    </section>
    <ActivityList revision={`${libraryRevision}:${trashRevision}`} onOpen={open} />
    <Tips />
    <section className="landing-library flex flex-col gap-2" role="region" aria-labelledby="landing-library">
      <Heading level="section" id="landing-library">Library</Heading>
      <ManuscriptTree key={libraryRevision} selectedChapter={null} disabled={false} collapsedExcept={resume.find((item): item is Extract<ResumeState, { status: "ready" }> => item.status === "ready")?.universeId ?? null}
        onSelect={open} onChange={() => {
          setTrashRevision(value => value + 1);
        }} />
    </section>
    <RecentlyDeleted key={trashRevision} onChange={() => {
      setLibraryRevision(value => value + 1);
    }} />
  </main>;
}
