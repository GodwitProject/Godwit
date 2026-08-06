import { render, screen, fireEvent, within } from '@testing-library/react';
import { LogsTable, type LogsTableProps } from './LogsTable';
import type { RequestLog } from '../../lib/logs';

const fixtures: RequestLog[] = [
  {
    id: 'log-1',
    timestamp: '2026-08-06T10:00:00Z',
    requestId: 'req_a1b2c3',
    model: 'gpt-4',
    provider: 'openai',
    status: 200,
    tokensIn: 120,
    tokensOut: 45,
    cost: 0.0124,
    latencyMs: 812,
    apiKeyPrefix: 'sk_live_a9f2',
    requestBody: { model: 'gpt-4', messages: [] },
    responseBody: { choices: [] },
    finishReason: 'stop',
    piiDetected: false,
    moderationStatus: 'allowed',
    fallbackUsed: false,
    timeline: [{ time: '10:00:00.000', event: 'received' }],
  },
  {
    id: 'log-2',
    timestamp: '2026-08-06T09:30:00Z',
    requestId: 'req_d4e5f6',
    model: 'claude-3-opus',
    provider: 'anthropic',
    status: 429,
    tokensIn: 0,
    tokensOut: 0,
    cost: 0,
    latencyMs: 45,
    apiKeyPrefix: 'sk_live_b2c3',
    requestBody: {},
    responseBody: {},
    finishReason: null,
    piiDetected: true,
    moderationStatus: 'blocked',
    fallbackUsed: true,
    timeline: [{ time: '09:30:00.000', event: 'rate_limited' }],
  },
  {
    id: 'log-3',
    timestamp: '2026-08-06T09:00:00Z',
    requestId: 'req_g7h8i9',
    model: 'gpt-3.5-turbo',
    provider: 'openai',
    status: 500,
    tokensIn: 10,
    tokensOut: 0,
    cost: 0.0002,
    latencyMs: 3100,
    apiKeyPrefix: 'sk_live_c4d5',
    requestBody: {},
    responseBody: {},
    finishReason: 'error',
    piiDetected: false,
    moderationStatus: 'not_checked',
    fallbackUsed: false,
    timeline: [{ time: '09:00:00.000', event: 'upstream_error' }],
  },
];

const noop = () => {};

const baseProps: LogsTableProps = {
  logs: fixtures,
  onSelect: noop,
  total: fixtures.length,
  page: 1,
  pageSize: 50,
  onPageChange: noop,
};

describe('LogsTable', () => {
  it('renders request ids, models, providers, statuses and tokens', () => {
    render(<LogsTable {...baseProps} />);
    expect(screen.getByText('req_a1b2c3')).toBeInTheDocument();
    expect(screen.getByText('gpt-4')).toBeInTheDocument();
    expect(screen.getByText('anthropic')).toBeInTheDocument();
    expect(screen.getByText('$0.0124')).toBeInTheDocument();
    expect(screen.getByText('812ms')).toBeInTheDocument();
  });

  it('applies status badge variants for success, warning and error', () => {
    render(<LogsTable {...baseProps} />);
    expect(screen.getByText('200 OK')).toHaveClass('bg-success/10');
    expect(screen.getByText('429 Ratelimit')).toHaveClass('bg-warning/10');
    expect(screen.getByText('500 Error')).toHaveClass('bg-error/10');
  });

  it('renders request ids in monospace', () => {
    render(<LogsTable {...baseProps} />);
    expect(screen.getByText('req_a1b2c3')).toHaveClass('font-mono');
  });

  it('opens detail when a request id is clicked', () => {
    const onSelect = vi.fn();
    render(<LogsTable {...baseProps} onSelect={onSelect} />);
    fireEvent.click(screen.getByText('req_a1b2c3'));
    expect(onSelect).toHaveBeenCalledWith(fixtures[0]);
  });

  it('sorts by timestamp when the timestamp header is clicked', () => {
    render(<LogsTable {...baseProps} />);
    const header = screen.getByText((content) => content.trim().startsWith('Timestamp'));
    fireEvent.click(header);
    const rows = screen.getAllByRole('row');
    const bodyRows = rows.slice(1);
    const first = within(bodyRows[0] as HTMLElement).getByText(/req_/);
    expect(first.textContent).toBe('req_g7h8i9');
  });

  it('renders pagination controls', () => {
    const onPageChange = vi.fn();
    render(<LogsTable {...baseProps} page={1} pageSize={2} total={10} onPageChange={onPageChange} />);
    fireEvent.click(screen.getByText('Next'));
    expect(onPageChange).toHaveBeenCalledWith(2);
  });

  it('renders empty state when there are no logs', () => {
    render(<LogsTable {...baseProps} logs={[]} />);
    expect(screen.getByText('No logs found.')).toBeInTheDocument();
  });
});
