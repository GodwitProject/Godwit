import { render, screen } from '@testing-library/react';
import { RecentLogsTable } from './RecentLogsTable';
import type { RequestLog } from '@/lib/logs';

const fixtures: RequestLog[] = [
  {
    id: 'log-1',
    api_key_id: 'key-1',
    model: 'gpt-4',
    provider: 'openai',
    capability: 'chat',
    tokens_in: 100,
    tokens_out: 50,
    duration_ms: 812,
    streamed: false,
    cost_usd: 0.0124,
    status: 'success',
    created_at: '2026-08-06T10:00:00Z',
  },
];

describe('RecentLogsTable', () => {
  it('renders recent log rows', () => {
    render(<RecentLogsTable logs={fixtures} />);
    expect(screen.getByText('log-1')).toBeInTheDocument();
    expect(screen.getByText('gpt-4')).toBeInTheDocument();
  });

  it('renders an empty state when there are no logs', () => {
    render(<RecentLogsTable logs={[]} />);
    expect(screen.getByText('No live metric data yet')).toBeInTheDocument();
    expect(screen.queryByRole('table')).not.toBeInTheDocument();
  });
});
