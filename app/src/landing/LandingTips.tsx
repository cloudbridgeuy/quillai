import { useState } from "react";
import { Button } from "../ui/Button";
import { Heading } from "../ui/Heading";
import { dismissTips, readTipsDismissed, TIPS } from "./tips";

export function Tips() {
  const [dismissed, setDismissed] = useState(readTipsDismissed);
  if (dismissed) return null;
  return <section className="flex flex-col gap-3 rounded-card bg-paper p-5 shadow-sheet" aria-labelledby="landing-tips">
    <div className="flex items-center justify-between gap-4">
      <Heading level="section" id="landing-tips">Tips</Heading>
      <Button onClick={() => { dismissTips(); setDismissed(true); }}>Dismiss</Button>
    </div>
    <ul className="m-0 flex list-disc flex-col gap-1 pl-5 text-ui text-graphite">
      {TIPS.map(tip => <li key={tip}>{tip}</li>)}
    </ul>
  </section>;
}
