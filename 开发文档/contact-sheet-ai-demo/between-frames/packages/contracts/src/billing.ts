import { z } from 'zod';

const currency = z.enum(['USD','CNY','EUR','JPY','KRW']);
export const subscriptionPriceSchema = z.object({
  id: z.string().max(120), interval: z.enum(['month','year']), amount: z.number().int().positive(), currency,
  includedCredits: z.number().positive(),
});
export const subscriptionCatalogSchema = z.object({ salesAvailable: z.boolean(), plans: z.array(z.object({
  id: z.enum(['free','pro']), name: z.string().max(100), description: z.string(), features: z.array(z.string()), prices: z.array(subscriptionPriceSchema),
})) });
export const billingOverviewSchema = z.object({
  planId: z.enum(['free','pro']), planName: z.string().max(100), status: z.enum(['free','active','trialing','past_due','canceled']),
  currentPeriodEnd: z.iso.datetime().nullable(), cancelAtPeriodEnd: z.boolean(),
  usage: z.object({ remainingPercent: z.number().int().min(0).max(100).nullable(), hasAllowance: z.boolean(), availableCredits: z.number().nonnegative(), totalCredits: z.number().nonnegative() }),
  invoices: z.array(z.object({ id: z.string(), date: z.iso.datetime(), amount: z.number().int().nonnegative(), currency, status: z.enum(['paid','open','void']), url: z.string().url().nullable() })),
  manageAvailable: z.boolean(), pendingCheckout: z.boolean().default(false),
});
export type SubscriptionPrice = z.infer<typeof subscriptionPriceSchema>;
export type SubscriptionPlan = z.infer<typeof subscriptionCatalogSchema>['plans'][number];
export type SubscriptionCatalog = z.infer<typeof subscriptionCatalogSchema>;
export type BillingOverview = z.infer<typeof billingOverviewSchema>;
export const UNAVAILABLE_SUBSCRIPTION_CATALOG: SubscriptionCatalog = { salesAvailable: false, plans: [
  { id: 'free', name: '基础版', description: '', features: ['local','selection','demo','sharing','history'], prices: [] },
  { id: 'pro', name: 'FocalRoom Pro', description: '', features: ['local','selection','ai','sharing','history'], prices: [] },
] };
