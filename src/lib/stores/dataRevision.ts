import { writable } from "svelte/store";

export type RevisionDomain = "global" | "works" | "tasks" | "waiting" | "calendar" | "inbox" | "workspace" | "brief" | "providers" | "analysis" | "proposals" | "documents" | "storage";
export type DataRevision = Record<RevisionDomain, number>;

export const dataRevision = writable<DataRevision>({
  global: 0,
  works: 0,
  tasks: 0,
  waiting: 0,
  calendar: 0,
  inbox: 0,
  workspace: 0,
  brief: 0,
  providers: 0,
  analysis: 0,
  proposals: 0,
  documents: 0,
  storage: 0
});

export function invalidate(...domains: RevisionDomain[]): void {
  const requested: RevisionDomain[] = domains.length > 0 ? domains : ["global"];
  dataRevision.update((current) => {
    const next = { ...current };
    for (const domain of requested) next[domain] += 1;
    if (!requested.includes("global")) next.global += 1;
    return next;
  });
}
