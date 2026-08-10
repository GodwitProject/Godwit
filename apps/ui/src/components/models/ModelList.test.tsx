import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { BrowserRouter } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { ModelList } from './ModelList';

const mockFetch = vi.fn();
global.fetch = mockFetch;

function wrapper({ children }: { children: React.ReactNode }) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return (
    <QueryClientProvider client={client}>
      <BrowserRouter>{children}</BrowserRouter>
    </QueryClientProvider>
  );
}

const model = {
  id: '1',
  public_id: 'gpt-4o',
  provider: 'openai',
  provider_profile_id: 'profile-1',
  provider_model_id: 'openai-gpt-4o',
  capabilities: ['chat'],
  pricing: { input_price_per_million: 5, output_price_per_million: 15 },
  config: {},
  created_at: '2024-01-01T00:00:00Z',
};

describe('ModelList', () => {
  beforeEach(() => {
    mockFetch.mockReset();
  });

  it('renders models and delete confirmation', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ data: [model] }),
    } as Response);

    render(<ModelList />, { wrapper });
    await waitFor(() => expect(screen.getByText('openai-gpt-4o')).toBeInTheDocument());

    fireEvent.click(screen.getByRole('button', { name: /delete/i }));
    expect(screen.getByText(/delete model/i)).toBeInTheDocument();
    expect(screen.getByText(/Are you sure you want to delete "gpt-4o"\?/i)).toBeInTheDocument();
  });

  it('opens edit modal', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ data: [model] }),
    } as Response);

    render(<ModelList />, { wrapper });
    await waitFor(() => expect(screen.getByText('openai-gpt-4o')).toBeInTheDocument());

    fireEvent.click(screen.getByRole('button', { name: /edit/i }));
    expect(screen.getByText('Edit model')).toBeInTheDocument();
    expect(screen.getByDisplayValue('gpt-4o')).toBeInTheDocument();
  });
});
