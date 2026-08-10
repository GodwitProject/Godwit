import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { ModelForm } from './ModelForm';

const mockFetch = vi.fn();
global.fetch = mockFetch;

function wrapper({ children }: { children: React.ReactNode }) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}

describe('ModelForm', () => {
  beforeEach(() => {
    mockFetch.mockReset();
    mockFetch.mockResolvedValue({
      ok: true,
      json: async () => ({
        data: [
          {
            id: '11111111-1111-1111-1111-111111111111',
            name: 'OpenAI',
            protocol: 'openai',
            base_url: null,
            allow_wildcard: false,
            enabled: true,
            has_credentials: true,
            created_at: '',
          },
        ],
      }),
    } as Response);
  });

  it('submits create payload with provider derived from profile', async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn().mockResolvedValue(undefined);
    render(<ModelForm mode="create" onSubmit={onSubmit} />, { wrapper });

    await waitFor(() => expect(screen.getByRole('option', { name: 'OpenAI' })).toBeInTheDocument());

    await user.type(screen.getByLabelText('Public ID'), 'gpt-4o');
    await user.selectOptions(screen.getByLabelText('Provider profile'), '11111111-1111-1111-1111-111111111111');
    await user.type(screen.getByLabelText('Provider model ID'), 'gpt-4o');
    await user.click(screen.getByLabelText('Embedding'));
    await user.click(screen.getByRole('button', { name: /create/i }));

    await waitFor(() => expect(onSubmit).toHaveBeenCalled());
    const payload = onSubmit.mock.calls[0][0];
    expect(payload.public_id).toBe('gpt-4o');
    expect(payload.provider_profile_id).toBe('11111111-1111-1111-1111-111111111111');
    expect(payload.provider_model_id).toBe('gpt-4o');
    expect(payload.provider).toBe('openai');
    expect(payload.capabilities).toContain('embedding');
  });

  it('submits edit payload', async () => {
    const onSubmit = vi.fn().mockResolvedValue(undefined);
    render(
      <ModelForm
        mode="edit"
        defaultValues={{ public_id: 'gpt-4o', capabilities: ['chat'] }}
        onSubmit={onSubmit}
      />,
      { wrapper }
    );

    fireEvent.input(screen.getByLabelText('Public ID'), { target: { value: 'gpt-4o-latest' } });
    fireEvent.click(screen.getByRole('button', { name: /save/i }));

    await waitFor(() => expect(onSubmit).toHaveBeenCalled());
    const payload = onSubmit.mock.calls[0][0];
    expect(payload.public_id).toBe('gpt-4o-latest');
    expect(payload.capabilities).toEqual(['chat']);
  });
});
