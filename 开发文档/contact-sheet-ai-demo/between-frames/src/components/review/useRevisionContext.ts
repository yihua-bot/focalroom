import { useEffect, useState } from 'react';

type Loader<T> = (projectId: string, draftId: string, offset: number) => Promise<T | null>;
export function useRevisionContext<T>(load: Loader<T> | undefined, projectId: string, draftId: string) {
  const [state, setState] = useState({ root: draftId, draftId, offset: 0, trail: [] as string[], generation: 0 });
  const location = state.root === draftId ? state : { root: draftId, draftId, offset: 0, trail: [], generation: 0 };
  const key = JSON.stringify([projectId, location.root, location.draftId, location.offset, location.generation]);
  const [result, setResult] = useState<{ key: string; page?: T | null; failed: boolean }>();
  useEffect(() => {
    if (!load) return;
    let active = true;
    load(projectId, location.draftId, location.offset).then(page => { if (active) setResult({ key, page, failed: false }); })
      .catch(() => { if (active) setResult({ key, failed: true }); });
    return () => { active = false; };
  }, [load, projectId, location.draftId, location.offset, key]);
  const current = result?.key === key ? result : undefined;
  const navigate = (nextDraft: string, offset: number, trail = location.trail) => setState(previous => ({ root: draftId, draftId: nextDraft, offset, trail, generation: previous.generation + 1 }));
  return { location, page: current?.page, busy: Boolean(load && !current), failed: current?.failed ?? false, navigate };
}
