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
      .catch(() => { if (active) setError('无法读取共享状态，请重试。'); });
    return () => { active = false; };
  }, [repository, projectId]);
  const login = () => { if (access) access.signIn('sharing'); else setLoginOpen(true); };
  const run = async (action: 'resume' | 'abort') => {
    if (!account?.authenticated) { login(); return; }
    if (!matches || !consent || !status || busy) return;
    setBusy(true); onBusyChange?.(true); setError('');
    try {
      setStatus(await repository.share({ projectId, expectedRevision: revision, baseUrl: base, webUrl: web, action, previewUploadAuthorized: consent }));
    } catch { setError('共享未完成，请检查网络后重试。'); }
    finally {
      setBusy(false); onBusyChange?.(false);
      const current = await repository.status(projectId).catch(() => null);
      if (current) setStatus(current);
      onChanged?.();
    }
  };
  return <section className="info-card sharing-controls">
    <h3>邀请他人审稿</h3>
    <p>只共享照片预览，原图留在你的电脑上。准备期间暂停编辑；完成后请在线审稿，本机项目仅供查看。</p>
    {status?.authority === 'shared' ? <p role="status">已开启在线审稿。{status.remote?.webUrl && <SharedReviewLink projectId={projectId} url={status.remote.webUrl} />}</p> : <>
      {status?.authority === 'local-promoting' && <p role="status">共享尚未完成，可以继续或取消。</p>}
      {account?.authenticated && <p className="sharing-account">{account.user?.name || account.user?.email}</p>}
      {account?.authenticated && !matches && <p role="alert">请使用创建此共享项目的账户继续。</p>}
      <label className="sync-toggle"><input type="checkbox" checked={consent} disabled={busy || !matches} onChange={event => setConsent(event.target.checked)} />共享照片预览，不包含原图和拍摄位置信息。共享后在网页中继续编辑；不会自动将照片用于 AI 分析。</label>
      <div className="dialog-actions">
        <Button variant="primary" disabled={busy || Boolean(account?.authenticated && (!matches || !consent || !status))} onClick={() => void run('resume')}>
          {busy ? '正在准备共享…' : !account?.authenticated ? '登录并共享' : status?.authority === 'local-promoting' ? '继续共享' : '开始共享'}
        </Button>
        {status?.authority === 'local-promoting' && <Button disabled={busy || !matches || !consent} onClick={() => void run('abort')}>取消共享</Button>}
      </div>
    </>}
    {error && <p role="alert" className="error-message">{error}</p>}
    {loginOpen && <LoginDialog repository={repository.auth} status={account} reason="sharing" onClose={() => setLoginOpen(false)} onStarted={refresh} />}
  </section>;
}
