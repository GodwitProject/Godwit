import { render, screen, fireEvent } from '@testing-library/react';
import { KeyList, type KeyListProps } from './KeyList';
import type { ApiKey } from '../../lib/keys';

const fixtures: ApiKey[] = [
  {
    id: 'key-1',
    user_id: null,
    team_id: null,
    organization_id: 'org-1',
    name: 'Production Gateway',
    key_prefix: 'sk_live_a1b2',
    scopes: ['read', 'write'],
    allowed_models: ['gpt-4'],
    budget_limit_usd: 100,
    budget_spent_usd: 42.5,
    rate_limit_requests_per_minute: 1000,
    rate_limit_tokens_per_minute: null,
    expires_at: null,
    disabled: false,
    created_at: '2026-01-01T00:00:00Z',
  },
  {
    id: 'key-2',
    user_id: 'user-2',
    team_id: null,
    organization_id: null,
    name: 'Legacy Key',
    key_prefix: 'sk_live_c3d4',
    scopes: ['read'],
    allowed_models: ['claude-3-opus'],
    budget_limit_usd: null,
    budget_spent_usd: 5.25,
    rate_limit_requests_per_minute: null,
    rate_limit_tokens_per_minute: null,
    expires_at: '2026-09-01T00:00:00Z',
    disabled: true,
    created_at: '2026-02-01T00:00:00Z',
  },
];

const noop = () => {};

const baseProps: KeyListProps = {
  keys: fixtures,
  onSelect: noop,
  onToggleActive: noop,
  onDelete: noop,
};

describe('KeyList', () => {
  it('renders key names and scopes badges', () => {
    render(<KeyList {...baseProps} />);

    expect(screen.getByText('Production Gateway')).toBeInTheDocument();
    expect(screen.getByText('Legacy Key')).toBeInTheDocument();

    const readBadges = screen.getAllByText('read');
    expect(readBadges.length).toBeGreaterThan(0);
    expect(readBadges[0]).toHaveClass('text-[oklch(40%_0.14_260)]');
  });

  it('shows prefix in monospace', () => {
    render(<KeyList {...baseProps} />);
    const prefix = screen.getByText('sk_live_a1b2');
    expect(prefix).toHaveClass('font-mono');
  });

  it('shows spent amount', () => {
    render(<KeyList {...baseProps} />);
    expect(screen.getByText('$42.50')).toBeInTheDocument();
  });

  it('renders status toggle checked for active and unchecked for disabled', () => {
    render(<KeyList {...baseProps} />);
    const toggles = screen.getAllByRole('checkbox');
    expect(toggles[0]).toBeChecked();
    expect(toggles[1]).not.toBeChecked();
  });

  it('calls onToggleActive when a toggle is clicked', () => {
    const onToggleActive = vi.fn();
    render(<KeyList {...baseProps} onToggleActive={onToggleActive} />);
    const toggles = screen.getAllByRole('checkbox');
    fireEvent.click(toggles[0]);
    expect(onToggleActive).toHaveBeenCalledWith(fixtures[0]);
  });

  it('calls onSelect when a row is clicked', () => {
    const onSelect = vi.fn();
    render(<KeyList {...baseProps} onSelect={onSelect} />);
    fireEvent.click(screen.getByText('Production Gateway'));
    expect(onSelect).toHaveBeenCalledWith(fixtures[0]);
  });

  it('opens actions menu and triggers delete', () => {
    const onDelete = vi.fn();
    render(<KeyList {...baseProps} onDelete={onDelete} />);

    fireEvent.click(screen.getByLabelText('Actions for Production Gateway'));
    fireEvent.click(screen.getByText('Delete'));

    expect(onDelete).toHaveBeenCalledWith(fixtures[0]);
  });

  it('renders empty state when there are no keys', () => {
    render(<KeyList {...baseProps} keys={[]} />);
    expect(screen.getByText('No API keys created yet.')).toBeInTheDocument();
  });
});
