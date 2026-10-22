export type BillingInterval = 'month' | 'year';
export type { SubscriptionPrice, SubscriptionPlan, SubscriptionCatalog, BillingOverview } from '@betweenframes/contracts';
import type { SubscriptionCatalog, BillingOverview } from '@betweenframes/contracts';
export interface BillingRepository {
  catalog(): Promise<SubscriptionCatalog>;
  overview(): Promise<BillingOverview>;
  checkout(priceId: string, requestId: string): Promise<void>;
  manage(): Promise<void>;
  cancelCheckout(): Promise<void>;
}
