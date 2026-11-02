import { useCallback, useEffect, useRef, useState } from 'react';
import type { CloudAuthRepository, CloudStatus } from '../../domain/cloud-auth';
import { zh } from '../../app/copy';

export function useCloudStatus(repository?: CloudAuthRepository) {
  const [status, setStatus] = useState<CloudStatus | null>(null);
  const sequence = useRef(0);
  const refresh = useCallback(async () => {
    if (!repository) return null;
    const ticket = ++sequence.current;
    return repository.status().then(value => {
      if (ticket === sequence.current) setStatus(value);
      return value;
    }).catch(() => {
      if (ticket === sequence.current) setStatus({ authenticated: false, waiting: false, origin: '', user: null, error: zh.cloudConnectionRetry });
      return null;
    });
  }, [repository]);
  useEffect(() => {
    let active = true;
    const bump = () => { if (active) refresh(); };
    bump();
    window.addEventListener('focus', bump);
    let cleanup: (() => void) | undefined;
    void repository?.subscribe(bump).then(fn => { if (active) cleanup = fn; else fn(); }).catch(() => {});
    const current = sequence;
    return () => { active = false; current.current += 1; cleanup?.(); window.removeEventListener('focus', bump); };
  }, [repository, refresh]);
  return { status, refresh };
}
