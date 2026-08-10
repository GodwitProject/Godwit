import { describe, it, expect, vi, beforeEach } from 'vitest';
import { createElement } from 'react';
import { renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import type { ReactNode } from 'react';
import { useModels, useCreateModel } from './useModels';

const mockFetch = vi.fn();
global.fetch = mockFetch;

function wrapper({ children }: { children: ReactNode }) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return createElement(QueryClientProvider, { client }, children);
}

describe('useModels', () => {
  beforeEach(() => {
    mockFetch.mockReset();
  });

  it('fetches models', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ data: [] }),
    } as Response);

    const { result } = renderHook(() => useModels(), { wrapper });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data).toEqual([]);
  });

  it('creates a model and invalidates list', async () => {
    mockFetch
      .mockResolvedValueOnce({ ok: true, json: async () => ({ data: [] }) } as Response)
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          id: '1',
          public_id: 'gpt-4',
          provider_profile_id: 'profile-1',
          provider_model_id: 'gpt-4',
          provider: 'openai',
          capabilities: ['chat'],
          pricing: { input_price_per_million: 1, output_price_per_million: 2 },
        }),
      } as Response)
      .mockResolvedValueOnce({ ok: true, json: async () => ({ data: [{ id: '1' }] }) } as Response);

    const { result } = renderHook(() => useCreateModel(), { wrapper });
    result.current.mutate({
      public_id: 'gpt-4',
      provider_profile_id: 'profile-1',
      provider_model_id: 'gpt-4',
      provider: 'openai',
      capabilities: ['chat'],
      input_price_per_million: 1,
      output_price_per_million: 2,
    });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
  });
});
