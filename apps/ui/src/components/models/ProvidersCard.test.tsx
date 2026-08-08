import { render, screen, fireEvent } from '@testing-library/react';
import { ProvidersCard, type ProvidersCardProps } from './ProvidersCard';

const fixtures: ProvidersCardProps['providers'] = [
  {
    id: 'p-openai',
    name: 'OpenAI',
    protocol: 'openai',
    base_url: 'https://api.openai.com/v1',
    allow_wildcard: true,
    enabled: true,
    has_credentials: true,
    created_at: '2026-01-01T00:00:00Z',
  },
  {
    id: 'p-anthropic',
    name: 'Anthropic',
    protocol: 'anthropic',
    base_url: 'https://api.anthropic.com',
    allow_wildcard: false,
    enabled: false,
    has_credentials: false,
    created_at: '2026-01-02T00:00:00Z',
  },
];

describe('ProvidersCard', () => {
  it('renders provider names, credentials status and the active count', () => {
    render(<ProvidersCard providers={fixtures} onToggle={() => {}} />);

    expect(screen.getByText('OpenAI')).toBeInTheDocument();
    expect(screen.getByText('Anthropic')).toBeInTheDocument();
    expect(screen.getByText('Configured')).toBeInTheDocument();
    expect(screen.getByText('Missing')).toBeInTheDocument();
    expect(screen.getByText('active of 1/2')).toBeInTheDocument();
  });

  it('calls onToggle with the new enabled state when a provider switch is toggled', () => {
    const onToggle = vi.fn();
    render(<ProvidersCard providers={fixtures} onToggle={onToggle} />);

    const switches = screen.getAllByRole('checkbox');
    expect(switches).toHaveLength(2);
    expect(switches[0]).toBeChecked();

    fireEvent.click(switches[1]);
    expect(onToggle).toHaveBeenCalledWith('p-anthropic', true);
  });
});
