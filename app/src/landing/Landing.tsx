import { useState } from "react";
import { ManuscriptTree } from "../md_editor/components/ManuscriptTree";
import { Button } from "../ui/Button";
import { Heading } from "../ui/Heading";
import { editorUrl, LAST_EDITED_KEY, parseLastEdited, type LastEdited } from "./routes";

function readLastEdited(): LastEdited | null {
  try {
    return parseLastEdited(window.localStorage.getItem(LAST_EDITED_KEY));
  } catch {
    return null;
  }
}

export function Landing() {
  const [lastEdited] = useState(readLastEdited);
  return <main className="mx-auto flex max-w-3xl flex-col gap-6 px-4 py-6">
    <Heading level="app">QuillAI</Heading>
    <section className="flex flex-col gap-2" aria-labelledby="landing-resume">
      <Heading level="section" id="landing-resume">Resume</Heading>
      {lastEdited ? <div>
        <Button variant="primary" onClick={() => window.location.assign(editorUrl(window.location.href, lastEdited.scope))}>
          Resume {lastEdited.title}
        </Button>
      </div> : <p className="m-0 text-ui text-graphite">Nothing edited in this browser yet. Open or create a piece below.</p>}
    </section>
    <section className="flex flex-col gap-2" role="region" aria-labelledby="landing-library">
      <Heading level="section" id="landing-library">Library</Heading>
      <ManuscriptTree selectedChapter={null} disabled={false}
        onSelect={scope => window.location.assign(editorUrl(window.location.href, scope))} />
    </section>
  </main>;
}
