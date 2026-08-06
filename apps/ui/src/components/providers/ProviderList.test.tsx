import { render, screen } from '@testing-library/react';
import { ProviderList, type ProviderListProps } from './ProviderList';

const fixtures: ProviderListProps['providers'] = [
  {
    id: 'provider-openai',
    name: 'OpenAI',
    protocol: 'openai',
    base_url: 'https://api.openai.com/v1',
    allow_wildcard: true,
    enabled: true,
    has_credentials: true,
    created_at: '2026-01-01T00:00:00Z',
  },
  {
    id: 'provider-anthropic',
    name: 'Anthropic',
    protocol: 'anthropic',
    base_url: 'https://api.anthropic.com',
    allow_wildcard: false,
    enabled: false,
    has_credentials: false,
    created_at: '2026-01-02T00:00:00Z',
  },
];

describe('ProviderList', () => {
  it('renders provider names, protocol, credentials and status badges', () => {
    render(<ProviderList providers={fixtures} />);

    expect(screen.getByText('OpenAI')).toBeInTheDocument();
    expect(screen.getByText('Anthropic')).toBeInTheDocument();

    expect(screen.getByText('https://api.openai.com/v1')).toBeInTheDocument();

    expect(screen.getByText('Configured')).toHaveClass('bg-success/10');
    expect(screen.getByText('Missing')).toHaveClass('bg-warning/10');
    expect(screen.getByText('Enabled')).toHaveClass('bg-success/10');
    expect(screen.getByText('Disabled')).toBeInTheDocument();
  });

  it('renders empty state when there are no providers', () => {
    render(<ProviderList providers={[]} />);
    expect(screen.queryByRole('table')).not.toBeInTheDocument();
    expect(screen.getByText('No providers configured yet.')).toBeInTheDocument();
  });
});
