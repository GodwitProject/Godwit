import { describe, it, expect } from 'vitest';
import { parsePrometheusMetrics } from './api';

describe('parsePrometheusMetrics', () => {
  it('parses the Godwit counters from Prometheus text', () => {
    const text = [
      '# HELP godwit_requests_total Total requests handled by the proxy',
      '# TYPE godwit_requests_total counter',
      'godwit_requests_total 1042',
      '# HELP godwit_tokens_total Total tokens processed',
      '# TYPE godwit_tokens_total counter',
      'godwit_tokens_total{side="input"} 51230',
      'godwit_tokens_total{side="output"} 20410',
      '# HELP godwit_cost_usd_total Total cost in USD',
      '# TYPE godwit_cost_usd_total counter',
      'godwit_cost_usd_total 0.9125',
      'godwit_active_requests 3',
    ].join('\n');

    const metrics = parsePrometheusMetrics(text);
    expect(metrics.requestsTotal).toBe(1042);
    expect(metrics.tokensTotal).toBe(71640); // sums label variants (input 51230 + output 20410)
    expect(metrics.costUsdTotal).toBeCloseTo(0.9125);
    expect(metrics.activeRequests).toBe(3);
  });

  it('defaults missing metrics to zero', () => {
    const metrics = parsePrometheusMetrics('some unrelated text\nfoo 1');
    expect(metrics.requestsTotal).toBe(0);
    expect(metrics.tokensTotal).toBe(0);
    expect(metrics.costUsdTotal).toBe(0);
    expect(metrics.activeRequests).toBe(0);
  });
});
