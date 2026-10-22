import { useEffect, useRef, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { Check, CreditCard, Sparkles } from 'lucide-react';
import { Modal } from '../../components/shared/Modal';
import { Button } from '../../components/shared/Button';
import type { BillingInterval, BillingRepository } from '../../domain/billing';
import type { CloudStatus } from '../../domain/cloud-auth';
import { useI18n, type Locale } from '../../i18n';
import { billingCopy } from './copy';
import { userFacingError } from '../../services/userFacingError';
import { UNAVAILABLE_SUBSCRIPTION_CATALOG } from '@betweenframes/contracts';

import './subscription.css';

export type SubscriptionPage = 'plans' | 'usage' | 'billing';
export function SubscriptionDialog({ repository, account, initialPage = 'plans', onLogin, onClose, locale: selectedLocale }: {
  repository?: BillingRepository; account: CloudStatus | null; initialPage?: SubscriptionPage; onLogin: (resume?: () => Promise<void>) => void; onClose: () => void;
  locale?: Locale;
}) {
  const i18n = useI18n();
  const locale = selectedLocale ?? i18n.locale;
  const copy = billingCopy[locale];
  const [page, setPage] = useState(initialPage);
  const [interval, setInterval] = useState<BillingInterval>('month');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');
  const [checkoutOpened, setCheckoutOpened] = useState(false);
  const pending = useRef<{ price: string; id: string } | null>(null);
  const catalog = useQuery({ queryKey: ['billing-catalog'], queryFn: () => repository!.catalog(), enabled: Boolean(repository), retry: false });
  const overview = useQuery({ queryKey: ['billing-overview', account?.user?.id], queryFn: () => repository!.overview(), enabled: Boolean(repository && account?.authenticated), staleTime: 0, gcTime: 0, retry: false });
  const refetchOverview = overview.refetch;
  const refresh = () => { void catalog.refetch(); if (account?.authenticated) void overview.refetch(); };
  useEffect(() => {
    const onFocus = () => { if (account?.authenticated && repository) void refetchOverview(); };
    window.addEventListener('focus', onFocus);
    return () => window.removeEventListener('focus', onFocus);
  }, [account?.authenticated, repository, refetchOverview]);
  const checkout = async (price: string) => {
    if (!repository || busy) return;
    if (!account?.authenticated) { onLogin(); return; }
    setBusy(true); setError('');
    if (pending.current?.price !== price) pending.current = { price, id: crypto.randomUUID() };
    try { await repository.checkout(price, pending.current.id); setCheckoutOpened(true); setNotice(copy.noAuto); }
    catch (cause) {
      if ((cause as {code?:string})?.code === 'BILLING_SESSION_EXPIRED') pending.current = null;
      setError(userFacingError(cause, copy.noAccess));
    }
    finally { setBusy(false); }
  };
  const money = (amount: number, currency: string) => {
    const formatter = new Intl.NumberFormat(locale, { style: 'currency', currency });
    return formatter.format(amount / 10 ** (formatter.resolvedOptions().maximumFractionDigits ?? 2));
  };
  const data = account?.authenticated ? overview.data : undefined;
  const displayCatalog = catalog.data ?? (!repository ? UNAVAILABLE_SUBSCRIPTION_CATALOG : undefined);
  const features = { local: copy.local, selection: copy.selection, demo: copy.demo, ai: copy.ai, sharing: copy.sharing, history: copy.history };
  return <Modal title={copy.title} onClose={() => { if (!busy) onClose(); }} wide className="subscription-modal" closeLabel={{zh:'关闭',en:'Close',ja:'閉じる',ko:'닫기'}[locale]}>
    <div className="subscription-layout">
      <nav className="subscription-nav" aria-label={copy.title}>{(['plans', 'usage', 'billing'] as const).map(key => <button key={key} aria-current={page === key ? 'page' : undefined} onClick={() => setPage(key)}>{copy[key]}</button>)}</nav>
      <section className="subscription-content">
        {page === 'plans' ? <>
          <header className="subscription-heading"><Sparkles size={25} /><h3>{copy.headline}</h3><p>{copy.intro}</p></header>
          <div className="subscription-period" role="group" aria-label={copy.plans}>{(['month', 'year'] as const).map(value => <button key={value} aria-pressed={interval === value} onClick={() => setInterval(value)}>{copy[value]}</button>)}</div>
          {catalog.isFetching && <p role="status">{copy.load}</p>}
          {catalog.isError ? <p role="alert">{copy.error}<Button onClick={() => void catalog.refetch()}>{copy.refresh}</Button></p> : <div className="subscription-plans">{(displayCatalog?.plans ?? []).map(plan => {
            const price = plan.prices.find(value => value.interval === interval);
            const current = data?.planId === plan.id;
            const paid = plan.id !== 'free';
            return <article key={plan.id} className={`subscription-plan ${paid ? 'subscription-plan-pro' : ''}`}>
              <span className="subscription-plan-label">{plan.id === 'free' ? copy.free : plan.name}</span>
              <p>{paid ? copy.proHint : copy.freeHint}</p>
              <div className="subscription-price">{price ? <><strong>{money(price.amount, price.currency)}</strong><span>{interval === 'month' ? copy.perMonth : copy.perYear}</span></> : paid ? <span>{copy.coming}</span> : <strong>{copy.freePrice}</strong>}</div>
              {price && <p>{copy.included.replace('{count}', new Intl.NumberFormat(locale, { maximumFractionDigits: 3 }).format(price.includedCredits))}</p>}
              <ul>{plan.features.filter(key => key in features).map(key => <li key={key}><Check size={16} /><span>{features[key as keyof typeof features]}</span></li>)}</ul>
              <Button variant={paid ? 'primary' : 'default'} disabled={busy || current || !paid || !price || !catalog.data?.salesAvailable} onClick={() => { if (price) void checkout(price.id); }}>{current ? copy.current : paid ? !price || !displayCatalog?.salesAvailable ? copy.coming : account?.authenticated ? copy.checkout : copy.login : copy.free}</Button>
            </article>;
          })}</div>}
          {!repository && <p role="status">{copy.coming}</p>}
          {catalog.data?.salesAvailable && <p className="subscription-fineprint">{copy.renewal}</p>}
        </> : !account?.authenticated ? <div className="subscription-empty"><CreditCard size={30} /><p>{copy.signInHint}</p><Button variant="primary" onClick={() => onLogin()}>{copy.login}</Button></div> : overview.isPending ? <p role="status">{copy.load}</p> : overview.isError || !data ? <p role="alert">{copy.error}<Button onClick={() => void overview.refetch()}>{copy.refresh}</Button></p> : <>
          <div className="subscription-current"><div><small>{copy.current}</small><h3>{data.planId === 'free' ? copy.free : data.planName}</h3></div><span>{data.status === 'free' ? copy.free : copy[data.status]}</span></div>
          {data.currentPeriodEnd && <p>{data.cancelAtPeriodEnd || data.status === 'canceled' ? copy.ends : copy.next} · {new Date(data.currentPeriodEnd).toLocaleDateString(locale)}</p>}
          {page === 'usage' ? <div className="subscription-usage"><h4>{copy.allowance}</h4><strong>{data.usage.remainingPercent === null ? copy.unallocated : `${data.usage.remainingPercent}%`}</strong><progress max={100} value={data.usage.remainingPercent ?? 0} aria-label={copy.allowance} /><p>{copy.remaining.replace('{count}', new Intl.NumberFormat(locale, { maximumFractionDigits: 3 }).format(data.usage.availableCredits))}</p><p>{copy.usageHint}</p><Button onClick={() => setPage('plans')}>{copy.plans}</Button></div> : <>
            {data.manageAvailable && <div className="subscription-manage"><p>{copy.manageHint}</p><Button disabled={busy} onClick={() => { setBusy(true); setError(''); void repository!.manage().catch(() => setError(copy.noAccess)).finally(() => setBusy(false)); }}>{copy.manage}</Button></div>}
            {data.invoices.length ? <table className="subscription-invoices"><thead><tr><th>{copy.date}</th><th>{copy.amount}</th><th>{copy.state}</th></tr></thead><tbody>{data.invoices.map(invoice => <tr key={invoice.id}><td>{new Date(invoice.date).toLocaleDateString(locale)}</td><td>{money(invoice.amount, invoice.currency)}</td><td>{copy[invoice.status]}</td></tr>)}</tbody></table> : <p className="subscription-empty">{copy.emptyInvoices}</p>}
          </>}
        </>}
        {error && <p role="alert">{error}</p>}
        {notice && <p role="status" className="subscription-fineprint">{notice}</p>}
        {(data?.pendingCheckout || (checkoutOpened && data?.planId !== 'pro')) && <Button disabled={busy} onClick={() => {
          setBusy(true); setError('');
          void repository!.cancelCheckout().then(() => { pending.current = null; setCheckoutOpened(false); setNotice(copy.paymentCanceled); })
            .catch(cause => setError(userFacingError(cause, copy.noAccess))).finally(() => { setBusy(false); refresh(); });
        }}>{copy.cancelPayment}</Button>}
        <footer className="subscription-footer"><Button size="small" disabled={busy || !repository} onClick={refresh}>{copy.refresh}</Button>{busy && <span role="status">{copy.load}</span>}</footer>
      </section>
    </div>
  </Modal>;
}
