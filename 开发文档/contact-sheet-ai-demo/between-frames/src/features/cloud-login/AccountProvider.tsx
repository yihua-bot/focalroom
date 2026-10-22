import { createContext, useContext, useEffect, useRef, useState, type ReactNode } from 'react';
import type { CloudAuthRepository, CloudStatus } from '../../domain/cloud-auth';
import type { BillingRepository } from '../../domain/billing';
import { LoginDialog, type LoginReason } from './LoginDialog';
import { useCloudStatus } from './useCloudStatus';
import { SubscriptionDialog, type SubscriptionPage } from '../subscription/SubscriptionDialog';

interface AccountAccess {
  auth?: CloudAuthRepository;
  billing?: BillingRepository;
  status: CloudStatus | null;
  refresh: () => void;
  signIn: (reason?: LoginReason) => void;
  requireSignIn: (reason: LoginReason, action: () => void | Promise<void>) => Promise<void>;
  openSubscription: (page?: SubscriptionPage) => void;
}
const AccountContext = createContext<AccountAccess | null>(null);
export const useAccount = () => useContext(AccountContext);

export function AccountProvider({ auth, billing, children }: { auth?: CloudAuthRepository; billing?: BillingRepository; children: ReactNode }) {
  const { status, refresh } = useCloudStatus(auth);
  const [reason, setReason] = useState<LoginReason | null>(null);
  const [subscriptionPage, setSubscriptionPage] = useState<SubscriptionPage | null>(null);
  const [error, setError] = useState('');
  const pending = useRef<(() => void | Promise<void>) | null>(null);
  const cancel = () => { pending.current = null; setReason(null); };
  const requireSignIn = async (feature: LoginReason, action: () => void | Promise<void>) => {
    let authenticated = false;
    try {
      // 同步界面状态，避免过期会话仍被旧的 authenticated 状态立即恢复。
      const current = await refresh();
      authenticated = Boolean(current?.authenticated);
    } catch { /* 连接失败仍显示可重试的登录入口，不执行操作。 */ }
    if (authenticated) {
      try { await action(); } catch { setError('操作未完成，请重试。'); }
      refresh(); return;
    }
    pending.current = action;
    setReason(feature);
  };
  useEffect(() => {
    if (!status?.authenticated || !reason) return;
    let active = true;
    // 后端登录通知到达后再恢复一次操作；关闭弹窗会清除待执行动作。
    void Promise.resolve().then(async () => {
      if (!active) return;
      const action = pending.current;
      pending.current = null;
      setReason(null);
      try { await action?.(); } catch { setError('操作未完成，请重试。'); }
    });
    return () => { active = false; };
  }, [status?.authenticated, reason]);
  return <AccountContext.Provider value={{ auth, billing, status, refresh,
    signIn: (feature = 'account') => { pending.current = null; setReason(feature); },
    requireSignIn, openSubscription: (page = 'plans') => setSubscriptionPage(page),
  }}>
    {children}
    {subscriptionPage && <SubscriptionDialog repository={billing} account={status} initialPage={subscriptionPage} onClose={() => setSubscriptionPage(null)} onLogin={() => { pending.current = null; setReason('subscription'); }} />}
    {reason && <LoginDialog repository={auth} status={status} reason={reason} onClose={cancel} onStarted={refresh} />}
    {error && <p className="toast" role="alert" onClick={() => setError('')}>{error}</p>}
  </AccountContext.Provider>;
}
