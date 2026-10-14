import { useSyncExternalStore } from 'react';

const subscribe = (callback: () => void) => {
  window.addEventListener('hashchange', callback);
  return () => window.removeEventListener('hashchange', callback);
};
export function useRoute(local = false) {
  const hash = useSyncExternalStore(subscribe, () => window.location.hash);
  const match = /^#\/project\/([^/]+)$/.exec(hash);
  return hash === '#/projects' || (local && !match) ? { page: 'projects' as const, projectId: null }
    : { page: 'workspace' as const, projectId: match ? match[1] : 'sample-project' };
}
export function navigate(projectId: string | null) {
  window.location.hash = projectId ? `/project/${projectId}` : '/projects';
}
