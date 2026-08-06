import { render, screen, fireEvent } from '@testing-library/react';
import { KeyList, type KeyListProps } from './KeyList';
import type { ApiKey } from '../../lib/keys';

const fixtures: ApiKey[] = [
  {
    id: 'key-1',
    name: 'Production Gateway',
    prefix: 'sk_live_a1b2',
    owner: 'Platform Team',
    scopes: ['read', 'write'],
    allowedModels: ['gpt-4'],
    budget: 100,
    rateLimitRpm: 1000,
    rateLimitTpm: null,
    expiresAt: null,
    spend30d: 42.5,
    requests24h: 1200,
    lastUsedAt: '2026-08-05T10:00:00Z',
    status: 'active',
    createdAt: '2026-01-01T00:00:00Z',
  },
  {
    id: 'key-2',
    name: 'Legacy Key',
    prefix: 'sk_live_c3d4',
    owner: 'Growth',
    scopes: ['read'],
    allowedModels: ['claude-3-opus'],
    budget: null,
    rateLimitRpm: null,
    rateLimitTpm: null,
    expiresAt: '2026-09-01T00:00:00Z',
    spend30d: 5.25,
    requests24h: 8,
    lastUsedAt: null,
    status: 'revoked',
    createdAt: '2026-02-01T00:00:00Z',
  },
];

const noop = () => {};

const baseProps: KeyListProps = {
  keys: fixtures,
  onSelect: noop,
  onEdit: noop,
  onRevoke: noop,
  onDelete: noop,
};

describe('KeyList', () => {
  it('renders key names and scopes badges', () => {
    render(<KeyList {...baseProps} />);

    expect(screen.getByText('Production Gateway')).toBeInTheDocument();
    expect(screen.getByText('Legacy Key')).toBeInTheDocument();
    expect(screen.getByText('Platform Team')).toBeInTheDocument();

    const readBadges = screen.getAllByText('read');
    expect(readBadges.length).toBeGreaterThan(0);
    expect(readBadges[0]).toHaveClass('bg-info/10');
  });

  it('shows prefix in monospace', () => {
    render(<KeyList {...baseProps} />);
    const prefix = screen.getByText('sk_live_a1b2');
    expect(prefix).toHaveClass('font-mono');
  });

  it('shows spend and requests', () => {
    render(<KeyList {...baseProps} />);
    expect(screen.getByText('$42.50')).toBeInTheDocument();
    expect(screen.getByText('1200')).toBeInTheDocument();
  });

  it('renders status toggle checked for active and unchecked for revoked', () => {
    render(<KeyList {...baseProps} />);
    const toggles = screen.getAllByRole('checkbox');
    expect(toggles[0]).toBeChecked();
    expect(toggles[1]).not.toBeChecked();
  });

  it('calls onSelect when a row is clicked', () => {
    const onSelect = vi.fn();
    render(<KeyList {...baseProps} onSelect={onSelect} />);
    fireEvent.click(screen.getByText('Production Gateway'));
    expect(onSelect).toHaveBeenCalledWith(fixtures[0]);
  });

  it('opens actions menu and triggers revoke', () => {
    const onRevoke = vi.fn();
    render(<KeyList {...baseProps} onRevoke={onRevoke} />);

    fireEvent.click(screen.getByLabelText('Actions for Production Gateway'));
    fireEvent.click(screen.getByText('Revoke'));

    expect(onRevoke).toHaveBeenCalledWith(fixtures[0]);
  });

  it('renders empty state when there are no keys', () => {
    render(<KeyList {...baseProps} keys={[]} />);
    expect(screen.getByText('No API keys created yet.')).toBeInTheDocument();
  });
});
