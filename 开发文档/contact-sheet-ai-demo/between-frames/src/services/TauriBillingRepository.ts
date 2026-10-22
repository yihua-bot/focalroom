import { subscriptionCatalogSchema as catalog, billingOverviewSchema as overview } from '@betweenframes/contracts';
import type { BillingRepository } from '../domain/billing';
import { invokeCommand } from './tauriCommand';

export class TauriBillingRepository implements BillingRepository {
  async catalog() { return catalog.parse(await invokeCommand('billing_catalog')); }
  async overview() { return overview.parse(await invokeCommand('billing_overview')); }
  async checkout(priceId: string, requestId: string) { await invokeCommand('billing_checkout', { priceId, requestId }); }
  async manage() { await invokeCommand('billing_manage'); }
  async cancelCheckout() { await invokeCommand('billing_cancel_checkout'); }
}
