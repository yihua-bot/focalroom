import { z } from 'zod';

const currency = z.enum(['USD','CNY','EUR','JPY','KRW']);
export const subscriptionPriceSchema = z.object({
  id: z.string().max(120), interval: z.enum(['month','year']), amount: z.number().int().positive(), currency,
  includedCredits: z.number().positive(),
});
export const subscriptionCatalogSchema = z.object({ salesAvailable: z.boolean(), plans: z.array(z.object({
  id: z.enum(['free','pro','studio']), name: z.string().max(100), description: z.string(), features: z.array(z.string()), prices: z.array(subscriptionPriceSchema),
})) });
export const billingOverviewSchema = z.object({
  planId: z.enum(['free','pro','studio']), planName: z.string().max(100), status: z.enum(['free','active','trialing','past_due','canceled']),
  currentPeriodEnd: z.iso.datetime().nullable(), cancelAtPeriodEnd: z.boolean(),
  usage: z.object({ remainingPercent: z.number().int().min(0).max(100).nullable(), hasAllowance: z.boolean(), availableCredits: z.number().nonnegative(), totalCredits: z.number().nonnegative() }),
  invoices: z.array(z.object({ id: z.string(), date: z.iso.datetime(), amount: z.number().int().nonnegative(), currency, status: z.enum(['paid','open','void']), url: z.string().url().nullable() })),
  manageAvailable: z.boolean(), pendingCheckout: z.boolean().default(false),
  canCancelCheckout: z.boolean().default(true),
});
export type SubscriptionPrice = z.infer<typeof subscriptionPriceSchema>;
export type SubscriptionPlan = z.infer<typeof subscriptionCatalogSchema>['plans'][number];
export type SubscriptionCatalog = z.infer<typeof subscriptionCatalogSchema>;
export type BillingOverview = z.infer<typeof billingOverviewSchema>;
export const UNAVAILABLE_SUBSCRIPTION_CATALOG: SubscriptionCatalog = { salesAvailable: false, plans: [
  { id: 'free', name: '基础版', description: '', features: ['local','selection','demo','sharing','history'], prices: [] },
  { id: 'pro', name: 'FocalRoom Pro', description: '', features: ['local','selection','ai','sharing','history'], prices: [] },
  { id: 'studio', name: 'FocalRoom Studio', description: '', features: ['local','selection','ai','sharing','history','studio'], prices: [] },
] };
/** J5 决策（2026-10-28）：免费 100 万 tokens/月 · Pro ¥39 · Studio ¥99 · CNY · 即时取消 · 免费层永久。 */
export const DECIDED_PRICING_CATALOG: SubscriptionCatalog = { salesAvailable: false, plans: [
  { id: 'free', name: '基础版', description: '每月 100 万 tokens · 永久免费', features: ['local','selection','demo','sharing','history'], prices: [] },
  { id: 'pro', name: 'FocalRoom Pro', description: '', features: ['local','selection','ai','sharing','history'],
    prices: [{ id: 'price_pro_monthly_cny', interval: 'month', amount: 3900, currency: 'CNY', includedCredits: 5000 }] },
  { id: 'studio', name: 'FocalRoom Studio', description: '', features: ['local','selection','ai','sharing','history','studio'],
    prices: [{ id: 'price_studio_monthly_cny', interval: 'month', amount: 9900, currency: 'CNY', includedCredits: 20000 }] },
] };
