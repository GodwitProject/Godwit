import { render, screen } from '@testing-library/react';
import { ModelsTable, type ModelsTableProps } from './ModelsTable';

const baseProps: ModelsTableProps = {
  models: [
    {
      id: 'm1',
      public_id: 'gpt-4o',
      provider: 'openai',
      provider_model_id: 'gpt-4o-2024-11-20',
      capabilities: ['chat'],
      pricing: null,
      created_at: '2026-01-01T00:00:00Z',
    },
    {
      id: 'm2',
      public_id: 'claude-sonnet-4',
      provider: 'anthropic',
      provider_model_id: 'claude-sonnet-4-20250514',
      capabilities: ['chat'],
      pricing: null,
      created_at: '2026-01-01T00:00:00Z',
    },
  ],
  latencyByModel: new Map([['gpt-4o', 944]]),
  protocolEnabled: new Set(['openai']),
};

describe('ModelsTable', () => {
  it('renders exposed model, provider, provider-side id and latency', () => {
    render(<ModelsTable {...baseProps} />);

    expect(screen.getByText('gpt-4o')).toBeInTheDocument();
    expect(screen.getByText('gpt-4o-2024-11-20')).toBeInTheDocument();
    expect(screen.getByText('claude-sonnet-4')).toBeInTheDocument();
    expect(screen.getByText('944 ms')).toBeInTheDocument();
  });

  it('marks a model as success when its provider is enabled and disabled otherwise', () => {
    render(<ModelsTable {...baseProps} />);

    const rows = screen.getAllByRole('row');
    expect(rows[1].textContent).toContain('Success');
    expect(rows[2].textContent).toContain('Disabled');
  });
});
