import { logsToCsv } from './exportCsv';
import type { RequestLog } from './logs';

const base: RequestLog = {
  id: 'log-1',
  api_key_id: null,
  model: 'gpt-4o',
  provider: 'openai',
  capability: 'chat',
  tokens_in: 100,
  tokens_out: 50,
  duration_ms: 812,
  streamed: false,
  cost_usd: 0.0124,
  status: 'success',
  created_at: '2026-08-06T10:00:00Z',
};

describe('logsToCsv', () => {
  it('writes a header row with the expected columns', () => {
    const csv = logsToCsv([]);
    const [header] = csv.split('\n');
    expect(header).toBe(
      'id,api_key_id,model,provider,capability,tokens_in,tokens_out,duration_ms,cost_usd,status,created_at'
    );
  });

  it('escapes commas and quotes in cell values', () => {
    const log: RequestLog = {
      ...base,
      model: 'model, "quoted"',
      status: 'some"thing',
    };
    const csv = logsToCsv([log]);
    const row = csv.split('\n')[1];
    expect(row).toContain('"model, ""quoted"""');
    expect(row).toContain('"some""thing"');
  });

  it('renders null and numeric values correctly', () => {
    const csv = logsToCsv([base]);
    const row = csv.split('\n')[1];
    expect(row).toContain(',100,50,812,0.0124,success,');
  });
});
