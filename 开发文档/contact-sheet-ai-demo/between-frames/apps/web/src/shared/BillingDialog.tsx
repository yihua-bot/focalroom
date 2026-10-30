import { useMemo } from 'react';
import { z } from 'zod';
import { billingOverviewSchema, subscriptionCatalogSchema } from '@betweenframes/contracts';
import type { BillingRepository } from '../../../../src/domain/billing';
import { SubscriptionDialog, type SubscriptionPage } from '../../../../src/features/subscription/SubscriptionDialog';
import { request, type Session } from '../shared-api';
import { useWebI18n } from '../i18n';
import './billing.css';

// 支付页只接受服务端返回的受信任地址；页面返回参数不代表付款成功。
export function paymentUrl(value: unknown, kind: 'checkout' | 'manage') {
  const url = new URL(z.object({ url: z.string().url() }).parse(value).url);
  // 支持两个支付网关：Stripe（checkout.stripe.com/c/）和 Polar（polar.sh/checkout/）
  const stripeHost = kind === 'checkout' ? 'checkout.stripe.com' : 'billing.stripe.com';
  const stripePrefix = kind === 'checkout' ? '/c/' : '/p/';
  const polarHost = kind === 'checkout' ? 'polar.sh' : 'polar.sh';
  const polarManagePattern = /^\/[^/]+\/portal(?:\/.*)?$/;
  const isStripe = url.hostname === stripeHost && url.pathname.startsWith(stripePrefix);
  const isPolarCheckout = url.hostname === polarHost && url.pathname.startsWith('/checkout/');
  const isPolarManage = url.hostname === polarHost && polarManagePattern.test(url.pathname);
  const valid = isStripe || (kind === 'checkout' ? isPolarCheckout : isPolarManage);
  if (url.protocol !== 'https:' || url.port || url.username || url.password || !valid) throw new Error('无法打开付款页面，请重试。');
  return url.href;
}

export function BillingDialog({ session, page, onClose }: { session: Session; page: SubscriptionPage; onClose: () => void }) {
  const { locale } = useWebI18n();
  const repository = useMemo<BillingRepository>(() => ({
    catalog: async () => subscriptionCatalogSchema.parse(await request('/billing/catalog', session)),
    overview: async () => billingOverviewSchema.parse(await request('/billing/overview', session)),
    checkout: async (priceId, requestId) => {
      const url = paymentUrl(await request('/billing/checkout', session, 'POST', { priceId, requestId }), 'checkout');
      location.assign(url);
    },
    manage: async () => { location.assign(paymentUrl(await request('/billing/manage', session, 'POST'), 'manage')); },
    cancelCheckout: async () => { await request('/billing/cancel-checkout', session, 'POST'); },
  }), [session]);
  return <SubscriptionDialog repository={repository} locale={locale} initialPage={page}
    account={{ authenticated: true, waiting: false, origin: location.origin, user: session, error: null }}
    onLogin={onClose} onClose={onClose} />;
}
