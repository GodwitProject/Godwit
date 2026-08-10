import { describe, it, expect, vi, beforeEach } from 'vitest';
import { createElement } from 'react';
import { renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import type { ReactNode } from 'react';
import { useProviderProfiles, useCreateProviderProfile } from './useProviderProfiles';

const mockFetch = vi.fn();
global.fetch = mockFetch;

function wrapper({ children }: { children: ReactNode }) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return createElement(QueryClientProvider, { client }, children);
}

describe('useProviderProfiles', () => {
  beforeEach(() => {
    mockFetch.mockReset();
  });

  it('fetches profiles', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ data: [] }),
    } as Response);

    const { result } = renderHook(() => useProviderProfiles(), { wrapper });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data).toEqual([]);
  });

  it('creates a profile and invalidates list', async () => {
    mockFetch
      .mockResolvedValueOnce({ ok: true, json: async () => ({ data: [] }) } as Response)
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({ id: '1', name: 'openai', protocol: 'openai' }),
      } as Response)
      .mockResolvedValueOnce({ ok: true, json: async () => ({ data: [{ id: '1' }] }) } as Response);

    const { result } = renderHook(() => useCreateProviderProfile(), { wrapper });
    result.current.mutate({
      name: 'openai',
      protocol: 'openai',
      base_url: '',
      allow_wildcard: false,
    });
    await waitFor(() => expect(result.current.isSuccess).toBe(true));
  });
});
