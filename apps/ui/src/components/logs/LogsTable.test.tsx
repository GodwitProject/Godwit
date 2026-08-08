import { render, screen, fireEvent, within } from '@testing-library/react';
import { LogsTable, type LogsTableProps } from './LogsTable';
import type { RequestLog } from '../../lib/logs';

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
  {
    id: 'log-2',
    api_key_id: 'key-2',
    model: 'claude-3-opus',
    provider: 'anthropic',
    capability: 'chat',
    tokens_in: 200,
    tokens_out: null,
    duration_ms: null,
    streamed: true,
    cost_usd: 0.03,
    status: 'error',
    created_at: '2026-08-06T09:30:00Z',
  },
  {
    id: 'log-3',
    api_key_id: 'key-1',
    model: 'gpt-3.5-turbo',
    provider: 'openai',
    capability: 'chat',
    tokens_in: null,
    tokens_out: 12,
    duration_ms: 3100,
    streamed: false,
    cost_usd: 0.0002,
    status: 'rate_limited',
    created_at: '2026-08-06T09:00:00Z',
  },
];

const noop = () => {};

const baseProps: LogsTableProps = {
  logs: fixtures,
  onSelect: noop,
  hasMore: false,
  onLoadMore: noop,
  loadingMore: false,
};

describe('LogsTable', () => {
  it('renders log ids, models, providers and costs', () => {
    render(<LogsTable {...baseProps} />);
    expect(screen.getByText('log-1')).toBeInTheDocument();
    expect(screen.getByText('gpt-4')).toBeInTheDocument();
    expect(screen.getByText('anthropic')).toBeInTheDocument();
    expect(screen.getByText('$0.0124')).toBeInTheDocument();
    expect(screen.getByText('812ms')).toBeInTheDocument();
  });

  it('renders an em dash for missing latency', () => {
    render(<LogsTable {...baseProps} />);
    expect(screen.getAllByText('—').length).toBeGreaterThan(0);
  });

  it('opens detail when a log id is clicked', () => {
    const onSelect = vi.fn();
    render(<LogsTable {...baseProps} onSelect={onSelect} />);
    fireEvent.click(screen.getByText('log-1'));
    expect(onSelect).toHaveBeenCalledWith(fixtures[0]);
  });

  it('sorts the accumulated loaded set when the timestamp header is clicked', () => {
    render(<LogsTable {...baseProps} />);
    const header = screen.getByText((content) => content.trim().startsWith('Request'));
    fireEvent.click(header);
    const rows = screen.getAllByRole('row');
    const bodyRows = rows.slice(1);
    const first = within(bodyRows[0] as HTMLElement).getByText(/^log-/);
    expect(first.textContent).toBe('log-3');
  });

  it('does not show a load-more button when hasMore is false', () => {
    render(<LogsTable {...baseProps} />);
    expect(screen.queryByText('Load more')).not.toBeInTheDocument();
  });

  it('shows a load-more button and calls onLoadMore when there are more pages', () => {
    const onLoadMore = vi.fn();
    render(<LogsTable {...baseProps} hasMore onLoadMore={onLoadMore} />);
    fireEvent.click(screen.getByText('Load more'));
    expect(onLoadMore).toHaveBeenCalledTimes(1);
  });

  it('renders empty state when there are no logs', () => {
    render(<LogsTable {...baseProps} logs={[]} />);
    expect(screen.getByText('No logs found.')).toBeInTheDocument();
  });
});
