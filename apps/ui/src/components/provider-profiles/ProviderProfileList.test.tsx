import { render, screen, fireEvent, waitFor, within } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { BrowserRouter } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { ProviderProfileList } from './ProviderProfileList';

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

describe('ProviderProfileList', () => {
  beforeEach(() => {
    mockFetch.mockReset();
  });

  it('renders profiles and delete confirmation', async () => {
    mockFetch
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          data: [
            {
              id: '1',
              name: 'openai',
              protocol: 'openai',
              base_url: null,
              allow_wildcard: false,
              enabled: true,
              has_credentials: true,
              created_at: '',
            },
          ],
        }),
      } as Response)
      .mockResolvedValueOnce({ ok: true, json: async () => ({ deleted: true }) } as Response)
      .mockResolvedValueOnce({ ok: true, json: async () => ({ data: [] }) } as Response);

    render(<ProviderProfileList />, { wrapper });
    await waitFor(() => expect(screen.getByText('openai', { selector: 'td:first-child' })).toBeInTheDocument());
    fireEvent.click(screen.getByRole('button', { name: /delete/i }));
    expect(screen.getByText(/delete provider profile/i)).toBeInTheDocument();

    fireEvent.click(within(screen.getByRole('dialog')).getByRole('button', { name: /delete$/i }));
  });

  it('shows an error when deletion fails', async () => {
    mockFetch
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          data: [
            {
              id: '1',
              name: 'openai',
              protocol: 'openai',
              base_url: null,
              allow_wildcard: false,
              enabled: true,
              has_credentials: true,
              created_at: '',
            },
          ],
        }),
      } as Response)
      .mockResolvedValueOnce({
        ok: false,
        status: 409,
        json: async () => ({ message: 'Profile is still referenced by models' }),
      } as Response);

    render(<ProviderProfileList />, { wrapper });
    await waitFor(() => expect(screen.getByText('openai', { selector: 'td:first-child' })).toBeInTheDocument());

    fireEvent.click(screen.getByRole('button', { name: /delete/i }));
    fireEvent.click(within(screen.getByRole('dialog')).getByRole('button', { name: /delete$/i }));

    await waitFor(() =>
      expect(screen.getByText('Profile is still referenced by models')).toBeInTheDocument()
    );
  });
});
