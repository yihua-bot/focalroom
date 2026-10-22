import { expect, it } from 'vitest';
import { paymentUrl } from './BillingDialog';
it('Web 支付只打开指定官方页面，不接受伪装域名或其他协议', () => {
  expect(paymentUrl({url:'https://checkout.stripe.com/c/pay/fixture'},'checkout')).toBe('https://checkout.stripe.com/c/pay/fixture');
  expect(paymentUrl({url:'https://billing.stripe.com/p/session/fixture'},'manage')).toBe('https://billing.stripe.com/p/session/fixture');
  for (const url of ['http://checkout.stripe.com/c/pay/x','https://checkout.stripe.com.evil.test/c/pay/x','https://a:secret@checkout.stripe.com/c/pay/x','https://checkout.stripe.com:8443/c/pay/x','https://checkout.stripe.com/redirect','javascript:alert(1)']) {
    expect(() => paymentUrl({url},'checkout')).toThrow();
  }
});
