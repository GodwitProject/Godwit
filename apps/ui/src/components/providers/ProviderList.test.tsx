import { render, screen, fireEvent } from '@testing-library/react';
import { ProviderList, type ProviderListProps } from './ProviderList';

const fixtures: ProviderListProps['providers'] = [
  {
    id: 'provider-openai',
    name: 'OpenAI',
    status: 'healthy',
    modelCount: 23,
    latencyP95: 342,
    errorRate: 0.0004,
    baseUrl: 'https://api.openai.com/v1',
    apiKey: 'sk-abc123xyz',
    timeoutMs: 30000,
    enabledModels: ['gpt-4', 'gpt-4-turbo', 'gpt-3.5-turbo'],
    fallbackChain: ['anthropic/claude-3-opus'],
    fallbackTriggered: 12,
  },
  {
    id: 'provider-anthropic',
    name: 'Anthropic',
    status: 'down',
    modelCount: 5,
    latencyP95: 0,
    errorRate: 1,
    baseUrl: 'https://api.anthropic.com',
    apiKey: 'sk-ant-123xyz',
    timeoutMs: 60000,
    enabledModels: ['claude-3-opus'],
    fallbackChain: [],
    fallbackTriggered: 0,
  },
];

describe('ProviderList', () => {
  it('renders provider names and status badges with correct variant classes', () => {
    render(<ProviderList providers={fixtures} />);

    expect(screen.getByText('OpenAI')).toBeInTheDocument();
    expect(screen.getByText('Anthropic')).toBeInTheDocument();

    const healthyBadge = screen.getByText('Healthy');
    const downBadge = screen.getByText('Down');

    expect(healthyBadge).toHaveClass('bg-success/10');
    expect(healthyBadge).toHaveClass('text-success');
    expect(downBadge).toHaveClass('bg-error/10');
    expect(downBadge).toHaveClass('text-error');
  });

  it('shows model counts and latency in monospace', () => {
    render(<ProviderList providers={fixtures} />);

    expect(screen.getByText('23')).toBeInTheDocument();
    expect(screen.getByText('5')).toBeInTheDocument();
    expect(screen.getByText('342ms')).toHaveClass('font-mono');
  });

  it('expands a row to reveal detail sections when clicked', () => {
    render(<ProviderList providers={fixtures} />);

    expect(screen.queryByText('https://api.openai.com/v1')).not.toBeInTheDocument();

    fireEvent.click(screen.getByText('OpenAI'));

    expect(screen.getByText('https://api.openai.com/v1')).toBeInTheDocument();
    expect(screen.getByText('sk-****-xyz')).toBeInTheDocument();
    expect(screen.getByText('30000ms')).toBeInTheDocument();
    expect(screen.getByText('anthropic/claude-3-opus')).toBeInTheDocument();
    expect(screen.getByText('Fallback triggered 12 times')).toBeInTheDocument();
  });

  it('renders nothing when there are no providers', () => {
    render(<ProviderList providers={[]} />);
    expect(screen.queryByRole('table')).not.toBeInTheDocument();
  });
});
