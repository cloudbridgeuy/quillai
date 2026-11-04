import { useEffect, useState } from "react";
import { getApiBaseUrl } from "../api_config";
import type { EditorScope } from "../manuscripts/contract";
import { Button } from "../ui/Button";
import { Heading } from "../ui/Heading";
import { activityGroups, parseActivity } from "./activity";

type Groups = ReturnType<typeof activityGroups>;

async function fetchActivity(): Promise<Groups> {
  const response = await fetch(`${getApiBaseUrl()}/activity`);
  if (!response.ok) throw new Error("Activity unavailable");
  return activityGroups(parseActivity(await response.json()), new Date());
}

export function ActivityList({ revision, onOpen }: { revision: string; onOpen: (scope: EditorScope) => void }) {
  const [state, setState] = useState<{ groups: Groups; failed: boolean }>({ groups: [], failed: false });
  useEffect(() => {
    let active = true;
    void fetchActivity().then(groups => {
      if (active) setState({ groups, failed: false });
    }).catch(() => {
      if (active) setState({ groups: [], failed: true });
    });
    return () => { active = false; };
  }, [revision]);
  if (!state.failed && state.groups.length === 0) return null;
  return <section className="flex flex-col gap-3 rounded-card bg-paper p-5 shadow-sheet" aria-labelledby="landing-activity">
    <Heading level="section" id="landing-activity">Recent activity</Heading>
    {state.failed ? <p className="m-0 text-ui text-graphite">Could not load recent activity.</p> : state.groups.map(group => <div key={group.label} className="flex flex-col gap-2">
      <Heading level="section" as="h3">{group.label}</Heading>
      {group.rows.map(row => <div key={JSON.stringify(row.scope)} className="flex items-center justify-between gap-4">
        <div><p className="m-0 text-title font-semibold text-ink">{row.title}</p>
          <time className="text-ui text-graphite" dateTime={row.changedAt}>{new Date(row.changedAt).toLocaleString()}</time></div>
        <Button onClick={() => onOpen(row.scope)}>Open</Button>
      </div>)}
    </div>)}
  </section>;
}
