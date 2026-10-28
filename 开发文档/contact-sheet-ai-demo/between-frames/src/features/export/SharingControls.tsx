import { zh } from '../../app/copy';
import { useCloudStatus } from '../cloud-login/useCloudStatus';
import { useAccount } from '../cloud-login/AccountProvider';
import { LoginDialog } from '../cloud-login/LoginDialog';
import { useEffect, useState } from 'react';
import type { SharingRepository, SharingStatus } from '../../domain/sharing';
import { Button } from '../../components/shared/Button';
import { SharedReviewLink } from '../../components/shared/SharedReviewLink';

export function SharingControls({ repository, projectId, revision, onChanged, onBusyChange }: {
  repository: SharingRepository; projectId: string; revision: number;
  onChanged?: () => void; onBusyChange?: (busy: boolean) => void;
}) {
  const [status, setStatus] = useState<SharingStatus | null>(null);
  const [consent, setConsent] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [loginOpen, setLoginOpen] = useState(false);
  const access = useAccount();
  const { status: account, refresh } = useCloudStatus(repository.auth);
  const origin = account?.authenticated ? account.origin : '';
  const base = status?.remote?.binding?.baseUrl ?? origin;
  const web = status?.remote?.binding?.webUrl ?? origin;
  const matches = Boolean(account?.authenticated && base === origin);
  useEffect(() => {
    let active = true;
    repository.status(projectId).then(value => { if (active) setStatus(value); })
      .catch(() => { if (active) setError(zh.sharingReadFailed); });
    return () => { active = false; };
  }, [repository, projectId]);
  const login = () => { if (access) access.signIn('sharing'); else setLoginOpen(true); };
  const run = async (action: 'resume' | 'abort') => {
    if (!account?.authenticated) { login(); return; }
    if (!matches || !consent || !status || busy) return;
    setBusy(true); onBusyChange?.(true); setError('');
    try {
      setStatus(await repository.share({ projectId, expectedRevision: revision, baseUrl: base, webUrl: web, action, previewUploadAuthorized: consent }));
    } catch { setError(zh.sharingFailed); }
    finally {
      setBusy(false); onBusyChange?.(false);
      const current = await repository.status(projectId).catch(() => null);
      if (current) setStatus(current);
      onChanged?.();
    }
  };
  return <section className="info-card sharing-controls">
    <h3>{zh.sharingInvite}</h3>
    <p>{zh.sharingHowItWorks}</p>
    {status?.authority === 'shared' ? <p role="status">{zh.sharingReady}{status.remote?.webUrl && <SharedReviewLink projectId={projectId} url={status.remote.webUrl} />}</p> : <>
      {status?.authority === 'local-promoting' && <p role="status">{zh.sharingIncomplete}</p>}
      {account?.authenticated && <p className="sharing-account">{account.user?.name || account.user?.email}</p>}
      {account?.authenticated && !matches && <p role="alert">{zh.sharingWrongAccount}</p>}
      <label className="sync-toggle"><input type="checkbox" checked={consent} disabled={busy || !matches} onChange={event => setConsent(event.target.checked)} />{zh.sharingPreviewConsent}</label>
      <div className="dialog-actions">
        <Button variant="primary" disabled={busy || Boolean(account?.authenticated && (!matches || !consent || !status))} onClick={() => void run('resume')}>
          {busy ? zh.sharingPreparing : !account?.authenticated ? zh.sharingLogin : status?.authority === 'local-promoting' ? zh.sharingContinue : zh.sharingStart}
        </Button>
        {status?.authority === 'local-promoting' && <Button disabled={busy || !matches || !consent} onClick={() => void run('abort')}>{zh.sharingCancel}</Button>}
      </div>
    </>}
    {error && <p role="alert" className="error-message">{error}</p>}
    {loginOpen && <LoginDialog repository={repository.auth} status={account} reason="sharing" onClose={() => setLoginOpen(false)} onStarted={refresh} />}
  </section>;
}
